#![no_std]
#![feature(allocator_api)]
#![cfg_attr(feature = "nightly", feature(new_uninit))]
#![cfg_attr(feature = "nightly", feature(maybe_uninit_slice))]
#![cfg_attr(feature = "nightly", feature(maybe_uninit_write_slice))]

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
use core::mem::MaybeUninit;

/// A first-in-first-out queue. Provides methods for bulk transfer of items similar to [ufotofu](https://crates.io/crates/ufotofu) [`BulkProducer`](https://docs.rs/ufotofu/0.1.0/ufotofu/sync/trait.BulkProducer.html)s and [`BulkConsumer`](https://docs.rs/ufotofu/0.1.0/ufotofu/sync/trait.BulkConsumer.html)s.
pub trait Queue {
    /// The type of items to manage in the queue.
    type Item: Copy;

    /// Return the number of items currently in the queue.
    fn len(&self) -> usize;

    /// Return whether the queue is empty. Must return `true` if and only if `self.len()` returns `0`.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Attempt to enqueue an item.
    ///
    /// Will return the item instead of enqueueing it if the queue is full at the time of calling.
    fn enqueue(&mut self, item: Self::Item) -> Option<Self::Item>;

    /// A low-level method for enqueueing multiple items at a time. If you are only *working* with
    /// queues (rather than implementing them yourself), you will probably want to ignore this method
    /// and use [Queue::bulk_enqueue] instead.
    ///
    /// Expose a non-empty slice of memory for the client code to fill with items that should
    /// be enqueued. To be used together with [Queue::consider_enqueued].
    ///
    /// Will return `None` if the queue is full at the time of calling.
    fn expose_slots(&mut self) -> Option<&mut [MaybeUninit<Self::Item>]>;

    /// A low-level method for enqueueing multiple items at a time. If you are only *working* with
    /// queues (rather than implementing them yourself), you will probably want to ignore this method
    /// and use [Queue::bulk_enqueue] instead.
    ///
    /// Inform the queue that `amount` many items have been written to the first `amount`
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
    ///
    /// #### Safety
    ///
    /// The queue implementation may assume the first `amount` many `expose_slots` that were most recently
    /// exposed to contain initialized memory after this call, even if the memory it exposed was
    /// originally uninitialized. Violating the invariants can cause the queue to read undefined
    /// memory, which triggers undefined behavior.
    ///
    /// Further, the queue implementation my assume any `expose_slots` slots beyond the first `amount` many
    /// to remain unchanged. In particular, the implementation may assume that those slots have *not*
    /// been set to [`MaybeUninit::uninit`].
    unsafe fn consider_enqueued(&mut self, amount: usize);

    /// Enqueue a non-zero number of items by reading them from a given buffer and returning how
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
                copy_from_slice(&mut slots[..amount], &buffer[..amount]);
                unsafe {
                    self.consider_enqueued(amount);
                }

                amount
            }
        }
    }

    /// Attempt to dequeue the next item.
    ///
    /// Will return `None` if the queue is empty at the time of calling.
    fn dequeue(&mut self) -> Option<Self::Item>;

    /// A low-level method for dequeueing multiple items at a time. If you are only *working* with
    /// queues (rather than implementing them yourself), you will probably want to ignore this method
    /// and use [Queue::bulk_dequeue] or [Queue::bulk_dequeue_uninit] instead.
    ///
    /// Expose a non-empty slice of items to be dequeued.
    /// The items in the slice must not have been emitted by `dequeue` before.
    /// To be used together with [Queue::consider_dequeued].
    ///
    /// Will return `None` if the queue is empty at the time of calling.
    fn expose_items(&mut self) -> Option<&[Self::Item]>;

    /// A low-level method for dequeueing multiple items at a time. If you are only *working* with
    /// queues (rather than implementing them yourself), you will probably want to ignore this method
    /// and use [Queue::bulk_dequeue] or [Queue::bulk_dequeue_uninit] instead.
    ///
    /// Mark `amount` many items as having been dequeued. Future calls to `dequeue` and to
    /// `expose_items` must act as if `dequeue` had been called `amount` many times.
    ///     
    /// #### Invariants
    ///
    /// Callers must not mark items as dequeued that had not previously been exposed by `expose_items`.
    fn consider_dequeued(&mut self, amount: usize);

    /// Dequeue a non-zero number of items by writing them into a given buffer and returning how
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
                buffer[..amount].copy_from_slice(&slots[..amount]);
                self.consider_dequeued(amount);

                amount
            }
        }
    }

    /// Dequeue a non-zero number of items by writing them into a given buffer of possible
    /// uninitialised memory and returning how many items were dequeued.
    ///
    /// Will return `0` if the queue is empty at the time of calling.
    ///
    /// #### Implementation Notes
    ///
    /// The default implementation orchestrates `expose_items` and `consider_dequeued` in a
    /// straightforward manner. Only provide your own implementation if you can do better
    /// than that.
    fn bulk_dequeue_uninit(&mut self, buffer: &mut [MaybeUninit<Self::Item>]) -> usize {
        match self.expose_items() {
            None => 0,
            Some(slots) => {
                let amount = min(slots.len(), buffer.len());
                copy_from_slice(&mut buffer[..amount], &slots[..amount]);
                self.consider_dequeued(amount);

                amount
            }
        }
    }
}

#[cfg(not(feature = "nightly"))]
#[inline(always)]
pub(crate) const unsafe fn slice_assume_init_ref<T>(slice: &[MaybeUninit<T>]) -> &[T] {
    // SAFETY: casting `slice` to a `*const [T]` is safe since the caller guarantees that
    // `slice` is initialized, and `MaybeUninit` is guaranteed to have the same layout as `T`.
    // The pointer obtained is valid since it refers to memory owned by `slice` which is a
    // reference and thus guaranteed to be valid for reads.
    unsafe { &*(slice as *const [MaybeUninit<T>] as *const [T]) }
}

#[cfg(feature = "nightly")]
#[inline(always)]
pub(crate) const unsafe fn slice_assume_init_ref<T>(slice: &[MaybeUninit<T>]) -> &[T] {
    MaybeUninit::slice_assume_init_ref(slice)
}

#[cfg(not(feature = "nightly"))]
#[inline(always)]
unsafe fn slice_assume_init_mut<T>(slice: &mut [MaybeUninit<T>]) -> &mut [T] {
    // SAFETY: similar to safety notes for `slice_get_ref`, but we have a
    // mutable reference which is also guaranteed to be valid for writes.
    unsafe { &mut *(slice as *mut [MaybeUninit<T>] as *mut [T]) }
}

#[cfg(not(feature = "nightly"))]
pub(crate) fn copy_from_slice<'a, T>(this: &'a mut [MaybeUninit<T>], src: &[T]) -> &'a mut [T]
where
    T: Copy,
{
    // SAFETY: &[T] and &[MaybeUninit<T>] have the same layout
    let uninit_src: &[MaybeUninit<T>] = unsafe { core::mem::transmute(src) };

    this.copy_from_slice(uninit_src);

    // SAFETY: Valid elements have just been copied into `this` so it is initialized
    unsafe { slice_assume_init_mut(this) }
}

#[cfg(feature = "nightly")]
pub(crate) fn copy_from_slice<'a, T>(this: &'a mut [MaybeUninit<T>], src: &[T]) -> &'a mut [T]
where
    T: Copy,
{
    MaybeUninit::copy_from_slice(this, src)
}
