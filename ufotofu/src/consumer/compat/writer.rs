use std::io::{self};

use futures_lite::{AsyncWrite, AsyncWriteExt};

use crate::prelude::*;
use crate::queues::Queue;

/// Creates a bulk consumer that forwards consumed items into an [`AsyncWrite`].
///
/// <br/>Counterpart: the [`producer::compat::reader::reader_to_bulk_producer`] function (but see also [`producer::compat::reader::buf_reader_to_bulk_producer`], which should be preferred whenever possible).
pub fn writer_to_bulk_consumer<W, Q>(writer: W, queue: Q) -> WriterToBulkConsumer<W, Q> {
    WriterToBulkConsumer::new(writer, queue)
}

/// Treats an [`AsyncWrite`] `W` as a [`BulkConsumer`].
///
/// Introduces an intermediate queue (of type `Q`) of bytes to meaningfully map the API of `AsyncWrite` to that of `BulkConsumer`.
///
/// <br/>Counterpart: the [`producer::compat::reader::ReaderToBulkProducer`] type (but see also [`producer::compat::reader::BufReaderToBulkProducer`], which should be preferred whenever possible).
#[derive(Debug)]
pub struct WriterToBulkConsumer<W, Q> {
    writer: W,
    queue: Q,
}

impl<W, Q> WriterToBulkConsumer<W, Q> {
    /// Wraps an [`AsyncWrite`] `W` as a [`BulkConsumer`].
    fn new(writer: W, queue: Q) -> Self {
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
            let amount = self
                .queue
                .expose_items(async |items| match self.writer.write_all(items).await {
                    Ok(()) => return (items.len(), Ok(items.len())),
                    Err(err) => return (0, Err(err)),
                })
                .await?;

            if amount == 0 {
                break;
            }
        }

        debug_assert!(!self.queue.is_full());

        Ok(())
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
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        if self.queue.is_full() {
            self.flush_internal_buffer().await?;
        }

        Ok(self.queue.expose_slots(async |buffer_slots| {
                debug_assert!(buffer_slots.len() > 0, "A non-full queue must expose at least one item slot when expose_slots is invoked.");
                f(buffer_slots).await
            }).await)
    }
}
