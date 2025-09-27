#![no_std]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::type_complexity)]
#![allow(async_fn_in_trait)]

//! Abstractions for asynchronously working with series of data ("streams" and "sinks").
//!
//! This crate provides alternatives to some abstractions of the popular [`futures`](https://docs.rs/futures/latest/futures) crate:
//!
//! - [`Producer`] and [`Consumer`] replace [`Stream`](https://docs.rs/futures/latest/futures/prelude/trait.Stream.html) and [`Sink`](https://docs.rs/futures/latest/futures/prelude/trait.Sink.html), and
//! - [`BulkProducer`] and [`BulkConsumer`] replace [`AsyncRead`](https://docs.rs/futures/latest/futures/prelude/trait.AsyncRead.html) [`AsyncWrite`](https://docs.rs/futures/latest/futures/prelude/trait.AsyncWrite.html).
//!
//! See the [`producer`] and [`consumer`] modules for thorough introductions to the designs.  
//! Read on for the core design choices which distinguish `ufotofu` from the `futures` crate:
//!
//! ## Fundamental Design Choices
//!
//! - Async trait methods, no poll-based interfaces.
//! - `nostd` by default.
//! - Fatal errors, no resumption of processing after an error was signalled.
//! - Full generics for bulk operations, no restriction to `u8` and `io::Error`.
//! - Bulk processing generalises item-by-item processing; the bulk traits extend the item-by-item traits.
//! - Bulk operations work with non-empty slices and must process nonzero quantities of items.
//! - Buffering is abstracted-over in traits, not provided by concrete structs.
//! - Emphasis on producer-consumer duality, neither is more expressive than the other.
//! - Producers emit a dedicated final value, consumers receive a dedicated value when closed.
//! - British spelling.
//!
//! ## Caveats
//!
//! Ufotofu makes some simplifying assumptions, which may render it unsuitable for you. Each assumption removes significant complexity around working with async Rust, but constrains applicability.
//!
//! - The futures returned by async ufotofu methods are `!Send`, they cannot be run on multi-threaded executors.
//! - Dropping any method-returned future before polling it to completion will leave the original object in an undefined state; subsequent method calls may display arbitrary (but always safe) behaviour.
//! - Unwinding any panic may leave ufotofu values in an undefined state. Do not attempt to recover from panics when using ufotofu.
//!
//! ## Crate Organisation
//!
//! [TODO]
//!
//! ## Feature Flags
//!
//! [TODO]

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
//! - Traits for converting values into consumers and producers: [`IntoProducer`], [`IntoBufferedProducer`], [`IntoBulkProducer`], [`IntoConsumer`], [`IntoBufferedConsumer`], and [`IntoBulkConsumer`].
//! - Extension traits which add to the core traits a variety of helpful methods: [`ProducerExt`], [`BufferedProducerExt`], [`BulkProducerExt`], [`ConsumerExt`], [`BufferedConsumerExt`], and [`BulkConsumerExt`].
//! - Piping data: [`pipe`] and [`bulk_pipe`].
//!
//! Further functionality, specific to producers and consumers respectively, is exposed in the [`producer`] and [`consumer`] modules. Types used by the extention traits are exposed in the [`producer_ext`] and [`consumer_ext`] modules.
//!
//! ## Feature Flags
//!
//! UFOTOFU gates several features that are only interesting under certain circumstances behind feature flags. These API docs document *all* functionality, though, as if all feature flags were activated.
//!
//! All functionality which relies on the Rust standard library is gated behind the `std` feature flag (enabled by default).
//!
//! All functionality which performs dynamic memory allocations is gated behind the `alloc` feature flag (enabled by default, implied by the `std` feature).
//!
//! All functionality specifically designed to aid in testing and development is gated behind the `dev` feature flag (disabled by default).
// //!
// //! All functionality which provides interoperability with other async sequence manipulation crates is gated behind the `compat` feature flag (disabled by default).

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

// We re-export Either here so we can reliably match against it in the macros we export. We hide it from our docs though.
#[doc(hidden)]
pub use either::Either;
use Either::*;

pub use ufotofu_macros::consume;

mod errors;
pub use errors::*;

pub mod producer;
use producer::*;
pub use producer::{
    BufferedProducer, BulkProducer, IntoBufferedProducer, IntoBulkProducer, IntoProducer, Producer,
};

pub mod producer_ext;
pub use producer_ext::{BulkProducerExt, ProducerExt};

pub mod consumer;
use consumer::*;
pub use consumer::{
    BufferedConsumer, BulkConsumer, Consumer, IntoBufferedConsumer, IntoBulkConsumer, IntoConsumer,
};

pub mod consumer_ext;
pub use consumer_ext::{BulkConsumerExt, ConsumerExt};

#[cfg(all(feature = "dev", feature = "alloc"))]
mod test_yielder;

/// A “prelude” for crates using the `ufotofu` crate.
///
/// This prelude is similar to the standard library’s prelude in that you’ll almost always want to import its entire contents, but unlike the standard library’s prelude you’ll have to do so manually:
///
/// use ufotofu::prelude::*;
///
/// The prelude may grow over time.
pub mod prelude {
    pub use crate::{
        consume, consumer, producer, BufferedConsumer, BufferedProducer, BulkConsumer,
        BulkConsumerExt, BulkProducer, BulkProducerExt, Consumer, ConsumerExt,
        IntoBufferedConsumer, IntoBufferedProducer, IntoBulkConsumer, IntoBulkProducer,
        IntoConsumer, IntoProducer, Producer, ProducerExt,
    };

    pub use either::Either::{self, Left, Right};

    pub use core::convert::Infallible;
}

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
