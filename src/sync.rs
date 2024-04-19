use core::cmp::min;
use core::mem::MaybeUninit;

use either::{Either, Left, Right};

/// A `Producer` a potentially infinite sequence, one item at a time.
///
/// The sequence consists of an arbitrary number of values of type `Self::Item`, followed by up to one value of type `Self::Final`. If you intend for the sequnce to be infinite, use the empty type `!` for `Self::Final`.
///
/// A producer can also signal an error of type `Self::Error` instead of producing an item.
pub trait Producer {
    /// The sequence produced by this producer *starts* with *arbitrarily many* values of this type.
    type Item;
    /// The sequence produced by this producer *ends* with *up to one* value of this type.
    type Final;
    /// The type of errors the producer can emit instead of doing its job.
    type Error;

    /// Attempt to produce the next item, which is either a regular repeated item, or the final item. If the sequence of items has not ended yet, but no item are available at the time of calling, the function must block until at least one more item becomes available (or it becomes clear that the final value or an error should be yielded).
    ///
    /// After this function returns the final item, or after it returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait returned a final item or an error.
    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error>;
}

/// A `Producer` that can eagerly perform side-effects to prepare values for later yielding.
pub trait BufferedProducer: Producer {
    /// Prepare some values for yielding. This function allows the `Producer` to perform side effects that it would otherwise have to do just-in-time when `produce` gets called.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait returned a final item or an error.
    fn slurp(&mut self) -> Result<(), Self::Error>;
}

/// A `Producer` that is able to produce several items with a single function call, in order to improve on the efficiency of the `Producer` trait. Semantically, there must be no differrence between producing items in bulk or one item at a time.
///
/// Note that `Self::Item` must be `Copy` for efficiency reasons.
pub trait BulkProducer: BufferedProducer
where
    Self::Item: Copy,
{
    /// Expose a non-empty slice of items to be produced (or the final value, or an error). The items in the slice must not have been emitted by `produce` before. If the sequence of items has not ended yet, but no item are available at the time of calling, the function must block until at least one more item becomes available (or it becomes clear that the final value or an error should be yielded).
    ///
    /// The producer should expose the largest contiguous slice it can expose efficiently. It should not perform copies to increase the size of the slice. Client code must be able to make progress even if this function always returns slices of size one.
    ///
    /// After this function returns the final item, or after it returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait returned a final item or an error.
    fn producer_slots(&self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error>;

    /// Mark `amount` many items as having been produced. Future calls to `produce` and to `producer_slots` must act as if `produce` had been called `amount` many times.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Callers must not mark items as produced that had not previously been exposed by `producer_slots`.
    ///
    /// Must not be called after any function of this trait returned a final item or an error.
    fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error>;

    /// Produce a non-zero number of items by writing them into a given buffer and returning how many items were produced. If the sequence of items has not ended yet, but no item are available at the time of calling, the function must block until at least one more item becomes available (or it becomes clear that the final value or an error should be yielded).
    ///
    /// After this function returns the final item, or after it returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait returned a final item or an error.
    ///
    /// #### Implementation Notes
    ///
    /// The default implementation orchestrates `producer_slots` and `did_produce` in a straightforward manner. Only provide your own implementation if you can do better than that.
    fn bulk_produce(
        &mut self,
        buf: &mut [MaybeUninit<Self::Item>],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        return Ok(self.producer_slots()?.map_left(|slots| {
            let amount = min(slots.len(), buf.len());
            MaybeUninit::copy_from_slice(&mut buf[0..amount], &slots[0..amount]);
            self.did_produce(amount)?;
            return amount;
        }));
    }
}

/// A `Consumer` a potentially infinite sequence, one item at a time.
///
/// The sequence consists of an arbitrary number of values of type `Self::Item`, followed by up to one value of type `Self::Final`. If you intend for the sequnce to be infinite, use the empty type `!` for `Self::Final`.
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
    /// Must not be called after any function of this trait returned an error, nor after `close` was called.
    fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error>;

    /// Attempt to consume the final item.
    ///
    /// After this function was called, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait returned an error, nor after `close` was called.
    fn close(&mut self, f: Self::Final) -> Result<(), Self::Error>;
}

/// A `Consumer` that can delay perform side-effects when consuming items. It must not delay performing side-effects when being closed. In other words, calling `close` should internally trigger flushing.
pub trait BufferedConsumer: Consumer {
    /// Perform any side-effects that got delayed for previously consumed items. This function allows the client code to force doing the (potentially expensive) side-effects. In exchange, the consumer gains the freedom to delay the side-effects of `consume` to improve efficiency.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait returned an error, nor after `close` was called.
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/// A `Producer` that is able to produce several items with a single function call, in order to improve on the efficiency of the `Producer` trait. Semantically, there must be no differrence between producing items in bulk or one item at a time.
///
/// Note that `Self::Item` must be `Copy` for efficiency reasons.
pub trait BulkConsumer: BufferedConsumer
where
    Self::Item: Copy,
{
    /// Expose a non-empty slice of memory, for the client code to fill with items hat should be consumed.
    ///
    /// The consumer should expose the largest contiguous slice it can expose efficiently. It should not perform copies to increase the size of the slice. Client code must be able to make progress even if this function always returns slices of size one.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait returned an error, nor after `close` was called.
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error>;

    /// Instruct the consumer to consume the first `amount` many items of the `consumer_slots` it has most recently exposed. The semantics must be equivalent to those of `consume` being called `amount` many times with exactly those items.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Callers must have written into (at least) the `amount` many first `consumer_slots` that were most recently exposed. Failure to uphold this invariant may cause undefined behavior.
    ///
    /// Must not be called after any function of this trait returned an error, nor after `close` was called.
    ///
    /// #### Safety
    ///
    /// The consumer may assume the `amount` first `consumer_slots` that were most recently exposed to contain initialized memory after this call, even if it exposed uninitialized memory. Violating the invariants can cause the consumer to read undefined memory, which triggers undefined behavior.
    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error>;

    /// Consume a non-zero number of items by reading them from a given buffer and returning how many items were consumed.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait returned an error, nor after `close` was called.
    ///
    /// #### Implementation Notes
    ///
    /// The default implementation orchestrates `consumer_slots` and `did_consume` in a straightforward manner. Only provide your own implementation if you can do better than that.
    fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        let slots = self.consumer_slots()?;
        let amount = min(slots.len(), buf.len());
        MaybeUninit::copy_from_slice(&mut slots[0..amount], &buf[0..amount]);
        unsafe {
            self.did_consume(amount)?;
        }
        return Ok(amount);
    }
}

/// Pipe as many items as possible from a producer into a consumer. Then call `close` on the consumer with the final value emitted by the producer.
pub fn pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: Producer,
    C: Consumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    loop {
        match producer.produce()? {
            Left(item) => consumer.consume(item)?,
            Right(f) => return Ok(consumer.close(f)?),
        }
    }
}

/// Efficiently pipe as many items as possible from a bulk producer into a bulk consumer. Then call `close` on the consumer with the final value emitted by the producer.
/// 
/// You should prefer this function over `bulk_consume_pipe` and `bulk_produce_pipe`; it selects whichever is more appropriate.
pub fn bulk_pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: BulkProducer,
    P::Item: Copy,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    return bulk_consume_pipe(producer, consumer);
}

/// Efficiently pipe as many items as possible from a bulk producer into a bulk consumer, by using `consumer.bulk_consume`. Then call `close` on the consumer with the final value emitted by the producer.
/// 
/// If you do not care whether to use `consumer.bulk_consume` or `producer.bulk_produce`, use `bulk_pipe` instead. This should almost always be the case.
pub fn bulk_consume_pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: BulkProducer,
    P::Item: Copy,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    loop {
        match producer.producer_slots()? {
            Left(slots) => {
                let amount = consumer.bulk_consume(slots)?;
                producer.did_produce(amount)?;
            }
            Right(f) => return Ok(consumer.close(f)?),
        }
    }
}

/// Efficiently pipe as many items as possible from a bulk producer into a bulk consumer, by using `producer.bulk_produce`. Then call `close` on the consumer with the final value emitted by the producer.
/// 
/// If you do not care whether to use `producer.bulk_produce` or `consumer.bulk_consume`, use `bulk_pipe` instead. This should almost always be the case.
pub unsafe fn bulk_produce_pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: BulkProducer,
    P::Item: Copy,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    loop {
        let slots = consumer.consumer_slots()?;
        match producer.bulk_produce(slots)? {
            Left(amount) => consumer.did_consume(amount)?,
            Right(f) => return Ok(consumer.close(f)?),
        }
    }
}
