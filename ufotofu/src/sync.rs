use core::cmp::min;

use either::{
    Either,
    Either::{Left, Right},
};

pub use crate::common::errors::{ConsumeFullSliceError, OverwriteFullSliceError};

pub mod consumer;
pub mod producer;

/// A `Consumer` consumes a potentially infinite sequence, one item at a time.
///
/// The sequence consists of an arbitrary number of values of type `Self::Item`, followed by
/// up to one value of type `Self::Final`. If you intend for the sequence to be infinite, use
/// [`Infallible`](core::convert::Infallible) for `Self::Final`.
///
/// A consumer can also signal an error of type `Self::Error` instead of consuming an item.
pub trait Consumer {
    /// The sequence consumed by this consumer *starts* with *arbitrarily many* values of this type.
    type Item;
    /// The sequence consumed by this consumer *ends* with *up to one* value of this type.
    type Final;
    /// The type of errors the consumer can emit instead of doing its job.
    type Error;

    /// Attempt to consume the next item.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait returned an error,
    /// nor after `close` was called.
    fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error>;

    /// Attempt to consume the final item.
    ///
    /// After this function is called, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after `close` was called.
    fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error>;

    /// Try to consume (clones of) *all* items in the given slice.
    /// Reports an error if the slice could not be consumed completely.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after `close` was called.
    ///
    /// #### Implementation Notes
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    fn consume_full_slice(
        &mut self,
        buf: &[Self::Item],
    ) -> Result<(), ConsumeFullSliceError<Self::Error>>
    where
        Self::Item: Clone,
    {
        for i in 0..buf.len() {
            let item = buf[i].clone();

            if let Err(err) = self.consume(item) {
                return Err(ConsumeFullSliceError {
                    consumed: i,
                    reason: err,
                });
            }
        }

        Ok(())
    }
}

/// A `Consumer` that can delay performing side-effects when consuming items.
///
/// It must not delay performing side-effects when being closed. In other words,
/// calling `close` should internally trigger flushing.
pub trait BufferedConsumer: Consumer {
    /// Perform any side-effects that were delayed for previously consumed items.
    ///
    /// This function allows the client code to force execution of the (potentially expensive)
    /// side-effects. In exchange, the consumer gains the freedom to delay the side-effects of
    /// `consume` to improve efficiency.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after `close` was called.
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/// A `Consumer` that is able to consume several items with a single function call, in order to
/// improve on the efficiency of the `Consumer` trait. Semantically, there must be no
/// difference between consuming items in bulk or one item at a time.
///
/// Note that `Self::Item` must be `Copy` for efficiency reasons.
pub trait BulkConsumer: BufferedConsumer
where
    Self::Item: Copy,
{
    /// A low-level method for consuming multiple items at a time. If you are only *working* with consumers (rather than *implementing* them), you will probably want to ignore this method and use [BulkConsumer::bulk_consume] instead.
    ///
    /// Expose a non-empty slice of memory for the client code to fill with items that should
    /// be consumed.
    ///
    /// The consumer should expose the largest contiguous slice it can expose efficiently.
    /// It should not perform copies to increase the size of the slice. Client code must be
    /// able to make progress even if this function always returns slices of size one.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after `close` was called.
    fn expose_slots(&mut self) -> Result<&mut [Self::Item], Self::Error>;

    /// A low-level method for consuming multiple items at a time. If you are only *working* with consumers (rather than *implementing* them), you will probably want to ignore this method and use [BulkConsumer::bulk_consume] instead.
    ///
    /// Instruct the consumer to consume the first `amount` many items of the slots
    /// it has most recently exposed. The semantics must be equivalent to those of `consume`
    /// being called `amount` many times with exactly those items.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Callers must have written into (at least) the `amount` many first slots that
    /// were most recently exposed. Failure to uphold this invariant may cause undefined behavior.
    ///
    /// Must not be called after any function of this trait has returned an error, nor after
    /// `close` was called.
    fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error>;

    /// Consume a non-zero number of items by reading them from a given buffer and returning how
    /// many items were consumed.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error, nor after
    /// `close` was called.
    ///
    /// #### Implementation Notes
    ///
    /// The default implementation orchestrates `expose_slots` and `consume_slots` in a
    /// straightforward manner. Only provide your own implementation if you can do better
    /// than that.
    fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        let slots = self.expose_slots()?;
        let amount = min(slots.len(), buf.len());
        slots[0..amount].copy_from_slice(&buf[0..amount]);
        self.consume_slots(amount)?;

        Ok(amount)
    }

    /// Try to bulk-consume (copies of) *all* items in the given slice.
    /// Reports an error if the slice could not be consumed completely.
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after `close` was called.
    ///
    /// #### Implementation Notes
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    fn bulk_consume_full_slice(
        &mut self,
        buf: &[Self::Item],
    ) -> Result<(), ConsumeFullSliceError<Self::Error>> {
        let mut consumed_so_far = 0;

        while consumed_so_far < buf.len() {
            match self.bulk_consume(&buf[consumed_so_far..]) {
                Ok(consumed_count) => consumed_so_far += consumed_count,
                Err(err) => {
                    return Err(ConsumeFullSliceError {
                        consumed: consumed_so_far,
                        reason: err,
                    });
                }
            }
        }

        Ok(())
    }
}

/// A `Producer` produces a potentially infinite sequence, one item at a time.
///
/// The sequence consists of an arbitrary number of values of type `Self::Item`, followed by
/// up to one value of type `Self::Final`. If you intend for the sequence to be infinite, use
/// [`Infallible`](core::convert::Infallible) for `Self::Final`.
///
/// A producer can also signal an error of type `Self::Error` instead of producing an item.
pub trait Producer {
    /// The sequence produced by this producer *starts* with *arbitrarily many* values of this type.
    type Item;
    /// The sequence produced by this producer *ends* with *up to one* value of this type.
    type Final;
    /// The type of errors the producer can emit instead of doing its job.
    type Error;

    /// Attempt to produce the next item, which is either a regular repeated item or the final item.
    /// If the sequence of items has not ended yet, but no item are available at the time of calling,
    /// the function must block until at least one more item becomes available (or it becomes clear
    /// that the final value or an error should be yielded).
    ///
    /// After this function returns the final item, or after it returns an error, no further
    /// functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned a final item or an error.
    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error>;

    /// Try to completely overwrite a slice with items from a producer.
    /// Reports an error if the slice could not be overwritten completely.
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after `close` was called.
    ///
    /// #### Implementation Notes
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    fn overwrite_full_slice(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<(), OverwriteFullSliceError<Self::Final, Self::Error>> {
        for i in 0..buf.len() {
            match self.produce() {
                Ok(Left(item)) => buf[i] = item,
                Ok(Right(fin)) => {
                    return Err(OverwriteFullSliceError {
                        overwritten: i,
                        reason: Left(fin),
                    })
                }
                Err(err) => {
                    return Err(OverwriteFullSliceError {
                        overwritten: i,
                        reason: Right(err),
                    })
                }
            }
        }

        Ok(())
    }
}

/// A `Producer` that can eagerly perform side-effects to prepare values for later yielding.
pub trait BufferedProducer: Producer {
    /// Prepare some values for yielding. This function allows the `Producer` to perform side
    /// effects that it would otherwise have to do just-in-time when `produce` gets called.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned a final item or an error.
    fn slurp(&mut self) -> Result<(), Self::Error>;
}

/// A `Producer` that is able to produce several items with a single function call, in order to
/// improve on the efficiency of the `Producer` trait. Semantically, there must be no difference
/// between producing items in bulk or one item at a time.
///
/// Note that `Self::Item` must be `Copy` for efficiency reasons.
pub trait BulkProducer: BufferedProducer
where
    Self::Item: Copy,
{
    /// A low-level method for producing multiple items at a time. If you are only *working* with producers (rather than *implementing* them), you will probably want to ignore this method and use [BulkProducer::bulk_produce] or [BulkProducer::bulk_produce_uninit] instead.
    ///
    ///  Expose a non-empty slice of items to be produced (or the final value, or an error).
    /// The items in the slice must not have been emitted by `produce` before. If the sequence
    /// of items has not ended yet, but no item is available at the time of calling, the
    /// function must block until at least one more item becomes available (or it becomes clear
    /// that the final value or an error should be yielded).
    ///
    /// The producer should expose the largest contiguous slice it can expose efficiently.
    /// It should not perform copies to increase the size of the slice. Client code must be
    /// able to make progress even if this function always returns slices of size one.
    ///
    /// After this function returns the final item, or after it returns an error, no further
    /// functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned a final item or an error.
    #[allow(clippy::type_complexity)]
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error>;

    /// A low-level method for producing multiple items at a time. If you are only *working* with producers (rather than *implementing* them), you will probably want to ignore this method and use [BulkProducer::bulk_produce] or [BulkProducer::bulk_produce_uninit] instead.
    ///
    ///  Mark `amount` many items as having been produced. Future calls to `produce` and to
    /// `expose_items` must act as if `produce` had been called `amount` many times.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Callers must not mark items as produced that had not previously been exposed by `expose_items`.
    ///
    /// Must not be called after any function of this trait returned a final item or an error.
    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error>;

    /// Produce a non-zero number of items by writing them into a given buffer and returning how
    /// many items were produced. If the sequence of items has not ended yet, but no item is
    /// available at the time of calling, the function must block until at least one more item
    /// becomes available (or it becomes clear that the final value or an error should be yielded).
    ///
    /// After this function returns the final item, or after it returns an error, no further
    /// functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned a final item or an error.
    ///
    /// #### Implementation Notes
    ///
    /// The default implementation orchestrates `expose_items` and `consider_produced` in a
    /// straightforward manner. Only provide your own implementation if you can do better
    /// than that.
    fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        match self.expose_items()? {
            Either::Left(slots) => {
                let amount = min(slots.len(), buf.len());
                buf[0..amount].copy_from_slice(&slots[0..amount]);

                self.consider_produced(amount)?;

                Ok(Either::Left(amount))
            }
            Either::Right(final_value) => Ok(Either::Right(final_value)),
        }
    }

    /// Try to completely overwrite a slice with items from a bulk producer.
    /// Reports an error if the slice could not be overwritten completely.
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after `close` was called.
    ///
    /// #### Implementation Notes
    ///
    /// This is a trait method for convenience, you should never need to
    /// replace the default implementation.
    fn bulk_overwrite_full_slice(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<(), OverwriteFullSliceError<Self::Final, Self::Error>> {
        let mut produced_so_far = 0;

        while produced_so_far < buf.len() {
            match self.bulk_produce(&mut buf[produced_so_far..]) {
                Ok(Left(count)) => produced_so_far += count,
                Ok(Right(fin)) => {
                    return Err(OverwriteFullSliceError {
                        overwritten: produced_so_far,
                        reason: Left(fin),
                    });
                }
                Err(err) => {
                    return Err(OverwriteFullSliceError {
                        overwritten: produced_so_far,
                        reason: Right(err),
                    });
                }
            }
        }

        Ok(())
    }
}

pub use crate::common::errors::PipeError;

/// Pipe as many items as possible from a producer into a consumer. Then call `close`
/// on the consumer with the final value emitted by the producer.
pub fn pipe<P, C>(producer: &mut P, consumer: &mut C) -> Result<(), PipeError<P::Error, C::Error>>
where
    P: Producer,
    C: Consumer<Item = P::Item, Final = P::Final>,
{
    loop {
        match producer.produce() {
            Ok(Either::Left(item)) => {
                match consumer.consume(item) {
                    Ok(()) => {
                        // No-op, continues with next loop iteration.
                    }
                    Err(consumer_error) => {
                        return Err(PipeError::Consumer(consumer_error));
                    }
                }
            }
            Ok(Either::Right(final_value)) => match consumer.close(final_value) {
                Ok(()) => {
                    return Ok(());
                }
                Err(consumer_error) => {
                    return Err(PipeError::Consumer(consumer_error));
                }
            },
            Err(producer_error) => {
                return Err(PipeError::Producer(producer_error));
            }
        }
    }
}

/// Efficiently pipe as many items as possible from a bulk producer into a bulk consumer
/// using `consumer.bulk_consume`. Then call `close` on the consumer with the final value
/// emitted by the producer.
pub fn bulk_pipe<P, C>(
    producer: &mut P,
    consumer: &mut C,
) -> Result<(), PipeError<P::Error, C::Error>>
where
    P: BulkProducer,
    P::Item: Copy,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
{
    loop {
        match producer.expose_items() {
            Ok(Either::Left(slots)) => {
                let amount = match consumer.bulk_consume(slots) {
                    Ok(amount) => amount,
                    Err(consumer_error) => return Err(PipeError::Consumer(consumer_error)),
                };
                match producer.consider_produced(amount) {
                    Ok(()) => {
                        // No-op, continues with next loop iteration.
                    }
                    Err(producer_error) => return Err(PipeError::Producer(producer_error)),
                };
            }
            Ok(Either::Right(final_value)) => {
                match consumer.close(final_value) {
                    Ok(()) => return Ok(()),
                    Err(consumer_error) => return Err(PipeError::Consumer(consumer_error)),
                };
            }
            Err(producer_error) => {
                return Err(PipeError::Producer(producer_error));
            }
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     use crate::sync::consumer::{IntoSlice, IntoVec};
//     use crate::sync::producer::FromSlice;

//     #[test]
//     fn pipes_from_slice_producer_to_slice_consumer() -> Result<(), PipeError<!, ()>> {
//         let mut buf = [0; 3];

//         let mut o = FromSlice::new(b"ufo");
//         let mut i = IntoSlice::new(&mut buf);

//         pipe(&mut o, &mut i)?;

//         let m = min(o.as_ref().len(), i.as_ref().len());
//         assert_eq!(&i.as_ref()[..m], &o.as_ref()[..m]);
//         assert_eq!(&buf, b"ufo");

//         Ok(())
//     }

//     #[test]
//     fn pipes_from_slice_producer_to_consumer_into_vec() -> Result<(), PipeError<!, !>> {
//         let mut o = FromSlice::new(b"tofu");
//         let mut i = IntoVec::new();

//         pipe(&mut o, &mut i)?;

//         assert_eq!(&i.into_vec(), b"tofu");

//         Ok(())
//     }

//     #[test]
//     fn bulk_pipes_from_slice_producer_to_slice_consumer() -> Result<(), PipeError<!, ()>> {
//         let mut buf = [0; 3];

//         let mut o = FromSlice::new(b"ufo");
//         let mut i = IntoSlice::new(&mut buf);

//         bulk_pipe(&mut o, &mut i)?;

//         let m = min(o.as_ref().len(), i.as_ref().len());
//         assert_eq!(&i.as_ref()[..m], &o.as_ref()[..m]);
//         assert_eq!(&buf, b"ufo");

//         Ok(())
//     }

//     #[test]
//     fn bulk_pipes_from_slice_producer_to_consumer_into_vec() -> Result<(), PipeError<!, !>> {
//         let mut o = FromSlice::new(b"tofu");
//         let mut i = IntoVec::new();

//         bulk_pipe(&mut o, &mut i)?;

//         assert_eq!(&i.into_vec(), b"tofu");

//         Ok(())
//     }
// }
