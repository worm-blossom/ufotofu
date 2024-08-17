use core::convert::Infallible;
use core::future::Future;
use core::pin::Pin;
use core::task::Poll;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

use either::{Either, Left, Right};
use futures::{Stream, StreamExt};
use wrapper::Wrapper;

use crate::local_nb::Producer;

/// Treat a [`local_nb::Producer`](crate::local_nb::Producer) as a [`Stream`].
pub struct ProducerToStream<P>
where
    P: Producer<Final = (), Error = Infallible>,
{
    producer: P,
    fut: Option<Pin<Box<dyn Future<Output = Result<Either<P::Item, P::Final>, P::Error>>>>>,
}

impl<P: Producer<Final = (), Error = Infallible>> ProducerToStream<P> {
    /// Wrap a [`local_nb::Producer`](crate::local_nb::Producer) as an [`Iterator`].
    pub fn new(producer: P) -> Self {
        Self {
            producer,
            fut: None,
        }
    }
}

impl<P: Producer<Final = (), Error = Infallible>> From<P> for ProducerToStream<P> {
    fn from(value: P) -> Self {
        Self::new(value)
    }
}

impl<P: Producer<Final = (), Error = Infallible>> Wrapper<P> for ProducerToStream<P> {
    fn into_inner(self) -> P {
        self.producer
    }
}

impl<P: Producer<Final = (), Error = Infallible>> AsRef<P> for ProducerToStream<P> {
    fn as_ref(&self) -> &P {
        &self.producer
    }
}

impl<P: Producer<Final = (), Error = Infallible>> AsMut<P> for ProducerToStream<P> {
    fn as_mut(&mut self) -> &mut P {
        &mut self.producer
    }
}

impl<P> Stream for ProducerToStream<P>
where
    P: Clone + Unpin + Producer<Final = (), Error = Infallible> + 'static,
{
    type Item = P::Item;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if self.fut.is_none() {
            let mut producer = self.producer.clone();
            let fut = async move { producer.produce().await };
            self.fut = Some(Box::pin(fut));
        }

        return Future::poll(
            self.fut.as_mut().unwrap(/* we just set it to Some if it was None */).as_mut(),
            cx,
        )
        .map(|yay| {
            match yay.unwrap(/* err is ! */) {
                Left(item) => Some(item),
                Right(()) => None,
            }
        });
    }
}

/// Treat an [`Stream`] as an [`local_nb::Producer`](crate::local_nb::Producer).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct StreamToProducer<S>(Pin<Box<S>>);

impl<S> StreamToProducer<S> {
    /// Wrap an [`Iterator`] as an [`local_nb::Producer`](crate::local_nb::Producer).
    pub fn new(stream: S) -> Self {
        Self(Box::pin(stream))
    }
}

impl<S> From<S> for StreamToProducer<S> {
    fn from(value: S) -> Self {
        Self::new(value)
    }
}

impl<S: Unpin> Wrapper<S> for StreamToProducer<S> {
    fn into_inner(self) -> S {
        *Pin::into_inner(self.0)
    }
}

impl<S> Producer for StreamToProducer<S>
where
    S: Stream,
{
    type Item = S::Item;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.0.next().await {
            Some(item) => Ok(Left(item)),
            None => Ok(Right(())),
        }
    }
}
