use core::convert::{AsMut, AsRef};

use either::Either;
use wrapper::Wrapper;

use crate::local_nb::producer::SyncToLocalNb;
use crate::local_nb::{BufferedProducer, BulkProducer, Producer};
use crate::sync::producer::Invariant as SyncInvariant;
use crate::sync::{
    BufferedProducer as SyncBufferedProducer, BulkProducer as SyncBulkProducer,
    Producer as SyncProducer,
};

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
pub struct Invariant<P>(SyncToLocalNb<SyncInvariant<P>>);

impl<P> Invariant<P> {
    /// Return a `Producer` that behaves exactly like the wrapped `Producer`
    /// `inner`, except that - when running tests - it performs runtime
    /// validation of API invariants and panics if they are violated by a
    /// caller.
    pub fn new(inner: P) -> Self {
        let invariant = SyncInvariant::new(inner);

        Invariant(SyncToLocalNb(invariant))
    }

    /// Checks the state of the `active` field and panics if the value is
    /// `false`.
    #[cfg(test)]
    fn check_inactive(&self) {
        self.0 .0.check_inactive()
    }
}

impl<P> AsRef<P> for Invariant<P> {
    fn as_ref(&self) -> &P {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<P> AsMut<P> for Invariant<P> {
    fn as_mut(&mut self) -> &mut P {
        let inner = self.0.as_mut();
        inner.as_mut()
    }
}

impl<P> Wrapper<P> for Invariant<P> {
    fn into_inner(self) -> P {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<P, T, F, E> Producer for Invariant<P>
where
    P: SyncProducer<Item = T, Final = F, Error = E>,
{
    type Item = T;
    type Final = F;
    type Error = E;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        let item_or_final = self.0.produce().await?;

        Ok(item_or_final)
    }
}

impl<P, T, F, E> BufferedProducer for Invariant<P>
where
    P: SyncBufferedProducer<Item = T, Final = F, Error = E>,
{
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.0.slurp().await?;

        Ok(())
    }
}

impl<P, T, F, E> BulkProducer for Invariant<P>
where
    P: SyncBulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    async fn producer_slots<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        T: 'a,
    {
        let slots = self.0.producer_slots().await?;

        Ok(slots)
    }

    async fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.did_produce(amount).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::*;

    use crate::local_nb::producer::SliceProducer;

    #[test]
    fn accepts_valid_did_produce_amount() {
        smol::block_on(async {
            // Create a slice producer with data that occupies four slots.
            let mut slice_producer = SliceProducer::new(b"tofu");

            // Copy data from three of the occupied slots and call `did_produce`.
            let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
            if let Ok(Either::Left(slots)) = slice_producer.producer_slots().await {
                MaybeUninit::copy_from_slice(&mut buf[0..3], &slots[0..3]);
                assert!(slice_producer.did_produce(3).await.is_ok());
            }
        })
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_produce` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_second_did_produce_with_amount_greater_than_available_slots() {
        smol::block_on(async {
            // Create a slice producer with data that occupies four slots.
            let mut slice_producer = SliceProducer::new(b"tofu");

            // Copy data from three of the occupied slots and call `did_produce`.
            let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
            if let Ok(Either::Left(slots)) = slice_producer.producer_slots().await {
                MaybeUninit::copy_from_slice(&mut buf[0..3], &slots[0..3]);
                assert!(slice_producer.did_produce(3).await.is_ok());
            }

            // Make a second call to `did_produce` which exceeds the number of available slots.
            let _ = slice_producer.did_produce(2).await;
        })
    }

    #[test]
    fn produces_final_value_on_producer_slots_after_complete_production() {
        smol::block_on(async {
            // Create a slice producer with data that occupies four slots.
            let mut slice_producer = SliceProducer::new(b"tofu");

            // Copy data from two of the occupied slots and call `did_produce`.
            let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
            if let Ok(Either::Left(slots)) = slice_producer.producer_slots().await {
                MaybeUninit::copy_from_slice(&mut buf[0..2], &slots[0..2]);
                assert!(slice_producer.did_produce(2).await.is_ok());
            }

            // Copy data from two of the occupied slots and call `did_produce`.
            let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
            if let Ok(Either::Left(slots)) = slice_producer.producer_slots().await {
                MaybeUninit::copy_from_slice(&mut buf[0..2], &slots[0..2]);
                assert!(slice_producer.did_produce(2).await.is_ok());
            }

            // Make a third call to `producer_slots` after all items have been yielded,
            // ensuring that the final value is returned.
            assert_eq!(
                slice_producer.producer_slots().await.unwrap(),
                Either::Right(())
            );
        })
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
        smol::block_on(async {
            let mut slice_producer = SliceProducer::new(b"ufo");
            loop {
                // Call `produce()` until the final value is emitted.
                if let Ok(Either::Right(_)) = slice_producer.produce().await {
                    break;
                }
            }

            let _ = slice_producer.produce().await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_slurp_after_final() {
        smol::block_on(async {
            let mut slice_producer = SliceProducer::new(b"ufo");
            loop {
                if let Ok(Either::Right(_)) = slice_producer.produce().await {
                    break;
                }
            }

            let _ = slice_producer.slurp().await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_producer_slots_after_final() {
        smol::block_on(async {
            let mut slice_producer = SliceProducer::new(b"ufo");
            loop {
                if let Ok(Either::Right(_)) = slice_producer.produce().await {
                    break;
                }
            }

            let _ = slice_producer.producer_slots().await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_did_produce_after_final() {
        smol::block_on(async {
            let mut slice_producer = SliceProducer::new(b"ufo");
            loop {
                if let Ok(Either::Right(_)) = slice_producer.produce().await {
                    break;
                }
            }

            let _ = slice_producer.did_produce(3).await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_bulk_produce_after_final() {
        smol::block_on(async {
            let mut slice_producer = SliceProducer::new(b"tofu");
            loop {
                if let Ok(Either::Right(_)) = slice_producer.produce().await {
                    break;
                }
            }

            let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
            let _ = slice_producer.bulk_produce(&mut buf).await;
        })
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_produce` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_produce_with_amount_greater_than_available_slots() {
        smol::block_on(async {
            let mut slice_producer = SliceProducer::new(b"ufo");

            let _ = slice_producer.did_produce(21).await;
        })
    }
}
