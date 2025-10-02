//! In-memory queues for adding buffering to arbitrary producers and consumers.
//!
//! This module provides the [`Queue`] trait for infallible in-memory queues supporting bulk push and pop operations.
//!
//! [`Queues`](Queue) can be used to create [buffered](crate::BufferedProducer) [versions](crate::BufferedConsumer) of arbitrary producers and consumers, via the [`ProducerExt::buffered`] and [`ConsumerExt::buffered`] methods. See their docs for more details.
//!
//! Ufotofu provides two concrete implementations of the [`Queue`] trait:
//!
//! - [`Static`]: A queue of static capacity, set at compile-time.
//! - [`Fixed`]: A queue of fixed capacity, set at creation time.
//!
//! Use [`ProducerExt::buffered_static`] or [`ProducerExt::buffered_fixed`] and [`ConsumerExt::buffered_static`] or [`ConsumerExt::buffered_fixed`] to conveniently use these specific queues to add buffering to any producer or consumer respectively.

mod fixed;
pub use fixed::*;

mod static_;
pub use static_::*;

/// A trait for infallbile first-in-first-out queues.
///
/// Beyond the entirely typical [`enqueue`](Queue::enqueue) and [`dequeue`](Queue::dequeue) methods, this trait also provides [`bulk_enqueue`](Queue::bulk_enqueue) and [`bulk_dequeue`](Queue::bulk_dequeue) methods for transferring multiple items with single method calls.
///
/// This trait describes infallible in-memory queues; its methods are synchronous and do not support error reporting.
pub trait Queue {
    /// The type of items to manage in the queue.
    type Item;

    /// Returns the number of items currently in the queue.
    fn len(&self) -> usize;

    /// Returns whether the queue is empty. Must return `true` if and only if `self.len()` returns `0`.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the maximum number of items that can be stored in the queue at the same time, or `None` if no upper bound is known.
    fn max_capacity(&self) -> Option<usize>;

    /// Attempts to enqueue an item.
    ///
    /// Will return the item instead of enqueueing it if the queue is full at the time of calling.
    fn enqueue(&mut self, item: Self::Item) -> Option<Self::Item>;

    /// Attempts to enqueue a number of items, cloned from the passed non-empty buffer. Returns how many items were enqueued — must return zero when the queue is already full.
    fn bulk_enqueue(&mut self, buffer: &[Self::Item]) -> usize
    where
        Self::Item: Clone;

    /// Attempts to dequeue the next item.
    ///
    /// Will return `None` if the queue is empty at the time of calling.
    fn dequeue(&mut self) -> Option<Self::Item>;

    /// Attempts to dequeue a number of items, moving them into the passed non-empty buffer. Returns how many items were dequeued — must return zero when the queue is empty at the time of calling.
    fn bulk_dequeue(&mut self, buffer: &mut [Self::Item]) -> usize;
}

/// A [`Queue`] of known maximum capacity. Such queues must never return [`None`] from [`max_capacity`](Queue::max_capacity).
pub trait BoundedQueue: Queue {
    /// Returns the maximum number of items that can be stored in the queue at the same time.
    fn bounded_capacity(&self) -> usize {
        self.max_capacity()
            .expect("A bounded queue must not report None as its max_capacity()")
    }

    /// Returns the remaining number of items that could be stored in the queue at the moment.
    fn available_slots(&self) -> usize {
        self.bounded_capacity() - self.len()
    }
}
