//! Producers — values that asynchronously yield a sequence of items.
//!
//! [`Producer`] is an asynchronous generalisation of [`Iterator`]; a producer lazily produces a sequence of items. There are three core differences between [`Iterator::next`] and the analogous [`Producer::produce`]:
//!
//! - `produce` is asynchronous;
//! - `produce` returns a result, allowing it to report fatal errors; and
//! - `produce` uses an [`Either`] to distinguish between repeated items ([`Left`]) and the final item ([`Right`]).
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
//! Whereas an iterator yields a sequence of arbitrarily many values of type [`Iterator::Item`] followed by up to one value of type `()`, a producer yields a sequence of arbitrarily many values of type [`Producer::Item`] followed by either up to one value of type [`Producer::Final`] or by up to one value of type [`Producer::Error`]. Producers with `Final = ()` and `Error = Infallible` are effectively asynchronous iterators.
//!
//! It is forbidden to call `produce` after a producer has emitted an error or its final item. Any such call may result in unspecified (but safe) behaviour.
//!
//! <br/>
//!
//! The [`consume`](crate::consume) macro provides a handy generalisation of `for` loop syntax. It can handle not only repeated items but optionally also final values and errors. The following example handles repeated items and the final item, and transparently propagates errors.
//!
//! ```
//! use ufotofu::prelude::*;
//! # fn main() {
//! # pollster::block_on(async{
//!
//! // The macro converts `[1, 2, 4]` into a producer.
//! consume![[1, 2, 4] {
//!     item it => print!("{it}, "),
//!     // We could remove the next line to simply ignore the final item.
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
//! Every producer automatically implements the [`ProducerExt`] trait, which provides a host of useful methods for working with producers. [TODO add proper example]
//!
//! ```
//! use ufotofu::prelude::*;
//! # pollster::block_on(async{
//! let mut p = [1, 2, 4].into_producer();
//!
//! assert_eq!(p.produce().await?, Left(1));
//! assert_eq!(p.produce().await?, Left(2));
//! assert_eq!(p.produce().await?, Left(4));
//! assert_eq!(p.produce().await?, Right(()));
//! # Result::<(), Infallible>::Ok(())
//! # });
//! ```
//!
//! <br/>
//!
//! ---
//!
//! <br/>
//!
//! Producing a sequence one item at a time can be inefficient. The [`BulkProducer`] trait extends [`Producer`] with the ability to produce multiple items at a time. The design is fully analogous to [`std::io::Read`] — the [`BulkProducer::bulk_produce`] method takes an `&mut [Self::Item]` as its argument, and returns how many items it placed in that buffer. Crucial differences to [`Read::read`](std::io::Read::read) are:
//!
//! - `bulk_produce` is asynchronous;
//! - `bulk_produce` can either fill the slice with regular items, yield an error, or yield the final item;
//! - `bulk_produce` works with arbitrary `Producer::Item` and `Producer::Error` types, not just `u8` and `io::Error`; and
//! - `bulk_produce` must not be called with an empty buffer, and it must write at least one item (when not signalling the end of the sequence with a final item or an error).
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
//! Every bulk producer automatically implements the [`BulkProducerExt`] trait, which provides bulk-production-based variants of the appropriate methods of [`ProducerExt`]. These bulk versions are typically more efficient and should be preferred whenever possible. [TODO proper example]
//!
//! ```
//! use ufotofu::prelude::*;
//! # pollster::block_on(async{
//! let mut p = [1, 2, 4].into_producer();
//!
//! assert_eq!(p.produce().await?, Left(1));
//! assert_eq!(p.produce().await?, Left(2));
//! assert_eq!(p.produce().await?, Left(4));
//! assert_eq!(p.produce().await?, Right(()));
//! # Result::<(), Infallible>::Ok(())
//! # });
//! ```
//!
//! <br/>
//!
//! ---
//!
//! <br/>
//!
//! [TODO] BufferedProducer

use core::convert::Infallible;

use either::Either::{self, *};

use crate::errors::*;

mod iterator_as_producer;
pub use iterator_as_producer::*;

mod clone_from_slice;
pub use clone_from_slice::*;

mod empty;
pub use empty::*;

#[cfg(feature = "alloc")]
mod vec_producer;
#[cfg(feature = "alloc")]
pub use vec_producer::*;

mod array_producer;
pub use array_producer::*;

/// A [`Producer`] produces a potentially infinite sequence, one item at a time.
///
/// The sequence consists of an arbitrary number of values of type [`Self::Item`], followed by
/// up to one value of type [`Self::Final`]. If you intend for the sequence to be infinite, use
/// [`Infallible`](core::convert::Infallible) for [`Self::Final`].
///
/// A producer may signal an error of type [`Self::Error`] instead of producing an item (whether repeated or final).
#[must_use = "producers are lazy and do nothing unless consumed"]
pub trait Producer {
    /// The sequence produced by this producer starts with *arbitrarily many* values of this type.
    type Item;
    /// The sequence produced by this producer ends with *up to one* value of this type.
    type Final;
    /// The type of errors the producer can emit instead of doing its job.
    type Error;

    /// Attempts to produce the next item, which is either a regular repeated item or the final item.
    ///
    /// After this function returns the final item, or after it returns an error, no further
    /// functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned a final item or an error.
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
/// with the `consume!` macro](crate::consume).
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

/// A [`Producer`] that can eagerly perform side-effects to prepare values for later yielding.
pub trait BufferedProducer: Producer {
    /// Asks the producer to prepare some values for yielding.
    ///
    /// This function allows the [`Producer`] to perform side effects that it would otherwise
    /// have to do just-in-time when [`produce`](Producer::produce) gets called.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned a final item or an error.
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
pub trait IntoBufferedProducer: IntoProducer<IntoProducer: BufferedProducer> {}

impl<P> IntoBufferedProducer for P where P: IntoProducer<IntoProducer: BufferedProducer> {}

/// A [`Producer`] that is able to produce several items with a single function call, in order to
/// improve on the efficiency of the [`Producer`] trait. Semantically, there must be no difference
/// between producing items in bulk or one item at a time.
pub trait BulkProducer: Producer {
    /// Produces a non-zero number of items by writing them into a given buffer and returning how
    /// many items were produced. The contents of the passed buffer do not influence the behaviour of this method.
    ///
    /// After this function returns the final item, or after it returns an error, no further
    /// functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned a final item or an error.
    ///
    /// Despite implementations of this method ignoring the contents of `buf`, `buf` must still contain initialised memory.
    ///
    /// #### Implementation Notes
    ///
    /// This function must not read the contents of `buf`; its observable semantics must not depend on the contents of `buf` (with the sole exception of running the desctructors of items in `buf` it overwrites).
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
pub trait IntoBulkProducer: IntoProducer<IntoProducer: BulkProducer> {}

impl<P> IntoBulkProducer for P where P: IntoProducer<IntoProducer: BulkProducer> {}

// #[macro_use]
// mod macros;

// mod from_slice;
// pub use from_slice::FromSlice_ as FromSlice;

// mod from_boxed_slice;
// pub use from_boxed_slice::FromBoxedSlice_ as FromBoxedSlice;

// mod empty;
// pub use empty::Empty_ as Empty;

// mod map_item;
// pub use map_item::MapItem;

// mod map_final;
// pub use map_final::MapFinal;

// mod map_error;
// pub use map_error::MapError;

// mod limit;
// pub use limit::Limit;

// #[cfg(feature = "alloc")]
// mod merge;
// #[cfg(feature = "alloc")]
// pub use merge::Merge;

// #[cfg(feature = "compat")]
// mod reader;
// #[cfg(feature = "compat")]
// pub use reader::{BufReaderToBulkProducer, ReaderToBulkProducer};

// #[cfg(test)]
// mod invariant;
// #[cfg(not(test))]
// mod invariant_noop;
// #[cfg(test)]
// pub use invariant::Invariant;
// #[cfg(not(test))]
// pub use invariant_noop::Invariant;

// #[cfg(feature = "dev")]
// mod bulk_scrambler;
// #[cfg(feature = "dev")]
// pub use bulk_scrambler::{BulkProducerOperation, BulkScrambler_ as BulkScrambler};

// #[cfg(all(feature = "dev", feature = "alloc"))]
// mod test_producer;
// #[cfg(all(feature = "dev", feature = "alloc"))]
// pub use test_producer::{TestProducerBuilder, TestProducer_ as TestProducer};
