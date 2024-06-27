use core::mem::MaybeUninit;

use either::Either;

use crate::sync::{BulkProducer, Producer};

/// Information you get from the `pipe_into_slice` family of functions when the producer is unable to fill the complete slice.
///
/// `'a` is the lifetime of the slice, `T` the type of items of the slice, `F` the `Final` type of the producer, and `E` the `Error` type of the producer.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PipeIntoSliceError<'a, T, F, E> {
    /// The prefix of the argument slice that *was* filled successfully. The length of this is guaranteed to be strictly less than the length of the original slice.
    pub filled: &'a [T],
    /// Did completely filling the slice fail because the producer reached its final item, or because it yielded an error?
    pub reason: Either<F, E>,
}

/// Try to completely fill a slice with items from a producer.
/// Reports an error if the slice could not be filled completely.
pub fn pipe_into_slice<'a, T, P>(
    buf: &'a mut [T],
    producer: &mut P,
) -> Result<(), PipeIntoSliceError<'a, T, P::Final, P::Error>>
where
    P: Producer<Item = T>,
{
    unimplemented!()
}

/// Try to completely fill an uninitialised slice with items from a producer.
/// Reports an error if the slice could not be filled completely.
/// 
/// The `Ok` return value is a convenience that performs the converion of the input slice from possibly uninitialised memory to "normal" memory for you. It is guaranteed to point to the same memory as the input slice (and has the same length).
pub fn pipe_into_slice_uninit<'a, T, P>(
    buf: &'a mut [MaybeUninit<T>],
    producer: &mut P,
) -> Result<&'a mut [T], PipeIntoSliceError<'a, T, P::Final, P::Error>>
where
    P: Producer<Item = T>,
{
    unimplemented!()
}

/// Try to completely fill a slice with items from a bulk producer.
/// Reports an error if the slice could not be filled completely.
pub fn bulk_pipe_into_slice<'a, T, P>(
    buf: &'a mut [T],
    producer: &mut P,
) -> Result<(), PipeIntoSliceError<'a, T, P::Final, P::Error>>
where
    T: Copy,
    P: BulkProducer<Item = T>,
{
    unimplemented!()
}

/// Try to completely fill an uninitialised slice with items from a bulk producer.
/// Reports an error if the slice could not be filled completely.
/// 
/// The `Ok` return value is a convenience that performs the converion of the input slice from possibly uninitialised memory to "normal" memory for you. It is guaranteed to point to the same memory as the input slice (and has the same length).
pub fn bulk_pipe_into_slice_uninit<'a, T, P>(
    buf: &'a mut [MaybeUninit<T>],
    producer: &mut P,
) -> Result<&'a mut [T], PipeIntoSliceError<'a, T, P::Final, P::Error>>
where
    T: Copy,
    P: BulkProducer<Item = T>,
{
    unimplemented!()
}
