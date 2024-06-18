use core::convert::{AsMut, AsRef};

use either::Either;
use wrapper::Wrapper;

use crate::local_nb::{BufferedProducer, BulkProducer, Producer};

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
///   - `producer_slots`
///   - `did_produce`
///   - `bulk_produce`
/// - Must not call any of the prior functions after any of them had returned
///   an error.
/// - Must not call `did_produce` with an amount exceeding the number of available slots
#[derive(Debug, Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct Invariant<P> {
    /// An implementer of the `Producer` traits.
    inner: P,
}

impl<P> Invariant<P> {
    /// Return a `Producer` that behaves exactly like the wrapped `Producer`
    /// `inner`.
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

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.inner.produce().await
    }
}

impl<P, T, F, E> BufferedProducer for Invariant<P>
where
    P: BufferedProducer<Item = T, Final = F, Error = E>,
{
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.inner.slurp().await
    }
}

impl<P, T, F, E> BulkProducer for Invariant<P>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    async fn producer_slots<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        T: 'a,
    {
        self.inner.producer_slots().await
    }

    async fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.did_produce(amount).await
    }
}
