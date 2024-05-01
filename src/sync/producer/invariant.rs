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
#[derive(Debug)]
pub struct Invariant<P> {
    inner: P,
    active: bool,
    exposed_slots: usize,
}

impl<P> Invariant<P> {
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
                // TODO: Do we need to set `exposed_slots` to 0 for any
                // methods? The next call will panic due to `active` being
                // false, so it seems unnecessary.
                self.exposed_slots = 0;
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
            self.exposed_slots = 0;
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
                self.exposed_slots = 0;
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

#[cfg(test)]
mod tests {
    use super::*;

    use core::mem::MaybeUninit;

    use crate::sync::producer::Cursor;

    // Panic conditions:
    //
    // - `produce()` must not be called after final or error
    // - `slurp()` must not be called after final or error
    // - `producer_slots()` must not be called after final or error
    // - `did_produce()` must not be called after final or error
    // - `bulk_produce()` must not be called after final or error
    // - `did_produce(amount)` must not be called with `amount` greater that available slots

    // In each of the following tests, the final function call should panic.

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_produce_after_final() {
        let mut cursor = Cursor::new(b"ufo");
        loop {
            // Call `produce()` until the final value is emitted.
            if let Ok(Either::Right(_)) = cursor.produce() {
                break;
            }
        }

        let _ = cursor.produce();
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_slurp_after_final() {
        let mut cursor = Cursor::new(b"ufo");
        loop {
            if let Ok(Either::Right(_)) = cursor.produce() {
                break;
            }
        }

        let _ = cursor.slurp();
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_producer_slots_after_final() {
        let mut cursor = Cursor::new(b"ufo");
        loop {
            if let Ok(Either::Right(_)) = cursor.produce() {
                break;
            }
        }

        let _ = cursor.producer_slots();
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_did_produce_after_final() {
        let mut cursor = Cursor::new(b"ufo");
        loop {
            if let Ok(Either::Right(_)) = cursor.produce() {
                break;
            }
        }

        let _ = cursor.did_produce(3);
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_bulk_produce_after_final() {
        let mut cursor = Cursor::new(b"tofu");
        loop {
            if let Ok(Either::Right(_)) = cursor.produce() {
                break;
            }
        }

        let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
        let _ = cursor.bulk_produce(&mut buf);
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_produce` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_produce_with_amount_greater_than_available_slots() {
        let mut cursor = Cursor::new(b"ufo");

        let _ = cursor.did_produce(21);
    }
}
