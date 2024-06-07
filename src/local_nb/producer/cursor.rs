use core::convert::AsRef;

use either::Either;
use wrapper::Wrapper;

use crate::local_nb::{LocalBufferedProducer, LocalBulkProducer, LocalProducer};
use crate::sync::producer::Cursor as SyncCursor;
use crate::sync_to_local_nb::SyncToLocalNbProducer;

#[derive(Debug)]
/// Produces data from a slice.
pub struct Cursor<'a, T>(SyncToLocalNbProducer<SyncCursor<'a, T>>);

impl<'a, T> Cursor<'a, T> {
    /// Create a producer which produces the data in the given slice.
    pub fn new(slice: &'a [T]) -> Cursor<'a, T> {
        let cursor = SyncCursor::new(slice);

        Cursor(SyncToLocalNbProducer(cursor))
    }
}

impl<'a, T> AsRef<[T]> for Cursor<'a, T> {
    fn as_ref(&self) -> &[T] {
        let inner = self.0.as_ref();
        inner.as_ref()
    }
}

impl<'a, T> Wrapper<&'a [T]> for Cursor<'a, T> {
    fn into_inner(self) -> &'a [T] {
        let inner = self.0.into_inner();
        inner.into_inner()
    }
}

impl<'a, T: Clone> LocalProducer for Cursor<'a, T> {
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

impl<'a, T: Copy> LocalBufferedProducer for Cursor<'a, T> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.0.slurp().await
    }
}

impl<'a, T: Copy> LocalBulkProducer for Cursor<'a, T> {
    async fn producer_slots<'b>(
        &'b mut self,
    ) -> Result<Either<&'b [Self::Item], Self::Final>, Self::Error>
    where
        T: 'b,
    {
        self.0.producer_slots().await
    }

    async fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.did_produce(amount).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::mem::MaybeUninit;

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
            let mut cursor = Cursor::new(b"ufo");
            loop {
                // Call `produce()` until the final value is emitted.
                if let Ok(Either::Right(_)) = cursor.produce().await {
                    break;
                }
            }

            let _ = cursor.produce().await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_slurp_after_final() {
        smol::block_on(async {
            let mut cursor = Cursor::new(b"ufo");
            loop {
                if let Ok(Either::Right(_)) = cursor.produce().await {
                    break;
                }
            }

            let _ = cursor.slurp().await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_producer_slots_after_final() {
        smol::block_on(async {
            let mut cursor = Cursor::new(b"ufo");
            loop {
                if let Ok(Either::Right(_)) = cursor.produce().await {
                    break;
                }
            }

            let _ = cursor.producer_slots().await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_did_produce_after_final() {
        smol::block_on(async {
            let mut cursor = Cursor::new(b"ufo");
            loop {
                if let Ok(Either::Right(_)) = cursor.produce().await {
                    break;
                }
            }

            let _ = cursor.did_produce(3).await;
        })
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_bulk_produce_after_final() {
        smol::block_on(async {
            let mut cursor = Cursor::new(b"tofu");
            loop {
                if let Ok(Either::Right(_)) = cursor.produce().await {
                    break;
                }
            }

            let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
            let _ = cursor.bulk_produce(&mut buf).await;
        })
    }

    #[test]
    #[should_panic(
        expected = "may not call `did_produce` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_produce_with_amount_greater_than_available_slots() {
        smol::block_on(async {
            let mut cursor = Cursor::new(b"ufo");

            let _ = cursor.did_produce(21).await;
        })
    }
}
