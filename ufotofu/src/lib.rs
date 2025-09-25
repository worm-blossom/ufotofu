#![no_std]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::type_complexity)]
#![allow(async_fn_in_trait)]

//! # UFOTOFU
//!
//! UFOTOFU provides APIs for lazily producing or consuming sequences of arbitrary length, serving as async redesigns of traits such as [`Iterator`], [`io::Read`](std::io::Read), or [`io::Write`](std::io::Write). Highlights include
//!
//! - freely choosable error and item types, even for readers and writers,
//! - meaningful subtyping relations between streams and readers, and between sinks and writers,
//! - the ability to represent finite and infinite sequences on the type level, and
//! - `nostd` support.
//!
//! You can read an in-depth discussion of the API designs [here](https://github.com/AljoschaMeyer/lazy_on_principle/blob/main/main.pdf).
//!
//! ## Core Abstractions
//!
//! UFOTOFU is built around a small hierarchy of traits that describe how to produce or consume a sequence item by item.
//!
//! A [`Producer`] provides the items of a sequence to some client code, similar to the [`futures::Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html) and [`core::iter::Iterator`] traits. Client code can repeatedly request the next item, and receives either another item, an error, or a dedicated *final* item which may be of a different type than the repeated items. An *iterator* of `T`s corresponds to a *producer* of `T`s with final item type `()` and error type [`Infallible`](core::convert::Infallible).
//!
//! A [`Consumer`] accepts the items of a sequence from some client code, similar to the [`futures::Sink`](https://docs.rs/futures/latest/futures/sink/trait.Sink.html) trait. Client code can repeatedly add new items to the sequence, until it adds a single *final* item which may be of a different type than the repeated items. A final item type of `()` makes adding the final item equivalent to calling a conventional [`close`](https://docs.rs/futures/latest/futures/sink/trait.Sink.html#tymethod.poll_close) method.
//!
//! Producers and consumers are fully dual; the [`pipe`] function writes as much data as possible from a producer into a consumer.
//!
//! Consumers often buffer items in an internal queue before performing side-effects on data in larger chunks, such as writing data to the network only once a full packet can be filled. The [`BufferedConsumer`] trait extends the [`Consumer`] trait to allow client code to trigger effectful flushing of internal buffers. Dually, the [`BufferedProducer`] trait extends the [`Producer`] trait to allow client code to trigger effectful prefetching of data into internal buffers.
//!
//! Finally, the [`BulkProducer`] and [`BulkConsumer`] traits extend [`BufferedProducer`] and [`BufferedConsumer`] respectively with the ability to operate on whole slices of items at a time, similar to [`std::io::Read`] and [`std::io::Write`]. The [`bulk_pipe`] function leverages this ability to efficiently pipe data.
//!
//! ## Async Conventions
//!
//! UFOTOFU provides async APIs, and follows some consistent conventions. Most importantly, the futures returned by any UFOTOFU method are not expected to be cancellation safe: while it is allowed to drop a Future returned by an UFOTOFU trait method without polling it to completion, it is forbidden to call any further UFOTOFU trait methods on the same value afterwards. In other words: you must poll the future returned by any trait method to completion before calling the next trait method. This mirrors — as closely as possible — the design of sync APIs, where there is no way to partially execute a method either.
//!
//! Further, UFOTOFU never requires a `Send` bound on any futures. In other words, it can only be used with single-threaded async runtimes (or, more precisely, runtimes that confine the execution of any one future to a single thread).
//!
//! ## Module Organisation
//!
//! This top-level module provides the core UFOTOFU traits, and basic piping functionality:
//!
//! - Traits for producing sequences: [`Producer`], [`BufferedProducer`], and [`BulkProducer`].
//! - Traits for consuming sequences: [`Consumer`], [`BufferedConsumer`], and [`BulkConsumer`].
//! - Piping data: [`pipe`] and [`bulk_pipe`].
//!
//! Further functionality, specific to producers and consumers respectively, is exposed in the [`producer`] and [`consumer`] modules.
//!
//! ## Feature Flags
//!
//! UFOTOFU gates several features that are only interesting under certain circumstances behind feature flags. These API docs document *all* functionality, though, as if all feature flags were activated.
//!
//! All functionality which relies on the Rust standard library is gated behind the `std` feature flag (enabled by default).
//!
//! All functionality which performs dynamic memory allocations is gated behind the `alloc` feature flag (disabled by default, implied by the `std` feature).
//!
//! All functionality which provides interoperability with other async sequence manipulation crates is gated behind the `compat` feature flag (disabled by default).
//!
//! All functionality specifically designed to aid in testing and development is gated behind the `dev` feature flag (disabled by default).

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

// We reexport Either here so we can reliably match against it in the macros we export. We hide it from our docs though.
#[doc(hidden)]
pub use either::Either;
use Either::*;

pub use ufotofu_macros::consume;

mod errors;
pub use errors::*;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::convert::Infallible;

// pub mod consumer;
pub mod producer;

use producer::*;
pub use producer::{BufferedProducer, BulkProducer, IntoProducer, Producer};

// #[cfg(all(feature = "dev", feature = "alloc"))]
// mod test_yielder;

/// A [`Consumer`] consumes a potentially infinite sequence, one item at a time.
///
/// The sequence consists of an arbitrary number of values of type [`Self::Item`], followed by
/// up to one value of type [`Self::Final`]. If you intend for the sequence to be infinite, use
/// [`Infallible`] for [`Self::Final`].
///
/// A consumer may signal an error of type [`Self::Error`] instead of consuming any item (whether repeated or final).
#[must_use = "consumers are lazy and do nothing unless produced into"]
pub trait Consumer {
    /// The sequence consumed by this consumer starts with *arbitrarily many* values of this type.
    type Item;
    /// The sequence consumed by this consumer ends with *up to one* value of this type.
    type Final;
    /// The type of errors the consumer can emit instead of doing its job.
    type Error;

    /// Attempts to consume the next item.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait returned an error,
    /// nor after [`close`](Consumer::close) was called.
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error>;

    /// Attempts to consume the final item.
    ///
    /// After this function is called, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after [`close`](Consumer::close) was called.
    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error>;

    /// Tries to consume (clones of) *all* items in the given slice.
    /// Reports an error if the slice could not be consumed completely.
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
    async fn consume_full_slice(
        &mut self,
        buf: &[Self::Item],
    ) -> Result<(), ConsumeAtLeastError<Self::Error>>
    where
        Self::Item: Clone,
    {
        for i in 0..buf.len() {
            let item = buf[i].clone();

            if let Err(err) = self.consume(item).await {
                return Err(ConsumeAtLeastError {
                    count: i,
                    reason: err,
                });
            }
        }

        Ok(())
    }
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

/// Conversion into a [`Consumer`].
///
/// By implementing `IntoConsumer` for a type, you define how it will be
/// converted to a consumer.
/// ```
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

/// A [`Consumer`] that can delay performing side-effects when consuming items.
///
/// It must not delay performing side-effects when being closed. In other words,
/// calling [`close`](Consumer::close) should internally trigger flushing.
pub trait BufferedConsumer: Consumer {
    /// Forces the consumer to perform any side-effects that were delayed for previously consumed items.
    ///
    /// This function allows client code to force execution of the (potentially expensive)
    /// side-effects. In exchange, the consumer gains the freedom to delay the side-effects of
    /// [`consume`](Consumer::consume) to improve efficiency.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after [`close`](Consumer::close) was called.
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

/// Conversion into a [`BufferedConsumer`].
///
/// By implementing `IntoBufferedConsumer` for a type, you define how it will be
/// converted to a buffered consumer.
/// ```
pub trait IntoBufferedConsumer: IntoConsumer<IntoConsumer: BufferedConsumer> {}

impl<C: BufferedConsumer> IntoBufferedConsumer for C {}

/// A [`Consumer`] that is able to consume several items with a single function call, in order to
/// improve on the efficiency of the [`Consumer`] trait. Semantically, there must be no
/// difference between consuming items in bulk or one item at a time.
pub trait BulkConsumer: Consumer {
    /// Consumes a non-zero number of items by reading them from a given non-empty buffer, and returning how
    /// many items were consumed.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error, nor after
    /// [`close`](Consumer::close) was called.
    ///
    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error>;

    /// Tries to bulk-consume *all* items in the given slice.
    /// Reports an error if the slice could not be consumed completely.
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
    async fn bulk_consume_full_slice(
        &mut self,
        buf: &[Self::Item],
    ) -> Result<(), ConsumeAtLeastError<Self::Error>> {
        let mut consumed_so_far = 0;

        while consumed_so_far < buf.len() {
            match self.bulk_consume(&buf[consumed_so_far..]).await {
                Ok(consumed_count) => consumed_so_far += consumed_count,
                Err(err) => {
                    return Err(ConsumeAtLeastError {
                        count: consumed_so_far,
                        reason: err,
                    });
                }
            }
        }

        Ok(())
    }
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

/// Conversion into a [`BulkConsumer`].
///
/// By implementing `IntoBulkConsumer` for a type, you define how it will be
/// converted to a bulk consumer.
/// ```
pub trait IntoBulkConsumer: IntoConsumer<IntoConsumer: BulkConsumer> {}

impl<C: BulkConsumer> IntoBulkConsumer for C {}

/// Pipes as many items as possible from a [`Producer`] into a [`Consumer`]. Then calls [`close`](Consumer::close)
/// on the consumer with the final value emitted by the producer.
pub async fn pipe<P, C>(producer: P, consumer: C) -> Result<(), PipeError<P::Error, C::Error>>
where
    P: IntoProducer,
    C: IntoConsumer<Item = P::Item, Final = P::Final>,
{
    let mut consumer = consumer.into_consumer();
    consume![producer {
        item it => consumer.consume(it).await.map_err(PipeError::Consumer)?,
        final fin => Ok(consumer.close(fin).await.map_err(PipeError::Consumer)?),
        error err => Err(PipeError::Producer(err)),
    }]
}

/// Efficiently pipes as many items as possible from a [`BulkProducer`] into a [`BulkConsumer`], using the non-empty slice as an intermediate buffer.
/// Then calls [`close`](Consumer::close) on the consumer with the final value
/// emitted by the producer.
pub async fn bulk_pipe<P, C>(
    producer: P,
    consumer: C,
    buf: &mut [P::Item],
) -> Result<(), PipeError<P::Error, C::Error>>
where
    P: IntoBulkProducer<Item: Clone>,
    C: IntoBulkConsumer<Item = P::Item, Final = P::Final>,
{
    debug_assert!(buf.len() > 0);

    let mut producer = producer.into_producer();
    let mut consumer = consumer.into_consumer();

    loop {
        match producer.bulk_produce(buf).await {
            Err(err) => return Err(PipeError::Producer(err)),
            Ok(Right(fin)) => {
                return consumer.close(fin).await.map_err(PipeError::Consumer);
            }
            Ok(Left(amount)) => {
                consumer
                    .bulk_consume_full_slice(&buf[..amount])
                    .await
                    .map_err(|err| PipeError::Consumer(err.reason))?;
            }
        }
    }
}

// /// Pipes at most `count` many items from a [`Producer`] into a [`Consumer`],
// /// and reports how many items were piped.
// /// The producer has emitted its final item (which was then used to close the
// /// consumer) if and only if the number of returned items is strictly less
// /// than `count`.
// pub async fn pipe_at_most<P, C>(
//     producer: &mut P,
//     consumer: &mut C,
//     count: usize,
// ) -> Result<usize, PipeError<P::Error, C::Error>>
// where
//     P: Producer,
//     C: Consumer<Item = P::Item, Final = P::Final>,
// {
//      TODO use IntoProducer and IntoConsumer here
//     let mut piped = 0;
//     while piped < count {
//         match producer.produce().await {
//             Ok(Either::Left(item)) => {
//                 match consumer.consume(item).await {
//                     Ok(()) => {
//                         piped += 1;
//                         // Then continues with next loop iteration.
//                     }
//                     Err(consumer_error) => {
//                         return Err(PipeError::Consumer(consumer_error));
//                     }
//                 }
//             }
//             Ok(Either::Right(final_value)) => match consumer.close(final_value).await {
//                 Ok(()) => {
//                     return Ok(piped);
//                 }
//                 Err(consumer_error) => {
//                     return Err(PipeError::Consumer(consumer_error));
//                 }
//             },
//             Err(producer_error) => {
//                 return Err(PipeError::Producer(producer_error));
//             }
//         }
//     }

//     Ok(piped)
// }

// /// Efficiently pipes at most `count` many items from a [`BulkProducer`] into a [`BulkConsumer`] [`consumer.bulk_consume`](BulkConsumer::bulk_consume),
// /// and reports how many items were piped.
// /// The producer has emitted its final item (which was then used to close the
// /// consumer) if and only if the number of returned items is strictly less
// /// than `count`.
// pub async fn bulk_pipe_at_most<P, C>(
//     producer: &mut P,
//     consumer: &mut C,
//     count: usize,
// ) -> Result<usize, PipeError<P::Error, C::Error>>
// where
//     P: BulkProducer,
//     P::Item: Clone,
//     C: BulkConsumer<Item = P::Item, Final = P::Final>,
// {
//     let mut piped = 0;
//     while piped < count {
//         match producer.expose_items().await {
//             Ok(Either::Left(slots)) => {
//                 let max_slots = min(count - piped, slots.len());
//                 let amount = match consumer.bulk_consume(&slots[..max_slots]).await {
//                     Ok(amount) => amount,
//                     Err(consumer_error) => return Err(PipeError::Consumer(consumer_error)),
//                 };
//                 match producer.consider_produced(amount).await {
//                     Ok(()) => {
//                         piped += amount;
//                         // Then continues with next loop iteration.
//                     }
//                     Err(producer_error) => return Err(PipeError::Producer(producer_error)),
//                 };
//             }
//             Ok(Either::Right(final_value)) => {
//                 match consumer.close(final_value).await {
//                     Ok(()) => return Ok(piped),
//                     Err(consumer_error) => return Err(PipeError::Consumer(consumer_error)),
//                 };
//             }
//             Err(producer_error) => {
//                 return Err(PipeError::Producer(producer_error));
//             }
//         }
//     }

//     Ok(piped)
// }
