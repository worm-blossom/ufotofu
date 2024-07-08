use core::cmp::min;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

use arbitrary::{Arbitrary, Error as ArbitraryError, Unstructured};
use ufotofu_queues::{Fixed, Queue};
use wrapper::Wrapper;

use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

/// Operations which may be called against a consumer.
#[derive(Debug, PartialEq, Eq, Arbitrary, Clone)]
pub enum ConsumeOperation {
    Consume,
    ConsumerSlots(usize),
    Flush,
}

/// A sequence of heap-allocated operations for determining the method call patterns that a `Scramble` wrapper subjects its inner consumer to. Intended to be [generated by a fuzzer](https://rust-fuzz.github.io/book/cargo-fuzz/structure-aware-fuzzing.html).
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

/// A `Consumer` wrapper that scrambles the methods that get called on a wrapped consumer, without changing the observable semantics. Unless it uncovers buggy behavior on the wrapped consumer, that is.
pub struct Scramble<C, T, F, E> {
    /// The `Consumer` that we wrap. All consumer operations on the `Scramble`
    /// will be transformed into semantically equivalent scrambled operations,
    /// and then forwarded to the `inner` consumer.
    inner: C,
    /// A fixed capacity queue of items. We store items here before forwarding
    /// them to the `inner` consumer. Intermediate storage is necessary so that
    /// we can arbitrarily scramble operations before forwarding.
    buffer: Fixed<T>,
    /// The instructions on how to scramble consumer operations. We cycle
    /// through these round-robin.
    operations: Box<[ConsumeOperation]>,
    /// The next operation to call on the `inner` consumer once we need to empty
    /// our item queue.
    operations_index: usize,
    /// Satisfy the type checker, no useful semantics.
    phantom: PhantomData<(F, E)>,
}

impl<C: core::fmt::Debug, T: core::fmt::Debug, F, E> core::fmt::Debug for Scramble<C, T, F, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not print `self.phantom`.
        f.debug_struct("Scramble")
            .field("inner", &self.inner)
            .field("buffer", &self.buffer)
            .field("operations", &self.operations)
            .field("operations_index", &self.operations_index)
            .finish()
    }
}

impl<C, T, F, E> Scramble<C, T, F, E> {
    /// Create a new wrapper around `inner` that exercises the consumer trait methods of `inner` by cycling through the given `operations`. To provide this functionality, the wrapper must allocate an internal buffer of items, `capacity` sets the size of that buffer. Larger values allow for more bizarre method call patterns, smaller values consume less space (surprise!).
    pub fn new(inner: C, operations: ConsumeOperations, capacity: usize) -> Self {
        Scramble::<C, T, F, E> {
            inner,
            buffer: Fixed::new(capacity),
            operations: operations.0,
            operations_index: 0,
            phantom: PhantomData,
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
{
    type Item = T;
    type Final = F;
    type Error = E;

    fn consume(&mut self, item: T) -> Result<(), Self::Error> {
        // Attempt to add an item to the queue.
        //
        // The item will be returned if the queue is full.
        // In that case, perform operations until the queue is empty.
        if self.buffer.enqueue(item).is_some() {
            while self.buffer.len() > 0 {
                self.perform_operation()?;
            }

            // Now that the queue has been emptied, enqueue the item.
            //
            // Return value should always be `None` in this context so we
            // ignore it.
            let _ = self.buffer.enqueue(item);
        }

        Ok(())
    }

    fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        // Perform operations until the queue is empty.
        while self.buffer.len() > 0 {
            self.perform_operation()?;
        }

        // Then close the inner consumer using the final value.
        self.inner.close(final_val)?;

        Ok(())
    }
}

impl<C, T, F, E> BufferedConsumer for Scramble<C, T, F, E>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn flush(&mut self) -> Result<(), Self::Error> {
        // Perform operations until the queue is empty.
        while self.buffer.len() > 0 {
            self.perform_operation()?;
        }

        // Then flush the inner consumer.
        self.inner.flush()?;

        Ok(())
    }
}

impl<C, T, F, E> BulkConsumer for Scramble<C, T, F, E>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn expose_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        let amount = self.buffer.len();
        let capacity = self.buffer.capacity();

        // If the queue has available capacity, return writeable slots.
        if amount < capacity {
            let slots = self
                .buffer
                .expose_slots()
                .expect("queue should have available capacity");
            Ok(slots)
        } else {
            // Perform operations until the queue is empty.
            while self.buffer.len() > 0 {
                self.perform_operation()?;
            }

            // Return writeable slots.
            let slots = self.buffer.expose_slots().expect(
                "queue should have available capacity after being emptied by performing operations",
            );

            Ok(slots)
        }
    }

    unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.buffer.consider_enqueued(amount);

        Ok(())
    }
}

impl<C, T, F, E> Scramble<C, T, F, E>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn perform_operation(&mut self) -> Result<(), E> {
        debug_assert!(self.buffer.len() > 0);

        match self.operations[self.operations_index] {
            ConsumeOperation::Consume => {
                // Remove an item from the queue.
                let item = self
                    .buffer
                    .dequeue()
                    .expect("queue should contain an item for consumption");

                // Feed the item to the inner consumer.
                self.inner.consume(item)?;
            }
            ConsumeOperation::ConsumerSlots(n) => {
                // Remove items from the queue in bulk and place them in the inner consumer slots.
                //
                // Request writeable slots from the inner consumer.
                let slots = self.inner.expose_slots()?;

                // Set an upper bound on the slice of slots by comparing the number of available
                // inner slots and the number provided by the `ConsumerSlots` operation and taking
                // the lowest value.
                let slots_len = slots.len();
                let available_slots = &mut slots[..min(slots_len, n)];

                // Dequeue items into the inner consumer.
                let amount = self.buffer.bulk_dequeue_uninit(available_slots);

                // Report the amount of items consumed.
                unsafe {
                    self.inner.consume_slots(amount)?;
                }
            }
            ConsumeOperation::Flush => {
                // Flush the inner consumer.
                self.inner.flush()?;
            }
        }

        // Update the operations index.
        self.advance_operations_index();

        Ok(())
    }
}
