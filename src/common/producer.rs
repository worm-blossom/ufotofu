#[macro_use]
mod macros;

mod from_slice;
pub use from_slice::FromSlice_ as FromSlice;

mod from_boxed_slice;
pub use from_boxed_slice::FromBoxedSlice_ as FromBoxedSlice;

#[cfg(test)]
mod invariant;
#[cfg(not(test))]
mod invariant_noop;
#[cfg(test)]
pub use invariant::Invariant;
#[cfg(not(test))]
pub use invariant_noop::Invariant;

#[cfg(feature = "dev")]
mod scramble;
#[cfg(feature = "dev")]
pub use scramble::{ProduceOperations, Scramble_ as Scramble};

#[cfg(feature = "dev")]
mod test_producer;
#[cfg(feature = "dev")]
pub use test_producer::TestProducer_ as TestProducer;
