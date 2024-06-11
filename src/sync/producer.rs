mod invariant;
mod invariant_noop;
mod scramble;
mod slice_producer;

pub use scramble::{ProduceOperations, Scramble};
pub use slice_producer::SliceProducer;

// During testing we use a wrapper which panics on invariant transgressions.
// The no-op version of the wrapper is used for production code compilation.
#[cfg(test)]
pub use invariant::Invariant;
#[cfg(not(test))]
pub use invariant_noop::Invariant;
