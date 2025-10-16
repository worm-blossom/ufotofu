use crate::{prelude::*, queues::Queue};

/// A bulk producer wrapper which adds buffering: whenever tasked to produce an item while the internal buffer is empty, the producer eagerly fills its buffer with as many items from the wrapped producer as possible before emitting the requested item.
///
/// Prefer to use a [`BulkBuffered`](super::BulkBuffered) (which wraps *bulk* producers and thus implements more efficient slurping).
///
/// The internal buffer can be any value implementing the [`queues::Queue`](crate::queues::Queue) trait.
///
/// Use the `AsRef<P>` impl to access the wrapped producer.
///
/// Created via [`ProducerExt::buffered`].
///
/// <br/>Counterpart: the [consumer::Buffered] type.
#[derive(Debug)]

pub struct Buffered<P, Final, Error, Q> {
    inner: P,
    buffer: Q,
    last: Option<Result<Final, Error>>,
}

impl<P, Final, Error, Q> Buffered<P, Final, Error, Q> {
    pub(crate) fn new(inner: P, buffer: Q) -> Self {
        Self {
            inner,
            buffer,
            last: None,
        }
    }

    /// Consumes `self` and returns the wrapped producer.
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P, Final, Error, Q> AsRef<P> for Buffered<P, Final, Error, Q> {
    fn as_ref(&self) -> &P {
        &self.inner
    }
}

impl<P, Final, Error, Q> Buffered<P, Final, Error, Q>
where
    P: Producer<Item: Clone, Final = Final, Error = Error>,
    Q: Queue<Item = P::Item>,
{
    async fn fill_buffer_from_inner(&mut self) {
        while self.last.is_none() && !self.buffer.is_full() {
            match self
                .buffer
                .expose_slots(
                    async |items| match self.inner.overwrite_full_slice(items).await {
                        Ok(()) => return (items.len(), Ok(items.len())),
                        Err(err) => return (err.count, Err(err.reason)),
                    },
                )
                .await
            {
                Ok(0) => break,
                Ok(_) => { /* go to next iteration */ }
                Err(last) => {
                    self.last = Some(last);
                    break;
                }
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

impl<P, Final, Error, Q> Producer for Buffered<P, Final, Error, Q>
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

impl<P, Final, Error, Q> BulkProducer for Buffered<P, Final, Error, Q>
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
