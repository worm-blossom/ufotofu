use core::cmp::min;

use crate::{prelude::*, ProduceAtLeastError};

impl<P> ProducerExt for P where P: Producer {}

/// An extension trait for [`Producer`] that provides a variety of convenient combinator functions.
/// You never need to implement this trait yourself, it merely adds methods with default implementation to existing producers.
///
/// <br/>Counterpart: the [`ConsumerExt`](crate::ConsumerExt) trait.
pub trait ProducerExt: Producer {
    /// Tries to produce a regular item, and reports an error if the final value was produced instead.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::ProduceAtLeastError;
    /// # pollster::block_on(async{
    /// let mut p = [1, 2, 4].into_producer();
    ///
    /// assert_eq!(p.produce_item().await?, 1);
    /// assert_eq!(p.produce_item().await?, 2);
    /// assert_eq!(p.produce_item().await?, 4);
    /// assert_eq!(p.produce_item().await, Err(ProduceAtLeastError {
    ///     count: 0,
    ///     reason: Ok(()), // Would be an `Err` if `produce` would have errored.
    /// }));
    /// # Result::<(), ProduceAtLeastError<(), Infallible>>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: none, because [`Consumer`] splits up processing regular items and final items into separate methods.
    async fn produce_item(
        &mut self,
    ) -> Result<Self::Item, ProduceAtLeastError<Self::Final, Self::Error>> {
        match self.produce().await {
            Ok(Left(item)) => Ok(item),
            Ok(Right(fin)) => Err(ProduceAtLeastError {
                count: 0,
                reason: Ok(fin),
            }),
            Err(err) => Err(ProduceAtLeastError {
                count: 0,
                reason: Err(err),
            }),
        }
    }

    /// Tries to completely overwrite a slice with items from a producer.
    /// Reports an error if the slice could not be overwritten completely.
    ///
    /// When working with a bulk producer, use
    /// [`BulkProducerExt::bulk_overwrite_full_slice`] for greater efficiency.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::ProduceAtLeastError;
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0];
    /// let mut p = [1, 2, 4].into_producer();
    ///
    /// p.overwrite_full_slice(&mut arr[..]).await?;
    /// assert_eq!(arr, [1, 2]);
    ///
    /// assert_eq!(p.overwrite_full_slice(&mut arr[..]).await, Err(ProduceAtLeastError {
    ///     count: 1,
    ///     reason: Ok(()), // Would be an `Err` if `produce` would have errored.
    /// }));
    /// # Result::<(), ProduceAtLeastError<(), Infallible>>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`ConsumerExt::consume_full_slice`] method.
    async fn overwrite_full_slice(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<(), ProduceAtLeastError<Self::Final, Self::Error>> {
        for i in 0..buf.len() {
            match self.produce().await {
                Ok(Left(item)) => buf[i] = item,
                Ok(Right(fin)) => {
                    return Err(ProduceAtLeastError {
                        count: i,
                        reason: Ok(fin),
                    })
                }
                Err(err) => {
                    return Err(ProduceAtLeastError {
                        count: i,
                        reason: Err(err),
                    })
                }
            }
        }

        Ok(())
    }
}

impl<P> BulkProducerExt for P where P: BulkProducer {}

/// An extension trait for [`BulkProducer`] that provides a variety of convenient combinator functions.
/// You never need to implement this trait yourself, it merely adds methods with default implementation to existing bulk producers.
///
/// <br/>Counterpart: the [`BulkConsumerExt`](crate::BulkConsumerExt) trait.
pub trait BulkProducerExt: BulkProducer {
    /// Behaves exactly like [`BulkProducer::expose_items`], except the function argument is not async.
    ///
    /// # Examples
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = [1, 2, 4].into_producer();
    ///
    /// assert_eq!(p.expose_items_sync(|items| {
    ///     assert_eq!(items, &[1, 2, 4]);
    ///     return (3, "hi!");
    /// }).await?, Left("hi!"));
    /// assert_eq!(p.produce().await?, Right(()));
    ///
    /// // If we reported that we only consumed two items, the producer would later emit the `4`:
    /// let mut p2 = [1, 2, 4].into_producer();
    /// assert_eq!(p2.expose_items_sync(|items| {
    ///     assert_eq!(items, &[1, 2, 4]);
    ///     return (2, "hi!");
    /// }).await?, Left("hi!"));
    /// assert_eq!(p2.produce().await?, Left(4));
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`BulkConsumerExt::expose_slots_sync`] method.
    async fn expose_items_sync<F, R>(&mut self, f: F) -> Result<Either<R, Self::Final>, Self::Error>
    where
        F: FnOnce(&[Self::Item]) -> (usize, R),
    {
        self.expose_items(async |items| f(items)).await
    }

    /// Calls `self.expose_items`, clones the resulting items into the given buffer, and returns how many items where written there. Alternatively, forwards any final value or error. This method is mostly analogous to [`std::io::Read::read`].
    ///
    /// This method will return `Ok(Left(0))` only when `buf` has length zero. It *may* still return a final value or error instead when called with a zero-length buffer.
    ///
    /// Note that this function does not attempt to completely fill `buf`, it only does a *single* call to `self.expose_items` and then clones as many items as it has available (and as will fit). See [`BulkProducerExt::bulk_overwrite_full_slice`] if you want to *completely* fill a slice.
    ///
    /// # Example
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = [1, 2, 4].into_producer();
    /// let mut buf = [0, 0];
    ///
    /// assert_eq!(p.bulk_produce(&mut buf[..]).await?, Left(2));
    /// assert_eq!(buf, [1, 2]);
    /// assert_eq!(p.bulk_produce(&mut buf[..]).await?, Left(1));
    /// assert_eq!(buf, [4, 2]);
    /// assert_eq!(p.bulk_produce(&mut buf[..]).await?, Right(()));
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`BulkConsumer::bulk_consume`] method.
    async fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error>
    where
        Self::Item: Clone,
    {
        self.expose_items_sync(|items| {
            let amount = min(items.len(), buf.len());
            buf[..amount].clone_from_slice(&items[..amount]);
            (amount, amount)
        })
        .await
    }

    /// Tries to completely overwrite a slice with items from a bulk producer.
    /// Reports an error if the slice could not be overwritten completely.
    ///
    /// More efficient than [`ProducerExt::overwrite_full_slice`].
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::ProduceAtLeastError;
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0];
    /// let mut p = [1, 2, 4].into_producer();
    ///
    /// p.bulk_overwrite_full_slice(&mut arr[..]).await?;
    /// assert_eq!(arr, [1, 2]);
    ///
    /// assert_eq!(p.bulk_overwrite_full_slice(&mut arr[..]).await, Err(ProduceAtLeastError {
    ///     count: 1,
    ///     reason: Ok(()), // Would be an `Err` if `produce` would have errored.
    /// }));
    /// # Result::<(), ProduceAtLeastError<(), Infallible>>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`BulkConsumerExt::bulk_consume_full_slice`] method.
    async fn bulk_overwrite_full_slice(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<(), ProduceAtLeastError<Self::Final, Self::Error>>
    where
        Self::Item: Clone,
    {
        let mut produced_so_far = 0;

        while produced_so_far < buf.len() {
            match self.bulk_produce(&mut buf[produced_so_far..]).await {
                Ok(Left(count)) => produced_so_far += count,
                Ok(Right(fin)) => {
                    return Err(ProduceAtLeastError {
                        count: produced_so_far,
                        reason: Ok(fin),
                    });
                }
                Err(err) => {
                    return Err(ProduceAtLeastError {
                        count: produced_so_far,
                        reason: Err(err),
                    });
                }
            }
        }

        Ok(())
    }
}
