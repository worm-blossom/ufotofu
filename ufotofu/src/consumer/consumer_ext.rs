use core::cmp::min;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::{
    consumer::{Buffered, BulkBuffered},
    prelude::*,
    ConsumeAtLeastError,
};

#[cfg(feature = "dev")]
use crate::consumer::{BulkConsumerOperation, BulkScrambled};

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

    /// Turns `self` into a buffered bulk consumer.
    ///
    /// The returned consumer consumes items into a fifo queue. When that queue is full, it gets fully flushed into the wrapped consumer.
    ///
    /// Prefer to use a [`BulkConsumerExt::bulk_buffered`] (which can transfer queued items more efficiently into the wrapped bulk consumer).
    ///
    /// The internal buffer can be any value implementing the [`queues::Queue`](crate::queues::Queue) trait. See [`queues::new_static`](crate::queues::new_static) and [`queues::new_fixed`](crate::queues::new_fixed) for convenient ways of creating suitable queues.
    ///
    /// Use the `AsRef<C>` impl to access the wrapped consumer.
    ///
    /// # Examples
    ///
    /// The returned consumer will delay side-effects as long as possible.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// // Create a buffered version of `c`, with a buffer of two items.
    /// let mut buffered = c.buffered(queues::new_static::<_, 2>());
    ///
    /// // Consuming items with the new consumer will *not* write them into `arr` immediately.
    /// assert_eq!(buffered.bulk_consume(&[1, 2]).await?, 2);
    /// assert_eq!(arr, [0, 0, 0]);
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    ///
    /// Use [`Consumer::flush`] to force side-effects to happen.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// let mut buffered = c.buffered(queues::new_static::<_, 2>());
    ///
    /// assert_eq!(buffered.bulk_consume(&[1, 2]).await?, 2);
    /// // By flushing, we can trigger the side-effects for the buffered items.
    /// buffered.flush().await?;
    /// assert_eq!(arr, [1, 2, 0]);
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    ///
    /// Items will be flushed implicitly when consuming an item while the internal buffer is full.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// let mut buffered = c.buffered(queues::new_static::<_, 2>());
    ///
    /// assert_eq!(buffered.bulk_consume(&[1, 2]).await?, 2);
    /// // Consuming another item while the buffer is full triggers an implicit flush.
    /// // The third item is then placed in the buffer.
    /// buffered.consume(4).await?;
    /// assert_eq!(arr, [1, 2, 0]);
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    ///
    /// Note that buffering can result in successful consumption being reported, only to later encounter an error when flushing the buffer into the underlying consumer. This can result in items being lost.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// let mut buffered = c.buffered(queues::new_static::<_, 2>());
    ///
    /// assert_eq!(buffered.bulk_consume(&[1, 2]).await?, 2);
    /// // Consuming another item while the buffer is full triggers an implicit flush.
    /// // We can then continue filling up the buffer.
    /// buffered.consume(4).await?;
    /// buffered.consume(8).await?;
    ///
    /// // Flushing now (whether explicitly, by closing, or by consuming another item) will
    /// // trigger an error, because the underlying consumer can consume at most three items.
    /// // All items up until that error are flushed successfully though.
    /// assert_eq!(buffered.flush().await, Err(()));
    /// assert_eq!(arr, [1, 2, 4]);
    ///
    /// // The fourth item we consumed is lost forever. Such are the perils of buffered consumers.
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [ProducerExt::buffered] method.
    fn buffered<Q>(self, queue: Q) -> Buffered<Self, Q>
    where
        Self: Sized,
    {
        Buffered::new(self, queue)
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
    /// <br/>Counterpart: the [`BulkProducerExt::bulk_produce`] method.
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

    /// Turns `self` into a buffered bulk consumer.
    ///
    /// The returned consumer consumes items into a fifo queue. When that queue is full, it gets fully flushed into the wrapped consumer.
    ///
    /// More efficient than [`ConsumerExt::buffered`] (which has to flush its buffer by repeatedly calling `consume` instead of using bulk consumption).
    ///
    /// The internal buffer can be any value implementing the [`queues::Queue`](crate::queues::Queue) trait. See [`queues::new_static`](crate::queues::new_static) and [`queues::new_fixed`](crate::queues::new_fixed) for convenient ways of creating suitable queues.
    ///
    /// Use the `AsRef<C>` impl to access the wrapped consumer.
    ///
    /// # Examples
    ///
    /// The returned consumer will delay side-effects as long as possible.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// // Create a buffered version of `c`, with a buffer of two items.
    /// let mut buffered = c.bulk_buffered(queues::new_static::<_, 2>());
    ///
    /// // Consuming items with the new consumer will *not* write them into `arr` immediately.
    /// assert_eq!(buffered.bulk_consume(&[1, 2]).await?, 2);
    /// assert_eq!(arr, [0, 0, 0]);
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    ///
    /// Use [`Consumer::flush`] to force side-effects to happen.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// let mut buffered = c.bulk_buffered(queues::new_static::<_, 2>());
    ///
    /// assert_eq!(buffered.bulk_consume(&[1, 2]).await?, 2);
    /// // By flushing, we can trigger the side-effects for the buffered items.
    /// buffered.flush().await?;
    /// assert_eq!(arr, [1, 2, 0]);
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    ///
    /// Items will be flushed implicitly when consuming an item while the internal buffer is full.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// let mut buffered = c.bulk_buffered(queues::new_static::<_, 2>());
    ///
    /// assert_eq!(buffered.bulk_consume(&[1, 2]).await?, 2);
    /// // Consuming another item while the buffer is full triggers an implicit flush.
    /// // The third item is then placed in the buffer.
    /// buffered.consume(4).await?;
    /// assert_eq!(arr, [1, 2, 0]);
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    ///
    /// Note that buffering can result in successful consumption being reported, only to later encounter an error when flushing the buffer into the underlying consumer. This can result in items being lost.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let mut arr = [0, 0, 0];
    /// let mut c = (&mut arr).into_consumer();
    ///
    /// let mut buffered = c.bulk_buffered(queues::new_static::<_, 2>());
    ///
    /// assert_eq!(buffered.bulk_consume(&[1, 2]).await?, 2);
    /// // Consuming another item while the buffer is full triggers an implicit flush.
    /// // We can then continue filling up the buffer.
    /// buffered.consume(4).await?;
    /// buffered.consume(8).await?;
    ///
    /// // Flushing now (whether explicitly, by closing, or by consuming another item) will
    /// // trigger an error, because the underlying consumer can consume at most three items.
    /// // All items up until that error are flushed successfully though.
    /// assert_eq!(buffered.flush().await, Err(()));
    /// assert_eq!(arr, [1, 2, 4]);
    ///
    /// // The fourth item we consumed is lost forever. Such are the perils of buffered consumers.
    /// # Result::<(), ()>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [BulkProducerExt::bulk_buffered] method.
    fn bulk_buffered<Q>(self, queue: Q) -> BulkBuffered<Self, Q>
    where
        Self: Sized,
    {
        BulkBuffered::new(self, queue)
    }

    /// Turns `self` into a [scrambling](BulkScrambled) producer.
    ///
    /// The returned producer is semantically indistinguishable from `self`, but interacts with the original bulk producer according to a fixed (usually randomly generated) pattern of methods.
    ///
    /// See the [fuzz-testing tutorial](crate::fuzz_testing_tutorial) for typical usage.
    ///
    /// <br/>Counterpart: the [BulkProducerExt::bulk_scrambled] method.
    #[cfg(feature = "dev")]
    fn bulk_scrambled<Q>(self, buffer: Q, ops: Vec<BulkConsumerOperation>) -> BulkScrambled<Self, Q>
    where
        Self: Sized,
    {
        BulkScrambled::new(self, buffer, ops)
    }
}
