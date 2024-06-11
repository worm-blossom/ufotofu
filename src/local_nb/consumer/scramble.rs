use core::cmp::min;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

use arbitrary::{Arbitrary, Error as ArbitraryError, Unstructured};
use ufotofu_queues::fixed::Fixed;
use ufotofu_queues::Queue;
use wrapper::Wrapper;

use crate::local_nb::{LocalBufferedConsumer, LocalBulkConsumer, LocalConsumer};

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
    /// The `Consumer` that we wrap. All consumer operations on the `Scramble`
    /// will be transformed into semantically equivalent scrambled operations,
    /// and then forwarded to the `inner` consumer.
    inner: C,
    /// A fixed capacity queue of items. We store items here before forwarding
    /// them to the `inner` consumer. Intermediate storage is necessary so that
    /// we can arbitrarily scramble operations before forwarding.
    queue: Fixed<T>,
    /// The instructions on how to scramble consumer operations. We cycle
    /// through these round-robin.
    operations: Box<[ConsumeOperation]>,
    /// The next operation to call on the `inner` consumer once we need to empty
    /// our item queue.
    operations_index: usize,
    /// Satisfy the type checker, no useful semantics.
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

impl<C, T, F, E> LocalConsumer for Scramble<C, T, F, E>
where
    C: LocalBulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    type Item = T;
    type Final = F;
    type Error = E;

    async fn consume(&mut self, item: T) -> Result<(), Self::Error> {
        // Attempt to add an item to the queue.
        //
        // The item will be returned if the queue is full.
        // In that case, perform operations until the queue is empty.
        if self.queue.enqueue(item).is_some() {
            while self.queue.amount() > 0 {
                self.perform_operation().await?;
            }

            // Now that the queue has been emptied, enqueue the item.
            //
            // Return value should always be `None` in this context so we
            // ignore it.
            let _ = self.queue.enqueue(item);
        }

        Ok(())
    }

    async fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        // Perform operations until the queue is empty.
        while self.queue.amount() > 0 {
            self.perform_operation().await?;
        }

        // Then close the inner consumer using the final value.
        self.inner.close(final_val).await?;

        Ok(())
    }
}

impl<C, T, F, E> LocalBufferedConsumer for Scramble<C, T, F, E>
where
    C: LocalBulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    async fn flush(&mut self) -> Result<(), Self::Error> {
        // Perform operations until the queue is empty.
        while self.queue.amount() > 0 {
            self.perform_operation().await?;
        }

        // Then flush the inner consumer.
        self.inner.flush().await?;

        Ok(())
    }
}

impl<C, T, F, E> LocalBulkConsumer for Scramble<C, T, F, E>
where
    C: LocalBulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    async fn consumer_slots<'a>(
        &'a mut self,
    ) -> Result<&'a mut [MaybeUninit<Self::Item>], Self::Error>
    where
        T: 'a,
    {
        let amount = self.queue.amount();
        let capacity = self.queue.capacity();

        // If the queue has available capacity, return writeable slots.
        if amount < capacity {
            let slots = self
                .queue
                .enqueue_slots()
                .expect("queue should have available capacity");
            Ok(slots)
        } else {
            // Perform operations until the queue is empty.
            while self.queue.amount() > 0 {
                self.perform_operation().await?;
            }

            // Return writeable slots.
            let slots = self.queue.enqueue_slots().expect(
                "queue should have available capacity after being emptied by performing operations",
            );

            Ok(slots)
        }
    }

    async unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.queue.did_enqueue(amount);

        Ok(())
    }
}

impl<C, T, F, E> Scramble<C, T, F, E>
where
    C: LocalBulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    async fn perform_operation(&mut self) -> Result<(), E> {
        debug_assert!(self.queue.amount() > 0);

        match self.operations[self.operations_index] {
            ConsumeOperation::Consume => {
                // Remove an item from the queue.
                let item = self
                    .queue
                    .dequeue()
                    .expect("queue should contain an item for consumption");

                // Feed the item to the inner consumer.
                self.inner.consume(item).await?;
            }
            ConsumeOperation::ConsumerSlots(n) => {
                // Remove items from the queue in bulk and place them in the inner consumer slots.
                //
                // Request writeable slots from the inner consumer.
                let slots = self.inner.consumer_slots().await?;

                // Set an upper bound on the slice of slots by comparing the number of available
                // inner slots and the number provided by the `ConsumerSlots` operation and taking
                // the lowest value.
                let slots_len = slots.len();
                let available_slots = &mut slots[..min(slots_len, n)];

                // Dequeue items into the inner consumer.
                let amount = self.queue.bulk_dequeue(available_slots);

                // Report the amount of items consumed.
                unsafe {
                    self.inner.did_consume(amount).await?;
                }
            }
            ConsumeOperation::Flush => {
                // Flush the inner consumer.
                self.inner.flush().await?;
            }
        }

        // Update the operations index.
        self.advance_operations_index();

        Ok(())
    }
}

// To the interested reader: this implementation uses `n` only as an upper bound on how many slots
// it fills at once. A more thorough scrambler should fill *exactly* `n` slots. But this requires
// some additional state keeping (waiting until item queue has enough items) which we have not
// implemented yet. We might, though, eventually.
