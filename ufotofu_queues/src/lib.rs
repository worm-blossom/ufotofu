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

/// A first-in-first-out queue. Provides methods for bulk transfer of items similar to [ufotofu](https://crates.io/crates/ufotofu) [`BulkProducer`](https://docs.rs/ufotofu/0.1.0/ufotofu/sync/trait.BulkProducer.html)s and [`BulkConsumer`](https://docs.rs/ufotofu/0.1.0/ufotofu/sync/trait.BulkConsumer.html)s.
pub trait Queue {
    /// The type of items to manage in the queue.
    type Item;

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

    /// Enqueues a non-zero number of items by reading them from a given buffer and returning how
    /// many items were enqueued.
    ///
    /// Will return `0` if the queue is full at the time of calling.
    fn bulk_enqueue(&mut self, buffer: &[Self::Item]) -> usize;

    /// Attempts to dequeue the next item.
    ///
    /// Will return `None` if the queue is empty at the time of calling.
    fn dequeue(&mut self) -> Option<Self::Item>;

    /// Dequeues a non-zero number of items by writing them into a given buffer and returning how
    /// many items were dequeued.
    ///
    /// Will return `0` if the queue is empty at the time of calling.
    fn bulk_dequeue(&mut self, buffer: &mut [Self::Item]) -> usize;
}
