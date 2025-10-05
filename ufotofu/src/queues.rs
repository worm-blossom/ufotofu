//! In-memory queues for adding buffering to arbitrary producers and consumers.
//!
//! This module provides the [`Queue`] trait for infallible in-memory queues supporting bulk push and pop operations. You can safely consider this trait (and this module) a low-level implementation detail you likely will not interact with directly. Queues power some useful functionality, such as creating [buffered](crate::BufferedProducer) [versions](crate::BufferedConsumer) of arbitrary producers and consumers, via the [`ProducerExt::buffered`] and [`ConsumerExt::buffered`] methods.
//!
//! Ufotofu provides one concrete implementations of the [`Queue`] trait: the [`Contiguous`] queue, which uses a single contiguous slice of items for storage. For queue creation, see [`new_contiguous`], [`new_static`], and [`new_fixed`].
//!
//! Functionality relying on queues typically comes with methods which handle queue creation for you. For example, [`ProducerExt::buffered_static`] or [`ProducerExt::buffered_fixed`] and [`ConsumerExt::buffered_static`] or [`ConsumerExt::buffered_fixed`] are specialised versions of [`ProducerExt::buffered`] and [`ConsumerExt::buffered`] respectively.
//!
//! [TODO] examples

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};

use core::cmp::min;

mod contiguous;
pub use contiguous::*;

/// A trait for infallbile first-in-first-out queues.
///
/// Beyond the entirely typical [`enqueue`](Queue::enqueue) and [`dequeue`](Queue::dequeue) methods, this trait also provides ufotofu-style [`expose_slots`](Queue::expose_slots) and [`expose_items`](Queue::expose_items) methods for transferring multiple items with single method calls.
///
/// This trait describes infallible in-memory queues; its methods are synchronous and do not support error reporting.
///
/// Queues must be able to store at least one item. In other words, enqueueing into an empty queue must always succeed.
///
/// See also the [`BoundedQueue`] trait for queues of a known maximum capacity, and the [`QueueExt`] trait for helper methods on all queues.
pub trait Queue {
    /// The type of items to manage in the queue.
    type Item;

    /// Returns the number of items currently in the queue.
    fn len(&self) -> usize;

    /// Returns whether the queue is empty. Must return `true` if and only if `self.len()` returns `0`.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns whether the queue is empty, i.e., whether calling `enqueue` would fail to enqueue an item.
    ///
    /// When this method is called and the queue is full, a dynamically sized queue is encouraged to interpret the call as intent to enqueue more items, and to attempt to increase its size in response.
    fn is_full(&self) -> bool;

    /// Returns the maximum number of items that can be stored in the queue at the same time, or `None` if no upper bound is known.
    fn max_capacity(&self) -> Option<usize>;

    /// Attempts to enqueue an item.
    ///
    /// Will return the item instead of enqueueing it if the queue is full at the time of calling. Enqueueing into an empty queue must always suceed.
    ///
    /// <br/>Counterpart: [`Queue::dequeue`]
    fn enqueue(&mut self, item: Self::Item) -> Option<Self::Item>;

    /// Exposes a non-empty mutable slice of items to an async function, the function mutates it and then reports to the queue how many of these items should now be considered to have been enqueued.
    ///
    /// The slice passed to the async function must be empty if and only if the queue is full at the time.
    ///
    /// The function further returns a value of some arbitrary type `R`, the `expose_slots` method returns that value.
    ///
    /// The function must not return a number greater than the size of the slice with which it was called.
    ///
    /// <br/>Counterpart: the [`Queue::expose_items`] method.
    async fn expose_slots<F, R>(&mut self, f: F) -> R
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R);

    /// Attempts to dequeue the next item.
    ///
    /// Will return `None` if the queue is empty at the time of calling.
    ///
    /// <br/>Counterpart: [`Queue::enqueue`]
    fn dequeue(&mut self) -> Option<Self::Item>;

    /// Exposes a slice of items to an async function, the function then reports to the queue how many of these items should now be considered to have been dequeued.
    ///
    /// The slice passed to the async function must be empty if and only if the queue has no items enqueued at the time.
    ///
    /// The function further returns a value of some arbitrary type `R`, the `expose_items` method returns that value.
    ///
    /// The function must not return a number greater than the size of the slice with which it was called.
    ///
    /// <br/>Counterpart: the [`Queue::expose_slots`] method.
    async fn expose_items<F, R>(&mut self, f: F) -> R
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, R);
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

/// An extension trait for [`Queue`] that provides helper methods.
/// You never need to implement this trait yourself, it merely adds methods with default implementation to existing queues.
pub trait QueueExt: Queue {
    /// Enqueues a number of items, cloned from the given buffer. Returns how many items were enqueued — returns zero when the queue is already full.
    ///
    /// <br/>Counterpart: the [`QueueExt::bulk_dequeue`] method.
    async fn bulk_enqueue(&mut self, buffer: &[Self::Item]) -> usize
    where
        Self::Item: Clone,
    {
        self.expose_slots(async |slots| {
            let amount = min(slots.len(), buffer.len());
            slots[..amount].clone_from_slice(&buffer[..amount]);

            (amount, amount)
        })
        .await
    }

    /// Dequeues a number of items, by cloning them into the given buffer. Returns how many items were dequeued — returns zero when the queue was already already.
    ///
    /// <br/>Counterpart: the [`QueueExt::bulk_enqueue`] method.
    async fn bulk_dequeue(&mut self, buffer: &mut [Self::Item]) -> usize
    where
        Self::Item: Clone,
    {
        self.expose_items(async |items| {
            let amount = min(items.len(), buffer.len());
            buffer[..amount].clone_from_slice(&items[..amount]);

            (amount, amount)
        })
        .await
    }
}

impl<Q> QueueExt for Q where Q: Queue {}

/// Creates a new queue, using the given value of type `S` as a buffer for items of type `T`, where `T: Default`.
///
/// You probably want `S` to implement `AsRef<[T]>` and `AsMut<[T]>`, otherwise the returned [`Contiguous`] does not implement [`Queue`].
///
/// See [`new_contiguous_with`] for queue creation for item types which do not implement [`Default`].[TODO] example
pub fn new_contiguous<S, T>(buffer: S) -> Contiguous<S, T>
where
    T: Default,
{
    Contiguous::new(buffer, Default::default)
}

/// Creates a new queue, using the given value of type `S` as a buffer for items of type `T`.
///
/// The `initialise_memory` function is used internally to ensure that all queue slots contain valid memory at all times. The specific choice of `T` returned by that function does not affect the observable semantics of the queue at all.
///
/// You probably want `S` to implement `AsRef<[T]>` and `AsMut<[T]>`, otherwise the returned [`Contiguous`] does not implement [`Queue`].
///
/// See [`new_contiguous`] for more convenient queue creation for item types implementing [`Default`].[TODO] example
pub fn new_contiguous_with<S, T>(buffer: S, initialise_memory: fn() -> T) -> Contiguous<S, T> {
    Contiguous::new(buffer, initialise_memory)
}

/// Creates a new queue, using a statically sized array as a buffer for items of type `T`, where `T: Default`.
///
/// See [`new_static_with`] for array-backed queue creation for item types which do not implement [`Default`].[TODO] example
pub fn new_static<T, const N: usize>() -> Contiguous<[T; N], T>
where
    T: Default,
{
    Contiguous::new(core::array::from_fn(|_| T::default()), T::default)
}

/// Creates a new queue, using a statically sized array as a buffer for items of type `T`.
///
/// The `initialise_memory` function is used internally to ensure that all queue slots contain valid memory at all times. The specific choice of `T` returned by that function does not affect the observable semantics of the queue at all.
///
/// See [`new_static`] for more convenient static queue creation for item types implementing [`Default`].[TODO] example
pub fn new_static_with<T, const N: usize>(initialise_memory: fn() -> T) -> Contiguous<[T; N], T> {
    Contiguous::new(
        core::array::from_fn(|_| initialise_memory()),
        initialise_memory,
    )
}

/// Creates a new queue, using a `Box<[T]>` as a buffer for items of type `T`, where `T: Default`. The buffer size is fixed at creation.
///
/// See [`new_fixed_with`] for box-backed queue creation for item types which do not implement [`Default`].[TODO] example
#[cfg(feature = "alloc")]
pub fn new_fixed<T>(capacity: usize) -> Contiguous<Box<[T]>, T>
where
    T: Default,
{
    let mut v = Vec::with_capacity(capacity);
    v.resize_with(capacity, Default::default);
    Contiguous::new(v.into_boxed_slice(), T::default)
}

/// Creates a new queue, using a `Box<[T]>` as a buffer for items of type `T`. The buffer size is fixed at creation.
///
/// The `initialise_memory` function is used internally to ensure that all queue slots contain valid memory at all times. The specific choice of `T` returned by that function does not affect the observable semantics of the queue at all.
///
/// See [`new_fixed`] for more convenient box-backed queue creation for item types implementing [`Default`].[TODO] example
#[cfg(feature = "alloc")]
pub fn new_fixed_with<T>(capacity: usize, initialise_memory: fn() -> T) -> Contiguous<Box<[T]>, T> {
    let mut v = Vec::with_capacity(capacity);
    v.resize_with(capacity, initialise_memory);
    Contiguous::new(v.into_boxed_slice(), initialise_memory)
}
