use core::cmp::min;

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
    /// Behaves exactly like [`BulkConsumer::expose_slots`], except the function argument is not async.
    ///
    /// # Examples
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// assert_eq!(c.expose_slots_sync(|mut slots| {
    ///     slots[0] = 1;
    ///     slots[1] = 2;
    ///     slots[2] = 4;
    ///     (3, "hi!")
    /// }).await?, "hi!");
    /// assert_eq!(c.consume(8).await, Err(()));
    ///
    /// assert_eq!(arr, [1, 2, 4]);
    ///
    /// // If we reported that we only wrote two items, the consumer would later accept another item:
    /// let mut arr2 = [0, 0, 0];
    /// let mut c2 = (&mut arr2).into_consumer();
    ///
    /// assert_eq!(c2.expose_slots_sync(|mut slots| {
    ///     slots[0] = 1;
    ///     slots[1] = 2;
    ///     slots[2] = 4;
    ///     (2, "hi!")
    /// }).await?, "hi!");
    /// assert_eq!(c2.consume(8).await, Ok(()));
    ///
    /// assert_eq!(arr2, [1, 2, 8]);
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`BulkProducerExt::expose_items_sync`] method.
    async fn expose_slots_sync<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(&mut [Self::Item]) -> (usize, R),
    {
        self.expose_slots(async |slots| f(slots)).await
    }

    /// Calls `self.expose_slots`, clones items from the given buffer into the exposed slots, and returns how many items where written there. Alternatively, forwards any error. This method is mostly analogous to [`std::io::Write::write`].
    ///
    /// This method will return `Ok(Left(0))` only when `buf` has length zero. It *may* still forward an error instead when called with a zero-length buffer.
    ///
    /// Note that this function does not attempt to consume all items in `buf`, it only does a *single* call to `self.expose_slots` and then clones as many items as it has available (and as will fit). See [`BulkConsumerExt::bulk_consume_full_slice`] if you want to *completely* consume a slice.
    ///
    /// # Example
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// assert_eq!(c.bulk_consume(&[1, 2]).await?, 2);
    /// assert_eq!(c.bulk_consume(&[4, 8]).await?, 1);
    ///
    /// assert_eq!(arr, [1, 2, 4]);
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`BulkProducer::bulk_produce`] method.
    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error>
    where
        Self::Item: Clone,
    {
        self.expose_slots_sync(|slots| {
            let amount = min(slots.len(), buf.len());
            slots[..amount].clone_from_slice(&buf[..amount]);
            (amount, amount)
        })
        .await
    }

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
