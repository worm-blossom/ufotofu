use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

use alloc::vec::Vec;
use arbitrary::{Arbitrary, Unstructured};
use wrapper::Wrapper;

use crate::common::consumer::Invariant;
use crate::common::consumer::{IntoVec, Scramble};
use crate::local_nb::{
    BufferedConsumer as BufferedConsumerLocalNb, BulkConsumer as BulkConsumerLocalNb,
    Consumer as ConsumerLocalNb,
};
use crate::sync::{BufferedConsumer, BulkConsumer, Consumer};

use super::ConsumeOperations;

/// A consumer for fuzz testing purposes. You can only construct this via its `Arbitrary` implementation. It successfully operates, until it decides to error after a random number of operations.
///
/// Use the `Wrapper::into_inner` implementation to obtain a `Vec` of all consumed items.
#[derive(Arbitrary)]
pub struct TestConsumer_<Item, Final, Error>(Invariant<TestConsumer<Item, Final, Error>>);

invarianted_impl_debug!(TestConsumer_<Item: Debug, Final: Debug, Error: Debug>);

invarianted_impl_as_ref!(TestConsumer_<Item, Final, Error>; [Item]);
invarianted_impl_wrapper!(TestConsumer_<Item, Final, Error>; Vec<Item>);

invarianted_impl_consumer_sync_and_local_nb!(TestConsumer_<Item: Copy, Final, Error> Item Item; Final Final; Error Error);
invarianted_impl_buffered_consumer_sync_and_local_nb!(TestConsumer_<Item: Copy, Final, Error>);
invarianted_impl_bulk_consumer_sync_and_local_nb!(TestConsumer_<Item: Copy, Final, Error>);

#[derive(Debug)]
struct TestConsumer<Item, Final, Error> {
    inner: Scramble<IntoVec<Item>, Item, (), !>,
    error: Option<Error>,
    countdown_till_error: usize,
    phantom: PhantomData<Final>,
}

impl<Item, Final, Error> TestConsumer<Item, Final, Error> {
    fn as_ref(&self) -> &[Item] {
        self.inner.as_ref().as_ref()
    }
}

impl<Item, Final, Error> Wrapper<Vec<Item>> for TestConsumer<Item, Final, Error> {
    fn into_inner(self) -> Vec<Item> {
        return self.inner.into_inner().into_inner();
    }
}

impl<Item, Final, Error> Consumer for TestConsumer<Item, Final, Error>
where
    Item: Copy,
{
    type Item = Item;
    type Final = Final;
    type Error = Error;

    fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            let _ = BufferedConsumer::flush(&mut self.inner);

            return Err(self.error.take().expect(
                "Do not call consume after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(Consumer::consume(&mut self.inner, item).unwrap()); // may unwrap because Err<!>
        }
    }

    fn close(&mut self, _f: Self::Final) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            let _ = BufferedConsumer::flush(&mut self.inner);

            return Err(self.error.take().expect(
                "Do not close consume after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(Consumer::close(&mut self.inner, ()).unwrap()); // may unwrap because Err<!>
        }
    }
}

impl<Item, Final, Error> BufferedConsumer for TestConsumer<Item, Final, Error>
where
    Item: Copy,
{
    fn flush(&mut self) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            let _ = BufferedConsumer::flush(&mut self.inner);

            return Err(self.error.take().expect(
                "Do not call flush after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(BufferedConsumer::flush(&mut self.inner).unwrap()); // may unwrap because Err<!>
        }
    }
}

impl<Item, Final, Error> BulkConsumer for TestConsumer<Item, Final, Error>
where
    Item: Copy,
{
    fn expose_slots(&mut self) -> Result<&mut [core::mem::MaybeUninit<Self::Item>], Self::Error> {
        if self.countdown_till_error == 0 {
            let _ = BufferedConsumer::flush(&mut self.inner);

            return Err(self.error.take().expect(
                "Do not call consumer_slots after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(BulkConsumer::expose_slots(&mut self.inner).unwrap());
            // may unwrap because Err<!>
        }
    }

    unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            let _ = BufferedConsumer::flush(&mut self.inner);
            return Err(self.error.take().expect(
                "Do not call did_consume after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(BulkConsumer::consume_slots(&mut self.inner, amount).unwrap());
            // may unwrap because Err<!>
        }
    }
}

impl<Item, Final, Error> ConsumerLocalNb for TestConsumer<Item, Final, Error>
where
    Item: Copy,
{
    type Item = Item;
    type Final = Final;
    type Error = Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            let _ = BufferedConsumerLocalNb::flush(&mut self.inner).await;

            return Err(self.error.take().expect(
                "Do not call consume after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(ConsumerLocalNb::consume(&mut self.inner, item)
                .await
                .unwrap());
            // may unwrap because Err<!>
        }
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            let _ = BufferedConsumerLocalNb::flush(&mut self.inner).await;

            return Err(self.error.take().expect(
                "Do not close consume after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(ConsumerLocalNb::close(&mut self.inner, ()).await.unwrap());
            // may unwrap because Err<!>
        }
    }
}

impl<Item, Final, Error> BufferedConsumerLocalNb for TestConsumer<Item, Final, Error>
where
    Item: Copy,
{
    async fn flush(&mut self) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            let _ = BufferedConsumerLocalNb::flush(&mut self.inner).await;

            return Err(self.error.take().expect(
                "Do not call flush after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(BufferedConsumerLocalNb::flush(&mut self.inner)
                .await
                .unwrap());
            // may unwrap because Err<!>
        }
    }
}

impl<Item, Final, Error> BulkConsumerLocalNb for TestConsumer<Item, Final, Error>
where
    Item: Copy,
{
    async fn expose_slots<'a>(
        &'a mut self,
    ) -> Result<&'a mut [MaybeUninit<Self::Item>], Self::Error>
    where
        Self::Item: 'a,
    {
        if self.countdown_till_error == 0 {
            let _ = BufferedConsumerLocalNb::flush(&mut self.inner).await;

            return Err(self.error.take().expect(
                "Do not call consumer_slots after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(BulkConsumerLocalNb::expose_slots(&mut self.inner)
                .await
                .unwrap());
            // may unwrap because Err<!>
        }
    }

    async unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            let _ = BufferedConsumerLocalNb::flush(&mut self.inner).await;
            return Err(self.error.take().expect(
                "Do not call did_consume after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(BulkConsumerLocalNb::consume_slots(&mut self.inner, amount)
                .await
                .unwrap());
            // may unwrap because Err<!>
        }
    }
}

impl<'a, Item, Final, Error> Arbitrary<'a> for TestConsumer<Item, Final, Error>
where
    Item: Arbitrary<'a>,
    Error: Arbitrary<'a>,
{
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, arbitrary::Error> {
        let ops: ConsumeOperations = Arbitrary::arbitrary(u)?;
        let error: Error = Arbitrary::arbitrary(u)?;
        let countdown: usize = Arbitrary::arbitrary(u)?;
        let capacity: usize = Arbitrary::arbitrary(u)?;

        return Ok(TestConsumer {
            inner: Scramble::new(IntoVec::new(), ops, capacity.clamp(128, 512)),
            error: Some(error),
            countdown_till_error: countdown,
            phantom: PhantomData,
        });
    }
}
