#![no_std]
#![feature(maybe_uninit_write_slice)]
#![feature(maybe_uninit_uninit_array)]
#![feature(never_type)]
#![feature(allocator_api)]
#![feature(vec_push_within_capacity)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

use core::mem::MaybeUninit;

/// [`Future`](core::future::Future)-based, non-blocking versions of the ufotofu APIs, for *single-threaded* executors.
///
/// For an introduction and high-level overview, see the [toplevel documentation](crate).
///
/// Core functionality:
///
/// - Traits for producing sequences: [`Producer`](local_nb::Producer), [`BufferedProducer`](local_nb::BufferedProducer), and [`BulkProducer`](local_nb::BulkProducer).
/// - Traits for consuming sequences: [`Consumer`](local_nb::Consumer), [`BufferedConsumer`](local_nb::BufferedConsumer), and [`BulkConsumer`](local_nb::BulkConsumer).
/// - Piping data: [`pipe`](local_nb::pipe) and [`bulk_pipe`](local_nb::bulk_pipe).
///
/// Beyond the core traits, ufotofu offers functionality for working with producers and consumers in the [`producer`](local_nb::producer) and [`consumer`](local_nb::consumer) modules respectively.
pub mod local_nb;
/// [`Future`](core::future::Future)-based, non-blocking versions of the ufotofu APIs, for *multi-threaded* executors.
///
/// For an introduction and high-level overview, see the [toplevel documentation](crate).
///
/// Core functionality:
///
/// - Traits for producing sequences: [`Producer`](nb::Producer), [`BufferedProducer`](nb::BufferedProducer), and [`BulkProducer`](nb::BulkProducer).
/// - Traits for consuming sequences: [`Consumer`](nb::Consumer), [`BufferedConsumer`](nb::BufferedConsumer), and [`BulkConsumer`](nb::BulkConsumer).
/// - Piping data: [`pipe`](nb::pipe) and [`bulk_pipe`](nb::bulk_pipe).
pub mod nb;
/// Synchronous, blocking versions of the ufotofu APIs.
///
/// For an introduction and high-level overview, see the [toplevel documentation](crate).
///
/// Core functionality:
///
/// - Traits for producing sequences: [`Producer`](sync::Producer), [`BufferedProducer`](sync::BufferedProducer), and [`BulkProducer`](sync::BulkProducer).
/// - Traits for consuming sequences: [`Consumer`](sync::Consumer), [`BufferedConsumer`](sync::BufferedConsumer), and [`BulkConsumer`](sync::BulkConsumer).
/// - Piping data: [`pipe`](sync::pipe) and [`bulk_pipe`](sync::bulk_pipe).
///
/// Beyond the core traits, ufotofu offers functionality for working with producers and consumers in the [`producer`](sync::producer) and [`consumer`](sync::consumer) modules respectively.
pub mod sync;

pub(crate) fn maybe_uninit_slice_mut<T>(s: &mut [T]) -> &mut [MaybeUninit<T>] {
    let ptr = s.as_mut_ptr().cast::<MaybeUninit<T>>();
    unsafe { core::slice::from_raw_parts_mut(ptr, s.len()) }
}
