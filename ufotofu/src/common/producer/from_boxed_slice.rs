use core::convert::{AsRef, Infallible};
use core::fmt::Debug;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{
    alloc::{Allocator, Global},
    boxed::Box,
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{
    alloc::{Allocator, Global},
    boxed::Box,
    vec::Vec,
};

use either::Either;
use wrapper::Wrapper;

use crate::sync::producer::Invariant;
use crate::sync::{BufferedProducer, BulkProducer, Producer};

/// Produces data from a boxed slice.
pub struct FromBoxedSlice_<T, A: Allocator = Global>(Invariant<FromBoxedSlice<T, A>>);

invarianted_impl_debug!(FromBoxedSlice_<T: Debug, A: Allocator + Debug>);

impl<T, A: Allocator> FromBoxedSlice_<T, A> {
    /// Create a producer which produces the data in the given boxed slice.
    pub fn new(v: Box<[T], A>) -> FromBoxedSlice_<T, A> {
        let invariant = Invariant::new(FromBoxedSlice(v, 0));

        FromBoxedSlice_(invariant)
    }

    /// Create a producer which produces the data in the given vector.
    pub fn from_vec(v: Vec<T, A>) -> FromBoxedSlice_<T, A> {
        let invariant = Invariant::new(FromBoxedSlice(v.into_boxed_slice(), 0));

        FromBoxedSlice_(invariant)
    }
}

invarianted_impl_as_ref!(FromBoxedSlice_<T, A: Allocator>; [T]);
invarianted_impl_wrapper!(FromBoxedSlice_<T, A: Allocator>; Box<[T], A>);

invarianted_impl_producer_sync_and_local_nb!(FromBoxedSlice_<T: Clone, A: Allocator> Item T;
    /// Emitted once the end of the boxed slice has been reached.
    Final ();
    Error Infallible
);
invarianted_impl_buffered_producer_sync_and_local_nb!(FromBoxedSlice_<T: Clone, A: Allocator>);
invarianted_impl_bulk_producer_sync_and_local_nb!(FromBoxedSlice_<T: Copy, A: Allocator>);

#[derive(Debug)]
struct FromBoxedSlice<T, A: Allocator = Global>(Box<[T], A>, usize);

impl<T, A: Allocator> AsRef<[T]> for FromBoxedSlice<T, A> {
    fn as_ref(&self) -> &[T] {
        self.0.as_ref()
    }
}

impl<T, A: Allocator> Wrapper<Box<[T], A>> for FromBoxedSlice<T, A> {
    fn into_inner(self) -> Box<[T], A> {
        self.0
    }
}

impl<T: Clone, A: Allocator> Producer for FromBoxedSlice<T, A> {
    /// The type of the items to be produced.
    type Item = T;
    /// The final value emitted once the end of the slice has been reached.
    type Final = ();
    /// The producer can never error.
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

impl<T: Clone, A: Allocator> BufferedProducer for FromBoxedSlice<T, A> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        // There are no effects to perform so we simply return.
        Ok(())
    }
}

impl<T: Copy, A: Allocator> BulkProducer for FromBoxedSlice<T, A> {
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

sync_producer_as_local_nb!(FromBoxedSlice<T: Clone, A: Allocator>);
sync_buffered_producer_as_local_nb!(FromBoxedSlice<T: Clone, A: Allocator>);
sync_bulk_producer_as_local_nb!(FromBoxedSlice<T: Copy, A: Allocator>);

// #[cfg(test)]
// mod tests {
//     use super::super::*;
//     use crate::sync::*;

//     use core::mem::MaybeUninit;

//     use either::Either;

//     // The debug output hides the internals of using semantically transparent wrappers.
//     #[test]
//     fn debug_output_hides_transparent_wrappers() {
//         let prod = FromBoxedSlice::new(b"ufo".to_vec());
//         assert_eq!(format!("{:?}", prod), "FromBoxedSlice([117, 102, 111], 0)");
//     }

//     // Panic conditions:
//     //
//     // - `produce()` must not be called after final or error
//     // - `slurp()` must not be called after final or error
//     // - `producer_slots()` must not be called after final or error
//     // - `did_produce()` must not be called after final or error
//     // - `bulk_produce()` must not be called after final or error
//     // - `did_produce(amount)` must not be called with `amount` greater that available slots

//     // In each of the following tests, the final function call should panic.

//     #[test]
//     #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
//     fn panics_on_produce_after_final() {
//         let mut slice_producer = FromBoxedSlice::new(b"ufo".to_vec());
//         loop {
//             // Call `produce()` until the final value is emitted.
//             if let Ok(Either::Right(_)) = slice_producer.produce() {
//                 break;
//             }
//         }

//         let _ = slice_producer.produce();
//     }

//     #[test]
//     #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
//     fn panics_on_slurp_after_final() {
//         let mut slice_producer = FromBoxedSlice::new(b"ufo".to_vec());
//         loop {
//             if let Ok(Either::Right(_)) = slice_producer.produce() {
//                 break;
//             }
//         }

//         let _ = slice_producer.slurp();
//     }

//     #[test]
//     #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
//     fn panics_on_producer_slots_after_final() {
//         let mut slice_producer = FromBoxedSlice::new(b"ufo".to_vec());
//         loop {
//             if let Ok(Either::Right(_)) = slice_producer.produce() {
//                 break;
//             }
//         }

//         let _ = slice_producer.expose_items();
//     }

//     #[test]
//     #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
//     fn panics_on_did_produce_after_final() {
//         let mut slice_producer = FromBoxedSlice::new(b"ufo".to_vec());
//         loop {
//             if let Ok(Either::Right(_)) = slice_producer.produce() {
//                 break;
//             }
//         }

//         let _ = slice_producer.consider_produced(3);
//     }

//     #[test]
//     #[should_panic(expected = "may not call `Producer` methods after the sequence has ended")]
//     fn panics_on_bulk_produce_after_final() {
//         let mut slice_producer = FromBoxedSlice::new(b"tofu".to_vec());
//         loop {
//             if let Ok(Either::Right(_)) = slice_producer.produce() {
//                 break;
//             }
//         }

//         let mut buf: [MaybeUninit<u8>; 4] = MaybeUninit::uninit_array();
//         let _ = slice_producer.bulk_produce_uninit(&mut buf);
//     }

//     #[test]
//     #[should_panic(
//         expected = "may not call `consider_produced` with an amount exceeding the total number of exposed slots"
//     )]
//     fn panics_on_did_produce_with_amount_greater_than_available_slots() {
//         let mut slice_producer = FromBoxedSlice::new(b"ufo".to_vec());

//         let _ = slice_producer.consider_produced(21);
//     }
// }
