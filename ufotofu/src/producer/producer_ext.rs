use either::Either::*;

use crate::errors::*;
use crate::BulkProducer;
use crate::Producer;

impl<P> ProducerExt for P where P: Producer {}

/// An extension trait for [`Producer`] that provides a variety of convenient combinator functions.
///
/// <br/>Counterpart: the [`ConsumerExt`](crate::ConsumerExt) trait.
pub trait ProducerExt: Producer {
    /// Tries to produce a regular item, and reports an error if the final item was produced instead.
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
    ///
    /// <br/>Counterpart: the [TODO] method.
    async fn produce_item(
        &mut self,
    ) -> Result<Self::Item, ProduceAtLeastError<Self::Final, Self::Error>> {
        match self.produce().await {
            Ok(Left(item)) => Ok(item),
            Ok(Right(fin)) => Err(ProduceAtLeastError {
                count: 0,
                reason: Left(fin),
            }),
            Err(err) => Err(ProduceAtLeastError {
                count: 0,
                reason: Right(err),
            }),
        }
    }

    /// Tries to completely overwrite a slice with items from a producer.
    /// Reports an error if the slice could not be overwritten completely.
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
    ///
    /// <br/>Counterpart: the [TODO] method
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
                        reason: Left(fin),
                    })
                }
                Err(err) => {
                    return Err(ProduceAtLeastError {
                        count: i,
                        reason: Right(err),
                    })
                }
            }
        }

        Ok(())
    }
}

impl<P> BulkProducerExt for P where P: BulkProducer {}

/// An extension trait for [`BulkProducer`] that provides a variety of convenient combinator functions.
pub trait BulkProducerExt: BulkProducer {
    /// Tries to completely overwrite a slice with items from a bulk producer.
    /// Reports an error if the slice could not be overwritten completely.
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
    ///
    /// <br/>Counterpart: the [TODO] method
    async fn bulk_overwrite_full_slice(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<(), ProduceAtLeastError<Self::Final, Self::Error>> {
        let mut produced_so_far = 0;

        while produced_so_far < buf.len() {
            match self.bulk_produce(&mut buf[produced_so_far..]).await {
                Ok(Left(count)) => produced_so_far += count,
                Ok(Right(fin)) => {
                    return Err(ProduceAtLeastError {
                        count: produced_so_far,
                        reason: Left(fin),
                    });
                }
                Err(err) => {
                    return Err(ProduceAtLeastError {
                        count: produced_so_far,
                        reason: Right(err),
                    });
                }
            }
        }

        Ok(())
    }
}
