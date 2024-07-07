//! Useful functionality for working with producers.
//!
//! ## Obtaining Producers
//!
//! The [SliceProducer] produces items from a given slice.
//!
//! ## Development Helpers
//!
//! The [Invariant] adaptor wraps any producer and makes it panic during tests when some client code violates the API contracts imposed by the producer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom producers; all producers in the ufotofu crate use this wrapper internally as well.
//!
//! The [Scramble] adaptor exists for testing purposes only; it turns a "sensible" pattern of `produce`, `bulk_produce` and `slurp` calls into a much wilder (but still valid) pattern of method calls on the wrapped producer. This is useful for testing corner-cases (you'd rarely write test code that slurps multple times in succession by hand, for example). To generate the method call patterns, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).
//! Todo: Add docs for test_producer
//!
//! ## Reading from Producers
//!
//! The [pipe_into_slice] and [bulk_pipe_into_slice] functions try to fill a slice from a (bulk) producer; using a bulk producer is more efficient. The [pipe_into_slice_uninit] and [bulk_pipe_into_slice_uninit] variants do not require the slice to be initialised first.

mod invariant;
mod invariant_noop;
mod pipe_into_slice;
mod slice_producer;

#[cfg(any(feature = "dev", doc))]
mod scramble;

pub use pipe_into_slice::*;
pub use slice_producer::SliceProducer_ as SliceProducer;

#[cfg(any(feature = "dev", doc))]
pub use scramble::{ProduceOperations, Scramble};

// During testing we use a wrapper which panics on invariant transgressions.
// The no-op version of the wrapper is used for production code compilation.
#[cfg(test)]
pub use invariant::Invariant;
#[cfg(not(test))]
pub use invariant_noop::Invariant;

#[cfg(any(feature = "dev", doc))]
mod test_producer;
#[cfg(any(feature = "dev", doc))]
pub use test_producer::*;

mod from_vec;
pub use from_vec::FromVec_ as FromVec;
