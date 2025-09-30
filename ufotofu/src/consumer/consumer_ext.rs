use crate::{prelude::*, ConsumeAtLeastError};

impl<C> ConsumerExt for C where C: Consumer {}

/// An extension trait for [`Consumer`] that provides a variety of convenient combinator functions.
/// You never need to implement this trait yourself, it merely adds methods with default implementation to existing producers.
///
/// <br/>Counterpart: the [`ProducerExt`](crate::ProducerExt) trait.
pub trait ConsumerExt: Consumer {
    /// Tries to completely consume a slice.
    /// Reports an error if the slice could not be consumed completely.
    ///
    /// When working with a bulk consumer, use
    /// [`BulkConsumerExt::bulk_consume_full_slice`] for greater efficiency.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::ConsumeAtLeastError;
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// c.consume_full_slice(&[1, 2]).await?;
    ///
    /// assert_eq!(c.consume_full_slice(&[4, 8]).await, Err(ConsumeAtLeastError {
    ///     count: 1,
    ///     reason: (),
    /// }));
    ///
    /// assert_eq!(arr, [1, 2, 4]);
    /// # Result::<(), ConsumeAtLeastError<()>>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`ProducerExt::overwrite_full_slice`] method.
    async fn consume_full_slice(
        &mut self,
        buf: &[Self::Item],
    ) -> Result<(), ConsumeAtLeastError<Self::Error>>
    where
        Self::Item: Clone,
    {
        for i in 0..buf.len() {
            match self.consume(buf[i].clone()).await {
                Ok(()) => { /* no-op */ }
                Err(err) => {
                    return Err(ConsumeAtLeastError {
                        count: i,
                        reason: err,
                    })
                }
            }
        }

        Ok(())
    }
}

impl<C> BulkConsumerExt for C where C: BulkConsumer {}

/// An extension trait for [`BulkConsumer`] that provides a variety of convenient combinator functions.
/// You never need to implement this trait yourself, it merely adds methods with default implementation to existing bulk consumers.
///
/// <br/>Counterpart: the [`BulkProducerExt`](crate::BulkProducerExt) trait.
pub trait BulkConsumerExt: BulkConsumer {
    /// Tries to completely consume a slice.
    /// Reports an error if the slice could not be consumed completely.
    ///
    /// More efficient than [`ConsumerExt::consume_full_slice`].
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::ConsumeAtLeastError;
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// c.bulk_consume_full_slice(&[1, 2]).await?;
    ///
    /// assert_eq!(c.bulk_consume_full_slice(&[4, 8]).await, Err(ConsumeAtLeastError {
    ///     count: 1,
    ///     reason: (),
    /// }));
    ///
    /// assert_eq!(arr, [1, 2, 4]);
    /// # Result::<(), ConsumeAtLeastError<()>>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`ProducerExt::overwrite_full_slice`] method.
    async fn bulk_consume_full_slice(
        &mut self,
        buf: &[Self::Item],
    ) -> Result<(), ConsumeAtLeastError<Self::Error>>
    where
        Self::Item: Clone,
    {
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
