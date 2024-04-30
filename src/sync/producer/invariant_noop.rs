use core::convert::{AsMut, AsRef};

use either::Either;
use wrapper::Wrapper;

use crate::sync::{BufferedProducer, BulkProducer, Producer};

#[derive(Debug)]
pub struct Invariant<I> {
    inner: I,
}

impl<I> Invariant<I> {
    pub fn new(inner: I) -> Self {
        Invariant { inner }
    }
}

impl<I> AsRef<I> for Invariant<I> {
    fn as_ref(&self) -> &I {
        &self.inner
    }
}

impl<I> AsMut<I> for Invariant<I> {
    fn as_mut(&mut self) -> &mut I {
        &mut self.inner
    }
}

impl<I> Wrapper<I> for Invariant<I> {
    fn into_inner(self) -> I {
        self.inner
    }
}

impl<I, T, F, E> Producer for Invariant<I>
where
    I: Producer<Item = T, Final = F, Error = E>,
{
    type Item = T;
    type Final = F;
    type Error = E;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.inner.produce()
    }
}

impl<I, T, F, E> BufferedProducer for Invariant<I>
where
    I: BufferedProducer<Item = T, Final = F, Error = E>,
{
    fn slurp(&mut self) -> Result<(), Self::Error> {
        self.inner.slurp()
    }
}

impl<I, T, F, E> BulkProducer for Invariant<I>
where
    I: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn producer_slots(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        self.inner.producer_slots()
    }

    fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.did_produce(amount)
    }
}
