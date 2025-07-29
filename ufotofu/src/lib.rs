#![no_std]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::type_complexity)]

//! # UFOTOFU
//!
//! UFOTOFU provides APIs for lazily producing or consuming sequences of arbitrary length, serving as async redesigns of traits such as [`Iterator`], [`io::Read`](std::io::Read), or [`io::Write`](std::io::Write). Highlights include
//!
//! - bulk data transfer without temporary buffers,
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
//! Finally, the [`BulkProducer`] and [`BulkConsumer`] traits extend [`BufferedProducer`] and [`BufferedConsumer`] respectively with the ability to operate on whole slices of items at a time, similar to [`std::io::Read`] and [`std::io::Write`]. The [`bulk_pipe`] function leverages this ability to efficiently pipe data — unlike when using the standard library's [Read](std::io::Read) and [Write](std::io::Write) traits, this is possible without allocating an auxiliary buffer.
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

use core::cmp::min;
use core::future::Future;

use either::Either::{self, *};

// This allows macros to use `ufotofu` instead of `crate`, which might become
// convenient some day.
extern crate self as ufotofu;

#[macro_use]
mod common_macros;

mod errors;
pub use errors::*;

pub mod consumer;
pub mod producer;

#[cfg(all(feature = "dev", feature = "alloc"))]
mod fixed_queue;
#[cfg(all(feature = "dev", feature = "alloc"))]
mod test_yielder;

/// A [`Consumer`] consumes a potentially infinite sequence, one item at a time.
///
/// The sequence consists of an arbitrary number of values of type [`Self::Item`], followed by
/// up to one value of type [`Self::Final`]. If you intend for the sequence to be infinite, use
/// [`Infallible`](core::convert::Infallible) for [`Self::Final`].
///
/// A consumer may signal an error of type [`Self::Error`] instead of consuming any item (whether repeated or final).
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
    fn consume(&mut self, item: Self::Item) -> impl Future<Output = Result<(), Self::Error>>;

    /// Attempts to consume the final item.
    ///
    /// After this function is called, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after [`close`](Consumer::close) was called.
    fn close(&mut self, fin: Self::Final) -> impl Future<Output = Result<(), Self::Error>>;

    /// Tries to consume (clones of) *all* items in the given slice.
    /// Reports an error if the slice could not be consumed completely.
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
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
    fn consume_full_slice(
        &mut self,
        buf: &[Self::Item],
    ) -> impl Future<Output = Result<(), ConsumeAtLeastError<Self::Error>>>
    where
        Self::Item: Clone,
    {
        async {
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
    fn flush(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
}

/// A [`Consumer`] that is able to consume several items with a single function call, in order to
/// improve on the efficiency of the [`Consumer`] trait. Semantically, there must be no
/// difference between consuming items in bulk or one item at a time.
pub trait BulkConsumer: BufferedConsumer {
    /// A low-level method for consuming multiple items at a time. If you are only *working* with consumers (rather than *implementing* them), you will probably want to ignore this method and use [BulkConsumer::bulk_consume] instead.
    ///
    /// Exposes a non-empty slice of memory for client code to fill with items that should
    /// be consumed.
    ///
    /// The consumer should expose the largest contiguous slice it can expose efficiently.
    /// It should not perform copies to increase the size of the slice. Client code must be
    /// able to make progress even if this function always returns slices of size one.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after [`close`](Consumer::close) was called.
    fn expose_slots<'a>(
        &'a mut self,
    ) -> impl Future<Output = Result<&'a mut [Self::Item], Self::Error>>
    where
        Self::Item: 'a;

    /// A low-level method for consuming multiple items at a time. If you are only *working* with consumers (rather than *implementing* them), you will probably want to ignore this method and use [BulkConsumer::bulk_consume] instead.
    ///
    /// Instructs the consumer to consume the first `amount` many items of the slots
    /// it has most recently exposed. The semantics must be equivalent to those of [`consume`](Consumer::consume)
    /// being called `amount` many times with exactly those items.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Callers must have written into (at least) the `amount` many first slots that
    /// were most recently exposed.
    ///
    /// Must not be called after any function of this trait has returned an error, nor after
    /// [`close`](Consumer::close) was called.
    fn consume_slots(&mut self, amount: usize) -> impl Future<Output = Result<(), Self::Error>>;

    /// Consumes a non-zero number of items by reading them from a given buffer and returning how
    /// many items were consumed.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error, nor after
    /// [`close`](Consumer::close) was called.
    ///
    /// #### Implementation Notes
    ///
    /// The default implementation orchestrates [`expose_slots`](BulkConsumer::expose_slots) and [`consume_slots`](BulkConsumer::consume_slots) in a
    /// straightforward manner. Only provide your own implementation if you can do better
    /// than that.
    fn bulk_consume(
        &mut self,
        buf: &[Self::Item],
    ) -> impl Future<Output = Result<usize, Self::Error>>
    where
        Self::Item: Clone,
    {
        async {
            let slots = self.expose_slots().await?;
            let amount = min(slots.len(), buf.len());
            slots[0..amount].clone_from_slice(&buf[0..amount]);
            self.consume_slots(amount).await?;

            Ok(amount)
        }
    }

    /// Tries to bulk-consume (copies of) *all* items in the given slice.
    /// Reports an error if the slice could not be consumed completely.
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
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
    fn bulk_consume_full_slice(
        &mut self,
        buf: &[Self::Item],
    ) -> impl Future<Output = Result<(), ConsumeAtLeastError<Self::Error>>>
    where
        Self::Item: Clone,
    {
        async {
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
}

/// A [`Producer`] produces a potentially infinite sequence, one item at a time.
///
/// The sequence consists of an arbitrary number of values of type [`Self::Item`], followed by
/// up to one value of type [`Self::Final`]. If you intend for the sequence to be infinite, use
/// [`Infallible`](core::convert::Infallible) for [`Self::Final`].
///
/// A producer may signal an error of type [`Self::Error`] instead of producing an item (whether repeated or final).
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
    fn produce(
        &mut self,
    ) -> impl Future<Output = Result<Either<Self::Item, Self::Final>, Self::Error>>;

    /// Tries to produce a regular item, and reports an error if the final item was produced instead.
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
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
    fn produce_item(
        &mut self,
    ) -> impl Future<Output = Result<Self::Item, ProduceAtLeastError<Self::Final, Self::Error>>>
    {
        async {
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
    }

    /// Tries to completely overwrite a slice with items from a producer.
    /// Reports an error if the slice could not be overwritten completely.
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
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
    fn overwrite_full_slice(
        &mut self,
        buf: &mut [Self::Item],
    ) -> impl Future<Output = Result<(), ProduceAtLeastError<Self::Final, Self::Error>>> {
        async {
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
    fn slurp(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
}

/// A [`Producer`] that is able to produce several items with a single function call, in order to
/// improve on the efficiency of the [`Producer`] trait. Semantically, there must be no difference
/// between producing items in bulk or one item at a time.
pub trait BulkProducer: BufferedProducer {
    /// A low-level method for producing multiple items at a time. If you are only *working* with producers (rather than *implementing* them), you will probably want to ignore this method and use [BulkProducer::bulk_produce] instead.
    ///
    /// Exposes a non-empty slice of items to be produced (or the final value, or an error).
    /// The items in the slice must not have been emitted by [`produce`](Producer::produce) before.
    ///
    /// The producer should expose the largest contiguous slice it can expose efficiently.
    /// It should not perform copies to increase the size of the slice. Client code must be
    /// able to make progress even if this function always returns slices of size one.
    ///
    /// After this function returns the final item, or after it returns an error, no further
    /// functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned a final item or an error.
    fn expose_items<'a>(
        &'a mut self,
    ) -> impl Future<Output = Result<Either<&'a [Self::Item], Self::Final>, Self::Error>>
    where
        Self::Item: 'a;

    /// A low-level method for producing multiple items at a time. If you are only *working* with producers (rather than *implementing* them), you will probably want to ignore this method and use [BulkProducer::bulk_produce] instead.
    ///
    /// Marks `amount` many items as having been produced. Future calls to [`produce`](Producer::produce) and to
    /// [`expose_items`](BulkProducer::expose_items) must act as if [`produce`](Producer::produce) had been called `amount` many times.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Callers must not mark items as produced that had not previously been exposed by [`expose_items`](BulkProducer::expose_items).
    ///
    /// Must not be called after any function of this trait returned a final item or an error.
    fn consider_produced(&mut self, amount: usize)
        -> impl Future<Output = Result<(), Self::Error>>;

    /// Produces a non-zero number of items by writing them into a given buffer and returning how
    /// many items were produced.
    ///
    /// After this function returns the final item, or after it returns an error, no further
    /// functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned a final item or an error.
    ///
    /// #### Implementation Notes
    ///
    /// The default implementation orchestrates [`expose_items`](BulkProducer::expose_items) and [`consider_produced`](BulkProducer::consider_produced) in a
    /// straightforward manner. Only provide your own implementation if you can do better
    /// than that.
    fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> impl Future<Output = Result<Either<usize, Self::Final>, Self::Error>>
    where
        Self::Item: Clone,
    {
        async {
            match self.expose_items().await? {
                Either::Left(slots) => {
                    let amount = min(slots.len(), buf.len());
                    buf[0..amount].clone_from_slice(&slots[0..amount]);

                    self.consider_produced(amount).await?;

                    Ok(Either::Left(amount))
                }
                Either::Right(final_value) => Ok(Either::Right(final_value)),
            }
        }
    }

    /// Tries to completely overwrite a slice with items from a bulk producer.
    /// Reports an error if the slice could not be overwritten completely.
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
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
    fn bulk_overwrite_full_slice(
        &mut self,
        buf: &mut [Self::Item],
    ) -> impl Future<Output = Result<(), ProduceAtLeastError<Self::Final, Self::Error>>>
    where
        Self::Item: Clone,
    {
        async {
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
}

/// Pipes as many items as possible from a [`Producer`] into a [`Consumer`]. Then calls [`close`](Consumer::close)
/// on the consumer with the final value emitted by the producer.
pub async fn pipe<P, C>(
    producer: &mut P,
    consumer: &mut C,
) -> Result<(), PipeError<P::Error, C::Error>>
where
    P: Producer,
    C: Consumer<Item = P::Item, Final = P::Final>,
{
    loop {
        match producer.produce().await {
            Ok(Either::Left(item)) => {
                match consumer.consume(item).await {
                    Ok(()) => {
                        // No-op, continues with next loop iteration.
                    }
                    Err(consumer_error) => {
                        return Err(PipeError::Consumer(consumer_error));
                    }
                }
            }
            Ok(Either::Right(final_value)) => match consumer.close(final_value).await {
                Ok(()) => {
                    return Ok(());
                }
                Err(consumer_error) => {
                    return Err(PipeError::Consumer(consumer_error));
                }
            },
            Err(producer_error) => {
                return Err(PipeError::Producer(producer_error));
            }
        }
    }
}

/// Efficiently pipes as many items as possible from a [`BulkProducer`] into a [`BulkConsumer`]
/// using [`consumer.bulk_consume`](BulkConsumer::bulk_consume). Then calls [`close`](Consumer::close) on the consumer with the final value
/// emitted by the producer.
pub async fn bulk_pipe<P, C>(
    producer: &mut P,
    consumer: &mut C,
) -> Result<(), PipeError<P::Error, C::Error>>
where
    P: BulkProducer,
    P::Item: Clone,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
{
    loop {
        match producer.expose_items().await {
            Ok(Either::Left(slots)) => {
                let amount = match consumer.bulk_consume(slots).await {
                    Ok(amount) => amount,
                    Err(consumer_error) => return Err(PipeError::Consumer(consumer_error)),
                };
                match producer.consider_produced(amount).await {
                    Ok(()) => {
                        // No-op, continues with next loop iteration.
                    }
                    Err(producer_error) => return Err(PipeError::Producer(producer_error)),
                };
            }
            Ok(Either::Right(final_value)) => {
                match consumer.close(final_value).await {
                    Ok(()) => return Ok(()),
                    Err(consumer_error) => return Err(PipeError::Consumer(consumer_error)),
                };
            }
            Err(producer_error) => {
                return Err(PipeError::Producer(producer_error));
            }
        }
    }
}

/// Pipes at most `count` many items from a [`Producer`] into a [`Consumer`],
/// and reports how many items were piped.
/// The producer has emitted its final item (which was then used to close the
/// consumer) if and only if the number of returned items is strictly less
/// than `count`.
pub async fn pipe_at_most<P, C>(
    producer: &mut P,
    consumer: &mut C,
    count: usize,
) -> Result<usize, PipeError<P::Error, C::Error>>
where
    P: Producer,
    C: Consumer<Item = P::Item, Final = P::Final>,
{
    let mut piped = 0;
    while piped < count {
        match producer.produce().await {
            Ok(Either::Left(item)) => {
                match consumer.consume(item).await {
                    Ok(()) => {
                        piped += 1;
                        // Then continues with next loop iteration.
                    }
                    Err(consumer_error) => {
                        return Err(PipeError::Consumer(consumer_error));
                    }
                }
            }
            Ok(Either::Right(final_value)) => match consumer.close(final_value).await {
                Ok(()) => {
                    return Ok(piped);
                }
                Err(consumer_error) => {
                    return Err(PipeError::Consumer(consumer_error));
                }
            },
            Err(producer_error) => {
                return Err(PipeError::Producer(producer_error));
            }
        }
    }

    Ok(piped)
}

/// Efficiently pipes at most `count` many items from a [`BulkProducer`] into a [`BulkConsumer`] [`consumer.bulk_consume`](BulkConsumer::bulk_consume),
/// and reports how many items were piped.
/// The producer has emitted its final item (which was then used to close the
/// consumer) if and only if the number of returned items is strictly less
/// than `count`.
pub async fn bulk_pipe_at_most<P, C>(
    producer: &mut P,
    consumer: &mut C,
    count: usize,
) -> Result<usize, PipeError<P::Error, C::Error>>
where
    P: BulkProducer,
    P::Item: Clone,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
{
    let mut piped = 0;
    while piped < count {
        match producer.expose_items().await {
            Ok(Either::Left(slots)) => {
                let max_slots = min(count - piped, slots.len());
                let amount = match consumer.bulk_consume(&slots[..max_slots]).await {
                    Ok(amount) => amount,
                    Err(consumer_error) => return Err(PipeError::Consumer(consumer_error)),
                };
                match producer.consider_produced(amount).await {
                    Ok(()) => {
                        piped += amount;
                        // Then continues with next loop iteration.
                    }
                    Err(producer_error) => return Err(PipeError::Producer(producer_error)),
                };
            }
            Ok(Either::Right(final_value)) => {
                match consumer.close(final_value).await {
                    Ok(()) => return Ok(piped),
                    Err(consumer_error) => return Err(PipeError::Consumer(consumer_error)),
                };
            }
            Err(producer_error) => {
                return Err(PipeError::Producer(producer_error));
            }
        }
    }

    Ok(piped)
}
