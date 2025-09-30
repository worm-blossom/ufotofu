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
//! The [`consume`] macro provides a handy generalisation of `for` loop syntax. It can handle not only repeated items but optionally also final values and errors. The following example handles repeated items and the final value, and transparently propagates errors.
//!
//! ```
//! use ufotofu::prelude::*;
//! # fn main() {
//! # pollster::block_on(async{
//!
//! // The macro converts `[1, 2, 4]` into a producer.
//! consume![[1, 2, 4] {
//!     item it => print!("{it}, "),
//!     // We could remove the next line to simply ignore the final value.
//!     final () => println!("and done!"),
//!     // The following line would “catch” and print any producer error.
//!     // error err => println!({err}),
//! }];
//! // Prints `1, 2, 4, and done!`.
//! # Result::<(), Infallible>::Ok(())
//! # });
//! # }
//! ```
//!
//! The [`IntoProducer`] trait describes types which can be converted into producers. In the preceding example, this trait allowed the `consume!` macro to convert the array `[1, 2, 4]` into a producer of these three items. The standard library counterpart to `IntoProducer` is [`IntoIterator`].
//!
//! <br/>
//!
//! Every producer automatically implements the [`ProducerExt`] trait, which provides useful methods for working with producers.
//!
//! <br/
//!
//! >Producing a sequence one item at a time can be inefficient. The [`BulkProducer`] trait extends [`Producer`] with the ability to produce multiple items at a time. The design is fully analogous to [`std::io::Read`] — the [`BulkProducer::bulk_produce`] method takes a `&mut [Self::Item]` as its argument, and returns how many items it placed in that buffer. Crucial differences to [`Read::read`](std::io::Read::read) are:
//!
//! - `bulk_produce` is asynchronous;
//! - `bulk_produce` can either fill the slice with regular items, yield an error, or yield the final value;
//! - `bulk_produce` works with arbitrary `Producer::Item` and `Producer::Error` types, not just `u8` and `io::Error`; and
//! - `bulk_produce` must not be called with an empty buffer, and it must write at least one item (when not signalling the end of the sequence with a final value or an error).
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
//! <br/>
//!
//! Every bulk producer automatically implements the [`BulkProducerExt`] trait, which provides bulk-production-based variants of several methods of [`ProducerExt`]. These bulk versions are typically more efficient and should be preferred whenever possible.
//!
//! <br/>
//!
//! Finally, the [`BufferedProducer`] trait describes producers which can eagerly perform side-effects in order to make subsequent `produce` and `bulk_produce` calls run more efficiently. The classic example is a producer of bytes which reads a file from disk: it should most certainly prefetch many bytes at a time instead of reading them on demand. While any producer can perform such optimisations on-demand, the [`BufferedProducer`] trait additionally provides the [`slurp`](BufferedProducer::slurp) method, by which calling code can instruct the producer to perform preparatory side-effects even without the need to actually produce any data yet.
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
}

impl<P: Producer> Producer for &mut P {
    type Item = P::Item;
    type Final = P::Final;
    type Error = P::Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        (*self).produce().await
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
}

impl Producer for Infallible {
    type Item = Infallible;
    type Final = Infallible;
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        unreachable!()
    }
}

/// Conversion into a [`Producer`].
///
/// By implementing `IntoProducer` for a type, you define how it will be
/// converted to a producer. This is common for types which describe a
/// collection of some kind.
///
/// One benefit of implementing `IntoIterator` is that your type will [work
/// with the `consume!` macro](consume).
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

/// A [`BulkProducer`] is a producer that can emit multiple items with a single method call, by placing many items in the buffer argument of the [`BulkProducer::bulk_produce`] method and returning how many items were placed. Semantically, there should be no difference between bulk production or item-by-item production.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut p = [1, 2, 4].into_producer();
/// let mut buf = [0, 0];
///
/// assert_eq!(p.bulk_produce(&mut buf[..]).await?, Left(2));
/// assert_eq!(buf, [1, 2]);
/// assert_eq!(p.bulk_produce(&mut buf[..]).await?, Left(1));
/// assert_eq!(buf, [4, 2]);
/// assert_eq!(p.bulk_produce(&mut buf[..]).await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [`BulkConsumer`] trait.
pub trait BulkProducer: Producer {
    /// Attempts to produce one or more regular items, or the final value. This method may fail, returning an `Err` instead.
    ///
    /// When producing regular items, the producer must write the items to a contiguous prefix of the given buffer, and then return the number of written items via `Ok(Left(amount))`. The `amount` must not be zero. The producer can assume that the buffer it receives is non-empty.
    ///
    /// If this method returns the final value or an error, it must not mutate the buffer.
    ///
    /// The contents of the passed buffer must not influence the behaviour of this method, implementations should preferrably not read the buffer contents at all.
    ///
    /// After this function returns the final value or after it returns an error, no further
    /// methods of this trait may be invoked.
    ///
    /// The restrictions on implementations (write at least one item, do not read the buffer, only mutate a prefix, no mutation when returning the final value or an error) and for callers (do not pass an empty buffer, do not call after a final value or an error) all follow from a single axiom: in terms of the produced sequence of values, bulk production must be indistinguishable from repeatedly calling `produce`. The only difference should be improved performance.
    ///
    /// This has direct consequences returning errors. Suppose `bulk_produce` is called with a buffer of length seven. The bulk producer determines that it could produce four items before having to report an error. The bulk producer must then write the four items and return `Ok(Left(4))`. Only on the next producer method call can it report the error.
    ///
    /// # Invariants
    ///
    /// Must not be called after any method of this trait has returned a final value or an error.
    ///
    /// Must not be called with an empty buffer.
    ///
    /// <br/>Counterpart: the [`BulkConsumer::bulk_consume`] method.
    async fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error>;
}

impl<P: BulkProducer> BulkProducer for &mut P {
    async fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        (*self).bulk_produce(buf).await
    }
}

#[cfg(feature = "alloc")]
impl<P: BulkProducer> BulkProducer for alloc::boxed::Box<P> {
    async fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        self.as_mut().bulk_produce(buf).await
    }
}

impl BulkProducer for Infallible {
    async fn bulk_produce(
        &mut self,
        _buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
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

/// A [`BufferedProducer`] is a producer that can eagerly perform side-effects to make subsequent `produce` (and `bulk_produce`) calls more efficient.
///
/// The classic example of a [`BufferedProducer`] is a producer of bytes which reads a file from disk: it should most certainly prefetch many bytes at a time instead of reading them on demand. While any producer can perform such optimisations on-demand, the [`BufferedProducer`] trait additionally provides the [`slurp`](BufferedProducer::slurp) method, by which calling code can instruct the producer to perform preparatory side-effects even without the need to actually produce any data yet.
///
/// <br/>Counterpart: the [`BufferedConsumer`] trait.
pub trait BufferedProducer: Producer {
    /// Attempts to perform any effectful actions that might make future calls to `produce` and `bulk_produce` more efficient.
    ///
    /// This function allows the [`Producer`] to perform side-effects that it would otherwise
    /// have to do just-in-time when [`produce`](Producer::produce) or [`bulk_produce`](BulkProducer::bulk_produce) get called.
    ///
    /// After this function returns an error, no further methods of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any method of this trait has returned a final value or an error.
    ///
    /// <br/>Counterpart: the [`BufferedConsumer::flush`] method.
    async fn slurp(&mut self) -> Result<(), Self::Error>;
}

impl<P: BufferedProducer> BufferedProducer for &mut P {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        (*self).slurp().await
    }
}

#[cfg(feature = "alloc")]
impl<P: BufferedProducer> BufferedProducer for alloc::boxed::Box<P> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.as_mut().slurp().await
    }
}

impl BufferedProducer for Infallible {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        unreachable!()
    }
}

/// Conversion into a [`BufferedProducer`].
///
/// This trait is automatically implemented by implementing [`IntoProducer`] with the associated producer being a buffered producer.
///
/// <br/>Counterpart: the [`IntoBufferedConsumer`] trait.
pub trait IntoBufferedProducer: IntoProducer<IntoProducer: BufferedProducer> {}

impl<P> IntoBufferedProducer for P where P: IntoProducer<IntoProducer: BufferedProducer> {}
