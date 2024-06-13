#![no_std]
#![feature(maybe_uninit_write_slice)]
#![feature(maybe_uninit_uninit_array)]
#![feature(never_type)]
#![feature(allocator_api)]
#![feature(vec_push_within_capacity)]

//! # UFOTOFU
//! 
//! Ufotofu provides APIs for lazily producing or consuming sequences of arbitrary length. Highlights of ufotofu include
//! 
//! - consistent error handling semantics across all supported modes of sequence processing,
//! - meaningful subtyping relations between, for example, streams and readers,
//! - absence of needless specialization of error types or item types,
//! - fully analogous APIs for synchronous and asynchronous code, and
//! - the ability to chain sequences of heterogenous types.
//! 
//! You can read an in-depth discussion of the API designs [here](https://github.com/AljoschaMeyer/lazy_on_principle/blob/main/main.pdf).
//! 
//! ## Crate Organisation
//! 
//! The crate is split into three high-level modules:
//! 
//! - [`sync`] provides APIs for synchronous, blocking abstractions (think [`core::iter::Iterator`]),
//! - [`local_nb`] provides [`Future`](core::future::Future)-based, non-blocking APIs (think [`futures::stream::Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html)) for *single-threaded* executors, and
//! - [`nb`] provides [`Future`](core::future::Future)-based, non-blocking APIs (think [`futures::stream::Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html)) for *multi-threaded* executors.
//! 
//! All three modules implement the same concepts; the only differences are whether functions are asynchronous, and, if so, whether futures implement [`Send`].

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

use core::mem::MaybeUninit;

pub mod local_nb;
pub mod nb;
pub mod sync;

pub(crate) fn maybe_uninit_slice_mut<T>(s: &mut [T]) -> &mut [MaybeUninit<T>] {
    let ptr = s.as_mut_ptr().cast::<MaybeUninit<T>>();
    unsafe { core::slice::from_raw_parts_mut(ptr, s.len()) }
}

// sync/nb/nb_send, basics: producer-buffered-bulk, consumer-buffered-bulk, piping, wrappers, feature flags, queues, converters