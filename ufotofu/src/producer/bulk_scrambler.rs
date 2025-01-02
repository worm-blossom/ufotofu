use core::cmp::min;
use core::fmt::Debug;
use core::num::NonZeroUsize;
use std::vec::Vec;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

use crate::fixed_queue::Fixed;
use arbitrary::Arbitrary;
use either::Either::{self, *};

use crate::producer::Invariant;
use crate::{BufferedProducer, BulkProducer, Producer};

/// Operations which may be called against a [`BulkProducer`].
#[derive(Debug, PartialEq, Eq, Arbitrary, Clone)]
pub enum BulkProducerOperation {
    Produce,
    ConsiderProduced(NonZeroUsize),
    Slurp,
}

fn do_operations_make_progress(ops: &[BulkProducerOperation]) -> bool {
    for operation in ops {
        if *operation != BulkProducerOperation::Slurp {
            return true;
        }
    }

    false
}

/// A [`BulkProducer`] wrapper that scrambles the methods that get called on a wrapped producer. Will turn any "sensible" access pattern (say, calling `produce` repeatedly) into a more interesting pattern, as determined by the [`BulkProducerOperation`]s that are supplied in the constructor.
///
/// The scrambler emits the same items that it receives from the wrapped producer in the same order. It is allowed to access multiple items in bulk from the wrapped producer even if the code interacting with the scrambler has not yet requested that many items.
///
/// To be used in fuzz testing: when you want to test a [`BulkProducer`] you implemented, test a scrambled version of that producer instead, to exercise many different method call patterns even if the test itself only performs a simplistic method call pattern.
pub struct BulkScrambler_<C, T, F, E>(Invariant<BulkScrambler<C, T, F, E>>);

invarianted_impl_debug!(BulkScrambler_<C: Debug, T: Debug, F: Debug, E: Debug>);
invarianted_impl_as_ref!(BulkScrambler_<C, T, F, E>; C);
invarianted_impl_as_mut!(BulkScrambler_<C, T, F, E>; C);

impl<P, T: Default, F, E> BulkScrambler_<P, T, F, E> {
    /// Creates a new wrapper around `inner` that exercises the producer trait methods of `inner` by cycling through the given `operations`.
    ///
    /// If the `operations` do not make any progress (they only slurp), a single produce operation is appended.
    ///
    /// Chances are you want to use the [`Arbitrary`] trait to generate the `operations` rather than crafting them manually.
    pub fn new(inner: P, operations: Vec<BulkProducerOperation>) -> Self {
        Self::new_with_capacity(inner, operations, 2048)
    }

    /// Creates a new wrapper around `inner` that exercises the producer trait methods of `inner` by cycling through the given `operations`. To provide this functionality, the wrapper allocates an internal buffer of items, `capacity` sets the size of that buffer. Larger values allow for more distinct method call patterns, smaller values consume less memory (linearly so). [`BulkScrambler::new`](super::BulkScrambler::new) uses a capacity of `2048`.
    ///
    /// If the `operations` do not make any progress (they only slurp), a single produce operation is appended.
    ///
    /// Chances are you want to use the [`Arbitrary`] trait to generate the `operations` rather than crafting them manually.
    pub fn new_with_capacity(
        inner: P,
        mut operations: Vec<BulkProducerOperation>,
        capacity: usize,
    ) -> Self {
        if !do_operations_make_progress(&operations[..]) {
            operations.push(BulkProducerOperation::Produce);
        }

        BulkScrambler_(Invariant::new(BulkScrambler::<P, T, F, E> {
            inner,
            buffer: Fixed::new(capacity),
            last: None,
            operations: operations.into_boxed_slice(),
            operations_index: 0,
        }))
    }
}

impl<C, T, F, E> BulkScrambler_<C, T, F, E> {
    /// Consumes `self` and returns the wrapped producer.
    pub fn into_inner(self) -> C {
        self.0.into_inner().into_inner()
    }
}

impl<C, T: Clone, F, E> Producer for BulkScrambler_<C, T, F, E>
where
    C: BulkProducer<Item = T, Final = F, Error = E>,
{
    type Item = T;
    type Final = F;
    type Error = E;

    invarianted_producer_methods!();
}

impl<C, T: Clone, F, E> BufferedProducer for BulkScrambler_<C, T, F, E>
where
    C: BulkProducer<Item = T, Final = F, Error = E>,
{
    invarianted_buffered_producer_methods!();
}

impl<C, T: Clone, F, E> BulkProducer for BulkScrambler_<C, T, F, E>
where
    C: BulkProducer<Item = T, Final = F, Error = E>,
{
    invarianted_bulk_producer_methods!();
}

#[derive(Debug)]
struct BulkScrambler<P, T, F, E> {
    /// The producer that we wrap. All producer operations on the `BulkScrambler`
    /// will be transformed into semantically equivalent scrambled operations,
    /// and then forwarded to the `inner` producer.
    inner: P,
    /// A fixed capacity queue of items. We store items here that we request
    /// from the inner producer before the calling code wants to access them.
    buffer: Fixed<T>,
    /// The last value the inner producer has yielded.
    last: Option<Result<F, E>>,
    /// The instructions on how to scramble producer operations. We cycle
    /// through these round-robin.
    operations: Box<[BulkProducerOperation]>,
    /// The next operation to call on the `inner` producer once we need to fill
    /// our item queue.
    operations_index: usize,
}

impl<P, T, F, E> BulkScrambler<P, T, F, E> {
    /// Consumes `self` and returns the wrapped producer.
    fn into_inner(self) -> P {
        self.inner
    }

    fn advance_operations_index(&mut self) {
        self.operations_index = (self.operations_index + 1) % self.operations.len();
    }
}

impl<P, T, F, E> AsRef<P> for BulkScrambler<P, T, F, E> {
    fn as_ref(&self) -> &P {
        &self.inner
    }
}

impl<P, T, F, E> AsMut<P> for BulkScrambler<P, T, F, E> {
    fn as_mut(&mut self) -> &mut P {
        &mut self.inner
    }
}

// Operates by draining its `queue` and `last` and filling them via the
// `operations` when they become empty.
impl<P, T: Clone, F, E> Producer for BulkScrambler<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
{
    type Item = T;
    type Final = F;
    type Error = E;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        if let Some(last) = self.fill_queue().await {
            return last.map(|fin| Right(fin));
        }

        // Now that the queue has been filled, dequeue an item.
        let item = self
            .buffer
            .dequeue()
            .expect("queue should have been filled by performing operations");

        Ok(Either::Left(item))
    }
}

impl<P, T: Clone, F, E> BufferedProducer for BulkScrambler<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
{
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        // Fill queue as much as possible. The if-let is entered if the queue is empty afterward.
        if let Some(last) = self.fill_queue().await {
            match last {
                Ok(fin) => {
                    // Got a final value, which we buffer, to be emitted when the next item should be produced.
                    self.last = Some(Ok(fin));
                }
                Err(err) => {
                    // Got an error, which we can yield immediately (since we know the queue to be empty).
                    return Err(err);
                }
            }
        }

        // Slurp the inner producer. If we get an error and the queue is empty, return the error.
        if let Err(err) = self.inner.slurp().await {
            if self.buffer.len() == 0 {
                return Err(err);
            } else {
                self.last = Some(Err(err));
            }
        }

        // If we reach this, then everything worked and nothing of note happened.
        Ok(())
    }
}

impl<P, T: Clone, F, E> BulkProducer for BulkScrambler<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
{
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'a,
    {
        if let Some(last) = self.fill_queue().await {
            return last.map(|fin| Right(fin));
        }

        // Return readable slots.
        let slots = self
            .buffer
            .expose_items()
            .expect("queue should contain items after being filled by performing operations");

        Ok(Either::Left(slots))
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.buffer.consider_dequeued(amount);

        Ok(())
    }
}

impl<P, T: Clone, F, E> BulkScrambler<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
{
    async fn fill_queue(&mut self) -> Option<Result<F, E>> {
        // First we aim to fill the queue by performing operations.
        //
        // While the final value has not been set and the queue is not full, perform operations.
        // If the final value is returned from an operation, set the final value.
        while self.last.is_none() && self.buffer.len() < self.buffer.capacity() {
            match self.perform_operation().await {
                Ok(None) => {
                    // no-op
                }
                Ok(Some(fin)) => self.last = Some(Ok(fin)),
                Err(err) => self.last = Some(Err(err)),
            }
        }

        // Return the last value if the queue is empty and the value
        // was previously returned from an operation.
        if self.buffer.len() == 0 {
            self.last.take()
        } else {
            None
        }
    }

    async fn perform_operation(&mut self) -> Result<Option<F>, E> {
        debug_assert!(self.buffer.len() < self.buffer.capacity());

        match self.operations[self.operations_index] {
            BulkProducerOperation::Produce => match self.inner.produce().await? {
                Either::Left(item) => {
                    let yay = self.buffer.enqueue(item);
                    debug_assert!(yay.is_none());
                }
                Either::Right(fin) => return Ok(Some(fin)),
            },
            // Attempt to expose slots from the inner producer.
            BulkProducerOperation::ConsiderProduced(n) => {
                let n: usize = n.into();
                match self.inner.expose_items().await? {
                    Either::Left(slots) => {
                        // Set an upper bound on the slice of slots by comparing the number of available
                        // inner slots and the number provided by the `ProducerSlots` operation and taking
                        // the lowest value.
                        let slots_len = slots.len();
                        let available_slots = &slots[..min(slots_len, n)];

                        // Enqueue items into the inner producer.
                        let amount = self.buffer.bulk_enqueue(available_slots);

                        // Report the amount of items produced.
                        self.inner.consider_produced(amount).await?;
                    }
                    // If the final value was produced, return it.
                    Either::Right(final_val) => return Ok(Some(final_val)),
                }
            }
            BulkProducerOperation::Slurp => {
                // Slurp the inner producer.
                self.inner.slurp().await?;
            }
        }

        // Update the operations index.
        self.advance_operations_index();

        Ok(None)
    }
}
