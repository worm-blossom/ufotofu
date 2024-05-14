use core::cmp::min;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

use arbitrary::{Arbitrary, Error as ArbitraryError, Unstructured};
use thiserror::Error;
use ufotofu_queues::fixed::{Fixed, FixedQueueError};
use ufotofu_queues::Queue;
use wrapper::Wrapper;

use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

#[derive(Debug, Error)]
pub enum ScrambleError {
    #[error("an infallible action failed")]
    Never,
    #[error(transparent)]
    Queue(#[from] FixedQueueError),
}

impl From<!> for ScrambleError {
    fn from(_never: !) -> ScrambleError {
        Self::Never
    }
}

/// Operations which may be called against a consumer.
#[derive(Debug, PartialEq, Eq, Arbitrary, Clone)]
pub enum ConsumeOperation {
    Consume,
    ConsumerSlots(usize),
    Flush,
}

/// A sequence of heap-allocated consume operations.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ConsumeOperations(Box<[ConsumeOperation]>);

impl ConsumeOperations {
    /// Checks that the operations contain at least one non-flush operation
    /// and that no more than 256 different operations are being allocated.
    pub fn new(operations: Box<[ConsumeOperation]>) -> Option<Self> {
        let mut found_non_flush = false;
        for operation in operations.iter() {
            if *operation != ConsumeOperation::Flush {
                found_non_flush = true;
                break;
            }
        }

        if found_non_flush && operations.len() <= 256 {
            Some(ConsumeOperations(operations))
        } else {
            None
        }
    }
}

impl<'a> Arbitrary<'a> for ConsumeOperations {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, ArbitraryError> {
        match Self::new(Arbitrary::arbitrary(u)?) {
            Some(ops) => Ok(ops),
            None => Err(ArbitraryError::IncorrectFormat),
        }
    }

    #[inline]
    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <Box<[ConsumeOperation]> as Arbitrary<'a>>::size_hint(depth)
    }
}

/// A `Consumer` wrapper that maintains a queue of items, as well as a sequence
/// of operations and an operation index.
#[derive(Debug)]
pub struct Scramble<C, T, F, E> {
    /// An implementer of the `Consumer` traits.
    inner: C,
    /// A fixed capacity queue of items.
    queue: Fixed<T>,
    /// A sequence of consume operations.
    operations: Box<[ConsumeOperation]>,
    /// An index into the sequence of consume operations.
    operations_index: usize,
    /// Zero-sized types for final item and error generics.
    fe: PhantomData<(F, E)>,
}

impl<C, T, F, E> Scramble<C, T, F, E> {
    pub fn new(inner: C, operations: ConsumeOperations, capacity: usize) -> Self {
        Scramble::<C, T, F, E> {
            inner,
            queue: Fixed::new(capacity),
            operations: operations.0,
            operations_index: 0,
            fe: PhantomData,
        }
    }

    fn advance_operations_index(&mut self) {
        self.operations_index = (self.operations_index + 1) % self.operations.len();
    }
}

impl<C, T, F, E> AsRef<C> for Scramble<C, T, F, E> {
    fn as_ref(&self) -> &C {
        &self.inner
    }
}

impl<C, T, F, E> AsMut<C> for Scramble<C, T, F, E> {
    fn as_mut(&mut self) -> &mut C {
        &mut self.inner
    }
}

impl<C, T, F, E> Wrapper<C> for Scramble<C, T, F, E> {
    fn into_inner(self) -> C {
        self.inner
    }
}

impl<C, T, F, E> Consumer for Scramble<C, T, F, E>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
    E: Debug,
{
    type Item = T;
    type Final = F;
    type Error = ScrambleError;

    fn consume(&mut self, item: T) -> Result<(), Self::Error> {
        // Attempt to add an item to the queue.
        //
        // The attempt will fail if the queue is full. In that case, perform operations
        // until the queue is empty.
        if self.queue.enqueue(item).is_err() {
            while self.queue.amount() > 0 {
                self.perform_operation()?;
            }

            // Now that the queue has been emptied, enqueue the item.
            self.queue.enqueue(item)?;
        }

        Ok(())
    }

    fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        // Perform operations until the queue is empty.
        while self.queue.amount() > 0 {
            self.perform_operation()?;
        }

        // Then close the inner consumer using the final value.
        self.inner.close(final_val).unwrap();

        Ok(())
    }
}

impl<C, T, F, E> BufferedConsumer for Scramble<C, T, F, E>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
    E: Debug,
{
    fn flush(&mut self) -> Result<(), Self::Error> {
        // Perform operations until the queue is empty.
        while self.queue.amount() > 0 {
            self.perform_operation()?;
        }

        // Then flush the inner consumer.
        let _ = self.inner.flush();

        Ok(())
    }
}

impl<C, T, F, E> BulkConsumer for Scramble<C, T, F, E>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
    E: Debug,
{
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        let amount = self.queue.amount();
        let capacity = self.queue.capacity();

        // If the queue has available capacity, return writeable slots.
        if amount < capacity {
            let slots = self.queue.enqueue_slots()?;
            Ok(slots)
        } else {
            // Perform operations until the queue is empty.
            while self.queue.amount() > 0 {
                self.perform_operation()?;
            }

            // Return writeable slots.
            let slots = self.queue.enqueue_slots()?;

            Ok(slots)
        }
    }

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.queue.did_enqueue(amount)?;

        Ok(())
    }
}

impl<C, T, F, E> Scramble<C, T, F, E>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
    E: Debug,
{
    fn perform_operation(&mut self) -> Result<(), ScrambleError> {
        debug_assert!(self.queue.amount() > 0);

        match self.operations[self.operations_index] {
            ConsumeOperation::Consume => {
                // Remove an item from the queue.
                let item = self.queue.dequeue()?;
                // Feed the item to the inner consumer.
                self.inner.consume(item).unwrap();
            }
            ConsumeOperation::ConsumerSlots(n) => {
                // Remove items from the queue in bulk and place them in the inner consumer slots.
                //
                // Request writeable slots from the inner consumer.
                let slots = self.inner.consumer_slots().unwrap();

                // Set an upper bound on the slice of slots by comparing the number of available
                // inner slots and the number provided by the `ConsumerSlots` operation and taking
                // the lowest value.
                let slots_len = slots.len();
                let available_slots = &mut slots[..min(slots_len, n)];

                // Dequeue items into the inner consumer and report the amount consumed.
                let amount = self.queue.bulk_dequeue(available_slots)?;
                unsafe {
                    self.inner.did_consume(amount).unwrap();
                }
            }
            ConsumeOperation::Flush => {
                // Flush the inner consumer.
                self.inner.flush().unwrap();
            }
        }

        // Update the operations index.
        self.advance_operations_index();

        Ok(())
    }
}
