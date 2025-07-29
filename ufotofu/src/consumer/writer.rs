use std::io::{self};

use smol::io::{AsyncWrite, AsyncWriteExt};
use ufotofu_queues::Queue;

use crate::{BufferedConsumer, BulkConsumer, Consumer};

/// Treats an [`AsyncWrite`] `W` as a [`BulkConsumer`].
/// Introduces an intermediate queue of bytes to meaningfully map the API of `AsyncWrite` to that of `BulkConsumer`. The wrapper tries to *not* use that buffer whenever possible. Only when calling `BulkConsumer::expose_slots` does it fill the buffer, to offer a slice. When interacting with the wrapper solely through `BulkConsumer::bulk_consume` or other methods implemented in terms of `bulk_consume`, the extra buffer gets fully sidestepped.
#[derive(Debug)]
pub struct WriterToBulkConsumer<W, Q> {
    writer: W,
    queue: Q,
}

impl<W, Q> WriterToBulkConsumer<W, Q> {
    /// Wraps [`AsyncWrite`] `W` as a [`BulkConsumer`].
    pub fn new(writer: W, queue: Q) -> Self {
        Self { writer, queue }
    }

    /// Recovers the original writer and the internal queue.
    pub fn into_inner(self) -> (W, Q) {
        (self.writer, self.queue)
    }
}

impl<W, Q> WriterToBulkConsumer<W, Q>
where
    W: AsyncWrite + Unpin,
    Q: Queue<Item = u8>,
{
    async fn flush_internal_buffer(&mut self) -> Result<(), io::Error> {
        loop {
            match self.queue.expose_items() {
                None => return Ok(()),
                Some(queue) => self.writer.write_all(queue).await?,
            }
        }
    }
}

impl<W, Q> Consumer for WriterToBulkConsumer<W, Q>
where
    W: AsyncWrite + Unpin,
    Q: Queue<Item = u8>,
{
    type Item = u8;
    type Final = ();
    type Error = io::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.flush_internal_buffer().await?;

        self.writer.write_all(&[item]).await
    }

    /// Flushes the internal buffer and the writer.
    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        self.flush_internal_buffer().await?;
        self.writer.close().await
    }
}

impl<W, Q> BufferedConsumer for WriterToBulkConsumer<W, Q>
where
    W: AsyncWrite + Unpin,
    Q: Queue<Item = u8>,
{
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.flush_internal_buffer().await?;
        self.writer.flush().await
    }
}

impl<W, Q> BulkConsumer for WriterToBulkConsumer<W, Q>
where
    W: AsyncWrite + Unpin,
    Q: Queue<Item = u8>,
{
    async fn expose_slots<'a>(&'a mut self) -> Result<&'a mut [Self::Item], Self::Error>
    where
        Self::Item: 'a,
    {
        self.flush_internal_buffer().await?;
        return Ok(self.queue.expose_slots().unwrap(/* we just flushed the buffer */));
    }

    async fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.queue.consider_enqueued(amount);
        Ok(())
    }

    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        self.flush_internal_buffer().await?;
        self.writer.write(buf).await
    }
}
