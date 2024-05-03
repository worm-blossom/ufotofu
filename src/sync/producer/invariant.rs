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
///   - `producer_slots`
///   - `did_produce`
///   - `bulk_produce`
/// - Must not call any of the prior functions after any of them had returned
///   an error.
/// - Must not call `did_produce` with an amount exceeding the number of available slots
///   previously exposed by a call to `producer_slots`.
#[derive(Debug, Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct Invariant<P> {
    /// An implementer of the `Producer` traits.
    inner: P,
    /// The status of the producer. `true` while the caller may call trait
    /// methods, `false` once that becomes disallowed (because a method returned
    /// an error, or because `close` was called).
    active: bool,
    /// The maximum `amount` that a caller may supply to `did_produce`.
    exposed_slots: usize,
}

impl<P> Invariant<P> {
    /// Return a `Producer` that behaves exactly like the wrapped `Producer`
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
        self.check_inactive();

        self.inner
            .produce()
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

impl<P, T, F, E> BufferedProducer for Invariant<P>
where
    P: BufferedProducer<Item = T, Final = F, Error = E>,
{
    fn slurp(&mut self) -> Result<(), Self::Error> {
        self.check_inactive();

        self.inner.slurp().inspect_err(|_| {
            self.active = false;
        })
    }
}

impl<P, T, F, E> BulkProducer for Invariant<P>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn producer_slots(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        self.check_inactive();

        self.inner
            .producer_slots()
            .inspect(|either| match either {
                Either::Left(slots) => self.exposed_slots = slots.len(),
                Either::Right(_) => self.active = false,
            })
            .inspect_err(|_| {
                self.active = false;
            })
    }

    fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.check_inactive();

        if amount > self.exposed_slots {
            panic!("may not call `did_produce` with an amount exceeding the total number of exposed slots");
        } else {
            self.exposed_slots -= amount;
        }

        // Proceed with the inner call to `did_produce` and return the result.
        self.inner.did_produce(amount)
    }
}
