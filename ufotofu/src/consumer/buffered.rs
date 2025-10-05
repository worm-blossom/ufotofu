use crate::{prelude::*, queues::Queue};

/// A bulk consumer wrapper which collects consumed items in an internal buffer before flushing them one-by-one into the wrapped consumer.
///
/// Prefer to use a [`BulkBuffered`](super::BulkBuffered) (which wraps *bulk* consumers and thus implements more efficient flushing).
///
/// The internal buffer can be any value implementing the [`queues::Queue`](crate::queues::Queue) trait.
///
/// Use the `AsRef<C>` impl to access the wrapped consumer.
///
/// Created via [`ConsumerExt::buffered`].
///
/// <br/>Counterpart: the [producer::Buffered] type.
#[derive(Debug)]

pub struct Buffered<C, Q> {
    inner: C,
    buffer: Q,
}

impl<C, Q> Buffered<C, Q> {
    pub(crate) fn new(inner: C, buffer: Q) -> Self {
        Self { inner, buffer }
    }

    /// Consumes `self` and returns the wrapped consumer.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<C, Q> AsRef<C> for Buffered<C, Q> {
    fn as_ref(&self) -> &C {
        &self.inner
    }
}

impl<C, Q> Buffered<C, Q>
where
    C: Consumer<Item: Clone>,
    Q: Queue<Item = C::Item>,
{
    async fn write_buffer_to_inner(&mut self) -> Result<(), C::Error> {
        loop {
            let amount = self
                .buffer
                .expose_items(
                    async |items| match self.inner.consume_full_slice(items).await {
                        Ok(()) => return (items.len(), Ok(items.len())),
                        Err(err) => return (err.count, Err(err.reason)),
                    },
                )
                .await?;

            if amount == 0 {
                break;
            }
        }

        debug_assert!(!self.buffer.is_full());

        Ok(())
    }
}

impl<C, Q> Consumer for Buffered<C, Q>
where
    C: Consumer<Item: Clone>,
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

impl<C, Q> BulkConsumer for Buffered<C, Q>
where
    C: Consumer<Item: Clone>,
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
