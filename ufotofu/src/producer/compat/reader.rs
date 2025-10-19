use std::io::{self, ErrorKind};

use either::{Either, Left, Right};

use crate::queues::Queue;
use futures_lite::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt};

use crate::prelude::*;

/// Creates a bulk producer that draws its items from an [`AsyncBufRead`].
///
/// <br/>Counterpart: none, because [`futures_lite`] does not have a `BufWriter` trait.
pub fn buf_reader_to_bulk_producer<R>(reader: R) -> BufReaderToBulkProducer<R> {
    BufReaderToBulkProducer::new(reader)
}

/// Treat an [`AsyncBufRead`] as a [`BulkProducer`].
///
/// <br/>Counterpart: none, because [`futures_lite`] does not have a `BufWriter` trait.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BufReaderToBulkProducer<R>(R);

impl<R> BufReaderToBulkProducer<R> {
    /// Wraps an [`AsyncBufRead`] as a [`BulkProducer`].
    fn new(reader: R) -> Self {
        Self(reader)
    }

    /// Recovers the wrapped reader.
    pub fn into_inner(self) -> R {
        self.0
    }
}

impl<R> AsRef<R> for BufReaderToBulkProducer<R> {
    fn as_ref(&self) -> &R {
        &self.0
    }
}

impl<R> AsMut<R> for BufReaderToBulkProducer<R> {
    fn as_mut(&mut self) -> &mut R {
        &mut self.0
    }
}

impl<R> Producer for BufReaderToBulkProducer<R>
where
    R: AsyncBufRead + Unpin,
{
    type Item = u8;
    type Final = ();
    type Error = io::Error;

    /// Signals `Final` if the inner `read` method ever produces no bytes.
    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        let mut buf = [0; 1];
        match self.0.read_exact(&mut buf).await {
            Err(err) => {
                if err.kind() == ErrorKind::UnexpectedEof {
                    Ok(Right(()))
                } else {
                    Err(err)
                }
            }
            Ok(()) => Ok(Left(buf[0])),
        }
    }

    /// Calls `BufRead::fill_buf` without exposing the buffer if successful.
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.0.fill_buf().await?;
        Ok(())
    }
}

impl<R> BulkProducer for BufReaderToBulkProducer<R>
where
    R: AsyncBufRead + Unpin,
{
    async fn expose_items<F, Ret>(&mut self, f: F) -> Result<Either<Ret, Self::Final>, Self::Error>
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, Ret),
    {
        let buf = self.0.fill_buf().await?;

        if buf.is_empty() {
            Ok(Right(()))
        } else {
            let (amount, ret) = f(buf).await;
            self.0.consume(amount);
            Ok(Left(ret))
        }
    }
}

/// Creates a bulk producer that draws its items from an [`AsyncRead`].
///
/// When possible, use [`buf_reader_to_bulk_producer`] instead, because it introduces no extra buffering at all.
///
/// <br/>Counterpart: the [`consumer::compat::writer::writer_to_bulk_consumer`] function.
pub fn reader_to_bulk_producer<R, Q>(reader: R, queue: Q) -> ReaderToBulkProducer<R, Q> {
    ReaderToBulkProducer::new(reader, queue)
}

/// Treat an [`AsyncRead`] as an [`BulkProducer`]. Introduces an intermediate buffer of bytes to meaningfully map the API of `AsyncRead` to that of `BulkProducer`.
///
/// When possible, use [`BufReaderToBulkProducer`] instead, because it introduces no extra buffering at all.
///
/// <br/>Counterpart: the [`consumer::compat::writer::WriterToBulkConsumer`] type.
#[derive(Debug)]
pub struct ReaderToBulkProducer<R, Q> {
    reader: R,
    buffer: Q,
    last: Option<Result<(), io::Error>>,
}

impl<R, Q> ReaderToBulkProducer<R, Q> {
    /// Wraps an [`AsyncRead`] as a [`BulkProducer`].
    fn new(reader: R, queue: Q) -> Self {
        Self {
            reader,
            buffer: queue,
            last: None,
        }
    }

    /// Recovers the wrapped reader and the internal queue.
    pub fn into_inner(self) -> (R, Q) {
        (self.reader, self.buffer)
    }
}

impl<R, Q> AsRef<R> for ReaderToBulkProducer<R, Q> {
    fn as_ref(&self) -> &R {
        &self.reader
    }
}

impl<R, Q> ReaderToBulkProducer<R, Q>
where
    R: AsyncRead + Unpin,
    Q: Queue<Item = u8>,
{
    async fn fill_buffer_from_inner(&mut self) {
        while self.last.is_none() && !self.buffer.is_full() {
            match self
                .buffer
                .expose_slots(async |slots| match self.reader.read(slots).await {
                    Ok(amount) => {
                        return (amount, Ok(amount));
                    }
                    Err(err) => return (0, Err(err)),
                })
                .await
            {
                Ok(0) => {
                    self.last = Some(Ok(()));
                    break;
                }
                Ok(_) => { /* go to next iteration */ }
                Err(err) => {
                    self.last = Some(Err(err));
                    break;
                }
            }
        }

        debug_assert!(self.last.is_some() || !self.buffer.is_empty());
    }

    fn check_last(&mut self) -> Option<Result<(), io::Error>> {
        if !self.buffer.is_empty() {
            None
        } else {
            self.last.take()
        }
    }
}

impl<R, Q> Producer for ReaderToBulkProducer<R, Q>
where
    R: AsyncRead + Unpin,
    Q: Queue<Item = u8>,
{
    type Item = u8;
    type Final = ();
    type Error = io::Error;

    /// Signals `Final` if the inner `read` method ever produces no bytes.
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
            None => Ok(()),
        }
    }
}

impl<R, Q> BulkProducer for ReaderToBulkProducer<R, Q>
where
    R: AsyncRead + Unpin,
    Q: Queue<Item = u8>,
{
    /// Signals `Final` if the inner `read` method ever produces no bytes.
    async fn expose_items<F, Ret>(&mut self, f: F) -> Result<Either<Ret, Self::Final>, Self::Error>
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, Ret),
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
