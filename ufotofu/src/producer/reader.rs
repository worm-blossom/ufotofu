use std::io::{self, ErrorKind};

use either::{Either, Left, Right};

use smol::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt};
use ufotofu_queues::Queue;

use crate::{BufferedProducer, BulkProducer, Producer};

/// Treat an [`AsyncBufRead`] as a [`BulkProducer`].
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BufReaderToBulkProducer<R>(R);

impl<R> BufReaderToBulkProducer<R> {
    /// Wraps a [`AsyncBufRead`] as a [`BulkProducer`].
    pub fn new(reader: R) -> Self {
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
        match read_exact(&mut self.0, &mut buf).await {
            // match self.0.read_exact(&mut buf).await {
            Err(err) => {
                if err.kind() == ErrorKind::UnexpectedEof {
                    return Ok(Right(()));
                } else {
                    return Err(err);
                }
            }
            Ok(()) => {
                return Ok(Left(buf[0]));
            }
        }
    }
}

impl<R> BufferedProducer for BufReaderToBulkProducer<R>
where
    R: AsyncBufRead + Unpin,
{
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
    /// Signals `Final` if the inner `fill_buf` method ever produces no bytes.
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'a,
    {
        let buf = self.0.fill_buf().await?;

        if buf.len() == 0 {
            return Ok(Right(()));
        } else {
            return Ok(Left(self.0.fill_buf().await?));
        }
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        Ok(self.0.consume(amount))
    }

    /// Signals `Final` if the inner `read` method ever produces no bytes.
    async fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        let amount = self.0.read(buf).await?;

        if amount == 0 {
            return Ok(Right(()));
        } else {
            return Ok(Left(amount));
        }
    }
}

/// Treat an [`AsyncRead`] as an [`BulkProducer`]. Introduces an intermediate buffer of bytes to meaningfully map the API of `AsyncRead` to that of `BulkProducer`. The wrapper tries to *not* use that buffer whenever possible. Only when calling `BulkProducer::expose_items` does it fill the buffer, to offer a slice. When interacting with the wrapper solely through `BulkProducer::bulk_produce` or other methods implemented in terms of `bulk_produce`, the extra buffer gets fully sidestepped.
///
/// When possible, use [`BufReadToBulkProducer`] instead, because it introduces no extra buffering at all.
#[derive(Debug)]
pub struct ReaderToBulkProducer<R, Q> {
    reader: R,
    queue: Q,
}

impl<R, Q> ReaderToBulkProducer<R, Q> {
    /// Wraps an [`AsyncRead`] as a [`BulkProducer`].
    pub fn new(reader: R, queue: Q) -> Self {
        Self { reader, queue }
    }

    /// Recovers the wrapped reader and the internal queue.
    pub fn into_inner(self) -> (R, Q) {
        (self.reader, self.queue)
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
        match self.queue.dequeue() {
            None => {
                // Sidestep the buffer completely. We only fill it when we need to for `BulkProducer::expose_items`.
                let mut buf = [17; 1];
                // match self.reader.read_exact(&mut buf[..]).await {
                match read_exact(&mut self.reader, &mut buf[..]).await {
                    Err(err) => {
                        if err.kind() == ErrorKind::UnexpectedEof {
                            return Ok(Right(()));
                        } else {
                            return Err(err);
                        }
                    }
                    Ok(()) => {
                        return Ok(Left(buf[0]));
                    }
                }
            }
            Some(byte) => {
                return Ok(Left(byte));
            }
        }
    }
}

async fn read_exact<R>(r: &mut R, buf: &mut [u8]) -> Result<(), std::io::Error>
where
    R: AsyncRead + Unpin,
{
    let mut read = 0;

    while read < buf.len() {
        let read_in_this_call = r.read(&mut buf[read..]).await?;
        read += read_in_this_call;
    }

    Ok(())
}

impl<R, Q> BufferedProducer for ReaderToBulkProducer<R, Q>
where
    R: AsyncRead + Unpin,
    Q: Queue<Item = u8>,
{
    /// A no-op that always succeeds
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<R, Q> BulkProducer for ReaderToBulkProducer<R, Q>
where
    R: AsyncRead + Unpin,
    Q: Queue<Item = u8>,
{
    /// Signals `Final` if the inner `read` method ever produces no bytes.
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'a,
    {
        if self.queue.is_empty() {
            // No buffered items, so buffer some and then expose them.
            let buf_slots = self.queue.expose_slots().unwrap(); // All slots are free.

            if self.reader.read(buf_slots).await? == 0 {
                return Ok(Right(()));
            } else {
                return Ok(Left(self.queue.expose_items().unwrap())); // We just filled the queue.
            }
        } else {
            return Ok(Left(self.queue.expose_items().unwrap()));
        }
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.queue.consider_dequeued(amount);
        Ok(())
    }

    /// Signals `Final` if the inner `read` method ever produces no bytes.
    async fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        if self.queue.is_empty() {
            // Sidestep the buffer completely.
            let amount = self.reader.read(buf).await?;

            if amount == 0 {
                return Ok(Right(()));
            } else {
                return Ok(Left(amount));
            }
        } else {
            return Ok(Left(self.queue.bulk_dequeue(buf)));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use crate::{consumer::WriterToBulkConsumer, Consumer};

    use super::*;

    use ufotofu_queues::Fixed;

    use futures::join;
    use smol::{
        io::AsyncWriteExt,
        net::{TcpListener, TcpStream},
    };

    // See https://github.com/smol-rs/futures-lite/issues/132 , our dependencies appear to be a bit wobbly =S

    // #[test]
    // fn tcp_impl_is_not_broken() {
    //     smol::block_on(async {
    //         let send = async {
    //             let mut stream = TcpStream::connect("127.0.0.1:8087").await.unwrap();
    //             stream.write_all(&[42]).await.unwrap();
    //             println!("about to close");
    //             assert_eq!((), stream.close().await.unwrap());
    //             println!("closed");
    //         };

    //         let receive = async {
    //             let listener = TcpListener::bind("127.0.0.1:8087").await.unwrap();
    //             let (mut stream, _addr) = listener.accept().await.unwrap();

    //             let mut buf = [0; 1];
    //             assert_eq!((), stream.read_exact(&mut buf[..]).await.unwrap());
    //             // assert!(stream.read_exact(&mut buf[..]).await.is_err());
    //             // println!("{:?}", stream.read_exact(&mut buf[..]).await);
    //             println!("{:?}", read_exact(&mut stream, &mut buf[..]).await);
    //         };

    //         join!(receive, send);
    //     });
    // }

    // #[test]
    // fn adaptor_regression_test() {
    //     let (input, sender_queue_capacity, rec_queue_capacity) = (vec![7], 3, 3);

    //     println!("\nstarted adaptor_regression_test");

    //     pollster::block_on(async {
    //         let send = async {
    //             let stream = TcpStream::connect("127.0.0.1:8089").await.unwrap();

    //             let sender_queue: Fixed<u8> = Fixed::new(sender_queue_capacity);
    //             let mut sender = WriterToBulkConsumer::new(stream, sender_queue);

    //             for datum in input.iter() {
    //                 assert_eq!((), sender.consume(*datum).await.unwrap());
    //                 println!("inputting {:?}", *datum);
    //             }
    //             println!("about to close");
    //             assert_eq!((), sender.close(()).await.unwrap());
    //             println!("closed");
    //         };

    //         let receive = async {
    //             let listener = TcpListener::bind("127.0.0.1:8089").await.unwrap();
    //             let (stream, _addr) = listener.accept().await.unwrap();

    //             let rec_queue: Fixed<u8> = Fixed::new(rec_queue_capacity);
    //             let mut receiver = ReaderToBulkProducer::new(stream, rec_queue);

    //             for datum in input.iter() {
    //                 assert_eq!(Left(*datum), receiver.produce().await.unwrap());
    //                 println!("successfully checked we got {:?}", *datum);
    //             }

    //             println!("about to do the produce call that should yield Right(())");
    //             assert_eq!(Right(()), receiver.produce().await.unwrap());
    //         };

    //         join!(receive, send);
    //     });
    // }
}

// pollster::block_on(async {
//         let send = async {
//             let stream = TcpStream::connect("127.0.0.1:8087").await.unwrap();

//             let sender_queue: Fixed<u8> = Fixed::new(sender_queue_capacity);
//             let sender = WriterToBulkConsumer::new(stream, sender_queue);
//             let mut sender = consumer::BulkScrambler::new(sender, consume_ops);

//             for datum in input.iter() {
//                 assert_eq!((), sender.consume(*datum).await.unwrap());
//             }
//             assert_eq!((), sender.close(()).await.unwrap());
//         };

//         let receive = async {
//             let listener = TcpListener::bind("127.0.0.1:8087").await.unwrap();
//             let (stream, _addr) = listener.accept().await.unwrap();
//             let stream = BufReader::new(stream);

//             let receiver = BufReaderToBulkProducer::new(stream);
//             let mut receiver = producer::BulkScrambler::new(receiver, produce_ops);

//             for datum in input.iter() {
//                 assert_eq!(Left(*datum), receiver.produce().await.unwrap());
//             }

//             assert_eq!(Right(()), receiver.produce().await.unwrap());
//         };

//         join!(receive, send);
//     });
