#![no_std]
#![cfg_attr(feature = "nightly", feature(allocator_api))]
#![cfg_attr(feature = "nightly", feature(try_with_capacity))]

//! A [trait](Queue) and implementations of non-blocking, infallible [FIFO queues](https://en.wikipedia.org/wiki/Queue_(abstract_data_type)) that support bulk enqueueing and bulk dequeueing via APIs inspired by [ufotofu](https://crates.io/crates/ufotofu).
//!
//! ## Queue Implementations
//!
//! So far, there are two implementations:
//!
//! - [`Fixed`], which is a heap-allocated ring-buffer of unchanging capacity. It is gated behind the `std` or `alloc` feature, the prior of which is enabled by default.
//! - [`Static`], which works exactly like [`Fixed`], but is backed by an array of static capacity. It requires no allocations.
//!
//! Future plans include an elastic queue that grows and shrinks its capacity within certain parameters, to free up memory under low load.
//!
//! ## Features
//!
//! The `std` and `alloc` features control functionality that relies on the standard library or dynamic memory allocation respectively.
//!
//! The `nightly` features enables allocator-aware APIs and some optimisations that rely on nightly APIs.

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(any(feature = "std", feature = "alloc"))]
mod fixed;
#[cfg(any(feature = "std", feature = "alloc"))]
pub use fixed::Fixed;

mod static_;
pub use static_::Static;

use core::cmp::min;

/// A first-in-first-out queue. Provides methods for bulk transfer of items similar to [ufotofu](https://crates.io/crates/ufotofu) [`BulkProducer`](https://docs.rs/ufotofu/0.1.0/ufotofu/sync/trait.BulkProducer.html)s and [`BulkConsumer`](https://docs.rs/ufotofu/0.1.0/ufotofu/sync/trait.BulkConsumer.html)s.
pub trait Queue {
    /// The type of items to manage in the queue.
    type Item: Clone;

    /// Returns the number of items currently in the queue.
    fn len(&self) -> usize;

    /// Returns whether the queue is empty. Must return `true` if and only if `self.len()` returns `0`.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Attempts to enqueue an item.
    ///
    /// Will return the item instead of enqueueing it if the queue is full at the time of calling.
    fn enqueue(&mut self, item: Self::Item) -> Option<Self::Item>;

    /// A low-level method for enqueueing multiple items at a time. If you are only *working* with
    /// queues (rather than implementing them yourself), you will probably want to ignore this method
    /// and use [Queue::bulk_enqueue] instead.
    ///
    /// Exposes a non-empty slice of memory for the client code to fill with items that should
    /// be enqueued. To be used together with [Queue::consider_enqueued].
    ///
    /// Will return `None` if the queue is full at the time of calling.
    fn expose_slots(&mut self) -> Option<&mut [Self::Item]>;

    /// A low-level method for enqueueing multiple items at a time. If you are only *working* with
    /// queues (rather than implementing them yourself), you will probably want to ignore this method
    /// and use [Queue::bulk_enqueue] instead.
    ///
    /// Informs the queue that `amount` many items have been written to the first `amount`
    /// indices of the `expose_slots` it has most recently exposed. The semantics must be
    /// equivalent to those of `enqueue` being called `amount` many times with exactly those
    /// items.
    ///
    /// #### Invariants
    ///
    /// Callers must have written into (at least) the `amount` many first `expose_slots` that
    /// were most recently exposed. Failure to uphold this invariant may cause undefined behavior.
    ///
    /// Calles must not have modified any `expose_slots` other than the first `amount` many.
    /// Failure to uphold this invariant may cause undefined behavior.
    fn consider_enqueued(&mut self, amount: usize);

    /// Enqueues a non-zero number of items by reading them from a given buffer and returning how
    /// many items were enqueued.
    ///
    /// Will return `0` if the queue is full at the time of calling.
    ///
    /// #### Implementation Notes
    ///
    /// The default implementation orchestrates `expose_slots` and `consider_queued` in a
    /// straightforward manner. Only provide your own implementation if you can do better
    /// than that.
    fn bulk_enqueue(&mut self, buffer: &[Self::Item]) -> usize {
        match self.expose_slots() {
            None => 0,
            Some(slots) => {
                let amount = min(slots.len(), buffer.len());
                slots[..amount].clone_from_slice(&buffer[..amount]);
                self.consider_enqueued(amount);

                amount
            }
        }
    }

    /// Attempts to dequeue the next item.
    ///
    /// Will return `None` if the queue is empty at the time of calling.
    fn dequeue(&mut self) -> Option<Self::Item>;

    /// A low-level method for dequeueing multiple items at a time. If you are only *working* with
    /// queues (rather than implementing them yourself), you will probably want to ignore this method
    /// and use [Queue::bulk_dequeue] instead.
    ///
    /// Exposes a non-empty slice of items to be dequeued.
    /// The items in the slice must not have been emitted by `dequeue` before.
    /// To be used together with [Queue::consider_dequeued].
    ///
    /// Will return `None` if the queue is empty at the time of calling.
    fn expose_items(&mut self) -> Option<&[Self::Item]>;

    /// A low-level method for dequeueing multiple items at a time. If you are only *working* with
    /// queues (rather than implementing them yourself), you will probably want to ignore this method
    /// and use [Queue::bulk_dequeue] instead.
    ///
    /// Marks `amount` many items as having been dequeued. Future calls to `dequeue` and to
    /// `expose_items` must act as if `dequeue` had been called `amount` many times.
    ///     
    /// #### Invariants
    ///
    /// Callers must not mark items as dequeued that had not previously been exposed by `expose_items`.
    fn consider_dequeued(&mut self, amount: usize);

    /// Dequeues a non-zero number of items by writing them into a given buffer and returning how
    /// many items were dequeued.
    ///
    /// Will return `0` if the queue is empty at the time of calling.
    ///
    /// #### Implementation Notes
    ///
    /// The default implementation orchestrates `expose_items` and `consider_dequeued` in a
    /// straightforward manner. Only provide your own implementation if you can do better
    /// than that.
    fn bulk_dequeue(&mut self, buffer: &mut [Self::Item]) -> usize {
        match self.expose_items() {
            None => 0,
            Some(slots) => {
                let amount = min(slots.len(), buffer.len());
                buffer[..amount].clone_from_slice(&slots[..amount]);
                self.consider_dequeued(amount);

                amount
            }
        }
    }
}
