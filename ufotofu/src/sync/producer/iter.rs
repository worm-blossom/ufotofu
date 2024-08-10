use core::convert::Infallible;

use either::{Either, Left, Right};
use wrapper::Wrapper;

use crate::sync::Producer;

/// Treat a [`sync::Producer`](crate::sync::Producer) as an [`Iterator`].
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ProducerToIterator<P>(P);

impl<P> ProducerToIterator<P> {
    /// Wrap a [`sync::Producer`](crate::sync::Producer) as an [`Iterator`].
    pub fn new(producer: P) -> Self {
        Self(producer)
    }
}

impl<P> From<P> for ProducerToIterator<P> {
    fn from(value: P) -> Self {
        Self(value)
    }
}

impl<P> Wrapper<P> for ProducerToIterator<P> {
    fn into_inner(self) -> P {
        self.0
    }
}

impl<P> AsRef<P> for ProducerToIterator<P> {
    fn as_ref(&self) -> &P {
        &self.0
    }
}

impl<P> AsMut<P> for ProducerToIterator<P> {
    fn as_mut(&mut self) -> &mut P {
        &mut self.0
    }
}

impl<P> Iterator for ProducerToIterator<P>
where
    P: Producer<Final = (), Error = Infallible>,
{
    type Item = P::Item;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        match unsafe { self.0.produce().unwrap_unchecked() } {
            Left(item) => Some(item),
            Right(()) => None,
        }
    }
}

/// Treat an [`Iterator`] as an [`sync::Producer`](crate::sync::Producer).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct IteratorToProducer<I>(I);

impl<I> IteratorToProducer<I> {
    /// Wrap an [`Iterator`] as an [`sync::Producer`](crate::sync::Producer).
    pub fn new(iter: I) -> Self {
        Self(iter)
    }
}

impl<I> From<I> for IteratorToProducer<I> {
    fn from(value: I) -> Self {
        Self(value)
    }
}

impl<I> Wrapper<I> for IteratorToProducer<I> {
    fn into_inner(self) -> I {
        self.0
    }
}

impl<I> AsRef<I> for IteratorToProducer<I> {
    fn as_ref(&self) -> &I {
        &self.0
    }
}

impl<I> AsMut<I> for IteratorToProducer<I> {
    fn as_mut(&mut self) -> &mut I {
        &mut self.0
    }
}

impl<I> Producer for IteratorToProducer<I>
where
    I: Iterator,
{
    type Item = I::Item;
    type Final = ();
    type Error = Infallible;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.0.next() {
            Some(item) => Ok(Left(item)),
            None => Ok(Right(())),
        }
    }
}
