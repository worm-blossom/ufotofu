use core::convert::{AsMut, AsRef};

use either::Either;
use wrapper::Wrapper;

use crate::sync::{BufferedProducer, BulkProducer, Producer};

/// A `Producer` wrapper that panics when callers violate API contracts such
/// as halting interaction after an error.
///
/// This wrapper only performs the checks when testing code (more specifically,
/// when `#[cfg(test)]` applies). In production builds, the wrapper does
/// nothing at all and compiles away without any overhead.
///
/// All producers implemented in this crate use this wrapper internally already.
/// We recommend to use this type for all custom producers as well.
///
/// #### Invariants
///
/// The wrapper enforces the following invariants:
///
/// - Must not call any of the following functions after the final item has been returned:
///   - `produce`
///   - `slurp`
///   - `expose_items`
///   - `consider_produced`
///   - `bulk_produce`
/// - Must not call any of the prior functions after any of them had returned
///   an error.
/// - Must not call `consider_produced` with an amount exceeding the number of available slots
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct Invariant<P> {
    /// An implementer of the `Producer` traits.
    inner: P,
}

impl<P: core::fmt::Debug> core::fmt::Debug for Invariant<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<P> Invariant<P> {
    /// Return a `Producer` that behaves exactly like the wrapped `Producer`
    /// `inner`.
    #[allow(dead_code)]
    pub fn new(inner: P) -> Self {
        Invariant { inner }
    }
}

impl<P> AsRef<P> for Invariant<P> {
    fn as_ref(&self) -> &P {
        &self.inner
    }
}

impl<P> AsMut<P> for Invariant<P> {
    fn as_mut(&mut self) -> &mut P {
        &mut self.inner
    }
}

impl<P> Wrapper<P> for Invariant<P> {
    fn into_inner(self) -> P {
        self.inner
    }
}

impl<P, T, F, E> Producer for Invariant<P>
where
    P: Producer<Item = T, Final = F, Error = E>,
{
    type Item = T;
    type Final = F;
    type Error = E;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.inner.produce()
    }
}

impl<P, T, F, E> BufferedProducer for Invariant<P>
where
    P: BufferedProducer<Item = T, Final = F, Error = E>,
{
    fn slurp(&mut self) -> Result<(), Self::Error> {
        self.inner.slurp()
    }
}

impl<P, T, F, E> BulkProducer for Invariant<P>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        self.inner.expose_items()
    }

    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.consider_produced(amount)
    }
}
