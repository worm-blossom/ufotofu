//! Useful functionality for working with consumers.
//!
//! ## Obtaining Consumers
//!
//! The [IntoSlice] consumes items into a given mutable slice.
//!
//! The [IntoVec] consumer consumes items into a `Vec` that grows to fit an arbitrary number of items. The [IntoVecFallible] consumer does the same, but reports memory allocation errors instead of panicking.
//!
//! ## Adaptors
//!
//! The [SyncToLocalNb] adaptor allows you to use a [`sync::Consumer`](crate::sync::Consumer) as a [`local_nb::Consumer`](crate::local_nb::Consumer).
//!
//! ## Development Helpers
//!
//! Use the [TestConsumer] for testing code that interacts with arbitrary consumers.
//!
//! The [Invariant] adaptor wraps any consumer and makes it panic during tests when some client code violates the API contracts imposed by the consumer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom consumers; all consumers in the ufotofu crate use this wrapper internally as well.
//!
//! The [Scramble] adaptor exists for testing purposes only; it turns a "sensible" pattern of `consume`, `bulk_consume` and `flush` calls into a much wilder (but still valid) pattern of method calls on the wrapped consumer. This is useful for testing corner-cases (you'd rarely write test code that flushes multiple times in succession by hand, for example). To generate the method call patterns, we recommend using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).

mod sync_to_local_nb;

pub use sync_to_local_nb::SyncToLocalNb;

pub use crate::common::consumer::{IntoSlice, Invariant};

#[cfg(feature = "alloc")]
pub use crate::common::consumer::{IntoVec, IntoVecFallible};

#[cfg(feature = "dev")]
pub use crate::common::consumer::{ConsumeOperations, Scramble};

#[cfg(all(feature = "dev", feature = "alloc"))]
pub use crate::common::consumer::{TestConsumer, TestConsumerBuilder};
