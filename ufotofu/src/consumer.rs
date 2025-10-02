//! Consumers — values that asynchronously process a sequence of items.
//!
//! A [`Consumer`] processed items of type [`Consumer::Item`], fed to it with the [`Consumer::consume`] method. After the calling code has moved all items into the consumer, it calls [`Consumer::close`] with a final value of type [`Consumer::Final`] to signal to the consumer that no more items will follow.
//!
//! ```
//! use ufotofu::prelude::*;
//! # pollster::block_on(async{
//! let mut my_first_consumer = vec![].into_consumer();
//!
//! my_first_consumer.consume(1).await?;
//! my_first_consumer.consume(2).await?;
//! my_first_consumer.consume(4).await?;
//!
//! let vec: Vec<_> = my_first_consumer.into();
//! assert_eq!(vec, vec![1, 2, 4]);
//! # Result::<(), Infallible>::Ok(())
//! # });
//! ```
//!
//! Consumers may emit errors of type [`Consumer::Error`] to indicate failure to process a regular item or the final value. It is forbidden to call `consume` or `close` after having called `close`, or after the consumer has emitted an error. Any such call may result in unspecified (but safe) behaviour.
//!
//! <br/>
//!
//! The [`IntoConsumer`] trait describes types which can be converted into consumers. This trait is implemented for the collection types of the standard library. It is also implemented on *mutabel references* to such collections, allowing you to consume into a collection without taking ownership of it:
//!
//! ```
//! use ufotofu::prelude::*;
//! # pollster::block_on(async{
//! let mut v = vec![];
//! let mut c = (&mut v).into_consumer();
//!
//! c.consume(1).await?;
//! c.consume(2).await?;
//! c.consume(4).await?;
//!
//! assert_eq!(v, vec![1, 2, 4]);
//! # Result::<(), Infallible>::Ok(())
//! # });
//! ```
//!
//! <br/>
//!
//! Every consumer automatically implements the [`ConsumerExt`] trait, which provides useful methods for working with consumers.
//!
//! <br/>
//!
//! Consuming a sequence one item at a time can be inefficient. The [`BulkConsumer`] trait extends [`Consumer`] with the ability to consume multiple items at a time. The design is fully analogous to [`std::io::Write`] — the [`BulkConsumer::bulk_consume`] method takes a `&[Self::Item]` as its argument, and returns how many items it read from that buffer. Crucial differences to [`Write::write`](std::io::Write::write) are:
//!
//! - `bulk_consume` is asynchronous;
//! - `bulk_consume` works with arbitrary `Producer::Item: Clone` and `Producer::Error` types, not just `u8` and `io::Error`; and
//! - `bulk_produce` must not be called with an empty buffer, and it must read at least one item (unless returning an error).
//!
//! ```
//! use ufotofu::prelude::*;
//! # pollster::block_on(async{
//! let mut arr = [0, 0, 0];
//! let mut c = (&mut arr).into_consumer();
//!
//! assert_eq!(c.bulk_consume(&[1, 2]).await?, 2);
//! assert_eq!(c.bulk_consume(&[4, 8]).await?, 1);
//!
//! assert_eq!(arr, [1, 2, 4]);
//! # Result::<(), ()>::Ok(())
//! # });
//! ```
//!
//! <br/>
//!
//! Every bulk consumer automatically implements the [`BulkConsumerExt`] trait, which provides bulk-consumption-based variants of several methods of [`ConsumerExt`]. These bulk versions are typically more efficient and should be preferred whenever possible.
//!
//! <br/>
//!
//! Finally, the [`BufferedConsumer`] trait describes consumers which can delay performing side-effects in order to make `consume` and `bulk_consume` calls run more efficiently. The classic example is a consumer of bytes which writes to a file on disk: it most certainly should not write every individual byte to disk, instead it should buffer bytes in memory and occasionally flush the buffer to disk. While any consumer can perform such an optimisation and flush during `consume` and `close` calls, the [`BufferedConsumer`] trait additionally provides the [`flush`](BufferedConsumer::flush) method, by which calling code can instruct the consumer to immediately perform the observable side-effects for all currently buffered data.
//!
//! See [`ConsumerExt::buffered`] for a generic way of how ufotofu can add buffering to arbitrary consumers.
//!
//! <br/>The counterpart to the [`consumer`] module is the [`producer`] module.

use crate::prelude::*;

mod consumer_ext;
pub use consumer_ext::*;

mod move_into_slice;
pub use move_into_slice::*;

mod full;
pub use full::*;

pub mod compat;

/// A [`Consumer`] lazily processes a sequence of items.
///
/// The sequence consists of an arbitrary number of items of type [`Consumer::Item`] — moved into the consumer via [`Consumer::consume`] — and is optionally terminated by a value of type [`Consumer::Final`] — moved into the consumer via [`Consumer::close`]. At any point, a consumer may report an error of type [`Consumer::Error`].
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut v = vec![];
/// let mut c = (&mut v).into_consumer();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// c.consume(4).await?;
///
/// assert_eq!(v, vec![1, 2, 4]);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// Calling code must uphold the following invariants:
///
/// - Do not call trait methods after calling `close`.
/// - Do not call trait methods after any method has yielded an error.
/// - Do not use the consumer after dropping any `Future` returned by any of its methods, unless the dropped future has been polled to completion.
/// - Do not use the consumer after catching an unwinding panic.
///
/// <br/>Counterpart: the [`Producer`] trait.
#[must_use = "consumers are lazy and do nothing unless fed with values and/or closed"]
pub trait Consumer {
    /// The sequence processed by this consumer starts with *arbitrarily many* values of this type.
    type Item;
    /// The sequence processed by this consumer ends with *up to one* value of this type.
    type Final;
    /// The type of errors the consumer can emit instead of doing its job.
    type Error;

    /// Attempts to process the next item. The method may fail, returning an `Err` instead.
    ///
    /// After this method returns an error, no further methods of this trait may be invoked.
    ///
    /// # Invariants
    ///
    /// Must not be called after this consumer has been closed, or after any method of this trait has returned an error.
    ///
    /// <br/>Counterpart: the [`Producer::produce`] method (when returning a regular item).
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error>;

    /// Attempts to process the final value. The method may fail, returning an `Err` instead.
    ///
    /// After this method was called, no further methods of this trait may be invoked.
    ///
    /// # Invariants
    ///
    /// Must not be called if this consumer has been closed already, or after any method of this trait has returned an error.
    ///
    /// <br/>Counterpart: the [`Producer::produce`] method (when returning a final value).
    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error>;
}

impl<C: Consumer> Consumer for &mut C {
    type Item = C::Item;
    type Final = C::Final;
    type Error = C::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        (*self).consume(item).await
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        (*self).close(fin).await
    }
}

#[cfg(feature = "alloc")]
impl<C: Consumer> Consumer for alloc::boxed::Box<C> {
    type Item = C::Item;
    type Final = C::Final;
    type Error = C::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.as_mut().consume(item).await
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        self.as_mut().close(fin).await
    }
}

impl Consumer for Infallible {
    type Item = Infallible;
    type Final = Infallible;
    type Error = Infallible;

    async fn consume(&mut self, _item: Self::Item) -> Result<(), Self::Error> {
        unreachable!()
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!()
    }
}

/// Conversion into a [`Consumer`].
///
/// By implementing `IntoConsumer` for a type, you define how it will be
/// converted to a consumer. This is common for types which describe a
/// collection of some kind.
///
/// <br/>Counterpart: the [`IntoProducer`] trait.
pub trait IntoConsumer {
    /// The type of repeated items being consumed.
    type Item;

    /// The type of the final value being consumed.
    type Final;

    /// The type of errors the consumer may emit.
    type Error;

    type IntoConsumer: Consumer<Item = Self::Item, Final = Self::Final, Error = Self::Error>;

    /// Creates a consumer from a value.
    fn into_consumer(self) -> Self::IntoConsumer;
}

impl<C: Consumer> IntoConsumer for C {
    type Item = C::Item;
    type Final = C::Final;
    type Error = C::Error;
    type IntoConsumer = C;

    #[inline]
    fn into_consumer(self) -> C {
        self
    }
}

impl IntoConsumer for () {
    type Item = Infallible;
    type Final = ();
    type Error = Infallible;
    type IntoConsumer = Full<()>;

    #[inline]
    fn into_consumer(self) -> Self::IntoConsumer {
        full()
    }
}

/// A [`BulkConsumer`] is a consumer that can receive multiple items with a single method call, by cloning them out of the buffer argument of the [`BulkConsumer::bulk_consume`] method and returning how many items were read. Semantically, there should be no difference between bulk consumption or item-by-item consumption.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut arr = [0, 0, 0];
/// let mut c = (&mut arr).into_consumer();
///
/// assert_eq!(c.bulk_consume(&[1, 2]).await?, 2);
/// assert_eq!(c.bulk_consume(&[4, 8]).await?, 1);
///
/// assert_eq!(arr, [1, 2, 4]);
/// # Result::<(), ()>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [`BulkProducer`] trait.
pub trait BulkConsumer: Consumer {
    /// Attempts to consume one or more regular items. This method may fail, returning an `Err` instead.
    ///
    /// When consuming regular items, the consumer must clone the items out of a contiguous prefix of the given buffer, and then return the number of cloned items via `Ok(amount)`. The `amount` must not be zero. The consumer can assume that the buffer it receives is non-empty.
    ///
    /// After this function returns an error, no further methods of this trait may be invoked.
    ///
    /// The restrictions on implementations (read at least one item, only read from a prefix) and for callers (do not pass an empty buffer, do not call after closing or an error) all follow from a single axiom: in terms of the consumed sequence of values, bulk consumption must be indistinguishable from repeatedly calling `consume`. The only difference should be improved performance.
    ///
    /// This has direct consequences returning errors. Suppose `bulk_consume` is called with a buffer of length seven. The bulk consumer determines that it could consume four items before having to report an error. The bulk consumer must then read the four items and return `Ok(4)`. Only on the next consumer method call can it report the error.
    ///
    /// # Invariants
    ///
    /// Must not be called after this consumer has been closed, or after any method of this trait has returned an error.
    ///
    /// Must not be called with an empty buffer.
    ///
    /// <br/>Counterpart: the [`BulkProducer::bulk_produce`] method (when producing regular items).
    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error>;
}

impl<C: BulkConsumer> BulkConsumer for &mut C {
    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        (*self).bulk_consume(buf).await
    }
}

#[cfg(feature = "alloc")]
impl<C: BulkConsumer> BulkConsumer for alloc::boxed::Box<C> {
    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        self.as_mut().bulk_consume(buf).await
    }
}

impl BulkConsumer for Infallible {
    async fn bulk_consume(&mut self, _buf: &[Self::Item]) -> Result<usize, Self::Error> {
        unreachable!()
    }
}

/// Conversion into a [`BulkConsumer`].
///
/// This trait is automatically implemented by implementing [`IntoConsumer`] with the associated consumer being a bulk consumer.
///
/// <br/>Counterpart: the [`IntoBulkProducer`] trait.
pub trait IntoBulkConsumer: IntoConsumer<IntoConsumer: BulkConsumer> {}

impl<C> IntoBulkConsumer for C where C: IntoConsumer<IntoConsumer: BulkConsumer> {}

/// A [`BufferedConsumer`] is a consumer that can delay performing side-effects to make `consume` (and `bulk_consume`) calls more efficient.
///
/// The classic example of a [`BufferedConsumer`] is a consumer of bytes which writes to a file from disk: it most certainly should not write every individual byte to disk, instead it should buffer bytes in memory and occasionally flush the buffer to disk. While any consumer can perform such an optimisation and flush during `consume` and `close` calls, the [`BufferedConsumer`] trait additionally provides the [`flush`](BufferedConsumer::flush) method, by which calling code can instruct the consumer to immediately perform the observable side-effects for all currently buffered data.
///
/// <br/>Counterpart: the [`BufferedProducer`] trait.
pub trait BufferedConsumer: Consumer {
    /// Attempts to perform any effectful actions that were delayed to make preceding calls to `consume` and `bulk_consume` more efficient.
    ///
    /// This function allows calling code to trigger side-effects which otherwise could only be triggered by calling [`close`](Consumer::close).
    ///
    /// After this function returns an error, no further methods of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after this consumer has been closed, or after any method of this trait has returned an error.
    ///
    /// <br/>Counterpart: the [`BufferedProducer::slurp`] method.
    async fn flush(&mut self) -> Result<(), Self::Error>;
}

impl<C: BufferedConsumer> BufferedConsumer for &mut C {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        (*self).flush().await
    }
}

#[cfg(feature = "alloc")]
impl<C: BufferedConsumer> BufferedConsumer for alloc::boxed::Box<C> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.as_mut().flush().await
    }
}

impl BufferedConsumer for Infallible {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        unreachable!()
    }
}

/// Conversion into a [`BufferedConsumer`].
///
/// This trait is automatically implemented by implementing [`IntoConsumer`] with the associated consumer being a buffered consumer.
///
/// <br/>Counterpart: the [`IntoBufferedProducer`] trait.
pub trait IntoBufferedConsumer: IntoConsumer<IntoConsumer: BufferedConsumer> {}

impl<C> IntoBufferedConsumer for C where C: IntoConsumer<IntoConsumer: BufferedConsumer> {}
