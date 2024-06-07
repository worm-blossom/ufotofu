use core::mem::MaybeUninit;

use either::Either;
use wrapper::Wrapper;

use crate::local_nb::{
    LocalBufferedConsumer, LocalBufferedProducer, LocalBulkConsumer, LocalBulkProducer,
    LocalConsumer, LocalProducer,
};
use crate::sync::{
    BufferedConsumer, BufferedProducer, BulkConsumer, BulkProducer, Consumer, Producer,
};

/// A `Consumer` wrapper that provides a non-blocking interface to the underlying API.
#[derive(Debug)]
pub struct SyncToLocalNbConsumer<C>(pub C);

impl<C> AsRef<C> for SyncToLocalNbConsumer<C> {
    fn as_ref(&self) -> &C {
        &self.0
    }
}

impl<C> AsMut<C> for SyncToLocalNbConsumer<C> {
    fn as_mut(&mut self) -> &mut C {
        &mut self.0
    }
}

impl<C> Wrapper<C> for SyncToLocalNbConsumer<C> {
    fn into_inner(self) -> C {
        self.0
    }
}

impl<C: Consumer> LocalConsumer for SyncToLocalNbConsumer<C> {
    type Item = C::Item;
    type Final = C::Final;
    type Error = C::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.0.consume(item)
    }

    async fn close(&mut self, f: Self::Final) -> Result<(), Self::Error> {
        self.0.close(f)
    }
}

impl<C: BufferedConsumer> LocalBufferedConsumer for SyncToLocalNbConsumer<C> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}

impl<C: BulkConsumer> LocalBulkConsumer for SyncToLocalNbConsumer<C>
where
    Self::Item: Copy,
{
    async fn consumer_slots<'a>(
        &'a mut self,
    ) -> Result<&'a mut [MaybeUninit<Self::Item>], Self::Error>
    where
        Self::Item: 'a,
    {
        self.0.consumer_slots()
    }

    async unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.did_consume(amount)
    }

    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        self.0.bulk_consume(buf)
    }
}

/// A `Producer` wrapper that provides a non-blocking interface to the underlying API.
#[derive(Debug)]
pub struct SyncToLocalNbProducer<P>(pub P);

impl<P> AsRef<P> for SyncToLocalNbProducer<P> {
    fn as_ref(&self) -> &P {
        &self.0
    }
}

impl<P> AsMut<P> for SyncToLocalNbProducer<P> {
    fn as_mut(&mut self) -> &mut P {
        &mut self.0
    }
}

impl<P> Wrapper<P> for SyncToLocalNbProducer<P> {
    fn into_inner(self) -> P {
        self.0
    }
}

impl<P: Producer> LocalProducer for SyncToLocalNbProducer<P> {
    type Item = P::Item;
    type Final = P::Final;
    type Error = P::Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce()
    }
}

impl<P: BufferedProducer> LocalBufferedProducer for SyncToLocalNbProducer<P> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.0.slurp()
    }
}

impl<P: BulkProducer> LocalBulkProducer for SyncToLocalNbProducer<P>
where
    Self::Item: Copy,
{
    async fn producer_slots<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'a,
    {
        self.0.producer_slots()
    }

    async fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.did_produce(amount)
    }

    async fn bulk_produce(
        &mut self,
        buf: &mut [MaybeUninit<Self::Item>],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        self.0.bulk_produce(buf)
    }
}
