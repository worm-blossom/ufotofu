//! Producers — values that asynchronously yield a sequence of items.
//!
//! [`Producer`] is an asynchronous generalisation of [`Iterator`]; a producer lazily produces a sequence of items. There are three core differences between [`Iterator::next`] and the analogous [`Producer::produce`]:
//!
//! - `produce` is asynchronous;
//! - `produce` returns a result, allowing it to report fatal errors; and
//! - `produce` uses an [`Either`] to distinguish between repeated items ([`Left`]) and the final value ([`Right`]).
//!
//! ```
//! use ufotofu::prelude::*;
//! # pollster::block_on(async{
//! let mut my_first_producer = [1, 2, 4].into_producer();
//!
//! assert_eq!(my_first_producer.produce().await?, Left(1));
//! assert_eq!(my_first_producer.produce().await?, Left(2));
//! assert_eq!(my_first_producer.produce().await?, Left(4));
//! assert_eq!(my_first_producer.produce().await?, Right(()));
//! # Result::<(), Infallible>::Ok(())
//! # });
//! ```
//!
//! Whereas an *iterator* yields a sequence of arbitrarily many values of type [`Iterator::Item`] followed by up to one value of type `()`, a producer yields a sequence of arbitrarily many values of type [`Producer::Item`] followed by either up to one value of type [`Producer::Final`] or by up to one value of type [`Producer::Error`]. Producers with `Final = ()` and `Error = Infallible` are effectively asynchronous iterators.
//!
//! It is forbidden to call `produce` after a producer has emitted an error or its final value. Any such call may result in unspecified (but safe) behaviour.
//!
//! <br/>
//!
//! The [`IntoProducer`] trait describes types which can be converted into producers. The standard library counterpart to `IntoProducer` is [`IntoIterator`].
//!
//! <br/>
//!
//! Every producer may eagerly perform side-effects to make subsequent `produce` calls more efficient. The classic example of a buffering producer is a producer of bytes which reads a file from disk: it should most certainly prefetch many bytes at a time instead of reading them on demand.
//!
//! The [`slurp`](Producer::slurp) method lets calling code instruct the producer to perform preparatory side-effects, even without the need to actually produce any data yet.
//!
//! <br/>
//!
//! Every producer automatically implements the [`ProducerExt`] trait, which provides useful methods for working with producers.
//!
//! <br/>Producing a sequence one item at a time can be inefficient. The [`BulkProducer`] trait extends [`Producer`] with the ability to produce multiple items at a time. This is enabled by the [`BulkProducer::expose_items`] method. You pass to this method an async function as the sole argument. The bulk producer calls that function, passing it a non-empty slice of items. The function can process these items in any way, and then returns a pair of values: first, the number of items the producer should now consider as having been produced, and second, an arbitrary value, to be returned by the `expose_items` call.
//!
//! ```
//! use ufotofu::prelude::*;
//! # pollster::block_on(async{
//! let mut p = [1, 2, 4].into_producer();
//!
//! assert_eq!(p.expose_items(async |items| {
//!     assert_eq!(items, &[1, 2, 4]);
//!     return (3, "hi!");
//! }).await?, Left("hi!"));
//! assert_eq!(p.produce().await?, Right(()));
//!
//! // If we reported that we only processed two items, the producer would later emit the `4`:
//! let mut p2 = [1, 2, 4].into_producer();
//! assert_eq!(p2.expose_items(async |items| {
//!     assert_eq!(items, &[1, 2, 4]);
//!     return (2, "hi!");
//! }).await?, Left("hi!"));
//! assert_eq!(p2.produce().await?, Left(4));
//! # Result::<(), Infallible>::Ok(())
//! # });
//! ```
//!
//! <br/>
//!
//! Every bulk producer automatically implements the [`BulkProducerExt`] trait, which provides bulk-production-based variants of several methods of [`ProducerExt`]. These bulk versions are typically more efficient and should be preferred whenever possible.
//!
//! Of particular note is the [`BulkProducerExt::bulk_produce`] method, which builds on `expose_items` and reimplements the way that, e.g., [`std::io::Read`] emits multiple items at a time: `bulk_produce` takes a mutable slice as its input, and the producer reports how many items it copied (cloned) into it.
//!
//! ```
//! use ufotofu::prelude::*;
//! # pollster::block_on(async{
//! let mut p = [1, 2, 4].into_producer();
//! let mut buf = [0, 0];
//!
//! assert_eq!(p.bulk_produce(&mut buf[..]).await?, Left(2));
//! assert_eq!(buf, [1, 2]);
//! assert_eq!(p.bulk_produce(&mut buf[..]).await?, Left(1));
//! assert_eq!(buf, [4, 2]);
//! assert_eq!(p.bulk_produce(&mut buf[..]).await?, Right(()));
//! # Result::<(), Infallible>::Ok(())
//! # });
//! ```
//!
//! <br/>The counterpart to the [`producer`] module is the [`consumer`] module.

use crate::prelude::*;

mod producer_ext;
pub use producer_ext::*;

mod clone_from_slice;
pub use clone_from_slice::*;

mod empty;
pub use empty::*;

pub mod compat;

mod buffered;
pub use buffered::*;

mod bulk_buffered;
pub use bulk_buffered::*;

#[cfg(feature = "dev")]
mod bulk_scrambled;
#[cfg(feature = "dev")]
pub use bulk_scrambled::*;

#[cfg(feature = "dev")]
mod test_producer;
#[cfg(feature = "dev")]
pub use test_producer::*;

/// A [`Producer`] lazily yields a sequence of items.
///
/// The sequence consists of an arbitrary number of items of type [`Producer::Item`], optionally terminated by either a value of type [`Producer::Final`] or a value of type [`Producer::Error`].
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut p = [1, 2, 4].into_producer();
///
/// assert_eq!(p.produce().await?, Left(1));
/// assert_eq!(p.produce().await?, Left(2));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// Every producer may eagerly perform side-effects to make subsequent `produce` calls more efficient. The classic example of a buffering producer is a producer of bytes which reads a file from disk: it should most certainly prefetch many bytes at a time instead of reading them on demand.
///
/// The [`slurp`](Producer::slurp) method lets calling code instruct the producer to perform preparatory side-effects, even without the need to actually produce any data yet.
///
/// Calling code must uphold the following invariants:
///
/// - Do not call trait methods after any method has yielded a final value or an error.
/// - Do not use the producer after dropping any `Future` returned by any of its methods, unless the dropped future has been polled to completion.
/// - Do not use the producer after catching an unwinding panic.
///
/// <br/>Counterpart: the [`Consumer`] trait.
#[must_use = "producers are lazy and do nothing unless consumed"]
pub trait Producer {
    /// The sequence produced by this producer starts with *arbitrarily many* values of this type.
    type Item;
    /// The sequence produced by this producer ends with *up to one* value of this type.
    type Final;
    /// The type of errors the producer can emit instead of doing its job.
    type Error;

    /// Attempts to produce the next item, which is either a regular repeated item or the final value.
    /// The method may fail, returning an `Err` instead.
    ///
    /// After this method returns the final value or after it returns an error, no further
    /// methods of this trait may be invoked.
    ///
    /// # Invariants
    ///
    /// Must not be called after any method of this trait has returned a final value or an error.
    ///
    /// <br/>Counterpart: the [`Consumer::consume`] method (when this returns a regular item), or the [`Consumer::close`] method (when this returns a final value).
    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error>;

    /// Attempts to perform any effectful actions that might make future calls to `produce` and `bulk_produce` more efficient.
    ///
    /// This function allows the [`Producer`] to perform side-effects that it would otherwise
    /// have to do just-in-time when [`produce`](Producer::produce) gets called.
    ///
    /// After this function returns an error, no further methods of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any method of this trait has returned a final value or an error.
    ///
    /// <br/>Counterpart: the [`Consumer::flush`] method.
    async fn slurp(&mut self) -> Result<(), Self::Error>;
}

impl<P: Producer> Producer for &mut P {
    type Item = P::Item;
    type Final = P::Final;
    type Error = P::Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        (*self).produce().await
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        (*self).slurp().await
    }
}

#[cfg(feature = "alloc")]
impl<P: Producer> Producer for alloc::boxed::Box<P> {
    type Item = P::Item;
    type Final = P::Final;
    type Error = P::Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.as_mut().produce().await
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.as_mut().slurp().await
    }
}

impl Producer for Infallible {
    type Item = Infallible;
    type Final = Infallible;
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        unreachable!()
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        unreachable!()
    }
}

/// Conversion into a [`Producer`].
///
/// By implementing `IntoProducer` for a type, you define how it will be
/// converted to a producer. This is common for types which describe a
/// collection of some kind.
///
/// <br/>Counterpart: the [`IntoConsumer`] trait.
pub trait IntoProducer {
    /// The type of repeated items being produced.
    type Item;

    /// The type of the final value being produced.
    type Final;

    /// The type of errors the producer may emit.
    type Error;

    type IntoProducer: Producer<Item = Self::Item, Final = Self::Final, Error = Self::Error>;

    /// Creates a producer from a value.
    fn into_producer(self) -> Self::IntoProducer;
}

impl<P: Producer> IntoProducer for P {
    type Item = P::Item;
    type Final = P::Final;
    type Error = P::Error;
    type IntoProducer = P;

    #[inline]
    fn into_producer(self) -> P {
        self
    }
}

impl IntoProducer for () {
    type Item = Infallible;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = Empty<()>;

    #[inline]
    fn into_producer(self) -> Self::IntoProducer {
        empty(())
    }
}

/// A [`BulkProducer`] is a producer that can emit multiple items with a single call of the [`BulkProducer::expose_items`] method.
///
/// This method takes an async function as its sole argument. The producer calls that function, passing it a non-empty slice of items. The function can process these items in any way, and then returns a pair of values: first, the number of items the producer should now consider as having been produced, and second, an arbitrary value, to be returned by the `expose_items` call.
///
/// See [`BulkProducerExt::bulk_produce`] for using bulk producers in a way analogous to [`std::io::Read::read`].
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut p = [1, 2, 4].into_producer();
///
/// assert_eq!(p.expose_items(async |items| {
///     assert_eq!(items, &[1, 2, 4]);
///     return (3, "hi!");
/// }).await?, Left("hi!"));
/// assert_eq!(p.produce().await?, Right(()));
///
/// // If we reported that we only processed two items, the producer would later emit the `4`:
/// let mut p2 = [1, 2, 4].into_producer();
/// assert_eq!(p2.expose_items(async |items| {
///     assert_eq!(items, &[1, 2, 4]);
///     return (2, "hi!");
/// }).await?, Left("hi!"));
/// assert_eq!(p2.produce().await?, Left(4));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// Semantically, there should be no difference between bulk production or item-by-item production.
///
/// <br/>Counterpart: the [`BulkConsumer`] trait.
pub trait BulkProducer: Producer {
    /// Exposes a non-empty number of items to a given async function, the function then reports to the producer how many items should now be considered produced.
    ///
    /// When the producer needs to yield an error or its final value, it must return them immediately from this method. Otherwise, i.e., when it wants to produce regular items, it must call the passed function `f` with a non-empty slice of items as the argument, and then poll `f` to completion. When `f` has yielded `(amount, t)`, the producer must adjust its state as if `produce` had been called `amount` many times, and then return `Ok(Left(t))`. It must not return `Ok(Right(_))` or `Err(_)` when having called `f`.
    ///
    /// After this function returns the final value or after it returns an error, no further
    /// methods of this trait may be invoked.
    ///
    /// # Invariants
    ///
    /// Must not be called after any method of this trait has returned a final value or an error.
    ///
    /// `f` must not return an `amount` strictly greater than the length of the buffer passed to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = [1, 2, 4].into_producer();
    ///
    /// assert_eq!(p.expose_items(async |items| {
    ///     assert_eq!(items, &[1, 2, 4]);
    ///     return (3, "hi!");
    /// }).await?, Left("hi!"));
    /// assert_eq!(p.produce().await?, Right(()));
    ///
    /// // If we reported that we only processed two items, the producer would later emit the `4`:
    /// let mut p2 = [1, 2, 4].into_producer();
    /// assert_eq!(p2.expose_items(async |items| {
    ///     assert_eq!(items, &[1, 2, 4]);
    ///     return (2, "hi!");
    /// }).await?, Left("hi!"));
    /// assert_eq!(p2.produce().await?, Left(4));
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`BulkConsumer::expose_slots`] method.
    async fn expose_items<F, R>(&mut self, f: F) -> Result<Either<R, Self::Final>, Self::Error>
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, R);
}

impl<P: BulkProducer> BulkProducer for &mut P {
    async fn expose_items<F, R>(&mut self, f: F) -> Result<Either<R, Self::Final>, Self::Error>
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, R),
    {
        (*self).expose_items(f).await
    }
}

#[cfg(feature = "alloc")]
impl<P: BulkProducer> BulkProducer for alloc::boxed::Box<P> {
    async fn expose_items<F, R>(&mut self, f: F) -> Result<Either<R, Self::Final>, Self::Error>
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, R),
    {
        self.as_mut().expose_items(f).await
    }
}

impl BulkProducer for Infallible {
    async fn expose_items<F, R>(&mut self, _f: F) -> Result<Either<R, Self::Final>, Self::Error>
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, R),
    {
        unreachable!()
    }
}

/// Conversion into a [`BulkProducer`].
///
/// This trait is automatically implemented by implementing [`IntoProducer`] with the associated producer being a bulk producer.
///
/// <br/>Counterpart: the [`IntoBulkConsumer`] trait.
pub trait IntoBulkProducer: IntoProducer<IntoProducer: BulkProducer> {}

impl<P> IntoBulkProducer for P where P: IntoProducer<IntoProducer: BulkProducer> {}
