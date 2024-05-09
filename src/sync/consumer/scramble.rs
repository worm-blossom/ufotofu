use core::cmp::min;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

use arbitrary::{Arbitrary, Error, Unstructured};
use ufotofu_queues::fixed::{Fixed, FixedQueueError};
use ufotofu_queues::Queue;
use wrapper::Wrapper;

use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

#[derive(Debug)]
pub struct ScrambleError(String);

impl From<FixedQueueError> for ScrambleError {
    fn from(err: FixedQueueError) -> ScrambleError {
        ScrambleError(err.to_string())
    }
}

impl From<!> for ScrambleError {
    fn from(never: !) -> ScrambleError {
        match never {}
    }
}

#[derive(Debug, PartialEq, Eq, Arbitrary, Clone)]
pub enum ConsumeOperation {
    Consume,
    ConsumerSlots(usize),
    Flush,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ConsumeOperations(Box<[ConsumeOperation]>);

impl ConsumeOperations {
    /// Checks that the operations contain at least one non-flush operation, and that at most 256 different operations are being allocated.
    pub fn new(operations: Box<[ConsumeOperation]>) -> Option<Self> {
        let mut found_non_flush = false;
        for op in operations.iter() {
            if *op != ConsumeOperation::Flush {
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
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, Error> {
        match Self::new(Arbitrary::arbitrary(u)?) {
            Some(ops) => Ok(ops),
            None => Err(Error::IncorrectFormat),
        }
    }

    #[inline]
    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <Box<[ConsumeOperation]> as Arbitrary<'a>>::size_hint(depth)
    }
}

#[derive(Debug)]
pub struct Scramble<C, T, F, E> {
    inner: C,
    queue: Fixed<T>,
    operations: Box<[ConsumeOperation]>,
    operations_index: usize,
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
        // Add the item to the queue.
        self.queue.enqueue(item)?;
        // While there are still items in the queue, perform an operation.
        while self.queue.amount() > 0 {
            self.perform_operation()?;
        }

        // TODO: Why do we enqueue a second time?
        self.queue.enqueue(item)?;

        Ok(())
    }

    fn close(&mut self, _final: Self::Final) -> Result<(), Self::Error> {
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
        // While there are still items in the queue, perform an operation.
        while self.queue.amount() > 0 {
            self.perform_operation()?;
        }

        // Flush the inner consumer.
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
        let current = self.queue.amount();
        let capacity = self.queue.capacity();

        // If the queue has available capacity, return writeable slots.
        if current < capacity {
            let slots = self.queue.enqueue_slots()?;
            Ok(slots)
        } else {
            // TODO: I don't think this is correct / intended behaviour.
            //
            // If the queue is at capacity, perform operations until empty.
            while self.queue.amount() > 0 {
                self.perform_operation().unwrap();
            }

            self.queue
                .enqueue_slots()
                .map_err(|err| ScrambleError(err.to_string()))
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
                // Update the operations index.
                self.operations_index = (self.operations_index + 1) % self.operations.len();
                // Feed the item to the inner consumer.
                self.inner.consume(item).unwrap();
            }
            ConsumeOperation::ConsumerSlots(n) => {
                // Remove items from the queue in bulk and place them in the inner consumer slots.
                if let Ok(slots) = self.inner.consumer_slots() {
                    let slots_len = slots.len();
                    let slots = &mut slots[..min(slots_len, n)];
                    let amount = self.queue.bulk_dequeue(slots).unwrap();
                    unsafe {
                        self.inner.did_consume(amount).unwrap();
                    }
                    self.operations_index = (self.operations_index + 1) % self.operations.len();
                }
            }
            ConsumeOperation::Flush => {
                if self.inner.flush().is_err() {
                    self.operations_index = (self.operations_index + 1) % self.operations.len();
                }
            }
        }

        Ok(())
    }
}
