// //! Useful functionality for working with producers.
// //!
// //! ## Obtaining Producers
// //!
// //! The [`FromSlice`] producer produces the items of a slice.
// //!
// //! The [`FromBoxedSlice`] producer takes ownership of a boxed slice (or vector) and produces its items.
// //!
// //! The [`Empty`] producer immediately produces its final item.
// //!
// //! ## Adaptors
// //!
// //! The [`MapItem`] adaptor wraps any producer and maps its emitted items with a function.
// //!
// //! The [`MapFinal`] adaptor wraps any producer and maps it final item with a function.
// //!
// //! The [`MapFinal`] adaptor wraps any producer and maps it error with a function.
// //!
// //! The [`Limit`] adaptor wraps any producer and limits how many items it may emit at most.
// //!
// //! ## Combining Producers
// //!
// //! The [`Merge`] adaptor wraps two producers and interleaves their items, drops the first `Final` item, but forwards the first `Error`.
// //!
// //! ## Development Helpers
// //!
// //! The [Invariant] adaptor wraps any producer and makes it panic during tests when some client code violates the API contracts imposed by the producer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom producers; all producers in the ufotofu crate use this wrapper internally.
// //!
// //! The [TestProducer] exists for testing code that interacts with arbitrary producers; it provides customisable behavior of which items to emit, when to emit the final item or an error, and varies the sizes of bulk buffers it exposes. To generate various configurations, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).
// //!
// //! The [BulkScrambler] exists for testing specific [`BulkProducer`](ufotofu::BulkProducer)s by exercising various interleavings of `produce`, `slurp`, and `expose_items` calls. To generate various configurations, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).
// //!
// //! ## Compatibility
// //!
// //! The [`ReaderToBulkProducer`] adaptor lets you treat a [`smol::io::AsyncRead`] as a [`BulkProducer`](ufotofu::BulkProducer) of bytes, and the more efficient [`BufReaderToBulkProducer`] adaptor lets you treat a [`smol::io::AsyncBufRead`] as a [`BulkProducer`](ufotofu::BulkProducer) of bytes.

use either::Either::{self, *};

use crate::errors::*;

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

mod iterator_as_producer;
pub use iterator_as_producer::*;

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

    /// Tries to produce a regular item, and reports an error if the final item was produced instead.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after [`close`](Consumer::close) was called.
    ///
    /// #### Implementation Notes
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    async fn produce_item(
        &mut self,
    ) -> Result<Self::Item, ProduceAtLeastError<Self::Final, Self::Error>> {
        match self.produce().await {
            Ok(Left(item)) => Ok(item),
            Ok(Right(fin)) => Err(ProduceAtLeastError {
                count: 0,
                reason: Left(fin),
            }),
            Err(err) => Err(ProduceAtLeastError {
                count: 0,
                reason: Right(err),
            }),
        }
    }

    /// Tries to completely overwrite a slice with items from a producer.
    /// Reports an error if the slice could not be overwritten completely.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after [`close`](Consumer::close) was called.
    ///
    /// #### Implementation Notes
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    async fn overwrite_full_slice(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<(), ProduceAtLeastError<Self::Final, Self::Error>> {
        for i in 0..buf.len() {
            match self.produce().await {
                Ok(Left(item)) => buf[i] = item,
                Ok(Right(fin)) => {
                    return Err(ProduceAtLeastError {
                        count: i,
                        reason: Left(fin),
                    })
                }
                Err(err) => {
                    return Err(ProduceAtLeastError {
                        count: i,
                        reason: Right(err),
                    })
                }
            }
        }

        Ok(())
    }
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

/// Conversion into a [`BufferedProducer`].
///
/// By implementing `IntoBufferedProducer` for a type, you define how it will be
/// converted to a buffered producer.
/// ```
pub trait IntoBufferedProducer: IntoProducer<IntoProducer: BufferedProducer> {}

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

    /// Tries to completely overwrite a slice with items from a bulk producer.
    /// Reports an error if the slice could not be overwritten completely.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after [`close`](Consumer::close) was called.
    ///
    /// #### Implementation Notes
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    async fn bulk_overwrite_full_slice(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<(), ProduceAtLeastError<Self::Final, Self::Error>> {
        let mut produced_so_far = 0;

        while produced_so_far < buf.len() {
            match self.bulk_produce(&mut buf[produced_so_far..]).await {
                Ok(Left(count)) => produced_so_far += count,
                Ok(Right(fin)) => {
                    return Err(ProduceAtLeastError {
                        count: produced_so_far,
                        reason: Left(fin),
                    });
                }
                Err(err) => {
                    return Err(ProduceAtLeastError {
                        count: produced_so_far,
                        reason: Right(err),
                    });
                }
            }
        }

        Ok(())
    }
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

/// Conversion into a [`BulkProducer`].
///
/// By implementing `IntoBulkProducer` for a type, you define how it will be
/// converted to a bulk producer.
/// ```
pub trait IntoBulkProducer: IntoProducer<IntoProducer: BulkProducer> {}
