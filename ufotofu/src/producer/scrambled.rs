use alloc::{boxed::Box, vec::Vec};

use arbitrary::Arbitrary;

use crate::{prelude::*, queues::Queue};

/// The different operations by which one can interact with a [`Producer`].
#[derive(Debug, PartialEq, Eq, Arbitrary, Clone, Copy)]
pub enum ProducerOperation {
    /// Call [`Producer::produce`].
    Produce,
    /// Call [`Producer::slurp`].
    Slurp,
}

// Internal helper functions to determine whether a given slice of operations contains at least one non-slurp operation.
fn do_operations_make_progress(ops: &[ProducerOperation]) -> bool {
    ops.iter().any(|op| !matches!(op, ProducerOperation::Slurp))
}

/// A producer wrapper which produces the same sequence as the wrapped producer, but which interacts with the wrapped producer by calling its methods according to a (usually randomly generated) predetermined pattern.
///
/// This type is intended for use in property testing, to test that some producer type (the wrapped one) behaves well even under unusual access patterns. See the [fuzz-testing tutorial](crate::fuzz_testing_tutorial) for typical usage.
///
/// Created via [`ProducerExt::scrambled`].
///
/// <br/>Counterpart: the [consumer::Scrambled] type.
#[derive(Debug)]
pub struct Scrambled<P, Q, Final, Error> {
    inner: P,
    buffer: Q,
    last: Option<Result<Final, Error>>,
    ops: Box<[ProducerOperation]>,
    op_index: usize,
}

// The `Scrambled` works as follows: while its `buffer` contains items, it simply produces those items (basically the same way as a `producer::Buffered` does). When the buffer is empty however and it needs to produce an item, it fills the buffer by requesting items from the `inner` producer. It requests these items by looping through the `ops` (jumping back to the first op after reaching the final one).

impl<P, Q, Final, Error> Scrambled<P, Q, Final, Error> {
    pub(crate) fn new(inner: P, buffer: Q, mut ops: Vec<ProducerOperation>) -> Self {
        if !do_operations_make_progress(&ops) {
            ops.push(ProducerOperation::Produce)
        }

        Self {
            inner,
            buffer,
            last: None,
            ops: ops.into_boxed_slice(),
            op_index: 0,
        }
    }

    /// Consumes `self` and returns the wrapped producer.
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P, Q, Final, Error> AsRef<P> for Scrambled<P, Q, Final, Error> {
    fn as_ref(&self) -> &P {
        &self.inner
    }
}

impl<P, Q, Final, Error> Scrambled<P, Q, Final, Error>
where
    P: Producer<Item: Clone, Final = Final, Error = Error>,
    Q: Queue<Item = P::Item>,
{
    // This is the fun part. Unlike for `Buffered`, we do not try to be efficient here, but instead we strictly follow our `ops`.
    async fn fill_buffer_from_inner(&mut self) {
        while self.last.is_none() && !self.buffer.is_full() {
            match self.ops[self.op_index] {
                ProducerOperation::Produce => match self.inner.produce().await {
                    Ok(Left(item)) => {
                        let res = self.buffer.enqueue(item);
                        debug_assert!(
                            res.is_none(),
                            "Enqueueing into a non-full queue must always succeed."
                        );
                    }
                    Ok(Right(fin)) => {
                        self.last = Some(Ok(fin));
                    }
                    Err(err) => {
                        self.last = Some(Err(err));
                    }
                },

                ProducerOperation::Slurp => match self.inner.slurp().await {
                    Ok(()) => { /* no-op */ }
                    Err(err) => self.last = Some(Err(err)),
                },
            }

            if self.op_index == self.ops.len() - 1 {
                self.op_index = 0;
            } else {
                self.op_index += 1;
            }
        }

        debug_assert!(self.last.is_some() || !self.buffer.is_empty());
    }

    fn check_last(&mut self) -> Option<Result<Final, Error>> {
        if !self.buffer.is_empty() {
            None
        } else {
            self.last.take()
        }
    }
}

impl<P, Q, Final, Error> Producer for Scrambled<P, Q, Final, Error>
where
    P: Producer<Item: Clone, Final = Final, Error = Error>,
    Q: Queue<Item = P::Item>,
{
    type Item = P::Item;
    type Final = P::Final;
    type Error = P::Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.check_last() {
            Some(Ok(fin)) => return Ok(Right(fin)),
            Some(Err(err)) => return Err(err),
            None => match self.buffer.dequeue() {
                Some(item) => return Ok(Left(item)),
                None => {
                    self.fill_buffer_from_inner().await;
                    match self.check_last() {
                        Some(Ok(fin)) => return Ok(Right(fin)),
                        Some(Err(err)) => return Err(err),
                        None => {
                            Ok(Left(self.buffer.dequeue().expect(
                                "Dequeueing from a non-empty queue must always suceed.",
                            )))
                        }
                    }
                }
            },
        }
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        match self.check_last() {
            Some(Ok(fin)) => {
                self.last = Some(Ok(fin));
                return Ok(());
            }
            Some(Err(err)) => {
                debug_assert!(self.buffer.is_empty());
                return Err(err);
            }
            None => {
                self.fill_buffer_from_inner().await;

                if self.last.is_none() {
                    match self.inner.slurp().await {
                        Ok(()) => return Ok(()),
                        Err(err) => {
                            if self.buffer.is_empty() {
                                return Err(err);
                            } else {
                                self.last = Some(Err(err));
                                return Ok(());
                            }
                        }
                    }
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl<P, Q, Final, Error> BulkProducer for Scrambled<P, Q, Final, Error>
where
    P: Producer<Item: Clone, Final = Final, Error = Error>,
    Q: Queue<Item = P::Item>,
{
    async fn expose_items<F, R>(&mut self, f: F) -> Result<Either<R, Self::Final>, Self::Error>
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, R),
    {
        match self.check_last() {
            Some(Ok(fin)) => return Ok(Right(fin)),
            Some(Err(err)) => return Err(err),
            None => {
                if self.buffer.is_empty() {
                    self.fill_buffer_from_inner().await;

                    match self.check_last() {
                        Some(Ok(fin)) => return Ok(Right(fin)),
                        Some(Err(err)) => return Err(err),
                        None => { /* continue with a non-empty buffer */ }
                    }
                }

                Ok(Left(self.buffer.expose_items(async |buffer_items| {
                    debug_assert!(buffer_items.len() > 0, "A non-empty queue must expose at least one item when expose_items is invoked");
                    f(buffer_items).await
                }).await))
            }
        }
    }
}
