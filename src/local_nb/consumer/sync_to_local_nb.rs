use core::mem::MaybeUninit;

use wrapper::Wrapper;

use crate::local_nb::{BufferedConsumer, BulkConsumer, Consumer};
use crate::sync;

/// Turns a [`sync::Consumer`](crate::sync::Consumer) into a [`local_nb::Consumer`](crate::local_nb::Consumer). Only use this to wrap types that never block and do not perform time-intensive computations.
pub struct SyncToLocalNb<C>(pub C);

impl<C: core::fmt::Debug> core::fmt::Debug for SyncToLocalNb<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

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
    async fn expose_slots<'a>(
        &'a mut self,
    ) -> Result<&'a mut [MaybeUninit<Self::Item>], Self::Error>
    where
        Self::Item: 'a,
    {
        self.0.expose_slots()
    }

    async unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.0.consume_slots(amount)
    }

    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        self.0.bulk_consume(buf)
    }
}
