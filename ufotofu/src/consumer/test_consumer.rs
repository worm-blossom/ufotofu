use core::cmp::min;

use alloc::{boxed::Box, vec::Vec};

use arbitrary::{size_hint, Arbitrary};
use derive_builder::Builder;

use crate::{consumer::compat, prelude::*, test_yielder::TestYielder};

pub fn build_test_consumer<Item, Final, Error>() -> TestConsumerBuilder<Item, Final, Error>
where
    Item: Clone,
    Final: Clone,
    Error: Clone,
{
    TestConsumerBuilder::create_empty()
}

impl<Item, Final, Error> TestConsumerBuilder<Item, Final, Error> {
    pub fn exposed_slots_sizes<VALUE: Into<Vec<usize>>>(&mut self, value: VALUE) -> &mut Self {
        let mut the_sizes: Vec<usize> = value.into().into_iter().filter(|size| *size > 0).collect();

        if the_sizes.is_empty() {
            the_sizes.push(usize::MAX);
        }

        let new = self;
        new.exposed_slots_sizes = Some(the_sizes.into_boxed_slice());
        new
    }

    pub fn yield_pattern<VALUE: Into<Vec<bool>>>(&mut self, value: VALUE) -> &mut Self {
        let new = self;
        new.yielder = Some(TestYielder::new(value.into().into_boxed_slice()));
        new
    }
}

#[derive(Debug, Clone, Builder)]
#[builder(no_std)]
pub struct TestConsumer<Item, Final, Error> {
    #[builder(default = "alloc::vec![].into_consumer()")]
    #[builder(setter(skip))]
    inner: compat::vec::IntoConsumer<Item>,
    #[builder(setter(skip))]
    fin: Option<Final>,
    #[builder(setter(strip_option))]
    err: Option<Error>,
    #[builder(default)]
    consumptions_until_error: usize,
    #[builder(default = "alloc::vec![usize::MAX].into_boxed_slice()")]
    #[builder(setter(custom))]
    exposed_slots_sizes: Box<[usize]>,
    #[builder(setter(skip))]
    exposed_slots_sizes_index: usize,
    #[builder(default)]
    #[builder(setter(custom))]
    yielder: TestYielder,
}

impl<Item, Final, Error> TestConsumer<Item, Final, Error> {
    pub fn as_slice(&self) -> &[Item] {
        self.inner.as_slice()
    }

    pub fn peek_final(&self) -> Option<&Final> {
        self.fin.as_ref()
    }

    pub fn into_consumed(self) -> (Vec<Item>, Option<Final>) {
        (self.inner.into(), self.fin)
    }

    pub fn peek_error(&self) -> Option<&Error> {
        self.err.as_ref()
    }

    pub fn did_error(&self) -> bool {
        self.err.is_none()
    }

    fn check_error(&mut self) -> Result<(), Error> {
        if self.consumptions_until_error == 0 {
            Err(self
                .err
                .take()
                .expect("Must not call Consumer methods after the consumer has emitted an error."))
        } else {
            Ok(())
        }
    }

    async fn maybe_yield(&mut self) {
        self.yielder.maybe_yield().await
    }
}

impl<Item, Final, Error> Consumer for TestConsumer<Item, Final, Error> {
    type Item = Item;
    type Final = Final;
    type Error = Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.maybe_yield().await;
        self.check_error()?;

        Consumer::consume(&mut self.inner, item).await.unwrap(); // may unwrap because Err<!>
        self.consumptions_until_error -= 1;
        Ok(())
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        self.maybe_yield().await;
        self.check_error()?;
        self.fin = Some(fin);

        Ok(())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.maybe_yield().await;
        self.check_error()?;

        Consumer::flush(&mut self.inner).await.unwrap(); // may unwrap because Err<!>
        Ok(())
    }
}

impl<Item, Final, Error> BulkConsumer for TestConsumer<Item, Final, Error>
where
    Item: Default,
{
    async fn expose_slots<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: AsyncFnOnce(&mut [Self::Item]) -> (usize, R),
    {
        self.maybe_yield().await;
        self.check_error()?;

        let max_len: usize = self.exposed_slots_sizes[self.exposed_slots_sizes_index].into();
        self.exposed_slots_sizes_index =
            (self.exposed_slots_sizes_index + 1) % self.exposed_slots_sizes.len();

        self.inner.prepare_slots(max_len);

        self.inner
            .expose_slots(async |inner_slots| {
                let inner_len = inner_slots.len();
                f(&mut inner_slots[..min(inner_len, max_len)]).await
            })
            .await
            .map_err(|_| unreachable!("Inner consumer is infallible"))
    }
}

impl<'a, Item: Arbitrary<'a>, Final: Arbitrary<'a>, Error: Arbitrary<'a>> Arbitrary<'a>
    for TestConsumer<Item, Final, Error>
where
    Item: Clone,
    Final: Clone,
    Error: Clone,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let err = Error::arbitrary(u)?;
        let consumptions_until_error = usize::arbitrary(u)?;
        let slot_sizes = Vec::<usize>::arbitrary(u)?;
        let yield_pattern = Vec::<bool>::arbitrary(u)?;

        let ret = build_test_consumer()
            .err(err)
            .consumptions_until_error(consumptions_until_error)
            .exposed_slots_sizes(slot_sizes)
            .yield_pattern(yield_pattern)
            .build();

        Ok(ret.unwrap())
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        size_hint::and_all(&[
            Error::size_hint(depth),
            usize::size_hint(depth),
            Vec::<usize>::size_hint(depth),
            Vec::<bool>::size_hint(depth),
        ])
    }
}

/// This implementation considers only the values that have been consumed so far. That is, two `TestConsumer`s are considered equal if they were supplied with equal items and final values (or no final value for both), irrespective of the details of how many item slots they had exposed with their `expose_items` calls, and irrespective of the pattern in which their async methods have yielded. In particular, it does not matter which future behaviour they will display either.
///
/// [TODO] doc test
impl<Item: PartialEq, Final: PartialEq, Error: PartialEq> PartialEq
    for TestConsumer<Item, Final, Error>
{
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice() && self.peek_final() == other.peek_final()
    }
}
