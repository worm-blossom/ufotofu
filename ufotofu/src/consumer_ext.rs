use crate::errors::*;
use crate::BulkConsumer;
use crate::Consumer;

impl<P> ConsumerExt for P where P: Consumer {}

/// An extension trait for [`Consumer`] that provides a variety of convenient combinator functions.
pub trait ConsumerExt: Consumer {
    /// Tries to consume (clones of) *all* items in the given slice.
    /// Reports an error if the slice could not be consumed completely.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after [`close`](Consumer::close) was called.
    ///
    /// #### Implementation Notes
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    async fn consume_full_slice(
        &mut self,
        buf: &[Self::Item],
    ) -> Result<(), ConsumeAtLeastError<Self::Error>>
    where
        Self::Item: Clone,
    {
        for i in 0..buf.len() {
            let item = buf[i].clone();

            if let Err(err) = self.consume(item).await {
                return Err(ConsumeAtLeastError {
                    count: i,
                    reason: err,
                });
            }
        }

        Ok(())
    }
}

impl<P> BulkConsumerExt for P where P: BulkConsumer {}

/// An extension trait for [`BulkConsumer`] that provides a variety of convenient combinator functions.
pub trait BulkConsumerExt: BulkConsumer {
    /// Tries to bulk-consume *all* items in the given slice.
    /// Reports an error if the slice could not be consumed completely.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after [`close`](Consumer::close) was called.
    ///
    /// #### Implementation Notes
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    async fn bulk_consume_full_slice(
        &mut self,
        buf: &[Self::Item],
    ) -> Result<(), ConsumeAtLeastError<Self::Error>> {
        let mut consumed_so_far = 0;

        while consumed_so_far < buf.len() {
            match self.bulk_consume(&buf[consumed_so_far..]).await {
                Ok(consumed_count) => consumed_so_far += consumed_count,
                Err(err) => {
                    return Err(ConsumeAtLeastError {
                        count: consumed_so_far,
                        reason: err,
                    });
                }
            }
        }

        Ok(())
    }
}
