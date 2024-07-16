#[cfg(test)]
mod invariant;
#[cfg(not(test))]
mod invariant_noop;
#[cfg(test)]
pub use invariant::Invariant;
#[cfg(not(test))]
pub use invariant_noop::Invariant;