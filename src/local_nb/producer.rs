//! Useful functionality for working with producers.
//! 
//! ## Obtaining Producers
//! 
//! The [SliceProducer] produces items from a given slice.
//! 
//! ## Adaptors
//! 
//! The [SyncToLocalNb] adaptor allows you to use a [`sync::Producer`](crate::sync::Producer) as a [`local_nb::LocalProducer`](crate::local_nb::LocalProducer).
//! 
//! ## Development Helpers
//! 
//! The [Invariant] adaptor wraps any producer and makes it panic during tests when some client code violates the API contracts imposed by the producer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom producers; all producers in the ufotofu crate use this wrapper internally as well.
//! 
//! The [Scramble] adaptor exists for testing purposes only; it turns a "sensible" pattern of `produce`, `bulk_produce` and `slurp` calls into a much wilder (but still valid) pattern of method calls on the wrapped producer. This is useful for testing corner-cases (you'd rarely write test code that slurps multple times in succession by hand, for example). To generate the method call patterns, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).

mod sync_to_local_nb;

mod invariant;
mod invariant_noop;
mod slice_producer;

#[cfg(any(feature = "dev", doc))]
mod scramble;

pub use slice_producer::SliceProducer;

pub use sync_to_local_nb::SyncToLocalNb;

#[cfg(any(feature = "dev", doc))]
pub use scramble::{ProduceOperations, Scramble};

// During testing we use a wrapper which panics on invariant transgressions.
// The no-op version of the wrapper is used for production code compilation.
#[cfg(test)]
pub use invariant::Invariant;
#[cfg(not(test))]
pub use invariant_noop::Invariant;