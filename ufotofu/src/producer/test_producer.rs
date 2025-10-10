use core::cmp::min;

use alloc::{boxed::Box, vec::Vec};

use arbitrary::{size_hint, Arbitrary};
use derive_builder::Builder;

use crate::{
    prelude::*,
    producer::{clone_from_owned_slice, CloneFromOwnedSlice},
    test_yielder::TestYielder,
};

pub fn build_test_producer<Item, Final, Error>() -> TestProducerBuilder<Item, Final, Error>
where
    Item: Clone,
    Final: Clone,
    Error: Clone,
{
    TestProducerBuilder::create_empty()
}

impl<Item, Final, Error> TestProducerBuilder<Item, Final, Error> {
    fn items<VALUE: Into<Vec<Item>>>(&mut self, value: VALUE) -> &mut Self {
        let new = self;
        new.inner = Some(clone_from_owned_slice(value.into()));
        new
    }

    fn fin<VALUE: Into<Final>>(&mut self, value: VALUE) -> &mut Self {
        let new = self;
        new.last = Some(Some(Ok(value.into())));
        new
    }

    fn err<VALUE: Into<Error>>(&mut self, value: VALUE) -> &mut Self {
        let new = self;
        new.last = Some(Some(Err(value.into())));
        new
    }

    fn exposed_items_sizes<VALUE: Into<Vec<usize>>>(&mut self, value: VALUE) -> &mut Self {
        let mut the_sizes: Vec<usize> = value.into().into_iter().filter(|size| *size > 0).collect();

        if the_sizes.is_empty() {
            the_sizes.push(usize::MAX);
        }

        let new = self;
        new.exposed_items_sizes = Some(the_sizes.into_boxed_slice());
        new
    }

    fn yield_pattern<VALUE: Into<Vec<bool>>>(&mut self, value: VALUE) -> &mut Self {
        let new = self;
        new.yielder = Some(TestYielder::new(value.into().into_boxed_slice()));
        new
    }
}

#[derive(Debug, Clone, Builder)]
#[builder(no_std)]
pub struct TestProducer<Item, Final, Error> {
    #[builder(default = "clone_from_owned_slice(alloc::vec![])")]
    #[builder(setter(custom))]
    inner: CloneFromOwnedSlice<Vec<Item>, Item>,
    #[builder(setter(strip_option))]
    last: Option<Result<Final, Error>>,
    #[builder(default = "alloc::vec![usize::MAX].into_boxed_slice()")]
    #[builder(setter(custom))]
    exposed_items_sizes: Box<[usize]>,
    #[builder(setter(skip))]
    exposed_item_sizes_index: usize,
    #[builder(default)]
    #[builder(setter(custom))]
    yielder: TestYielder,
}

impl<Item, Final, Error> TestProducer<Item, Final, Error> {
    pub fn as_slice(&self) -> &[Item] {
        self.inner.remaining()
    }

    pub fn peek_last(&self) -> Option<&Result<Final, Error>> {
        self.last.as_ref()
    }

    pub fn did_already_emit_last(&self) -> bool {
        self.last.is_none()
    }

    async fn maybe_yield(&mut self) {
        self.yielder.maybe_yield().await
    }
}

impl<Item: Clone, Final, Error> Producer for TestProducer<Item, Final, Error> {
    type Item = Item;
    type Final = Final;
    type Error = Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.maybe_yield().await;

        if self.inner.remaining().is_empty() {
            Ok(Right(self.last.take().unwrap()?))
        } else {
            Ok(Left(
                Producer::produce(&mut self.inner)
                    .await
                    .unwrap()
                    .unwrap_left(),
            ))
        }
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.maybe_yield().await;

        if self.inner.remaining().is_empty() && self.last.as_ref().unwrap().is_err() {
            let last_owned = self.last.take().unwrap();
            match last_owned {
                Ok(_) => unreachable!(),
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }
}

impl<Item: Clone, Final, Error> BulkProducer for TestProducer<Item, Final, Error> {
    async fn expose_items<F, R>(&mut self, f: F) -> Result<Either<R, Self::Final>, Self::Error>
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, R),
    {
        self.maybe_yield().await;

        match self
            .inner
            .expose_items(async |inner_items| {
                let inner_items_len = inner_items.len();

                let max_len: usize = self.exposed_items_sizes[self.exposed_item_sizes_index].into();
                self.exposed_item_sizes_index =
                    (self.exposed_item_sizes_index + 1) % self.exposed_items_sizes.len();

                f(&inner_items[..min(inner_items_len, max_len)]).await
            })
            .await
        {
            Ok(Left(yay)) => Ok(Left(yay)),
            Ok(Right(())) => Ok(Right(self.last.take().unwrap()?)),
            Err(_err) => unreachable!(),
        }
    }
}

impl<'a, Item: Arbitrary<'a>, Final: Arbitrary<'a>, Error: Arbitrary<'a>> Arbitrary<'a>
    for TestProducer<Item, Final, Error>
where
    Item: Clone,
    Final: Clone,
    Error: Clone,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let items = Vec::<Item>::arbitrary(u)?;
        let last = Result::<Final, Error>::arbitrary(u)?;
        let slot_sizes = Vec::<usize>::arbitrary(u)?;
        let yield_pattern = Vec::<bool>::arbitrary(u)?;

        let ret = build_test_producer()
            .items(items)
            .last(last)
            .exposed_items_sizes(slot_sizes)
            .yield_pattern(yield_pattern)
            .build();

        Ok(ret.unwrap())
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        size_hint::and_all(&[
            Vec::<Item>::size_hint(depth),
            Result::<Final, Error>::size_hint(depth),
            Vec::<usize>::size_hint(depth),
            Vec::<bool>::size_hint(depth),
        ])
    }
}

/// This implementation considers only the values that have been or will be emitted (regular, final, and error). That is, two `TestProducer`s are considered equal if they will produce equal sequences of values, irrespective of the details of how many item slots they will expose with each `expose_items` call, and irrespective of the pattern in which the async methods yield.
///
/// [TODO] doc test
impl<Item: PartialEq, Final: PartialEq, Error: PartialEq> PartialEq
    for TestProducer<Item, Final, Error>
{
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice() && self.peek_last() == other.peek_last()
    }
}
