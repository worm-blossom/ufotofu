mod into_vec;
mod into_vec_fallible;
mod invariant;
mod invariant_noop;
mod scramble;
mod slice_consumer;

pub use into_vec::IntoVec;
pub use into_vec_fallible::{IntoVecError, IntoVecFallible};
pub use scramble::{ConsumeOperations, Scramble};
pub use slice_consumer::{SliceConsumer, SliceConsumerFullError};

// During testing we use a wrapper which panics on invariant transgressions.
// The no-op version of the wrapper is used for production code compilation.
#[cfg(test)]
pub use invariant::Invariant;
#[cfg(not(test))]
pub use invariant_noop::Invariant;
