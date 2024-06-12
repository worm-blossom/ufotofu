#[cfg(any(feature = "std", feature = "alloc"))]
mod into_vec;
#[cfg(any(feature = "std", feature = "alloc"))]
mod into_vec_fallible;

mod invariant;
mod invariant_noop;
mod slice_consumer;

#[cfg(feature = "dev")]
mod scramble;

#[cfg(any(feature = "std", feature = "alloc"))]
pub use into_vec::IntoVec;
#[cfg(any(feature = "std", feature = "alloc"))]
pub use into_vec_fallible::IntoVecFallible;

pub use slice_consumer::SliceConsumer;

#[cfg(feature = "dev")]
pub use scramble::{ConsumeOperations, Scramble};
