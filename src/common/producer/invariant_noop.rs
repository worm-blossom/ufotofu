use either::Either;
use wrapper::Wrapper;

use crate::local_nb::{
    BufferedProducer as BufferedProducerLocalNb, BulkProducer as BulkProducerLocalNb,
    Producer as ProducerLocalNb,
};
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
#[cfg_attr(feature = "dev", derive(arbitrary::Arbitrary))]
pub struct Invariant<P> {
    inner: P,
}

impl<P: core::fmt::Debug> core::fmt::Debug for Invariant<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<P> Invariant<P> {
    /// Return a `Producer` that behaves exactly like the wrapped `Producer`
    /// `inner`, except that - when running tests - it performs runtime
    /// validation of API invariants and panics if they are violated by a
    /// caller.
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

impl<P> Producer for Invariant<P>
where
    P: Producer,
{
    type Item = P::Item;
    type Final = P::Final;
    type Error = P::Error;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.inner.produce()
    }
}

impl<P> BufferedProducer for Invariant<P>
where
    P: BufferedProducer,
{
    fn slurp(&mut self) -> Result<(), Self::Error> {
        self.inner.slurp()
    }
}

impl<P> BulkProducer for Invariant<P>
where
    P: BulkProducer,
    P::Item: Copy,
{
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        self.inner.expose_items()
    }

    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.consider_produced(amount)
    }
}

impl<P: ProducerLocalNb> ProducerLocalNb for Invariant<P> {
    type Item = P::Item;
    type Final = P::Final;
    type Error = P::Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.inner.produce().await
    }
}

impl<P: BufferedProducerLocalNb> BufferedProducerLocalNb for Invariant<P> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.inner.slurp().await
    }
}

impl<P: BulkProducerLocalNb> BulkProducerLocalNb for Invariant<P>
where
    P::Item: Copy,
{
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        P::Item: 'a,
    {
        self.inner.expose_items().await
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.consider_produced(amount).await
    }
}
