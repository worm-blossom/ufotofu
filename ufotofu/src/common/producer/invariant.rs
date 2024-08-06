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

impl<P> BufferedProducer for Invariant<P>
where
    P: BufferedProducer,
{
    fn slurp(&mut self) -> Result<(), Self::Error> {
        self.check_inactive();

        self.inner.slurp().inspect_err(|_| {
            self.active = false;
        })
    }
}

impl<P> BulkProducer for Invariant<P>
where
    P: BulkProducer,
    P::Item: Copy,
{
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        self.check_inactive();

        self.inner
            .expose_items()
            .inspect(|either| match either {
                Either::Left(slots) => self.exposed_slots = slots.len(),
                Either::Right(_) => self.active = false,
            })
            .inspect_err(|_| {
                self.active = false;
            })
    }

    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.check_inactive();

        if amount > self.exposed_slots {
            panic!("may not call `consider_produced` with an amount exceeding the total number of exposed slots");
        } else {
            self.exposed_slots -= amount;
        }

        self.inner.consider_produced(amount)
    }
}

impl<P: ProducerLocalNb> ProducerLocalNb for Invariant<P> {
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

impl<P: BufferedProducerLocalNb> BufferedProducerLocalNb for Invariant<P> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.check_inactive();

        self.inner.slurp().await.inspect_err(|_| {
            self.active = false;
        })
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

#[cfg(test)]
mod tests {
    // use super::super::*;

    // use crate::sync::consumer::{IntoVec, SliceProducer, SliceProducerFullError};
    // use crate::sync::*;

    // #[test]
    // fn accepts_valid_did_consume_amount() {
    //     // Create a slice consumer that exposes four slots.
    //     let mut buf = [0; 4];
    //     let mut slice_consumer = SliceProducer::new(&mut buf);

    //     // Copy data to three of the available slots and call `consume_slots`.
    //     let data = b"ufo";
    //     let slots = slice_consumer.expose_slots().unwrap();
    //     MaybeUninit::copy_from_slice(&mut slots[0..3], &data[0..3]);
    //     unsafe {
    //         assert!(slice_consumer.consume_slots(3).is_ok());
    //     }
    // }

    // #[test]
    // #[should_panic(
    //     expected = "may not call `consume_slots` with an amount exceeding the total number of exposed slots"
    // )]
    // fn panics_on_second_did_consume_with_amount_greater_than_available_slots() {
    //     // Create a slice consumer that exposes four slots.
    //     let mut buf = [0; 4];
    //     let mut slice_consumer = SliceProducer::new(&mut buf);

    //     // Copy data to three of the available slots and call `consume_slots`.
    //     let data = b"ufo";
    //     let slots = slice_consumer.expose_slots().unwrap();
    //     MaybeUninit::copy_from_slice(&mut slots[0..3], &data[0..3]);
    //     unsafe {
    //         assert!(slice_consumer.consume_slots(3).is_ok());
    //     }

    //     // Make a second call to `consume_slots` which exceeds the number of available slots.
    //     unsafe {
    //         let _ = slice_consumer.consume_slots(2);
    //     }
    // }

    // #[test]
    // fn errors_on_consumer_slots_when_none_are_available() {
    //     // Create a slice consumer that exposes four slots.
    //     let mut buf = [0; 4];
    //     let mut slice_consumer = SliceProducer::new(&mut buf);

    //     // Copy data to two of the available slots and call `consume_slots`.
    //     let data = b"tofu";
    //     let slots = slice_consumer.expose_slots().unwrap();
    //     MaybeUninit::copy_from_slice(&mut slots[0..2], &data[0..2]);
    //     unsafe {
    //         assert!(slice_consumer.consume_slots(2).is_ok());
    //     }

    //     // Copy data to two of the available slots and call `consume_slots`.
    //     let slots = slice_consumer.expose_slots().unwrap();
    //     MaybeUninit::copy_from_slice(&mut slots[0..2], &data[0..2]);
    //     unsafe {
    //         assert!(slice_consumer.consume_slots(2).is_ok());
    //     }

    //     // Make a third call to slots after all available slots have been used.
    //     assert_eq!(
    //         slice_consumer.expose_slots().unwrap_err(),
    //         SliceProducerFullError
    //     );
    // }

    // // Panic conditions:
    // //
    // // - `consume()` must not be called after `close()` or error
    // // - `close()` must not be called after `close()` or error
    // // - `flush()` must not be called after `close()` or error
    // // - `consumer_slots()` must not be called after `close()` or error
    // // - `did_consume()` must not be called after `close()` or error
    // // - `bulk_consume()` must not be called after `close()` or error
    // // - `did_consume(amount)` must not be called with `amount` greater than available slots

    // // In each of the following tests, the final function call should panic.

    // #[test]
    // #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    // fn panics_on_consume_after_close() {
    //     let mut into_vec = IntoVec::new();
    //     let _ = into_vec.close(());
    //     let _ = into_vec.consume(7);
    // }

    // #[test]
    // #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    // fn panics_on_close_after_close() {
    //     // Type annotations are required because we never provide a `T`.
    //     let mut into_vec: IntoVec<u8> = IntoVec::new();
    //     let _ = into_vec.close(());
    //     let _ = into_vec.close(());
    // }

    // #[test]
    // #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    // fn panics_on_flush_after_close() {
    //     let mut into_vec: IntoVec<u8> = IntoVec::new();
    //     let _ = into_vec.close(());
    //     let _ = into_vec.flush();
    // }

    // #[test]
    // #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    // fn panics_on_consumer_slots_after_close() {
    //     let mut into_vec: IntoVec<u8> = IntoVec::new();
    //     let _ = into_vec.close(());
    //     let _ = into_vec.expose_slots();
    // }

    // #[test]
    // #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    // fn panics_on_did_consume_after_close() {
    //     let mut into_vec: IntoVec<u8> = IntoVec::new();
    //     let _ = into_vec.close(());

    //     unsafe {
    //         let _ = into_vec.consume_slots(7);
    //     }
    // }

    // #[test]
    // #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    // fn panics_on_bulk_consume_after_close() {
    //     let mut into_vec = IntoVec::new();
    //     let _ = into_vec.close(());
    //     let _ = into_vec.bulk_consume(b"ufo");
    // }

    // #[test]
    // #[should_panic(
    //     expected = "may not call `consume_slots` with an amount exceeding the total number of exposed slots"
    // )]
    // fn panics_on_did_consume_with_amount_greater_than_available_slots() {
    //     let mut into_vec: IntoVec<u8> = IntoVec::new();

    //     unsafe {
    //         let _ = into_vec.consume_slots(21);
    //     }
    // }
}
