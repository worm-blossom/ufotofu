//! Useful functionality for working with producers.
//!
//! ## Obtaining Producers
//!
//! The [SliceProducer] produces items from a given slice.
//!
//! ## Adaptors
//!
//! The [SyncToLocalNb] adaptor allows you to use a [`sync::Producer`](crate::sync::Producer) as a [`local_nb::Producer`](crate::local_nb::Producer).
//!
//! ## Development Helpers
//!
//! The [Invariant] adaptor wraps any producer and makes it panic during tests when some client code violates the API contracts imposed by the producer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom producers; all producers in the ufotofu crate use this wrapper internally as well.
//!
//! The [Scramble] adaptor exists for testing purposes only; it turns a "sensible" pattern of `produce`, `bulk_produce` and `slurp` calls into a much wilder (but still valid) pattern of method calls on the wrapped producer. This is useful for testing corner-cases (you'd rarely write test code that slurps multiple times in succession by hand, for example). To generate the method call patterns, we recommend using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).
//! 
//! ## Reading from Producers
//!
//! The [pipe_into_slice] and [bulk_pipe_into_slice] functions try to fill a slice from a (bulk) producer; using a bulk producer is more efficient. The [pipe_into_slice_uninit] and [bulk_pipe_into_slice_uninit] variants do not require the slice to be initialised first.

mod sync_to_local_nb;

#[cfg(any(feature = "dev", doc))]
mod scramble;

pub use sync_to_local_nb::SyncToLocalNb;

#[cfg(any(feature = "dev", doc))]
pub use scramble::{ProduceOperations, Scramble};

#[cfg(any(feature = "dev", doc))]
mod test_producer;
#[cfg(any(feature = "dev", doc))]
pub use test_producer::*;

mod from_vec;
pub use from_vec::*;






pub use crate::common::producer::{Invariant, FromSlice};