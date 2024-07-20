// #![no_std]
#![feature(maybe_uninit_write_slice)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_slice)]
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
//! - fully analogous APIs for synchronous and asynchronous code,
//! - the ability to chain sequences of heterogenous types, and
//! - `nostd` support.
//!
//! You can read an in-depth discussion of the API designs [here](https://github.com/AljoschaMeyer/lazy_on_principle/blob/main/main.pdf).
//!
//! ## Core Abstractions
//!
//! Ufotofu is built around a small hierarchy of traits that describe how to produce or consume a sequence item by item.
//!
//! A [`Producer`](sync::Producer) provides the items of a sequence to some client code, similar to the [`futures::Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html) or the [`core::iter::Iterator`] traits. Client code can repeatedly request the next item, and receives either another item, an error, or a dedicated *final* item which may be of a different type than the repeated items. An *iterator* of `T`s corresponds to a *producer* of `T`s with final item type `()` and error type `!`.
//!
//! A [`Consumer`](sync::Consumer) accepts the items of a sequence from some client code, similar to the [`futures::Sink`](https://docs.rs/futures/latest/futures/sink/trait.Sink.html) traits. Client code can repeatedly add new items to the sequence, until it adds a single *final* item which may be of a different type than the repeated items. A final item type of `()` makes adding the final item equivalent to calling a conventional [`close`](https://docs.rs/futures/latest/futures/sink/trait.Sink.html#tymethod.poll_close) method.
//!
//! Producers and consumers are fully dual; the [pipe](sync::pipe) function writes as much data as possible from a producer into a consumer.
//!
//! Consumers often buffer items in an internal queue before performing side-effects on data in larger chunks, such as writing data to the network only once a full packet can be filled. The [`BufferedConsumer`](sync::BufferedConsumer) trait extends the [`Consumer`](sync::Consumer) trait to allow client code to trigger effectful flushing of internal buffers. Dually, the [`BufferedProducer`](sync::BufferedProducer) trait extends the [`Producer`](sync::Producer) trait to allow client code to trigger effectful prefetching of data into internal buffers.
//!
//! Finally, the [`BulkProducer`](sync::BulkProducer) and [`BulkConsumer`](sync::BulkConsumer) traits extend [`BufferedProducer`](sync::BufferedProducer) and [`BufferedConsumer`](sync::BufferedConsumer) respectively with the ability to operate on whole slices of items at a time, similar to [`std::io::Read`] and [`std::io::Write`]. The [bulk_pipe](sync::bulk_pipe) function leverages this ability to efficiently pipe data — unlike the standard library's [Read](std::io::Read) and [Write](std::io::Write) traits, this is possible without allocating an auxiliary buffer.
//!
//! ## Crate Organisation
//!
//! The ufotofu crate is split into three high-level modules:
//!
//! - [`sync`] provides APIs for synchronous, blocking abstractions (think [`core::iter::Iterator`]),
//! - [`local_nb`] provides [`Future`](core::future::Future)-based, non-blocking APIs (think [`futures::stream::Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html)) for *single-threaded* executors, and
//! - [`nb`] provides [`Future`](core::future::Future)-based, non-blocking APIs for *multi-threaded* executors.
//!
//! All three modules implement the same concepts; the only differences are whether functions are asynchronous, and, if so, whether futures implement [`Send`]. In particular, each module has its own version of the core traits for interacting with sequences.
//!
//! The [`nb`] module lacks most features of the [`sync`] and [`local_nb`] modules, but the core trait definitions are there, and we happily accept pull-requests.
//!
//! ## Feature Flags
//!
//! Ufotofu gates several features that are only interesting under certain circumstances behind feature flags. These API docs document *all* functionality, though, as if all feature flags were activated.
//!
//! All functionality that relies on the Rust standard library is gated behind the `std` feature flag (enabled by default).
//!
//! All functionality that performs dynamic memory allocations is gated behind the `alloc` feature flag (disabled by default, implied by the `std` feature).
//!
//! All functionality that aids in testing and development is gated behind the `dev` feature flag (disabled by default).

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

// This allows macros to use `ufotofu` instead of `crate`, which might become
// convenient some day.
extern crate self as ufotofu;

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

/// Functionality shared between several of the three core modules ([sync], [local_nb], and [nb]). You can safaely ignore this module, all functionality is exported amongst [sync], [local_nb], and [nb].
pub mod common;

pub(crate) fn maybe_uninit_slice_mut<T>(s: &mut [T]) -> &mut [MaybeUninit<T>] {
    let ptr = s.as_mut_ptr().cast::<MaybeUninit<T>>();
    unsafe { core::slice::from_raw_parts_mut(ptr, s.len()) }
}
