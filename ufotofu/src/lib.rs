#![no_std]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::type_complexity)]
#![allow(async_fn_in_trait)]

//! Abstractions for asynchronously working with series of data (“streams” and “sinks”).
//!
//! This crate provides alternatives to some abstractions of the popular [`futures`](https://docs.rs/futures/latest/futures) crate:
//!
//! - [`Producer`] and [`Consumer`] replace [`Stream`](https://docs.rs/futures/latest/futures/prelude/trait.Stream.html) and [`Sink`](https://docs.rs/futures/latest/futures/prelude/trait.Sink.html), and
//! - [`BulkProducer`] and [`BulkConsumer`] replace [`AsyncRead`](https://docs.rs/futures/latest/futures/prelude/trait.AsyncRead.html) and [`AsyncWrite`](https://docs.rs/futures/latest/futures/prelude/trait.AsyncWrite.html).
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
//! - Zero-copy bulk processing; the bulk traits *expose* slices instead of copying into or from passed slices.
//! - Buffering is abstracted-over in traits, not provided by concrete structs.
//! - Emphasis on producer-consumer duality, neither is more expressive than the other.
//! - Producers emit a dedicated final value, consumers receive a dedicated value when closed.
//! - British spelling.
//!
//! See the [ufotofu website](https://ufotofu.worm-blossom.org/) for a discussion of these design choices — the crate docs stay focussed on the *what*, not the *why*.
//!
//! ## Caveats
//!
//! Ufotofu makes some simplifying assumptions, which may render it unsuitable for you. Each assumption removes significant complexity around working with async Rust, but constrains applicability.
//!
//! - The futures returned by async ufotofu methods are `!Send`, they cannot be run on multi-threaded executors.
//! - Dropping any method-returned future before polling it to completion will leave the original object in an undefined state; subsequent method calls may display arbitrary (but always safe) behaviour.
//! - Unwinding any panic may leave ufotofu values in an undefined state. Do not attempt to recover from panics when using ufotofu.
//!
//! ## Module Overview
//!
//! The two central modules are [`producer`] and [`consumer`], they define the core abstractions of the crate.
//!
//! The [`queues`] module provides the [`Queue`](queues::Queue) trait for infallible in-memory queues with bulk push and pop operations, and some types implementing it. These power the buffered producer and consumer implementations of ufotofu.
//!
//! The [`fuzz_testing_tutorial`] demonstrates the utility types for [fuzz testing](https://rust-fuzz.github.io/book/introduction.html) that ufotofu provides.

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

// We re-export Either here so we can reliably match against it in the macros we export. We hide it from our docs though.
#[doc(hidden)]
pub use either::Either;

use prelude::*;

mod errors;
pub use errors::*;

pub mod producer;
pub use producer::{
    BulkProducer, BulkProducerExt, IntoBulkProducer, IntoProducer, Producer, ProducerExt,
};

pub mod consumer;
pub use consumer::{
    BulkConsumer, BulkConsumerExt, Consumer, ConsumerExt, IntoBulkConsumer, IntoConsumer,
};

pub mod queues;

#[cfg(all(feature = "dev", feature = "alloc"))]
mod test_yielder;

#[cfg(feature = "dev")]
pub mod fuzz_testing_tutorial;

/// A “prelude” for crates using the `ufotofu` crate.
///
/// This prelude is similar to the standard library’s prelude in that you’ll almost always want to import its entire contents, but unlike the standard library’s prelude you’ll have to do so manually:
///
/// use ufotofu::prelude::*;
///
/// The prelude may grow over time.
pub mod prelude {
    pub use crate::{
        consumer, producer, BulkConsumer, BulkConsumerExt, BulkProducer, BulkProducerExt, Consumer,
        ConsumerExt, IntoBulkConsumer, IntoBulkProducer, IntoConsumer, IntoProducer, Producer,
        ProducerExt,
    };

    #[cfg(feature = "dev")]
    pub use crate::{
        consumer::{
            build_test_consumer, TestConsumer, TestConsumerBuilder, TestConsumerBuilderError,
        },
        producer::{
            build_test_producer, TestProducer, TestProducerBuilder, TestProducerBuilderError,
        },
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
    let mut producer = producer.into_producer();
    let mut consumer = consumer.into_consumer();

    loop {
        match producer.produce().await {
            Ok(Left(it)) => consumer.consume(it).await.map_err(PipeError::Consumer)?,
            Ok(Right(fin)) => return Ok(consumer.close(fin).await.map_err(PipeError::Consumer)?),
            Err(err) => return Err(PipeError::Producer(err)),
        }
    }
}

/// Efficiently pipes as many items as possible from a [`BulkProducer`] into a [`BulkConsumer`], using [`BulkConsumerExt::bulk_consume`].
/// Then calls [`close`](Consumer::close) on the consumer with the final value
/// emitted by the producer.
pub async fn bulk_pipe<P, C>(producer: P, consumer: C) -> Result<(), PipeError<P::Error, C::Error>>
where
    P: IntoBulkProducer<Item: Clone>,
    C: IntoBulkConsumer<Item = P::Item, Final = P::Final>,
{
    let mut p = producer.into_producer();
    let mut c = consumer.into_consumer();

    loop {
        match p
            .expose_items(async |items| match c.bulk_consume(items).await {
                Ok(amount) => (amount, Ok(())),
                Err(consumer_error) => (0, Err(consumer_error)),
            })
            .await
        {
            Ok(Left(Ok(()))) => {
                // No-op, continues with next loop iteration.
            }
            Ok(Left(Err(consumer_err))) => return Err(PipeError::Consumer(consumer_err)),
            Ok(Right(fin)) => {
                match c.close(fin).await {
                    Ok(()) => return Ok(()),
                    Err(consumer_error) => return Err(PipeError::Consumer(consumer_error)),
                };
            }
            Err(producer_err) => return Err(PipeError::Producer(producer_err)),
        }
    }
}
