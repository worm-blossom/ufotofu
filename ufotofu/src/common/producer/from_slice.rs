use core::convert::{AsRef, Infallible};
use core::fmt::Debug;

use either::Either;
use wrapper::Wrapper;

use crate::common::producer::Invariant;
use crate::sync::{BufferedProducer, BulkProducer, Producer};

invarianted_producer_outer_type!(
    /// Produces data from a slice.
    FromSlice_ FromSlice <'a, T>
);

invarianted_impl_debug!(FromSlice_<'a, T: Debug>);

impl<'a, T> FromSlice_<'a, T> {
    /// Create a producer which produces the data in the given slice.
    pub fn new(slice: &'a [T]) -> FromSlice_<'a, T> {
        // Wrap the inner slice producer in the invariant type.
        let invariant = Invariant::new(FromSlice(slice, 0));

        FromSlice_(invariant)
    }

    /// Return the offset into the slice at which the next item will be produced.
    pub fn get_offset(&self) -> usize {
        (self.0).as_ref().1
    }

    /// Return the subslice of items that have been produced so far.
    pub fn get_produced_so_far(&self) -> &[T] {
        &(self.0).as_ref().0[..self.get_offset()]
    }

    /// Return the subslice of items that have not been produced yet.
    pub fn get_not_yet_produced(&self) -> &[T] {
        &(self.0).as_ref().0[self.get_offset()..]
    }
}

invarianted_impl_as_ref!(FromSlice_<'a, T>; [T]);
invarianted_impl_wrapper!(FromSlice_<'a, T>; &'a [T]);

invarianted_impl_producer_sync_and_local_nb!(FromSlice_<'a, T: Clone> Item T;
    /// Emitted once the end of the slice has been reached.
    Final ();
    Error Infallible
);
invarianted_impl_buffered_producer_sync_and_local_nb!(FromSlice_<'a, T: Clone>);
invarianted_impl_bulk_producer_sync_and_local_nb!(FromSlice_<'a, T: Copy>);

#[derive(Debug)]
struct FromSlice<'a, T>(&'a [T], usize);

impl<'a, T> AsRef<[T]> for FromSlice<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.0
    }
}

impl<'a, T> Wrapper<&'a [T]> for FromSlice<'a, T> {
    fn into_inner(self) -> &'a [T] {
        self.0
    }
}

impl<'a, T: Clone> Producer for FromSlice<'a, T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        if self.0.len() == self.1 {
            Ok(Either::Right(()))
        } else {
            let item = self.0[self.1].clone();
            self.1 += 1;

            Ok(Either::Left(item))
        }
    }
}

impl<'a, T: Clone> BufferedProducer for FromSlice<'a, T> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        // There are no effects to perform so we simply return.
        Ok(())
    }
}

impl<'a, T: Copy> BulkProducer for FromSlice<'a, T> {
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        let slice = &self.0[self.1..];
        if slice.is_empty() {
            Ok(Either::Right(()))
        } else {
            Ok(Either::Left(slice))
        }
    }

    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.1 += amount;

        Ok(())
    }
}

sync_producer_as_local_nb!(FromSlice<'a, T: Clone>);
sync_buffered_producer_as_local_nb!(FromSlice<'a, T: Clone>);
sync_bulk_producer_as_local_nb!(FromSlice<'a, T: Copy>);

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::sync::*;

    use either::Either;

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
        let mut slice_producer = FromSlice::new(b"ufo");
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
        let mut slice_producer = FromSlice::new(b"ufo");
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
        let mut slice_producer = FromSlice::new(b"ufo");
        loop {
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let _ = slice_producer.expose_items();
    }

    #[test]
    #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    fn panics_on_did_produce_after_final() {
        let mut slice_producer = FromSlice::new(b"ufo");
        loop {
            if let Ok(Either::Right(_)) = slice_producer.produce() {
                break;
            }
        }

        let _ = slice_producer.consider_produced(3);
    }

    // #[test]
    // #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
    // fn panics_on_bulk_produce_after_final() {
    //     let mut slice_producer = FromSlice::new(b"tofu");
    //     loop {
    //         if let Ok(Either::Right(_)) = slice_producer.produce() {
    //             break;
    //         }
    //     }

    //     let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
    //     let _ = slice_producer.bulk_produce_uninit(&mut buf);
    // }

    #[test]
    #[should_panic(
        expected = "may not call `consider_produced` with an amount exceeding the total number of exposed slots"
    )]
    fn panics_on_did_produce_with_amount_greater_than_available_slots() {
        let mut slice_producer = FromSlice::new(b"ufo");

        let _ = slice_producer.consider_produced(21);
    }
}
