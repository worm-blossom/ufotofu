use core::convert::{AsMut, AsRef};

use either::Either;
use wrapper::Wrapper;

use crate::sync::{BufferedProducer, BulkProducer, Producer};

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

    fn check_inactive(&self) {
        if !self.active {
            panic!("may not call `Producer` methods after the sequence has ended");
        }
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
        self.check_inactive();

        self.inner
            .produce()
            .inspect(|either| {
                // Mark the producer as inactive if the final value is emitted.
                if let Either::Right(_) = either {
                    self.active = false
                }
            })
            .inspect_err(|_| {
                self.active = false;
                // TODO: Do we need to set `exposed_slots` to 0 for any
                // methods? The next call will panic due to `active` being
                // false, so it seems unnecessary.
                self.exposed_slots = 0;
            })
    }
}

impl<I, T, F, E> BufferedProducer for Invariant<I>
where
    I: BufferedProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn slurp(&mut self) -> Result<(), Self::Error> {
        self.check_inactive();

        self.inner.slurp().inspect_err(|_| {
            self.active = false;
            self.exposed_slots = 0;
        })
    }
}

impl<I, T, F, E> BulkProducer for Invariant<I>
where
    I: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn producer_slots(&self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        self.check_inactive();

        self.inner
            .producer_slots()
            .inspect(|either| match either {
                // TODO: Cannot mutably borrow self...
                Either::Left(slots) => self.exposed_slots = slots.len(),
                Either::Right(_) => self.active = false,
            })
            .inspect_err(|_| {
                self.active = false;
                self.exposed_slots = 0;
            })
    }

    fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.check_inactive();

        if amount > self.exposed_slots {
            panic!("may not call `did_produce` with an amount exceeding the total number of exposed slots");
        } else {
            self.exposed_slots -= amount;
        }

        // Proceed with the inner call to `did_produce` and return the result.
        self.inner.did_produce(amount)
    }
}
