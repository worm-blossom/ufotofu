use core::cmp::min;
use core::fmt::Debug;
use core::num::NonZeroUsize;
use std::vec::Vec;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

use arbitrary::Arbitrary;
use ufotofu_queues::{Fixed, Queue};

use crate::consumer::Invariant;
use crate::{BufferedConsumer, BulkConsumer, Consumer};

/// An operation which may be called against a [`BulkConsumer`].
#[derive(Debug, PartialEq, Eq, Arbitrary, Clone)]
pub enum BulkConsumerOperation {
    Consume,
    ConsumeSlots(NonZeroUsize),
    Flush,
}

fn do_operations_make_progress(ops: &[BulkConsumerOperation]) -> bool {
    for operation in ops {
        if *operation != BulkConsumerOperation::Flush {
            return true;
        }
    }

    false
}

/// A [`BulkConsumer`] wrapper that scrambles the methods that get called on a wrapped consumer. Will turn any "sensible" access pattern (say, calling `consume` repeatedly) into a more interesting pattern, as determined by the [`BulkConsumerOperation`]s that are supplied in the constructor.
///
/// The scrambler feeds the same items that it receives into the wrapped consumer in the same order. It is allowed to buffer them before doing so, allowing for bulk transfer to the wrapped consumer even if the scrambler is only being fed with [`consume`](Consumer::consume) calls. As a consequence, the scrambler might accept items beyond the point where the wrapped consumer would already emit an error. Once those items are flushed into the wrapped consumer, the scrambler forwards the error immediately. The maximum delay in consumed items before an error is emitted is given by the `capacity` of [`BulkScrambler::new_with_capacity`](super::BulkScrambler::new_with_capacity) (which defaults to `2048` for [`BulkScrambler::new`](super::BulkScrambler::new)).
///
/// To be used in fuzz testing: when you want to test a [`BulkConsumer`] you implemented, test a scrambled version of that consumer instead, to exercise many different method call patterns even if the test itself only performs a simplistic method call pattern.
pub struct BulkScrambler_<C, T>(Invariant<BulkScrambler<C, T>>);

invarianted_impl_debug!(BulkScrambler_<C: Debug, T: Debug>);
invarianted_impl_as_ref!(BulkScrambler_<C, T>; C);
invarianted_impl_as_mut!(BulkScrambler_<C, T>; C);

impl<C, T: Default> BulkScrambler_<C, T> {
    /// Creates a new wrapper around `inner` that exercises the consumer trait methods of `inner` by cycling through the given `operations`.
    ///
    /// If the `operations` do not make any progress (they only flush), a single consume operation is appended.
    ///
    /// Chances are you want to use the [`Arbitrary`] trait to generate the `operations` rather than crafting them manually.
    pub fn new(inner: C, operations: Vec<BulkConsumerOperation>) -> Self {
        Self::new_with_capacity(inner, operations, 2048)
    }

    /// Creates a new wrapper around `inner` that exercises the consumer trait methods of `inner` by cycling through the given `operations`. To provide this functionality, the wrapper allocates an internal buffer of items, `capacity` sets the size of that buffer. Larger values allow for more distinct method call patterns, smaller values consume less memory (linearly so). [`BulkScrambler::new`](super::BulkScrambler::new) uses a capacity of `2048`.
    ///
    /// If the `operations` do not make any progress (they only flush), a single consume operation is appended.
    ///
    /// Chances are you want to use the [`Arbitrary`] trait to generate the `operations` rather than crafting them manually.
    pub fn new_with_capacity(
        inner: C,
        mut operations: Vec<BulkConsumerOperation>,
        capacity: usize,
    ) -> Self {
        if !do_operations_make_progress(&operations[..]) {
            operations.push(BulkConsumerOperation::Consume);
        }

        BulkScrambler_(Invariant::new(BulkScrambler {
            inner,
            buffer: Fixed::new(capacity),
            operations: operations.into_boxed_slice(),
            operations_index: 0,
        }))
    }
}

impl<C, T> BulkScrambler_<C, T> {
    /// Consumes `self` and returns the wrapped consumer.
    pub fn into_inner(self) -> C {
        self.0.into_inner().into_inner()
    }
}

impl<C, T: Clone, F, E> Consumer for BulkScrambler_<C, T>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
{
    type Item = T;
    type Final = F;
    type Error = E;

    invarianted_consumer_methods!();
}

impl<C, T: Clone, F, E> BufferedConsumer for BulkScrambler_<C, T>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
{
    invarianted_buffered_consumer_methods!();
}

impl<C, T: Clone, F, E> BulkConsumer for BulkScrambler_<C, T>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
{
    invarianted_bulk_consumer_methods!();
}

#[derive(Debug)]
struct BulkScrambler<C, T> {
    /// The consumer that we wrap. All consumer operations on the `BulkScrambler`
    /// will be transformed into semantically equivalent scrambled operations,
    /// and then forwarded to the `inner` consumer.
    inner: C,
    /// A fixed capacity queue of items. We store items here before forwarding
    /// them to the `inner` consumer. Intermediate storage is necessary so that
    /// we can arbitrarily scramble operations before forwarding.
    buffer: Fixed<T>,
    /// The instructions on how to scramble consumer operations. We cycle
    /// through these round-robin.
    operations: Box<[BulkConsumerOperation]>,
    /// The next operation to call on the `inner` consumer once we need to empty
    /// our item queue.
    operations_index: usize,
}

impl<C, T> BulkScrambler<C, T> {
    /// Consumes `self` and returns the wrapped consumer.
    fn into_inner(self) -> C {
        self.inner
    }

    fn advance_operations_index(&mut self) {
        self.operations_index = (self.operations_index + 1) % self.operations.len();
    }
}

impl<C, T> AsRef<C> for BulkScrambler<C, T> {
    fn as_ref(&self) -> &C {
        &self.inner
    }
}

impl<C, T> AsMut<C> for BulkScrambler<C, T> {
    fn as_mut(&mut self) -> &mut C {
        &mut self.inner
    }
}

impl<C, T: Clone, F, E> Consumer for BulkScrambler<C, T>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
{
    type Item = T;
    type Final = F;
    type Error = E;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        // Try buffering `item`, making space if necessary.
        if let Some(item) = self.buffer.enqueue(item) {
            while self.buffer.len() > 0 {
                self.perform_operation().await?;
            }

            let yay = self.buffer.enqueue(item);
            debug_assert!(yay.is_some()); // We just emptied the queue, so enqueing must succeed.
        }

        Ok(())
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        // Perform operations until the queue is empty.
        while self.buffer.len() > 0 {
            self.perform_operation().await?;
        }

        // Then close the inner consumer using the final value.
        self.inner.close(fin).await
    }
}

impl<C, T: Clone, F, E> BufferedConsumer for BulkScrambler<C, T>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
{
    async fn flush(&mut self) -> Result<(), Self::Error> {
        // Perform operations until the queue is empty.
        while self.buffer.len() > 0 {
            self.perform_operation().await?;
        }

        // Then flush the inner consumer.
        self.inner.flush().await
    }
}

impl<C, T: Clone, F, E> BulkConsumer for BulkScrambler<C, T>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
{
    async fn expose_slots<'a>(&'a mut self) -> Result<&'a mut [Self::Item], Self::Error>
    where
        Self::Item: 'a,
    {
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
                self.perform_operation().await?;
            }

            // Return writeable slots.
            let slots = self.buffer.expose_slots().expect(
                "queue should have available capacity after being emptied by performing operations",
            );

            Ok(slots)
        }
    }

    async fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.buffer.consider_enqueued(amount);

        Ok(())
    }
}

impl<C, T: Clone, F, E> BulkScrambler<C, T>
where
    C: BulkConsumer<Item = T, Final = F, Error = E>,
{
    async fn perform_operation(&mut self) -> Result<(), E> {
        debug_assert!(self.buffer.len() > 0);

        match self.operations[self.operations_index] {
            BulkConsumerOperation::Consume => {
                // Remove an item from the buffer.
                let item = self
                    .buffer
                    .dequeue()
                    .expect("queue should contain an item for consumption");

                // Feed the item to the inner consumer.
                self.inner.consume(item).await?;
            }
            BulkConsumerOperation::ConsumeSlots(n) => {
                // Remove items from the queue in bulk and place them in the inner consumer slots.
                let n: usize = n.into();

                // Request writeable slots from the inner consumer.
                let slots = self.inner.expose_slots().await?;

                // Set an upper bound on the slice of slots by comparing the number of available
                // inner slots and the number provided by the `ConsumerSlots` operation and taking
                // the lowest value.
                let slots_len = slots.len();
                let available_slots = &mut slots[..min(slots_len, n)];

                // Dequeue items into the inner consumer.
                let amount = self.buffer.bulk_dequeue(available_slots);

                // Report the amount of items consumed.
                self.inner.consume_slots(amount).await?;
            }
            BulkConsumerOperation::Flush => {
                // Flush the inner consumer.
                self.inner.flush().await?;
            }
        }

        // Update the operations index.
        self.advance_operations_index();

        Ok(())
    }
}
