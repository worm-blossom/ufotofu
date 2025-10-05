//! Consumers — values that asynchronously process a sequence of items.
//!
//! A [`Consumer`] processed items of type [`Consumer::Item`], fed to it with the [`Consumer::consume`] method. After the calling code has moved all items into the consumer, it calls [`Consumer::close`] with a final value of type [`Consumer::Final`] to signal to the consumer that no more items will follow.
//!
//! ```
//! use ufotofu::prelude::*;
//! # #[cfg(feature = "alloc")] {
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
//! # }
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
//! # #[cfg(feature = "alloc")] {
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
//! # }
//! ```
//!
//! <br/>
//!
//! Every consumer may delay performing side-effects to make `consume` (and `bulk_consume`) calls more efficient. The classic example of a [`BufferedConsumer`] is a consumer of bytes which writes to a file from disk: it most certainly should not write every individual byte to disk, instead it should buffer bytes in memory and occasionally flush the buffer to disk.
//!
//! The [`flush`](Consumer::flush) method lets calling code instruct the consumer to immediately perform the observable side-effects for all currently buffered data.
//!
//! <br/>
//!
//! Every consumer automatically implements the [`ConsumerExt`] trait, which provides useful methods for working with consumers.
//!
//! <br/>
//!
//! Consuming a sequence one item at a time can be inefficient. The [`BulkConsumer`] trait extends [`Consumer`] with the ability to consume multiple items at a time. This is enabled by the [`BulkConsumer::expose_slots`] method. You pass to this method an async function as the sole argument. The bulk consumer calls that function, passing it a mutable, non-empty slice of items. The function overwrite items in that buffer, and then returns a pair of values: first, the number of items from the buffer the consumer should now consume, and second, an arbitrary value, to be returned by the `expose_slots` call.
//!
//! ```
//! use ufotofu::prelude::*;
//! # pollster::block_on(async{
//! let mut arr = [0, 0, 0];
//! let mut c = (&mut arr).into_consumer();
//!
//! assert_eq!(c.expose_slots(async |mut slots| {
//!     slots[0] = 1;
//!     slots[1] = 2;
//!     slots[2] = 4;
//!     (3, "hi!")
//! }).await?, "hi!");
//! assert_eq!(c.consume(8).await, Err(()));
//!
//! assert_eq!(arr, [1, 2, 4]);
//!
//! // If we reported that we only wrote two items, the consumer would later accept another item:
//! let mut arr2 = [0, 0, 0];
//! let mut c2 = (&mut arr2).into_consumer();
//!
//! assert_eq!(c2.expose_slots(async |mut slots| {
//!     slots[0] = 1;
//!     slots[1] = 2;
//!     slots[2] = 4;
//!     (2, "hi!")
//! }).await?, "hi!");
//! assert_eq!(c2.consume(8).await, Ok(()));
//!
//! assert_eq!(arr2, [1, 2, 8]);
//! # Result::<(), ()>::Ok(())
//! # });
//! ```
//!
//! <br/>
//!
//! Every bulk consumer automatically implements the [`BulkConsumerExt`] trait, which provides bulk-consumption-based variants of several methods of [`ConsumerExt`]. These bulk versions are typically more efficient and should be preferred whenever possible.
//!
//! Of particular note is the [`BulkConsumerExt::bulk_consume`] method, which builds on `expose_slots` and reimplements the way that, e.g., [`std::io::Write`] accepts multiple items at a time: `bulk_consume` takes a slice as its input, and the consumer reports how many items from that slice it consumed.
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
//! <br/>The counterpart to the [`consumer`] module is the [`producer`] module.

use crate::prelude::*;

mod consumer_ext;
pub use consumer_ext::*;

mod move_into_slice;
pub use move_into_slice::*;

mod full;
pub use full::*;

pub mod compat;

mod buffered;
pub use buffered::*;

mod bulk_buffered;
pub use bulk_buffered::*;

/// A [`Consumer`] lazily processes a sequence of items.
///
/// The sequence consists of an arbitrary number of items of type [`Consumer::Item`] — moved into the consumer via [`Consumer::consume`] — and is optionally terminated by a value of type [`Consumer::Final`] — moved into the consumer via [`Consumer::close`]. At any point, a consumer may report an error of type [`Consumer::Error`].
///
/// ```
/// use ufotofu::prelude::*;
/// # #[cfg(feature = "alloc")] {
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
/// # }
/// ```
///
/// Every consumer may delay performing side-effects to make `consume` (and `bulk_consume`) calls more efficient. The classic example of a [`BufferedConsumer`] is a consumer of bytes which writes to a file from disk: it most certainly should not write every individual byte to disk, instead it should buffer bytes in memory and occasionally flush the buffer to disk.
///
/// The [`flush`](Consumer::flush) method lets calling code instruct the consumer to immediately perform the observable side-effects for all currently buffered data.
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

    /// Attempts to perform any effectful actions that were delayed to make preceding calls to `consume` more efficient.
    ///
    /// This function allows calling code to trigger side-effects which otherwise could only be triggered deliberately by calling [`close`](Consumer::close).
    ///
    /// After this function returns an error, no further methods of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after this consumer has been closed, or after any method of this trait has returned an error.
    ///
    /// <br/>Counterpart: the [`Producer::slurp`] method.
    async fn flush(&mut self) -> Result<(), Self::Error>;
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

    async fn flush(&mut self) -> Result<(), Self::Error> {
        (*self).flush().await
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

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.as_mut().flush().await
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

    async fn flush(&mut self) -> Result<(), Self::Error> {
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

/// A [`Bulkconsumer`] is a producer that can accept multiple items with a single call of the [`BulkConsumer::expose_slots`] method.
///
/// This method takes an async function as its sole argument. The consumer calls that function, passing it a mutable, non-empty slice of items. The function can mutate these items in any way, and then returns a pair of values: first, the number of items the consumer should now consider as having been consumed, and second, an arbitrary value, to be returned by the `expose_slots` call.
///
/// See [`BulkConsumerExt::bulk_consumer`] for using bulk consumers in a way analogous to [`std::io::Write::write`].
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut arr = [0, 0, 0];
/// let mut c = (&mut arr).into_consumer();
///
/// assert_eq!(c.expose_slots(async |mut slots| {
///     slots[0] = 1;
///     slots[1] = 2;
///     slots[2] = 4;
///     (3, "hi!")
/// }).await?, "hi!");
/// assert_eq!(c.consume(8).await, Err(()));
///
/// assert_eq!(arr, [1, 2, 4]);
///
/// // If we reported that we only wrote two items, the consumer would later accept another item:
/// let mut arr2 = [0, 0, 0];
/// let mut c2 = (&mut arr2).into_consumer();
///
/// assert_eq!(c2.expose_slots(async |mut slots| {
///     slots[0] = 1;
///     slots[1] = 2;
///     slots[2] = 4;
///     (2, "hi!")
/// }).await?, "hi!");
/// assert_eq!(c2.consume(8).await, Ok(()));
///
/// assert_eq!(arr2, [1, 2, 8]);
/// # Result::<(), ()>::Ok(())
/// # });
/// ```
///
/// Semantically, there should be no difference between bulk consumption or item-by-item consumption.
///
/// <br/>Counterpart: the [`BulkProducer`] trait.
pub trait BulkConsumer: Consumer {
    /// Exposes a non-empty number of item slots to a given async function, the function mutates those slots and then then reports to the consumer how many items should now be considered consumed.
    ///
    /// When the consumer needs to return an error, it must return it immediately from this method. Otherwise, i.e., when it wants to consume regular items, it must call the passed function `f` with a non-empty slice of items as the argument, and then poll `f` to completion. When `f` has yielded `(amount, t)`, the consumer must adjust its state as if `consume` had been called `amount` many times with copies of the first `amount` many items in the slice it had passed to `f`, and then return `Ok(t)`. It must not return `Err(_)` when having called `f`.
    ///
    /// After this function returns the final value or after it returns an error, no further
    /// methods of this trait may be invoked.
    ///
    /// The intention is for `f` to not read any contents of the passed slice, but to simply write items into the slice that should be consumed. Sadly, we cannot enforce this on the type level.
    ///
    /// # Invariants
    ///
    /// Must not be called after this consumer has been closed, or after any method of this trait has returned an error.
    ///
    /// `f` must not return an `amount` strictly greater than the length of the buffer passed to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// assert_eq!(c.expose_slots(async |mut slots| {
    ///     slots[0] = 1;
    ///     slots[1] = 2;
    ///     slots[2] = 4;
    ///     (3, "hi!")
    /// }).await?, "hi!");
    /// assert_eq!(c.consume(8).await, Err(()));
    ///
    /// assert_eq!(arr, [1, 2, 4]);
    ///
    /// // If we reported that we only wrote two items, the consumer would later accept another item:
    /// let mut arr2 = [0, 0, 0];
    /// let mut c2 = (&mut arr2).into_consumer();
    ///
    /// assert_eq!(c2.expose_slots(async |mut slots| {
    ///     slots[0] = 1;
    ///     slots[1] = 2;
    ///     slots[2] = 4;
    ///     (2, "hi!")
    /// }).await?, "hi!");
    /// assert_eq!(c2.consume(8).await, Ok(()));
    ///
    /// assert_eq!(arr2, [1, 2, 8]);
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`BulkProducer::expose_items`] method.
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R);
}

impl<C: BulkConsumer> BulkConsumer for &mut C {
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        (*self).expose_slots(f).await
    }
}

#[cfg(feature = "alloc")]
impl<C: BulkConsumer> BulkConsumer for alloc::boxed::Box<C> {
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        self.as_mut().expose_slots(f).await
    }
}

impl BulkConsumer for Infallible {
    async fn expose_slots<F, R>(&mut self, _f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
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
