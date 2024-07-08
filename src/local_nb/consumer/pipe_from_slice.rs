use crate::local_nb::{BulkConsumer, Consumer};

/// Information you get from the `pipe_from_slice` family of functions when the consumer is unable to consume the complete slice.
///
/// `E` the `Error` type of the consumer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipeFromSliceError<E> {
    /// The number of items that were consumed.
    pub consumed: usize,
    /// Why did the consumer stop accepting items?
    pub reason: E,
}

/// Try to completely consume the items of a slice with a consumer.
/// Reports an error if the slice could not be consumed completely.
pub async fn pipe_from_slice<T, C>(
    buf: &[T],
    consumer: &mut C,
) -> Result<(), PipeFromSliceError<C::Error>>
where
    T: Clone,
    C: Consumer<Item = T>,
{
    for i in 0..buf.len() {
        let item = buf[i].clone();

        if let Err(err) = consumer.consume(item).await {
            return Err(PipeFromSliceError {
                consumed: i,
                reason: err,
            });
        }
    }

    Ok(())
}

/// Try to completely consume the items of a slice with a bulk consumer.
/// Reports an error if the slice could not be consumed completely.
pub async fn bulk_pipe_from_slice<T, C>(
    buf: &[T],
    consumer: &mut C,
) -> Result<(), PipeFromSliceError<C::Error>>
where
    T: Copy,
    C: BulkConsumer<Item = T>,
{
    let mut consumed_so_far = 0;

    while consumed_so_far < buf.len() {
        match consumer.bulk_consume(buf).await {
            Ok(consumed_count) => consumed_so_far += consumed_count,
            Err(err) => {
                return Err(PipeFromSliceError {
                    consumed: consumed_so_far,
                    reason: err,
                });
            }
        }
    }

    Ok(())
}
