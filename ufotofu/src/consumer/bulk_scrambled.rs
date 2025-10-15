use core::{cmp::min, num::NonZeroUsize};

use alloc::{boxed::Box, vec::Vec};

use arbitrary::Arbitrary;

use crate::{prelude::*, queues::Queue};

/// The different operations by which one can interact with a [`BulkConsumer`].
#[derive(Debug, PartialEq, Eq, Arbitrary, Clone, Copy)]
pub enum BulkConsumerOperation {
    /// Call [`Consumer::consume`].
    Consume,
    /// Call [`Consumer::flush`].
    Flush,
    /// Call [`BulkConsumerExt::bulk_consume`] with a buffer whose size is at most the value given in this variant (capped by the number of free slots in the buffer of the [`BulkScrambled`] at the time of executing this operation).
    BulkConsume(NonZeroUsize),
}

// Internal helper functions to determine whether a given slice of operations contains at least one non-flush operation.
fn do_operations_make_progress(ops: &[BulkConsumerOperation]) -> bool {
    ops.iter()
        .any(|op| !matches!(op, BulkConsumerOperation::Flush))
}

/// A bulk consumer wrapper which forwards the sequence it consumes to the wrapped bulk consumer, but by calling its methods according to a (usually randomly generated) predetermined pattern.
///
/// This type is intended for use in property testing, to test that some bulk consumer type (the wrapped one) behaves well even under unusual access patterns. See the [fuzz-testing tutorial](crate::fuzz_testing_tutorial) for typical usage.
///
/// Created via [`BulkConsumerExt::bulk_scrambled`].
///
/// <br/>Counterpart: the [producer::BulkScrambled] type.
#[derive(Debug)]
pub struct BulkScrambled<C, Q> {
    inner: C,
    buffer: Q,
    ops: Box<[BulkConsumerOperation]>,
    op_index: usize,
}

// The `BulkScrambled` works as follows: while its `buffer` contains free slots, it simply consumes into those slots (basically the same way as a `consumer::BulkBuffered` does). When the buffer is full however and it needs to consume an item, it flushes the buffer to the `inner` consumer. It performs this flushing by looping through the `ops` (jumping back to the first op after reaching the final one).

impl<C, Q> BulkScrambled<C, Q> {
    pub(crate) fn new(inner: C, buffer: Q, mut ops: Vec<BulkConsumerOperation>) -> Self {
        if !do_operations_make_progress(&ops) {
            ops.push(BulkConsumerOperation::Consume)
        }

        Self {
            inner,
            buffer,
            ops: ops.into_boxed_slice(),
            op_index: 0,
        }
    }

    /// Consumes `self` and returns the wrapped bulk consumer.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<C, Q> AsRef<C> for BulkScrambled<C, Q> {
    fn as_ref(&self) -> &C {
        &self.inner
    }
}

impl<C, Q> BulkScrambled<C, Q>
where
    C: BulkConsumer<Item: Clone>,
    Q: Queue<Item = C::Item>,
{
    // This is the fun part. Unlike for `BulkBuffered`, we do not try to be efficient here, but instead we strictly follow our `ops`.
    async fn write_buffer_to_inner(&mut self) -> Result<(), C::Error> {
        while !self.buffer.is_empty() {
            match self.ops[self.op_index] {
                BulkConsumerOperation::Consume => {
                    let item = self
                        .buffer
                        .dequeue()
                        .expect("Dequeueing from a non-empty queue must always succeed.");
                    self.inner.consume(item).await?;
                }

                BulkConsumerOperation::Flush => self.inner.flush().await?,

                BulkConsumerOperation::BulkConsume(buffer_len) => {
                    self.buffer.expose_items(async |items| {
                        let items_len = items.len();
                        debug_assert!(items_len > 0, "A non-full queue must not call the callback in expose_items with an empty buffer");

                        match self.inner.bulk_consume(&items[..min(items_len, buffer_len.into())]).await {
                            Ok(amount) => (amount, Ok(())),
                            Err(err) => (0, Err(err)),
                        }
                    }).await?;
                }
            }

            if self.op_index == self.ops.len() - 1 {
                self.op_index = 0;
            } else {
                self.op_index += 1;
            }
        }

        debug_assert!(!self.buffer.is_full());

        Ok(())
    }
}

impl<C, Q> Consumer for BulkScrambled<C, Q>
where
    C: BulkConsumer<Item: Clone>,
    Q: Queue<Item = C::Item>,
{
    type Item = C::Item;
    type Final = C::Final;
    type Error = C::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        match self.buffer.enqueue(item) {
            None => return Ok(()),
            Some(item) => {
                self.write_buffer_to_inner().await?;
                let res = self.buffer.enqueue(item);
                debug_assert!(
                    res.is_none(),
                    "Enqueueing into an empty queue must always succeed."
                );
                return Ok(());
            }
        }
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        self.write_buffer_to_inner().await?;
        self.inner.close(fin).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.write_buffer_to_inner().await?;
        self.inner.flush().await
    }
}

impl<C, Q> BulkConsumer for BulkScrambled<C, Q>
where
    C: BulkConsumer<Item: Clone>,
    Q: Queue<Item = C::Item>,
{
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        if self.buffer.is_full() {
            self.write_buffer_to_inner().await?;
        }

        Ok(self.buffer.expose_slots(async |buffer_slots| {
                debug_assert!(buffer_slots.len() > 0, "A non-full queue must expose at least one item slot when expose_slots is invoked.");
                f(buffer_slots).await
            }).await)
    }
}
