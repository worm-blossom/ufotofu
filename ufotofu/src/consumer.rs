//! Useful functionality for working with consumers.
//!
//! ## Development Helpers
//!
//! The [Invariant] adaptor wraps any consumer and makes it panic during tests when some client code violates the API contracts imposed by the consumer traits. In production builds, the wrapper does nothing and compiles away without any overhead. We recommend using this wrapper as an implementation detail of all custom consumers; all consumers in the ufotofu crate use this wrapper internally.
//!
//! The [TestConsumer] exists for testing code that interacts with arbitrary consumers; it provides customisable behavior of how many items to consume before emitting a configurable error, and varies the sizes of bulk buffers it exposes. To generate various configurations, we recommed using a [fuzzer](https://rust-fuzz.github.io/book/introduction.html).

#[macro_use]
mod macros;

// mod into_slice;
// pub use into_slice::IntoSlice_ as IntoSlice;

#[cfg(feature = "alloc")]
mod into_vec;
#[cfg(feature = "alloc")]
use into_vec::IntoVec;

#[cfg(feature = "alloc")]
mod into_vec_fallible;
#[cfg(feature = "alloc")]
use into_vec_fallible::IntoVecFallible_ as IntoVecFallible;

#[cfg(test)]
mod invariant;
#[cfg(not(test))]
mod invariant_noop;
#[cfg(test)]
pub use invariant::Invariant;
#[cfg(not(test))]
pub use invariant_noop::Invariant;

// #[cfg(feature = "dev")]
// mod scramble;
// #[cfg(feature = "dev")]
// pub use scramble::{ConsumeOperations, Scramble_ as Scramble};

#[cfg(all(feature = "dev", feature = "alloc"))]
mod test_consumer;
#[cfg(all(feature = "dev", feature = "alloc"))]
pub use test_consumer::{TestConsumerBuilder, TestConsumer_ as TestConsumer};
