use std::io::{self, ErrorKind, Write};

use either::{Either, Left, Right};
use wrapper::Wrapper;

use ufotofu_queues::{Queue, Static};

use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

/// Treat a [`sync::BulkConsumer`](crate::sync::BulkConsumer) as a [`std::io::Write`].
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BulkConsumerToWrite<C>(C);

impl<C> BulkConsumerToWrite<C> {
    /// Wrap a [`sync::BulkConsumer`](crate::sync::BulkConsumer) as a [`std::io::Write`].
    pub fn new(producer: C) -> Self {
        Self(producer)
    }
}

impl<C> From<C> for BulkConsumerToWrite<C> {
    fn from(value: C) -> Self {
        Self(value)
    }
}

impl<C> Wrapper<C> for BulkConsumerToWrite<C> {
    fn into_inner(self) -> C {
        self.0
    }
}

impl<C> AsRef<C> for BulkConsumerToWrite<C> {
    fn as_ref(&self) -> &C {
        &self.0
    }
}

impl<C> AsMut<C> for BulkConsumerToWrite<C> {
    fn as_mut(&mut self) -> &mut C {
        &mut self.0
    }
}

/// Translates the final value of the producer into a return of `Ok(0)` for `read`.
/// Hence, `read` must not be called after it returns `Ok(0)` (or any error, for that matter).
impl<C> Write for BulkConsumerToWrite<C>
where
    C: BulkConsumer<Item = u8, Final = (), Error = io::Error>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.bulk_consume(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

/// Treat a [`Write`] as an [`sync::BulkConsumer`](crate::sync::BulkConsumer). Introduces an intermediate buffer of `N` bytes to meaningfully map the API of `Write` to that of `BulkConsumer`. The wrapper tries to *not* use that buffer whenever possible. Only when calling `BulkConsumer::expose_slots` does it fill the buffer, to offer a slice. When interacting with the wrapper solely through `BulkConsumer::bulk_consume` or other methods implemented in terms of `bulk_consume`, the extra buffer gets fully sidestepped.
#[derive(Debug)]
pub struct WriteToBulkConsumer<W, const N: usize = 64> {
    writer: W,
    buf: Static<u8, N>,
}

impl<W, const N: usize> WriteToBulkConsumer<W, N> {
    /// Wrap a [`Write`] as an [`sync::BulkConsumer`](crate::sync::BulkConsumer). Also adds an internal buffer of `N` bytes
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            buf: Static::new(),
        }
    }
}

impl<W, const N: usize> From<(W, Static<u8, N>)> for WriteToBulkConsumer<W, N> {
    fn from((writer, buf): (W, Static<u8, N>)) -> Self {
        Self { writer, buf }
    }
}

impl<W, const N: usize> Wrapper<(W, Static<u8, N>)> for WriteToBulkConsumer<W, N> {
    fn into_inner(self) -> (W, Static<u8, N>) {
        (self.writer, self.buf)
    }
}

// impl<W, const N: usize> AsRef<W> for WriteToBulkConsumer<W, N> {
//     fn as_ref(&self) -> &W {
//         &self.0
//     }
// }

// impl<W, const N: usize> AsMut<W> for WriteToBulkConsumer<W, N> {
//     fn as_mut(&mut self) -> &mut W {
//         &mut self.0
//     }
// }

impl<W, const N: usize> WriteToBulkConsumer<W, N>
where
    W: Write,
{
    fn flush_internal_buffer(&mut self) -> Result<(), io::Error> {
        loop {
            match self.buf.expose_items() {
                None => return Ok(()),
                Some(buf) => self.writer.write_all(buf)?,
            }
        }
    }

    fn retrying_writer_flush(&mut self) -> Result<(), io::Error> {
        loop {
            match self.writer.flush() {
                Err(err) if err.kind() == ErrorKind::Interrupted => {}
                res => return res,
            }
        }
    }
}

impl<W, const N: usize> Consumer for WriteToBulkConsumer<W, N>
where
    W: Write,
{
    type Item = u8;
    type Final = ();
    type Error = io::Error;

    /// Transparently retries on `ErrorKind::Interrupted`.
    fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.flush_internal_buffer()?;

        return self.writer.write_all(&[item]);
    }

    /// Flushes the internal buffer and the writer.
    ///
    /// Transparently retries on `ErrorKind::Interrupted`.
    fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        self.flush_internal_buffer()?;
        return self.retrying_writer_flush();
    }
}

impl<W, const N: usize> BufferedConsumer for WriteToBulkConsumer<W, N>
where
    W: Write,
{
    fn flush(&mut self) -> Result<(), Self::Error> {
        return self.retrying_writer_flush();
    }
}

impl<W, const N: usize> BulkConsumer for WriteToBulkConsumer<W, N>
where
    W: Write,
{
    /// Transparently retries on `ErrorKind::Interrupted`.
    fn expose_slots(&mut self) -> Result<&mut [Self::Item], Self::Error> {
        self.flush_internal_buffer()?;
        return Ok(self.buf.expose_slots().unwrap(/* we just flushed the buffer */));
    }

    /// Transparently retries on `ErrorKind::Interrupted`.
    fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.buf.consider_enqueued(amount);
        return Ok(());
    }

    /// Transparently retries on `ErrorKind::Interrupted`.
    fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        self.flush_internal_buffer()?;

        loop {
            match self.writer.write(buf) {
                Err(err) if err.kind() == ErrorKind::Interrupted => {}
                res => return res,
            }
        }
    }
}
