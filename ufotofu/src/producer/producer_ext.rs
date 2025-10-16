use core::cmp::min;

use crate::{
    prelude::*,
    producer::{Buffered, BulkBuffered},
    ProduceAtLeastError,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "dev")]
use crate::producer::{BulkProducerOperation, BulkScrambled};

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

    /// Turns `self` into a buffered bulk producer.
    ///
    /// Whenever the returned producer is tasked to produce an item while the internal buffer is empty, it eagerly fills its buffer with as many items from the wrapped producer as possible before emitting the requested item.
    ///
    /// Prefer to use a [`BulkProducerExt::bulk_buffered`] (which can fill its queue more efficiently with items from a bulk producer).
    ///
    /// The internal buffer can be any value implementing the [`queues::Queue`](crate::queues::Queue) trait. See [`queues::new_static`](crate::queues::new_static) and [`queues::new_fixed`](crate::queues::new_fixed) for convenient ways of creating suitable queues.
    ///
    /// Use the `AsRef<P>` impl to access the wrapped producer.
    ///
    /// # Examples
    ///
    /// The returned producer will eagerly fetch multiple items from the wrapped producer.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let data = [1, 2, 4];
    /// let mut p = producer::clone_from_slice(&data[..]);
    ///
    /// // Create a buffered version of `p`, with a buffer of two items.
    /// let mut buffered = p.buffered(queues::new_static::<_, 2>());
    ///
    /// // Produce a single first item now fetches *two* items from the wrapped producer.
    /// assert_eq!(buffered.produce().await?, Left(1));
    /// assert_eq!(buffered.into_inner().remaining(), &[4]);
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// Use [`Producer::slurp`] to force pre-fetching without actually producing anything.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let data = [1, 2, 4];
    /// let mut p = producer::clone_from_slice(&data[..]);
    ///
    /// let mut buffered = p.buffered(queues::new_static::<_, 2>());
    ///
    /// // Slurping triggers the side-effects of producing items from the inner consumer.
    /// buffered.slurp().await?;
    /// assert_eq!(buffered.into_inner().remaining(), &[4]);
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// While the buffer is not empty, producing more items will not invoke the wrapped producer.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let data = [1, 2, 4];
    /// let mut p = producer::clone_from_slice(&data[..]);
    ///
    /// let mut buffered = p.buffered(queues::new_static::<_, 2>());
    ///
    /// assert_eq!(buffered.produce().await?, Left(1));
    /// assert_eq!(buffered.produce().await?, Left(2));
    /// assert_eq!(buffered.into_inner().remaining(), &[4]);
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// When prefetching items encounters a final value or an error, this is *not* reported immediately. Only after all buffered items have been produced is the final value or error emitted.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let data = [1, 2, 4];
    /// let mut p = producer::clone_from_slice(&data[..]);
    ///
    /// let mut buffered = p.buffered(queues::new_static::<_, 2>());
    ///
    /// assert_eq!(buffered.produce().await?, Left(1));
    /// assert_eq!(buffered.produce().await?, Left(2));
    ///
    /// // Slurping now (whether explicitly or by calling `produce`) will internally encounter the
    /// // final value, but it is buffered instead of reported.
    /// assert_eq!(buffered.produce().await?, Left(4));
    /// // Only now is the final value emitted.
    /// assert_eq!(buffered.produce().await?, Right(()));
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [ConsumerExt::buffered] method.
    fn buffered<Q>(self, queue: Q) -> Buffered<Self, Self::Final, Self::Error, Q>
    where
        Self: Sized,
    {
        Buffered::new(self, queue)
    }

    /// Returns whether this producer and another producer emit equal sequences of items.
    ///
    /// This causes both producers actually emit all their values (and this method simply drops them).
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p1 = [1, 2, 4].into_producer();
    /// let mut p2 = [2, 4].into_producer();
    ///
    /// assert_eq!(p1.equals(&mut p2).await, false);
    ///
    /// let mut p3 = [1, 2, 4].into_producer();
    /// let mut p4 = [2, 4].into_producer();
    ///
    /// assert_eq!(p3.produce().await?, Left(1));
    /// assert_eq!(p3.equals(&mut p4).await, true);
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: none, because you cannot retroactively check whether two consumers have consumed equal sequences.
    async fn equals<P>(&mut self, other: &mut P) -> bool
    where
        P: Producer<Item = Self::Item, Final = Self::Final, Error = Self::Error>,
        Self::Item: PartialEq,
        Self::Final: PartialEq,
        Self::Error: PartialEq,
    {
        loop {
            match (self.produce().await, other.produce().await) {
                (Ok(Left(it1)), Ok(Left(it2))) => {
                    if it1 != it2 {
                        return false;
                    }
                }
                (Ok(Right(fin1)), Ok(Right(fin2))) => {
                    return fin1 == fin2;
                }
                (Err(err1), Err(err2)) => {
                    return err1 == err2;
                }
                _ => return false,
            }
        }
    }

    /// Exactly the same as [`ProducerExt::equals`] but also logs the return values of all `produce` calls to the terminal. Requires the `dev` feature.
    #[cfg(feature = "dev")]
    async fn equals_dbg<P>(&mut self, other: &mut P) -> bool
    where
        P: Producer<Item = Self::Item, Final = Self::Final, Error = Self::Error>,
        Self::Item: PartialEq + alloc::fmt::Debug,
        Self::Final: PartialEq + alloc::fmt::Debug,
        Self::Error: PartialEq + alloc::fmt::Debug,
    {
        loop {
            match (self.produce().await, other.produce().await) {
                (Ok(Left(it1)), Ok(Left(it2))) => {
                    std::println!("First items:\n{:?}\n{:?}", it1, it2);
                    if it1 != it2 {
                        return false;
                    }
                }
                (Ok(Right(fin1)), Ok(Right(fin2))) => {
                    std::println!("Final values:\n{:?}\n{:?}", fin1, fin2);
                    return fin1 == fin2;
                }
                (Err(err1), Err(err2)) => {
                    std::println!("Errors:\n{:?}\n{:?}", err1, err2);
                    return err1 == err2;
                }
                (ret1, ret2) => {
                    std::println!("Mismatching returns:\n{:?}\n{:?}", ret1, ret2);
                    return false;
                }
            }
        }
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
    /// // If we reported that we only processed two items, the producer would later emit the `4`:
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
    /// This method will return `Ok(Left(0))` only when `buf` has length zero. It *may* still forward a final value or error instead when called with a zero-length buffer.
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
    /// <br/>Counterpart: the [`BulkConsumerExt::bulk_consume`] method.
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

    /// Turns `self` into a buffered bulk producer.
    ///
    /// Whenever the returned producer is tasked to produce an item while the internal buffer is empty, it eagerly fills its buffer with as many items from the wrapped producer as possible before emitting the requested item.
    ///
    /// More efficient than [`ProducerExt::buffered`] (which has to fill its buffer by repeatedly calling `produce` instead of using bulk production).
    ///
    /// The internal buffer can be any value implementing the [`queues::Queue`](crate::queues::Queue) trait. See [`queues::new_static`](crate::queues::new_static) and [`queues::new_fixed`](crate::queues::new_fixed) for convenient ways of creating suitable queues.
    ///
    /// Use the `AsRef<P>` impl to access the wrapped producer.
    ///
    /// # Examples
    ///
    /// The returned producer will eagerly fetch multiple items from the wrapped producer.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let data = [1, 2, 4];
    /// let mut p = producer::clone_from_slice(&data[..]);
    ///
    /// // Create a buffered version of `p`, with a buffer of two items.
    /// let mut buffered = p.buffered(queues::new_static::<_, 2>());
    ///
    /// // Produce a single first item now fetches *two* items from the wrapped producer.
    /// assert_eq!(buffered.produce().await?, Left(1));
    /// assert_eq!(buffered.into_inner().remaining(), &[4]);
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// Use [`Producer::slurp`] to force pre-fetching without actually producing anything.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let data = [1, 2, 4];
    /// let mut p = producer::clone_from_slice(&data[..]);
    ///
    /// let mut buffered = p.buffered(queues::new_static::<_, 2>());
    ///
    /// // Slurping triggers the side-effects of producing items from the inner consumer.
    /// buffered.slurp().await?;
    /// assert_eq!(buffered.into_inner().remaining(), &[4]);
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// While the buffer is not empty, producing more items will not invoke the wrapped producer.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let data = [1, 2, 4];
    /// let mut p = producer::clone_from_slice(&data[..]);
    ///
    /// let mut buffered = p.buffered(queues::new_static::<_, 2>());
    ///
    /// assert_eq!(buffered.produce().await?, Left(1));
    /// assert_eq!(buffered.produce().await?, Left(2));
    /// assert_eq!(buffered.into_inner().remaining(), &[4]);
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// When prefetching items encounters a final value or an error, this is *not* reported immediately. Only after all buffered items have been produced is the final value or error emitted.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// use ufotofu::queues;
    ///
    /// # pollster::block_on(async{
    /// let data = [1, 2, 4];
    /// let mut p = producer::clone_from_slice(&data[..]);
    ///
    /// let mut buffered = p.buffered(queues::new_static::<_, 2>());
    ///
    /// assert_eq!(buffered.produce().await?, Left(1));
    /// assert_eq!(buffered.produce().await?, Left(2));
    ///
    /// // Slurping now (whether explicitly or by calling `produce`) will internally encounter the
    /// // final value, but it is buffered instead of reported.
    /// assert_eq!(buffered.produce().await?, Left(4));
    /// // Only now is the final value emitted.
    /// assert_eq!(buffered.produce().await?, Right(()));
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [BulkConsumerExt::bulk_buffered] method.
    fn bulk_buffered<Q>(self, queue: Q) -> BulkBuffered<Self, Self::Final, Self::Error, Q>
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
    /// <br/>Counterpart: the [BulkConsumerExt::bulk_scrambled] method.
    #[cfg(feature = "dev")]
    fn bulk_scrambled<Q>(
        self,
        buffer: Q,
        ops: Vec<BulkProducerOperation>,
    ) -> BulkScrambled<Self, Q, Self::Final, Self::Error>
    where
        Self: Sized,
    {
        BulkScrambled::new(self, buffer, ops)
    }
}
