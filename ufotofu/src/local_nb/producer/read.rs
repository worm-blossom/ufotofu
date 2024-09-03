use core::future::Future;
use core::pin::Pin;
use core::task::Poll;

use std::boxed::Box;
use std::io::{self, ErrorKind};

use futures::{AsyncBufRead, AsyncRead};

use either::{Either, Left, Right};
use wrapper::Wrapper;

use ufotofu_queues::{Queue, Static};

use crate::local_nb::{BufferedProducer, BulkProducer, Producer};

/// Treat a [`local_nb::BulkProducer`](crate::local_nb::BulkProducer) as a [`futures::AsyncBufRead`].
pub struct BulkProducerToAsyncBufRead<P>
where
    P: BulkProducer<Final = (), Error = io::Error>,
    <P as Producer>::Item: core::marker::Copy,
{
    producer: P,
    fut: Option<Pin<Box<dyn Future<Output = Result<Either<usize, P::Final>, P::Error>>>>>,
}

impl<P> BulkProducerToAsyncBufRead<P>
where
    P: BulkProducer<Final = (), Error = io::Error>,
    <P as Producer>::Item: core::marker::Copy,
{
    /// Wrap a [`local_nb::BulkProducer`](crate::local_nb::BulkProducer) as a [`futures::AsyncBufRead`].
    pub fn new(producer: P) -> Self {
        Self {
            producer,
            fut: None,
        }
    }
}

impl<P> From<P> for BulkProducerToAsyncBufRead<P>
where
    P: BulkProducer<Final = (), Error = io::Error>,
    <P as Producer>::Item: core::marker::Copy,
{
    fn from(value: P) -> Self {
        Self::new(value)
    }
}

impl<P> Wrapper<P> for BulkProducerToAsyncBufRead<P>
where
    P: BulkProducer<Final = (), Error = io::Error>,
    <P as Producer>::Item: core::marker::Copy,
{
    fn into_inner(self) -> P {
        self.producer
    }
}

impl<P> AsRef<P> for BulkProducerToAsyncBufRead<P>
where
    P: BulkProducer<Final = (), Error = io::Error>,
    <P as Producer>::Item: core::marker::Copy,
{
    fn as_ref(&self) -> &P {
        &self.producer
    }
}

impl<P> AsMut<P> for BulkProducerToAsyncBufRead<P>
where
    P: BulkProducer<Final = (), Error = io::Error>,
    <P as Producer>::Item: core::marker::Copy,
{
    fn as_mut(&mut self) -> &mut P {
        &mut self.producer
    }
}

/// Translates the final value of the producer into a return of `Ok(0)` for `read`.
/// Hence, `read` must not be called after it returns `Ok(0)` (or any error, for that matter).
impl<P> AsyncRead for BulkProducerToAsyncBufRead<P>
where
    P: Clone + Unpin + BulkProducer<Item = u8, Final = (), Error = io::Error> + 'static,
{
    fn poll_read(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
        buf: &mut [u8],
    ) -> core::task::Poll<io::Result<usize>> {
        if self.fut.is_none() {
            let mut producer = self.producer.clone();
            let fut = async move { producer.bulk_produce(buf).await };
            self.fut = Some(Box::pin(fut));
        }

        return Future::poll(
            self.fut.as_mut().unwrap(/* we just set it to Some if it was None */).as_mut(),
            cx,
        )
        .map(|yay| {
            yay.map(|either| match either {
                Left(amount) => amount,
                Right(()) => 0,
            })
        });
    }
}

// /// Translates the final value of the producer into a return of the empty slice.
// ///
// /// Will panic if `P::consider_produced()` yields an error, because `BufRead::consume` does
// /// not return a result.
// impl<P> AsyncBufRead for BulkProducerToAsyncBufRead<P>
// where
//     P: BulkProducer<Item = u8, Final = (), Error = io::Error>,
// {
//     /// Translates the final value of the producer into a return of the empty slice.
//     fn fill_buf(&mut self) -> io::Result<&[u8]> {
//         match self.0.expose_items()? {
//             Left(buf) => Ok(buf),
//             Right(()) => Ok(&[]),
//         }
//     }

//     /// Will panic if `P::consider_produced()` yields an error.
//     fn consume(&mut self, amt: usize) {
//         self.0
//             .consider_produced(amt)
//             .expect("`consider_produced` must not error when used in a `BulkProducerToBufRead`.")
//     }
// }

// /// Treat an [`AsyncBufRead`] as an [`local_nb::BulkProducer`](crate::local_nb::BulkProducer).
// #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
// pub struct AsyncBufReadToBulkProducer<R>(R);

// impl<R> AsyncBufReadToBulkProducer<R> {
//     /// Wrap an [`AsyncBufRead`] as an [`local_nb::BulkProducer`](crate::local_nb::BulkProducer).
//     pub fn new(reader: R) -> Self {
//         Self(reader)
//     }
// }

// impl<R> From<R> for AsyncBufReadToBulkProducer<R> {
//     fn from(value: R) -> Self {
//         Self(value)
//     }
// }

// impl<R> Wrapper<R> for AsyncBufReadToBulkProducer<R> {
//     fn into_inner(self) -> R {
//         self.0
//     }
// }

// impl<R> AsRef<R> for AsyncBufReadToBulkProducer<R> {
//     fn as_ref(&self) -> &R {
//         &self.0
//     }
// }

// impl<R> AsMut<R> for AsyncBufReadToBulkProducer<R> {
//     fn as_mut(&mut self) -> &mut R {
//         &mut self.0
//     }
// }

// impl<R> Producer for AsyncBufReadToBulkProducer<R>
// where
//     R: AsyncBufRead,
// {
//     type Item = u8;
//     type Final = ();
//     type Error = io::Error;

//     /// Signals `Final` if the inner `read` method ever produces no bytes.
//     ///
//     /// Transparently retries on `ErrorKind::Interrupted`.
//     fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
//         let mut buf = [0; 1];
//         match self.0.read_exact(&mut buf) {
//             Err(err) => {
//                 if err.kind() == ErrorKind::UnexpectedEof {
//                     return Ok(Right(()));
//                 } else {
//                     return Err(err);
//                 }
//             }
//             Ok(()) => {
//                 return Ok(Left(buf[0]));
//             }
//         }
//     }
// }

// impl<R> BufferedProducer for AsyncBufReadToBulkProducer<R>
// where
//     R: AsyncBufRead,
// {
//     /// Calls `BufRead::fill_buf` without exposing the buffer if successful.
//     ///
//     /// Transparently retries on `ErrorKind::Interrupted`.
//     fn slurp(&mut self) -> Result<(), Self::Error> {
//         loop {
//             match self.0.fill_buf() {
//                 Err(err) => {
//                     if err.kind() != ErrorKind::Interrupted {
//                         return Err(err);
//                     }
//                 }
//                 Ok(_) => return Ok(()),
//             }
//         }
//     }
// }

// impl<R> BulkProducer for AsyncBufReadToBulkProducer<R>
// where
//     R: AsyncBufRead,
// {
//     /// Signals `Final` if the inner `fill_buf` method ever produces no bytes.
//     ///
//     /// Transparently retries on `ErrorKind::Interrupted`.
//     fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
//         loop {
//             match self.0.fill_buf() {
//                 Err(err) => {
//                     if err.kind() != ErrorKind::Interrupted {
//                         return Err(err);
//                     }
//                 }
//                 Ok(buf) => {
//                     if buf.len() == 0 {
//                         return Ok(Right(()));
//                     } else {
//                         break; // returning Ok(Left(buf)) directly here makes the borrow checker complain...
//                     }
//                 }
//             }
//         }

//         return Ok(Left(self.0.fill_buf()?));
//     }

//     fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
//         Ok(self.0.consume(amount))
//     }

//     /// Signals `Final` if the inner `read` method ever produces no bytes.
//     ///
//     /// Transparently retries on `ErrorKind::Interrupted`.
//     fn bulk_produce(
//         &mut self,
//         buf: &mut [Self::Item],
//     ) -> Result<Either<usize, Self::Final>, Self::Error> {
//         loop {
//             match self.0.read(buf) {
//                 Err(err) => {
//                     if err.kind() != ErrorKind::Interrupted {
//                         return Err(err);
//                     }
//                 }
//                 Ok(amount) => {
//                     if amount == 0 {
//                         return Ok(Right(()));
//                     } else {
//                         return Ok(Left(amount));
//                     }
//                 }
//             }
//         }
//     }
// }

// /// Treat an [`AsyncRead`] as an [`local_nb::BulkProducer`](crate::local_nb::BulkProducer). Introduces an intermediate buffer of `N` bytes to meaningfully map the API of `Read` to that of `BulkProducer`. The wrapper tries to *not* use that buffer whenever possible. Only when calling `BulkProducer::expose_items` does it fill the buffer, to offer a slice. When interacting with the wrapper solely through `BulkProducer::bulk_produce` or other methods implemented in terms of `bulk_produce`, the extra buffer gets fully sidestepped.
// ///
// /// When possible, use [`AsyncBufReadToBulkProducer`] instead, because it introduces no extra buffering at all.
// #[derive(Debug)]
// pub struct AsyncReadToBulkProducer<R, const N: usize = 64> {
//     reader: R,
//     buf: Static<u8, N>,
// }

// impl<R, const N: usize> AsyncReadToBulkProducer<R, N> {
//     /// Wrap a [`AsyncRead`] as an [`local_nb::BulkProducer`](crate::local_nb::BulkProducer). Also adds an internal buffer of `N` bytes
//     pub fn new(reader: R) -> Self {
//         Self {
//             reader,
//             buf: Static::new(),
//         }
//     }
// }

// impl<R, const N: usize> From<(R, Static<u8, N>)> for AsyncReadToBulkProducer<R, N> {
//     fn from((reader, buf): (R, Static<u8, N>)) -> Self {
//         Self { reader, buf }
//     }
// }

// impl<R, const N: usize> Wrapper<(R, Static<u8, N>)> for AsyncReadToBulkProducer<R, N> {
//     fn into_inner(self) -> (R, Static<u8, N>) {
//         (self.reader, self.buf)
//     }
// }

// // impl<R, const N: usize> AsRef<R> for ReadToBulkProducer<R, N> {
// //     fn as_ref(&self) -> &R {
// //         &self.0
// //     }
// // }

// // impl<R, const N: usize> AsMut<R> for ReadToBulkProducer<R, N> {
// //     fn as_mut(&mut self) -> &mut R {
// //         &mut self.0
// //     }
// // }

// impl<R, const N: usize> Producer for AsyncReadToBulkProducer<R, N>
// where
//     R: AsyncRead,
// {
//     type Item = u8;
//     type Final = ();
//     type Error = io::Error;

//     /// Signals `Final` if the inner `read` method ever produces no bytes.
//     ///
//     /// Transparently retries on `ErrorKind::Interrupted`.
//     fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
//         match self.buf.dequeue() {
//             None => {
//                 // Sidestep the buffer completely. We only fill it when we need to for `BulkProducer::expose_items`.
//                 let mut buf = [0; 1];
//                 match self.reader.read_exact(&mut buf) {
//                     Err(err) => {
//                         if err.kind() == ErrorKind::UnexpectedEof {
//                             return Ok(Right(()));
//                         } else {
//                             return Err(err);
//                         }
//                     }
//                     Ok(()) => {
//                         return Ok(Left(buf[0]));
//                     }
//                 }
//             }
//             Some(byte) => {
//                 return Ok(Left(byte));
//             }
//         }
//     }
// }

// impl<R, const N: usize> BufferedProducer for AsyncReadToBulkProducer<R, N>
// where
//     R: AsyncRead,
// {
//     /// A no-op that always succeeds
//     fn slurp(&mut self) -> Result<(), Self::Error> {
//         Ok(())
//     }
// }

// impl<R, const N: usize> BulkProducer for AsyncReadToBulkProducer<R, N>
// where
//     R: AsyncRead,
// {
//     /// Signals `Final` if the inner `read` method ever produces no bytes.
//     ///
//     /// Transparently retries on `ErrorKind::Interrupted`.
//     fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
//         if self.buf.is_empty() {
//             // No buffered items, so buffer some and then expose them.
//             let buf_slots = self.buf.expose_slots().unwrap(); // All slots are free.

//             loop {
//                 match self.reader.read(buf_slots) {
//                     Err(err) => {
//                         if err.kind() != ErrorKind::Interrupted {
//                             return Err(err);
//                         }
//                     }
//                     Ok(0) => {
//                         return Ok(Right(()));
//                     }
//                     Ok(_) => {
//                         return Ok(Left(self.buf.expose_items().unwrap())); // We just filled the queue.
//                     }
//                 }
//             }
//         } else {
//             return Ok(Left(self.buf.expose_items().unwrap()));
//         }
//     }

//     fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
//         self.buf.consider_dequeued(amount);
//         Ok(())
//     }

//     /// Signals `Final` if the inner `read` method ever produces no bytes.
//     ///
//     /// Transparently retries on `ErrorKind::Interrupted`.
//     fn bulk_produce(
//         &mut self,
//         buf: &mut [Self::Item],
//     ) -> Result<Either<usize, Self::Final>, Self::Error> {
//         if self.buf.is_empty() {
//             // Sidestep the buffer completely.
//             loop {
//                 match self.reader.read(buf) {
//                     Err(err) => {
//                         if err.kind() != ErrorKind::Interrupted {
//                             return Err(err);
//                         }
//                     }
//                     Ok(amount) => {
//                         if amount == 0 {
//                             return Ok(Right(()));
//                         } else {
//                             return Ok(Left(amount));
//                         }
//                     }
//                 }
//             }
//         } else {
//             return Ok(Left(self.buf.bulk_dequeue(buf)));
//         }
//     }
// }
