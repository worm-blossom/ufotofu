//! Useful functionality for working with consumers.
//! 
//! ## Obtaining Consumers
//! 
//! The [SliceConsumer] consumes items into a given mutable slice.
//! 
//! The [IntoVec] consumer consumes items into a `Vec` that grows to fit an arbitrary number of items. The [IntoVecFallible] consumer does the same, but reports memory allocation errors instead of panicking.
//! 
//! ## Adaptors
//! 
//! TODO doc sync_to_nb_local here
//! 
//! ## Development Helpers
//! 
//! The [Invariant] adaptor wraps any consumer and makes it panic during tests when some client code violates the API contracts imposed by the consumer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom consumers; all consumers in the ufotofu crate use this wrapper internally as well.
//! 
//! The [Scramble] adaptor exists for testing purposes only; it turns a "sensible" pattern of `consume`, `bulk_consume` and `flush` calls into a much wilder (but still valid) pattern of method calls on the wrapped consumer. This is useful for testing corner-cases (you'd rarely write test code that flushes  multple times in succession by hand, for example). To generate the method call patterns, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).

#[cfg(any(feature = "std", feature = "alloc"))]
mod into_vec;
#[cfg(any(feature = "std", feature = "alloc"))]
mod into_vec_fallible;

mod invariant;
mod invariant_noop;
mod slice_consumer;

#[cfg(any(feature = "dev", doc))]
mod scramble;

#[cfg(any(feature = "std", feature = "alloc"))]
pub use into_vec::IntoVec;
#[cfg(any(feature = "std", feature = "alloc"))]
pub use into_vec_fallible::IntoVecFallible;

pub use slice_consumer::SliceConsumer;

#[cfg(any(feature = "dev", doc))]
pub use scramble::{ConsumeOperations, Scramble};

// During testing we use a wrapper which panics on invariant transgressions.
// The no-op version of the wrapper is used for production code compilation.
#[cfg(test)]
pub use invariant::Invariant;
#[cfg(not(test))]
pub use invariant_noop::Invariant;