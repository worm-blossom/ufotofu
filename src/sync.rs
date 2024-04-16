use core::cmp::min;
use core::mem::MaybeUninit;

use either::{Either, Left, Right};

pub trait Producer {
    type Item;
    type Final;
    type Error;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error>;
}

pub trait BufferedProducer: Producer {
    fn slurp(&mut self) -> Result<(), Self::Error>;
}

pub trait BulkProducer: BufferedProducer
where
    Self::Item: Copy,
{
    fn producer_slots(&self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error>;

    fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error>;

    fn bulk_produce(
        &mut self,
        buf: &mut [MaybeUninit<Self::Item>],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        return Ok(self.producer_slots()?.map_left(|slots| {
            let amount = min(slots.len(), buf.len());
            MaybeUninit::copy_from_slice(&mut buf[0..amount], &slots[0..amount]);
            self.did_produce(amount)?;
            return amount;
        }));
    }
}

pub trait Consumer {
    type Item;
    type Final;
    type Error;

    fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error>;

    fn close(&mut self, f: Self::Final) -> Result<(), Self::Error>;
}

pub trait BufferedConsumer: Consumer {
    fn flush(&mut self) -> Result<(), Self::Error>;
}

pub trait BulkConsumer: BufferedConsumer
where
    Self::Item: Copy,
{
    fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error>;

    unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error>;

    fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        let slots = self.consumer_slots()?;
        let amount = min(slots.len(), buf.len());
        MaybeUninit::copy_from_slice(&mut slots[0..amount], &buf[0..amount]);
        unsafe {
            self.did_consume(amount)?;
        }
        return Ok(amount);
    }
}

pub fn pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: Producer,
    C: Consumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    loop {
        match producer.produce()? {
            Left(item) => consumer.consume(item)?,
            Right(f) => return Ok(consumer.close(f)?),
        }
    }
}

pub fn bulk_pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: BulkProducer,
    P::Item: Copy,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    return bulk_consume_pipe(producer, consumer);
}

pub fn bulk_consume_pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: BulkProducer,
    P::Item: Copy,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    loop {
        match producer.producer_slots()? {
            Left(slots) => {
                let amount = consumer.bulk_consume(slots)?;
                producer.did_produce(amount)?;
            }
            Right(f) => return Ok(consumer.close(f)?),
        }
    }
}

pub unsafe fn bulk_produce_pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: BulkProducer,
    P::Item: Copy,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    loop {
        let slots = consumer.consumer_slots()?;
        match producer.bulk_produce(slots)? {
            Left(amount) => consumer.did_consume(amount)?,
            Right(f) => return Ok(consumer.close(f)?),
        }
    }
}
