use core::convert::AsRef;

use either::Either;
use wrapper::Wrapper;

use crate::local_nb::producer::SyncToLocalNb;
use crate::local_nb::{BufferedProducer, BulkProducer, Producer};
use crate::sync::producer::SliceProducer as SyncSliceProducer;

/// Produces data from a slice.
pub struct SliceProducer<'a, T>(SyncToLocalNb<SyncSliceProducer<'a, T>>);

impl<'a, T: core::fmt::Debug> core::fmt::Debug for SliceProducer<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a, T> SliceProducer<'a, T> {
    /// Create a producer which produces the data in the given slice.
    pub fn new(slice: &'a [T]) -> SliceProducer<'a, T> {
        let slice_producer = SyncSliceProducer::new(slice);

        SliceProducer(SyncToLocalNb(slice_producer))
    }

    /// Return the offset into the slice at which the next item will be produced.
    pub fn get_offset(&self) -> usize {
        (self.0).as_ref().get_offset()
    }

    /// Return the subslice of items that have been produced so far.
    pub fn get_produced_so_far(&self) -> &[T] {
        &(self.0).as_ref().get_produced_so_far()
    }

    /// Return the subslice of items that have not been produced yet.
    pub fn get_not_yet_produced(&self) -> &[T] {
        &(self.0).as_ref().get_not_yet_produced()
    }
}

impl<'a, T> AsRef<[T]> for SliceProducer<'a, T> {
    fn as_ref(&self) -> &[T] {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<'a, T> Wrapper<&'a [T]> for SliceProducer<'a, T> {
    fn into_inner(self) -> &'a [T] {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<'a, T: Clone> Producer for SliceProducer<'a, T> {
    /// The type of the items to be produced.
    type Item = T;
    /// The final value emitted once the end of the slice has been reached.
    type Final = ();
    /// The producer can never error.
    type Error = !;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'a, T: Copy> BufferedProducer for SliceProducer<'a, T> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.0.slurp().await
    }
}

impl<'a, T: Copy> BulkProducer for SliceProducer<'a, T> {
    async fn expose_items<'b>(
        &'b mut self,
    ) -> Result<Either<&'b [Self::Item], Self::Final>, Self::Error>
    where
        T: 'b,
    {
        self.0.expose_items().await
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.consider_produced(amount).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::mem::MaybeUninit;

    // The debug output hides the internals of using semantically transparent wrappers.
    #[test]
    fn debug_output_hides_transparent_wrappers() {
        let prod = SliceProducer::new(b"ufo");
        assert_eq!(format!("{:?}", prod), "SliceProducer([117, 102, 111], 0)");
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

            let _ = slice_producer.expose_items().await;
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

            let _ = slice_producer.consider_produced(3).await;
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
            let _ = slice_producer.bulk_produce_maybeuninit(&mut buf).await;
        })
    }

    #[test]
    #[should_panic(
        expected = "may not call `consider_produced` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_produce_with_amount_greater_than_available_slots() {
        smol::block_on(async {
            let mut slice_producer = SliceProducer::new(b"ufo");

            let _ = slice_producer.consider_produced(21).await;
        })
    }
}
