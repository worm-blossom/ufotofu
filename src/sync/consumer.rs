//! Useful functionality for working with consumers.
//!
//! ## Obtaining Consumers
//!
//! The [SliceConsumer] consumes items into a given mutable slice.
//!
//! The [IntoVec] consumer consumes items into a `Vec` that grows to fit an arbitrary number of items. The [IntoVecFallible] consumer does the same, but reports memory allocation errors instead of panicking.
//!
//! ## Development Helpers
//!
//! The [Invariant] adaptor wraps any consumer and makes it panic during tests when some client code violates the API contracts imposed by the consumer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom consumers; all consumers in the ufotofu crate use this wrapper internally as well.
//!
//! The [Scramble] adaptor exists for testing purposes only; it turns a "sensible" pattern of `consume`, `bulk_consume` and `flush` calls into a much wilder (but still valid) pattern of method calls on the wrapped consumer. This is useful for testing corner-cases (you'd rarely write test code that flushes  multple times in succession by hand, for example). To generate the method call patterns, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).
//!
//! The [TestConsumer] exists for fuzz testing purposes only. It consumes items until randomly emitting an error.
//!
//! ## Writing into Consumers
//!
//! The [pipe_from_slice] and [bulk_pipe_from_slice] functions try make a (bulk) consumer consume all data from a slice; using a bulk producer is more efficient.

// mod pipe_from_slice;
mod slice_consumer;

#[cfg(any(feature = "dev", doc))]
mod scramble;
#[cfg(any(feature = "dev", doc))]
mod test_consumer;

// pub use pipe_from_slice::*;
pub use slice_consumer::{SliceConsumer_ as SliceConsumer, SliceConsumerFullError};

#[cfg(any(feature = "dev", doc))]
pub use scramble::{ConsumeOperations, Scramble};
#[cfg(any(feature = "dev", doc))]
pub use test_consumer::TestConsumer;




pub use crate::common::consumer::Invariant;

#[cfg(feature = "alloc")]
pub use crate::common::consumer::{IntoVec, IntoVecError, IntoVecFallible};