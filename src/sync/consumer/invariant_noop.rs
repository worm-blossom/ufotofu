use core::mem::MaybeUninit;

use wrapper::Wrapper;

use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

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

impl<I, T, F, E> Consumer for Invariant<I>
where
    I: Consumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    type Item = T;
    type Final = F;
    type Error = E;

    fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.inner.consume(item)
    }

    fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.inner.close(final_val)
    }
}

impl<I, T, F, E> BufferedConsumer for Invariant<I>
where
    I: BufferedConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush()
    }
}

impl<I, T, F, E> BulkConsumer for Invariant<I>
where
    I: BulkConsumer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        self.inner.consumer_slots()
    }

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.inner.did_consume(amount)
    }
}
