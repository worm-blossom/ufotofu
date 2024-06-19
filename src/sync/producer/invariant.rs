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

    /// Checks the state of the `active` field and panics if the value is
    /// `false`.
    pub fn check_inactive(&self) {
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

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::*;

    use crate::sync::producer::SliceProducer;

    #[test]
    fn accepts_valid_did_produce_amount() {
        // Create a slice producer with data that occupies four slots.
        let mut slice_producer = SliceProducer::new(b"tofu");

        // Copy data from three of the occupied slots and call `did_produce`.
        let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
        if let Ok(Either::Left(slots)) = slice_producer.producer_slots() {
            MaybeUninit::copy_from_slice(&mut buf[0..3], &slots[0..3]);
            assert!(slice_producer.did_produce(3).is_ok());
        }
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_produce` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_second_did_produce_with_amount_greater_than_available_slots() {
        // Create a slice producer with data that occupies four slots.
        let mut slice_producer = SliceProducer::new(b"tofu");

        // Copy data from three of the occupied slots and call `did_produce`.
        let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
        if let Ok(Either::Left(slots)) = slice_producer.producer_slots() {
            MaybeUninit::copy_from_slice(&mut buf[0..3], &slots[0..3]);
            assert!(slice_producer.did_produce(3).is_ok());
        }

        // Make a second call to `did_produce` which exceeds the number of available slots.
        let _ = slice_producer.did_produce(2);
    }

    #[test]
    fn produces_final_value_on_producer_slots_after_complete_production() {
        // Create a slice producer with data that occupies four slots.
        let mut slice_producer = SliceProducer::new(b"tofu");

        // Copy data from two of the occupied slots and call `did_produce`.
        let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
        if let Ok(Either::Left(slots)) = slice_producer.producer_slots() {
            MaybeUninit::copy_from_slice(&mut buf[0..2], &slots[0..2]);
            assert!(slice_producer.did_produce(2).is_ok());
        }

        // Copy data from two of the occupied slots and call `did_produce`.
        let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
        if let Ok(Either::Left(slots)) = slice_producer.producer_slots() {
            MaybeUninit::copy_from_slice(&mut buf[0..2], &slots[0..2]);
            assert!(slice_producer.did_produce(2).is_ok());
        }

        // Make a third call to `producer_slots` after all items have been yielded,
        // ensuring that the final value is returned.
        assert_eq!(slice_producer.producer_slots().unwrap(), Either::Right(()));
    }

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
        let mut slice_producer = SliceProducer::new(b"ufo");
        loop {
            // Call `produce()` until the final value is emitted.
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let _ = slice_producer.produce();
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_slurp_after_final() {
        let mut slice_producer = SliceProducer::new(b"ufo");
        loop {
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let _ = slice_producer.slurp();
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_producer_slots_after_final() {
        let mut slice_producer = SliceProducer::new(b"ufo");
        loop {
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let _ = slice_producer.producer_slots();
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_did_produce_after_final() {
        let mut slice_producer = SliceProducer::new(b"ufo");
        loop {
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let _ = slice_producer.did_produce(3);
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_bulk_produce_after_final() {
        let mut slice_producer = SliceProducer::new(b"tofu");
        loop {
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
        let _ = slice_producer.bulk_produce(&mut buf);
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_produce` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_produce_with_amount_greater_than_available_slots() {
        let mut slice_producer = SliceProducer::new(b"ufo");

        let _ = slice_producer.did_produce(21);
    }
}
