mod invariant;
mod invariant_noop;
mod slice_producer;

#[cfg(feature = "dev")]
mod scramble;

pub use slice_producer::SliceProducer;

#[cfg(feature = "dev")]
pub use scramble::{ProduceOperations, Scramble};

// During testing we use a wrapper which panics on invariant transgressions.
// The no-op version of the wrapper is used for production code compilation.
#[cfg(test)]
pub use invariant::Invariant;
#[cfg(not(test))]
pub use invariant_noop::Invariant;
