use core::mem::MaybeUninit;

use wrapper::Wrapper;

use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

pub struct Invariant<I> {
    inner: I,
    active: bool,
    exposed_slots: usize,
}

impl<I> Invariant<I> {
    pub fn new(inner: I) -> Self {
        Invariant {
            inner,
            active: true,
            exposed_slots: 0,
        }
    }
}

impl<I> Wrapper<I> for Invariant<I> {
    fn into_inner(self) -> I {
        self.inner
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

impl<I> Invariant<I> {
    fn check_inactive(&self) {
        if !self.active {
            panic!("may not call `Consumer` methods after the sequence has ended");
        }
    }
}

impl<I: BulkConsumer<Item = T, Final = F, Error = E>, T: Copy, F, E> Consumer for Invariant<I> {
    type Item = T;
    type Final = F;
    type Error = E;

    fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.check_inactive();

        let res = self.inner.consume(item);
        if res.is_err() {
            // Since `consume()` returned an error, we need to ensure
            // that any future call to trait methods will panic.
            self.active = false;
            self.exposed_slots = 0;
        }

        res
    }

    fn close(&mut self, final_val: Self::Final) -> Result<(), Self::Error> {
        self.check_inactive();
        self.active = false;

        self.inner.close(final_val)
    }
}

impl<
        I: BufferedConsumer<Item = T, Final = F, Error = E>
            + BulkConsumer<Item = T, Final = F, Error = E>,
        T: Copy,
        F,
        E,
    > BufferedConsumer for Invariant<I>
{
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.check_inactive();

        let res = self.inner.flush();
        if res.is_err() {
            self.active = false;
            self.exposed_slots = 0;
        }

        res
    }
}

impl<I: BulkConsumer<Item = T, Final = F, Error = E>, T: Copy, F, E> BulkConsumer for Invariant<I> {
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error> {
        self.check_inactive();

        match self.inner.consumer_slots() {
            Ok(slots) => {
                self.exposed_slots = slots.len();

                Ok(slots)
            }
            Err(err) => {
                self.active = false;
                self.exposed_slots = 0;

                Err(err)
            }
        }
    }

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.check_inactive();

        if amount > self.exposed_slots {
            panic!(
                "may not call `did_consume` with an amount exceeding the total number of exposed slots"
            );
        } else {
            self.exposed_slots -= amount;
        }

        // Proceed with the inner call to `did_consume` and return the result.
        self.inner.did_consume(amount)
    }
}
