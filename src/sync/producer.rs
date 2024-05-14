mod cursor;
mod invariant;
mod invariant_noop;
mod scramble;

pub use cursor::Cursor;
pub use scramble::{ProduceOperations, Scramble, ScrambleError};

// During testing we use a wrapper which panics on invariant transgressions.
// The no-op version of the wrapper is used for production code compilation.
#[cfg(test)]
pub use invariant::Invariant;
#[cfg(not(test))]
pub use invariant_noop::Invariant;
