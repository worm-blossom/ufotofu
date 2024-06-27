use either::Either;

use crate::sync::{BulkConsumer, Consumer};

/// Information you get from the `pipe_from_slice` family of functions when the consumer is unable to consume the complete slice.
///
/// `E` the `Error` type of the consumer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipeFromSliceError<E> {
    /// The nuber of items that were consumed.
    pub consumed: usize,
    /// Why did the consumer stop accepting items?
    pub reason: E,
}

/// Try to completely consume the items of a slice with a consumer.
/// Reports an error if the slice could not be consumed completely.
pub fn pipe_from_slice<T, C>(
    buf: &[T],
    consumer: &mut C,
) -> Result<(), PipeFromSliceError<C::Error>>
where
    C: Consumer<Item = T>,
{
    unimplemented!()
}

/// Try to completely consume the items of a slice with a bulk consumer.
/// Reports an error if the slice could not be consumed completely.
pub fn bulk_pipe_from_slice<T, C>(
    buf: &[T],
    consumer: &mut C,
) -> Result<(), PipeFromSliceError<C::Error>>
where
    T: Copy,
    C: BulkConsumer<Item = T>,
{
    unimplemented!()
}