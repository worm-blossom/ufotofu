use core::cmp::min;
use core::future::Future;
use core::mem::MaybeUninit;

use either::{Either, Left, Right};

#[trait_variant::make(Producer: Send)]
pub trait LocalProducer {
    type Item;
    type Final;
    type Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error>;
}

pub trait LocalBufferedProducer: LocalProducer {
    async fn slurp(&mut self) -> Result<(), Self::Error>;
}

pub trait BufferedProducer: Producer {
    fn slurp(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

pub trait LocalBulkProducer: LocalBufferedProducer
where
    Self::Item: Copy,
{
    async fn producer_slots(&self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error>;

    async fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error>;

    async fn bulk_produce(
        &mut self,
        buf: &mut [MaybeUninit<Self::Item>],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        return Ok(self.producer_slots().await?.map_left(|slots| {
            let amount = min(slots.len(), buf.len());
            MaybeUninit::copy_from_slice(&mut buf[0..amount], &slots[0..amount]);
            self.did_produce(amount).await?;
            return amount;
        }));
    }
}

pub trait BulkProducer: BufferedProducer
where
    Self::Item: Copy,
{
    fn producer_slots(
        &self,
    ) -> impl Future<Output = Result<Either<&[Self::Item], Self::Final>, Self::Error>> + Send;

    fn did_produce(
        &mut self,
        amount: usize,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn bulk_produce(
        &mut self,
        buf: &mut [MaybeUninit<Self::Item>],
    ) -> impl Future<Output = Result<Either<usize, Self::Final>, Self::Error>> + Send {
        async {
            return Ok((self.producer_slots().await?).map_left(|slots| {
                let amount = min(slots.len(), buf.len());
                MaybeUninit::copy_from_slice(&mut buf[0..amount], &slots[0..amount]);
                self.did_produce(amount).await?;
                return amount;
            }));
        }
    }
}

#[trait_variant::make(Consumer: Send)]
pub trait LocalConsumer {
    type Item;
    type Final;
    type Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error>;

    async fn close(&mut self, f: Self::Final) -> Result<(), Self::Error>;
}

pub trait LocalBufferedConsumer: LocalConsumer {
    async fn flush(&mut self) -> Result<(), Self::Error>;
}

pub trait BufferedConsumer: Consumer {
    fn flush(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

pub trait LocalBulkConsumer: LocalBufferedConsumer
where
    Self::Item: Copy,
{
    async fn consumer_slots(&mut self) -> Result<&mut [MaybeUninit<Self::Item>], Self::Error>;

    async unsafe fn did_consume(&mut self, amount: usize) -> Result<(), Self::Error>;

    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        let slots = self.consumer_slots().await?;
        let amount = min(slots.len(), buf.len());
        MaybeUninit::copy_from_slice(&mut slots[0..amount], &buf[0..amount]);
        unsafe {
            self.did_consume(amount).await?;
        }
        return Ok(amount);
    }
}

pub trait BulkConsumer: BufferedConsumer
where
    Self::Item: Copy,
{
    fn consumer_slots(
        &mut self,
    ) -> impl Future<Output = Result<&mut [MaybeUninit<Self::Item>], Self::Error>> + Send;

    unsafe fn did_consume(
        &mut self,
        amount: usize,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn bulk_consume(
        &mut self,
        buf: &[Self::Item],
    ) -> impl Future<Output = Result<usize, Self::Error>> + Send {
        return async {
            let slots = self.consumer_slots().await?;
            let amount = min(slots.len(), buf.len());
            MaybeUninit::copy_from_slice(&mut slots[0..amount], &buf[0..amount]);
            unsafe {
                self.did_consume(amount).await?;
            }
            return Ok(amount);
        };
    }
}

pub async fn pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: Producer,
    C: Consumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    loop {
        match producer.produce().await? {
            Left(item) => consumer.consume(item).await?,
            Right(f) => return Ok(consumer.close(f).await?),
        }
    }
}

pub async fn bulk_pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: BulkProducer,
    P::Item: Copy,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    return bulk_consume_pipe(producer, consumer).await;
}

pub async fn bulk_consume_pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: BulkProducer,
    P::Item: Copy,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    loop {
        match producer.producer_slots().await? {
            Left(slots) => {
                let amount = consumer.bulk_consume(slots).await?;
                producer.did_produce(amount).await?;
            }
            Right(f) => return Ok(consumer.close(f).await?),
        }
    }
}

pub async unsafe fn bulk_produce_pipe<P, C, E>(producer: &mut P, consumer: &mut C) -> Result<(), E>
where
    P: BulkProducer,
    P::Item: Copy,
    C: BulkConsumer<Item = P::Item, Final = P::Final>,
    E: From<P::Error> + From<C::Error>,
{
    loop {
        let slots = consumer.consumer_slots().await?;
        match producer.bulk_produce(slots).await? {
            Left(amount) => consumer.did_consume(amount).await?,
            Right(f) => return Ok(consumer.close(f).await?),
        }
    }
}

// Also needs send versions of the pipe functions (and the others should be renamed to local_pipe_foo)?
fn todo() {
    42
}
