use core::marker::PhantomData;

use alloc::vec::Vec;
use arbitrary::{Arbitrary, Unstructured};
use wrapper::Wrapper;

use crate::local_nb::consumer::{IntoVec, Scramble};
use crate::local_nb::{BufferedConsumer, BulkConsumer, Consumer};

use super::ConsumeOperations;

/// A consumer for fuzz testing purposes. You can only construct this via its `Arbitrary` implementation. It successfully operates, until it decides to error after a random number of operations.
///
/// Use the `Wrapper::into_inner` implementation to obtain a `Vec` of all consumed items.
#[derive(Debug)]
pub struct TestConsumer<Item, Final, Error> {
    inner: Scramble<IntoVec<Item>, Item, (), !>,
    error: Option<Error>,
    countdown_till_error: usize,
    phantom: PhantomData<Final>,
}

impl<Item, Final, Error> TestConsumer<Item, Final, Error> {
    pub fn as_ref(&self) -> &[Item] {
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

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            return Err(self.error.take().expect(
                "Do not call consume after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(self.inner.consume(item).await.unwrap()); // may unwrap because Err<!>
        }
    }

    async fn close(&mut self, _f: Self::Final) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            return Err(self.error.take().expect(
                "Do not close consume after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(self.inner.close(()).await.unwrap()); // may unwrap because Err<!>
        }
    }
}

impl<Item, Final, Error> BufferedConsumer for TestConsumer<Item, Final, Error>
where
    Item: Copy,
{
    async fn flush(&mut self) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            return Err(self.error.take().expect(
                "Do not call flush after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(self.inner.flush().await.unwrap()); // may unwrap because Err<!>
        }
    }
}

impl<Item, Final, Error> BulkConsumer for TestConsumer<Item, Final, Error>
where
    Item: Copy,
{
    async fn expose_slots<'a>(
        &'a mut self,
    ) -> Result<&'a mut [core::mem::MaybeUninit<Self::Item>], Self::Error>
    where
        Item: 'a,
    {
        if self.countdown_till_error == 0 {
            return Err(self.error.take().expect(
                "Do not call consumer_slots after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(self.inner.expose_slots().await.unwrap()); // may unwrap because Err<!>
        }
    }

    async unsafe fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        if self.countdown_till_error == 0 {
            return Err(self.error.take().expect(
                "Do not call did_consume after close or after any trait function has caused an error.",
            ));
        } else {
            self.countdown_till_error -= 1;
            return Ok(self.inner.consume_slots(amount).await.unwrap()); // may unwrap because Err<!>
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
