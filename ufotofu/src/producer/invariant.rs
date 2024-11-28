use either::Either;

use crate::{BufferedProducer, BulkProducer, Producer};

/// A `Producer` wrapper that panics when callers violate API contracts such
/// as halting interaction after an error.
///
/// This wrapper only performs the checks while testing (more specifically,
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
/// The wrapper enforces the following invariants:
///
/// - Must not call trait methods of [`Producer`], [`BufferedProducer`], or [`BulkProducer`] after the final item has been returned.
/// - Must not call any of the trait methods after any of them have returned
///   an error.
/// - Must not call [`consider_produced`](BulkProducer::consider_produced) with an amount exceeding the number of available exposed items.
#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
#[cfg_attr(feature = "dev", derive(arbitrary::Arbitrary))]
pub struct Invariant<P> {
    inner: P,
    /// The status of the consumer. `true` while the caller may call trait
    /// methods, `false` once that becomes disallowed (because a method returned
    /// an error, or because `close` was called).
    active: bool,
    /// The maximum `amount` that a caller may supply to `consume_slots`.
    exposed_slots: usize,
}

impl<P: core::fmt::Debug> core::fmt::Debug for Invariant<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<P> Invariant<P> {
    /// Returns a producer that behaves exactly like the wrapped producer
    /// `inner`, except that - when running tests - it performs runtime
    /// validation of API invariants and panics if they are violated by a
    /// caller.
    pub fn new(inner: P) -> Self {
        Invariant {
            inner,
            active: true,
            exposed_slots: 0,
        }
    }

    /// Consumes `self` and returns the wrapped producer.
    pub fn into_inner(self) -> P {
        self.inner
    }

    /// Check the state of the `active` field and panic if the value is
    /// `false`.
    fn check_inactive(&self) {
        if !self.active {
            panic!("may not call `Producer` methods after the sequence has ended");
        }
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

impl<P: Producer> Producer for Invariant<P> {
    type Item = P::Item;
    type Final = P::Final;
    type Error = P::Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.check_inactive();

        self.inner
            .produce()
            .await
            .inspect(|either| {
                // Mark the producer as inactive if the final value is emitted.
                if let Either::Right(_) = either {
                    self.active = false
                }
            })
            .inspect_err(|_| {
                self.active = false;
            })
    }
}

impl<P: BufferedProducer> BufferedProducer for Invariant<P> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.check_inactive();

        self.inner.slurp().await.inspect_err(|_| {
            self.active = false;
        })
    }
}

impl<P: BulkProducer> BulkProducer for Invariant<P>
where
    P::Item: Copy,
{
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        P::Item: 'a,
    {
        self.check_inactive();

        self.inner
            .expose_items()
            .await
            .inspect(|either| match either {
                Either::Left(slots) => self.exposed_slots = slots.len(),
                Either::Right(_) => self.active = false,
            })
            .inspect_err(|_| {
                self.active = false;
            })
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.check_inactive();

        if amount > self.exposed_slots {
            panic!("may not call `consider_produced` with an amount exceeding the total number of exposed slots");
        } else {
            self.exposed_slots -= amount;
        }

        // Proceed with the inner call to `consider_produced` and return the result.
        self.inner.consider_produced(amount).await
    }
}
