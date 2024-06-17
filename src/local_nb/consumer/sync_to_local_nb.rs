use core::mem::MaybeUninit;

use wrapper::Wrapper;

use crate::local_nb::{BufferedConsumer, BulkConsumer, Consumer};
use crate::sync;

/// Turns a [`sync::Consumer`](crate::sync::Consumer) into a [`local_nb::Consumer`](crate::local_nb::Consumer). Only use this to wrap types that never block (and neither perform time-intensive computations).
#[derive(Debug)]
pub struct SyncToLocalNb<C>(pub C);

impl<C> AsRef<C> for SyncToLocalNb<C> {
    fn as_ref(&self) -> &C {
        &self.0
    }
}

impl<C> AsMut<C> for SyncToLocalNb<C> {
    fn as_mut(&mut self) -> &mut C {
        &mut self.0
    }
}

impl<C> Wrapper<C> for SyncToLocalNb<C> {
    fn into_inner(self) -> C {
        self.0
    }
}

impl<C: sync::Consumer> Consumer for SyncToLocalNb<C> {
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

impl<C: sync::BufferedConsumer> BufferedConsumer for SyncToLocalNb<C> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}

impl<C: sync::BulkConsumer> BulkConsumer for SyncToLocalNb<C>
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
