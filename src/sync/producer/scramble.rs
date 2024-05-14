use core::cmp::min;
use core::fmt::Debug;
use core::marker::PhantomData;

use arbitrary::{Arbitrary, Error as ArbitraryError, Unstructured};
use either::Either;
use thiserror::Error;
use ufotofu_queues::fixed::{Fixed, FixedQueueError};
use ufotofu_queues::Queue;
use wrapper::Wrapper;

use crate::sync::consumer::CursorFullError;
use crate::sync::{BufferedProducer, BulkProducer, Producer};

#[derive(Debug, Error)]
pub enum ScrambleError {
    #[error("an infallible action failed")]
    Never,
    #[error(transparent)]
    Queue(#[from] FixedQueueError),
    #[error(transparent)]
    Cursor(#[from] CursorFullError),
}

impl From<!> for ScrambleError {
    fn from(_never: !) -> ScrambleError {
        Self::Never
    }
}

/// Operations which may be called against a producer.
#[derive(Debug, PartialEq, Eq, Arbitrary, Clone)]
pub enum ProduceOperation {
    Produce,
    ProducerSlots(usize),
    Slurp,
}

/// A sequence of heap-allocated produce operations.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProduceOperations(Box<[ProduceOperation]>);

impl ProduceOperations {
    /// Checks that the operations contain at least one non-slurp operation
    /// and that no more than 256 different operations are being allocated.
    pub fn new(operations: Box<[ProduceOperation]>) -> Option<Self> {
        let mut found_non_slurp = false;
        for op in operations.iter() {
            if *op != ProduceOperation::Slurp {
                found_non_slurp = true;
                break;
            }
        }

        if found_non_slurp && operations.len() <= 256 {
            Some(ProduceOperations(operations))
        } else {
            None
        }
    }
}

impl<'a> Arbitrary<'a> for ProduceOperations {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, ArbitraryError> {
        match Self::new(Arbitrary::arbitrary(u)?) {
            Some(ops) => Ok(ops),
            None => Err(ArbitraryError::IncorrectFormat),
        }
    }

    #[inline]
    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <Box<[ProduceOperation]> as Arbitrary<'a>>::size_hint(depth)
    }
}

/// A `Producer` wrapper that maintains a queue of items, as well as a sequence
/// of operations and an operation index.
#[derive(Debug)]
pub struct Scramble<P, T, F, E> {
    /// An implementer of the `Producer` traits.
    inner: P,
    /// A fixed capacity queue of items.
    queue: Fixed<T>,
    /// A final value which may or may not have been returned from the inner producer.
    final_val: Option<F>,
    /// A sequence of produce operations.
    operations: Box<[ProduceOperation]>,
    /// An index into the sequence of produce operations.
    operations_index: usize,
    /// Zero-sized type for final item and error generics.
    e: PhantomData<E>,
}

impl<P, T, F, E> Scramble<P, T, F, E> {
    pub fn new(inner: P, operations: ProduceOperations, capacity: usize) -> Self {
        Scramble::<P, T, F, E> {
            inner,
            queue: Fixed::new(capacity),
            final_val: None,
            operations: operations.0,
            operations_index: 0,
            e: PhantomData,
        }
    }

    fn advance_operations_index(&mut self) {
        self.operations_index = (self.operations_index + 1) % self.operations.len();
    }
}

impl<P, T, F, E> AsRef<P> for Scramble<P, T, F, E> {
    fn as_ref(&self) -> &P {
        &self.inner
    }
}

impl<P, T, F, E> AsMut<P> for Scramble<P, T, F, E> {
    fn as_mut(&mut self) -> &mut P {
        &mut self.inner
    }
}

impl<P, T, F, E> Wrapper<P> for Scramble<P, T, F, E> {
    fn into_inner(self) -> P {
        self.inner
    }
}

// Operates by draining its `queue` and `final_val` and filling them via the
// `operations` when they become empty.
impl<P, T, F, E> Producer for Scramble<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
    E: Debug,
{
    type Item = T;
    type Final = F;
    type Error = ScrambleError;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        // If the queue is empty and the final value has been set,
        // return the final value.
        if self.queue.amount() == 0 && self.final_val.is_some() {
            let final_val = self.final_val.take().unwrap();
            return Ok(Either::Right(final_val));
        }

        // The attempt to dequeue an item will fail if the queue is empty.
        // In that case, perform operations to fill the queue - returning
        // the final value if it is produced.
        while self.queue.amount() == 0 {
            if let Some(final_val) = self.perform_operation()? {
                return Ok(Either::Right(final_val));
            }
        }

        // Now that the queue has been filled, dequeue an item.
        Ok(Either::Left(self.queue.dequeue()?))
    }
}

impl<P, T, F, E> BufferedProducer for Scramble<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
    E: Debug,
{
    fn slurp(&mut self) -> Result<(), Self::Error> {
        // TODO: I'm not convinced this behaviour is correct.
        if self.queue.amount() == 0 && self.final_val.is_some() {
            let _ = self.final_val.take().unwrap();
        }

        // Perform operations until the queue is full.
        while self.queue.amount() < self.queue.capacity() {
            if let Some(final_val) = self.perform_operation()? {
                // Set the final value if returned from an operation.
                self.final_val = Some(final_val);
            }
        }

        // Then slurp the inner producer.
        let _ = self.inner.slurp();

        Ok(())
    }
}

impl<P, T, F, E> BulkProducer for Scramble<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
    E: Debug,
{
    fn producer_slots(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        // Return the final value if the queue is empty and the value
        // was previously returned from an operation.
        if self.queue.amount() == 0 && self.final_val.is_some() {
            return Ok(Either::Right(self.final_val.take().unwrap()));
        }

        // Perform operations while the queue is empty.
        while self.queue.amount() == 0 {
            if let Some(final_val) = self.perform_operation()? {
                return Ok(Either::Right(final_val));
            }
        }

        // Return readable slots.
        let slots = self.queue.dequeue_slots()?;

        Ok(Either::Left(slots))
    }

    fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.queue.did_dequeue(amount)?;

        Ok(())
    }
}

impl<P, T, F, E> Scramble<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
    E: Debug,
{
    fn perform_operation(&mut self) -> Result<Option<F>, ScrambleError> {
        debug_assert!(self.queue.amount() < self.queue.capacity());

        match self.operations[self.operations_index] {
            // Attempt to produce an item from the inner producer.
            ProduceOperation::Produce => match self.inner.produce().unwrap() {
                // If an item was produced, add it to the queue.
                Either::Left(item) => self.queue.enqueue(item)?,
                // If the final value was produced, return it.
                Either::Right(final_val) => return Ok(Some(final_val)),
            },
            // Attempt to expose slots from the inner producer.
            ProduceOperation::ProducerSlots(n) => match self.inner.producer_slots().unwrap() {
                Either::Left(slots) => {
                    // Set an upper bound on the slice of slots by comparing the number of available
                    // inner slots and the number provided by the `ProducerSlots` operation and taking
                    // the lowest value.
                    let slots_len = slots.len();
                    let available_slots = &slots[..min(slots_len, n)];

                    // Enqueue items into the inner producer and report the amount produced.
                    let amount = self.queue.bulk_enqueue(available_slots)?;
                    self.inner.did_produce(amount).unwrap();
                }
                // If the final value was produced, return it.
                Either::Right(final_val) => return Ok(Some(final_val)),
            },
            ProduceOperation::Slurp => {
                // Slurp the inner producer.
                self.inner.slurp().unwrap();
            }
        }

        // Update the operations index.
        self.advance_operations_index();

        Ok(None)
    }
}
