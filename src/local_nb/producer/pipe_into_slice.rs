#![allow(clippy::type_complexity)]
use core::mem::MaybeUninit;

use either::Either::{self, Left, Right};

use crate::local_nb::{BulkProducer, Producer};

/// Information you get from the `pipe_into_slice` family of functions when the producer is unable to fill the complete slice.
///
/// `'a` is the lifetime of the slice, `T` the type of items of the slice, `F` the `Final` type of the producer, and `E` the `Error` type of the producer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipeIntoSliceError<'a, T, F, E> {
    /// The prefix of the argument slice that *was* filled successfully. The length of this is guaranteed to be strictly less than the length of the original slice.
    pub filled: &'a [T],
    /// Did completely filling the slice fail because the producer reached its final item, or because it yielded an error?
    pub reason: Either<F, E>,
}

/// Try to completely fill a slice with items from a producer.
/// Reports an error if the slice could not be filled completely.
pub async fn pipe_into_slice<'a, T, P>(
    buf: &'a mut [T],
    producer: &mut P,
) -> Result<(), PipeIntoSliceError<'a, T, P::Final, P::Error>>
where
    P: Producer<Item = T>,
{
    for i in 0..buf.len() {
        match producer.produce().await {
            Ok(Left(item)) => buf[i] = item,
            Ok(Right(fin)) => {
                return Err(PipeIntoSliceError {
                    filled: &buf[0..i],
                    reason: Left(fin),
                })
            }
            Err(err) => {
                return Err(PipeIntoSliceError {
                    filled: &buf[0..i],
                    reason: Right(err),
                })
            }
        }
    }

    Ok(())
}

/// Try to completely fill an uninitialised slice with items from a producer.
/// Reports an error if the slice could not be filled completely.
///
/// The `Ok` return value is a convenience that performs the converion of the input slice from possibly uninitialised memory to "normal" memory for you. It is guaranteed to point to the same memory as the input slice (and has the same length).
pub async fn pipe_into_slice_uninit<'a, T, P>(
    buf: &'a mut [MaybeUninit<T>],
    producer: &mut P,
) -> Result<&'a mut [T], PipeIntoSliceError<'a, T, P::Final, P::Error>>
where
    P: Producer<Item = T>,
{
    for i in 0..buf.len() {
        match producer.produce().await {
            Ok(Left(item)) => {
                let _ = buf[i].write(item);
            }
            Ok(Right(fin)) => {
                return Err(PipeIntoSliceError {
                    // We can do this because we know the first `i` positions of `buf` have been written to in the previous i iterations of this loop.
                    filled: unsafe { MaybeUninit::slice_assume_init_ref(&buf[0..i]) },
                    reason: Left(fin),
                });
            }
            Err(err) => {
                return Err(PipeIntoSliceError {
                    // We can do this because we know the first `i` positions of `buf` have been written to in the previous i iterations of this loop.
                    filled: unsafe { MaybeUninit::slice_assume_init_ref(&buf[0..i]) },
                    reason: Right(err),
                });
            }
        }
    }

    // We can do this because we know that `buf`'s slots from index 0 to `buf.len()` have been written to in the preceding loop.
    Ok(unsafe { MaybeUninit::slice_assume_init_mut(buf) })
}

/// Try to completely fill a slice with items from a bulk producer.
/// Reports an error if the slice could not be filled completely.
pub async fn bulk_pipe_into_slice<'a, T, P>(
    buf: &'a mut [T],
    producer: &mut P,
) -> Result<(), PipeIntoSliceError<'a, T, P::Final, P::Error>>
where
    T: Copy,
    P: BulkProducer<Item = T>,
{
    let mut produced_so_far = 0;

    while produced_so_far < buf.len() {
        match producer.bulk_produce(buf).await {
            Ok(Left(count)) => produced_so_far += count,
            Ok(Right(fin)) => {
                return Err(PipeIntoSliceError {
                    filled: &buf[0..produced_so_far],
                    reason: Left(fin),
                });
            }
            Err(err) => {
                return Err(PipeIntoSliceError {
                    filled: &buf[0..produced_so_far],
                    reason: Right(err),
                });
            }
        }
    }

    Ok(())
}

/// Try to completely fill an uninitialised slice with items from a bulk producer.
/// Reports an error if the slice could not be filled completely.
///
/// The `Ok` return value is a convenience that performs the converion of the input slice from possibly uninitialised memory to "normal" memory for you. It is guaranteed to point to the same memory as the input slice (and has the same length).
pub async fn bulk_pipe_into_slice_uninit<'a, T, P>(
    buf: &'a mut [MaybeUninit<T>],
    producer: &mut P,
) -> Result<&'a mut [T], PipeIntoSliceError<'a, T, P::Final, P::Error>>
where
    T: Copy,
    P: BulkProducer<Item = T>,
{
    let mut produced_so_far = 0;

    while produced_so_far < buf.len() {
        match producer.bulk_produce_uninit(buf).await {
            Ok(Left(count)) => produced_so_far += count,
            Ok(Right(fin)) => {
                return Err(PipeIntoSliceError {
                    // We can do this because we know that `buf`'s slots from index 0 to `produced_so_far` have been written to in the preceding iterations of this loop.
                    filled: unsafe { MaybeUninit::slice_assume_init_ref(&buf[0..produced_so_far]) },
                    reason: Left(fin),
                });
            }
            Err(err) => {
                return Err(PipeIntoSliceError {
                    // We can do this because we know that `buf`'s slots from index 0 to `produced_so_far` have been written to in the preceding iterations of this loop.
                    filled: unsafe { MaybeUninit::slice_assume_init_ref(&buf[0..produced_so_far]) },
                    reason: Right(err),
                });
            }
        }
    }

    // We can do this because we know that `buf`'s slots from index 0 to `produced_so_far` have been written to in the preceding loop.
    Ok(unsafe { MaybeUninit::slice_assume_init_mut(&mut buf[0..produced_so_far]) })
}
