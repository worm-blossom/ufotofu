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
//! ## Module Overview
//!
//! The two central modules are [`producer`] and [`consumer`], they define the core abstractions of the crate.
//!
//! The [`queues`] module provides the [`Queue`](queues::Queue) trait for infallible in-memory queues with bulk push and pop operations, and some types implementing it. These power the buffered producer and consumer implementations of ufotofu.

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

// We re-export Either here so we can reliably match against it in the macros we export. We hide it from our docs though.
#[doc(hidden)]
pub use either::Either;
use Either::*;

/// Conveniently consume the output of a [`Producer`].
///
/// This macro provides a generalisation of a `for` loop. It first converts a value into a producer via [`IntoProducer`], and then repeatedly calls `produce` until the final value or an error is emitted. The macro has three branches to specify how to handle regular items, final valu, and/or error respectively.
///
/// ```
/// use ufotofu::prelude::*;
/// # fn main() {
/// # pollster::block_on(async{
///
/// // The macro converts `[1, 2, 4]` into a producer via `[1, 2, 4].into_producer()`.
/// consume![[1, 2, 4] {
///     item it => print!("{it}, "),
///     final () => {
///         print!("and ");
///         println!("done!");
///     }
///     error _err => unreachable!("this producer is infallible"),
/// }];
/// // Prints `1, 2, 4, and done!`.
/// # Result::<(), Infallible>::Ok(())
/// # });
/// # }
/// ```
///
/// The `error` branch is optional — without it, the macro implicitly applies the `?` to the result of calling `produce`.
///
/// ```
/// use ufotofu::prelude::*;
/// # fn main() {
/// # pollster::block_on(async{
///
/// consume![[1, 2, 4] {
///     item it => print!("{it}, "),
///     final () => println!("and done!"),
/// }];
/// # Result::<(), Infallible>::Ok(())
/// # });
/// # }
/// ```
///
/// The `final` branch is also optional, but only if [`Producer::Final`] is `()`.
///
/// ```
/// use ufotofu::prelude::*;
/// # fn main() {
/// # pollster::block_on(async{
///
/// consume![[1, 2, 4] {
///     item it => print!("{it}, "),
///     // Could also omit the `error` branch.
///     error _err => unreachable!("this producer is infallible"),
/// }];
/// // Prints `1, 2, 4, `.
/// # Result::<(), Infallible>::Ok(())
/// # });
/// # }
/// ```
///
/// The `item` branch is the only mandatory branch. Each of the `item`, `final`, and `error` "keywords" can be followed by an arbitrary pattern. The order of the three kinds of branches is arbitrary.
///
/// You can specify multiple branches of the same kind, in order to match different patterns.
///
/// ```
/// use ufotofu::prelude::*;
/// # fn main() {
/// # pollster::block_on(async{
///
/// consume![[1, 2, 4] {
///     item 2 => print!("quack, "),
///     item it => print!("{it}, "),
///     final () => {
///         print!("and ");
///         println!("done!");
///     }
///     error _err => unreachable!("this producer is infallible"),
/// }];
/// // Prints `1, quack, 4, and done!`.
/// # Result::<(), Infallible>::Ok(())
/// # });
/// # }
/// ```
///
/// Finally, here is a demonstration of what the macro expands to when all three kinds of cases are present, slightly simplified for readability:
///
/// ```
/// use ufotofu::prelude::*;
/// # fn main() {
/// # pollster::block_on(async{
/// # let some_value = [1, 2, 4];
///
/// consume![some_value {
///     item 42 => println!("42"),
///     item pattern_item => println!("non-42 item"),
///     final pattern_final => println!("final"),
///     error pattern_error => println!("error"),
/// }];
/// # Result::<(), Infallible>::Ok(())
/// # });
/// # }
///
/// // Roughly expands to:
///
/// # fn expanded() {
/// # pollster::block_on(async{
/// # let some_value = [1, 2, 4];
/// # fn handle_item(){}
/// # fn handle_final(){}
/// # fn handle_error(){}
/// let mut producer = some_value.into_producer();
///
/// loop {
///     match producer.produce().await {
///         Ok(Left(42)) => println!("42"),
///         Ok(Left(pattern_item)) => println!("non-42 item"),
///         Ok(Right(pattern_final)) => println!("final"),
///         Err(pattern_error) => println!("error"),
///     }
/// }
/// # Result::<(), Infallible>::Ok(())
/// # });
/// # }
/// ```
///
/// <br/>Counterpart: none, because Rust has no counterpart to the `for` loop. In a certain sense, generators are this counterpart, but we have not implemented generator-like producer macros. Yet.
pub use ufotofu_macros::consume;

mod errors;
pub use errors::*;

pub mod producer;
pub use producer::{
    BufferedProducer, BulkProducer, BulkProducerExt, IntoBufferedProducer, IntoBulkProducer,
    IntoProducer, Producer, ProducerExt,
};

pub mod consumer;
pub use consumer::{
    BufferedConsumer, BulkConsumer, BulkConsumerExt, Consumer, ConsumerExt, IntoBufferedConsumer,
    IntoBulkConsumer, IntoConsumer,
};

pub mod queues;

// #[cfg(all(feature = "dev", feature = "alloc"))]
// mod test_yielder;

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

/// Efficiently pipes as many items as possible from a [`BulkProducer`] into a [`BulkConsumer`], using a non-empty slice as an intermediate buffer.
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
    debug_assert!(!buf.is_empty());

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
