use core::mem::MaybeUninit;

use either::Either;
use wrapper::Wrapper;

use crate::local_nb::{BufferedProducer, BulkProducer, Producer};
use crate::sync;

/// Turns a [`sync::Producer`](crate::sync::Producer) into a [`local_nb::Producer`](crate::local_nb::Producer). Only use this to wrap types that never block and do not perform time-intensive computations.
#[derive(Debug)]
pub struct SyncToLocalNb<P>(pub P);

impl<P> AsRef<P> for SyncToLocalNb<P> {
    fn as_ref(&self) -> &P {
        &self.0
    }
}

impl<P> AsMut<P> for SyncToLocalNb<P> {
    fn as_mut(&mut self) -> &mut P {
        &mut self.0
    }
}

impl<P> Wrapper<P> for SyncToLocalNb<P> {
    fn into_inner(self) -> P {
        self.0
    }
}

impl<P: sync::Producer> Producer for SyncToLocalNb<P> {
    type Item = P::Item;
    type Final = P::Final;
    type Error = P::Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce()
    }
}

impl<P: sync::BufferedProducer> BufferedProducer for SyncToLocalNb<P> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.0.slurp()
    }
}

impl<P: sync::BulkProducer> BulkProducer for SyncToLocalNb<P>
where
    Self::Item: Copy,
{
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'a,
    {
        self.0.expose_items()
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.consider_produced(amount)
    }

    async fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        self.0.bulk_produce(buf)
    }

    async fn bulk_produce_maybeuninit(
        &mut self,
        buf: &mut [MaybeUninit<Self::Item>],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        self.0.bulk_produce_maybeuninit(buf)
    }
}
