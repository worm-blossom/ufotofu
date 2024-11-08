use core::cmp::min;
use core::fmt::Debug;
use core::num::NonZeroUsize;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{boxed::Box, vec::Vec};
#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec};
use wrapper::Wrapper;

use arbitrary::{size_hint, Arbitrary};
use either::Either;
use either::Either::Left;
use either::Either::Right;

use crate::common::producer::Invariant;
use crate::common::test_yielder::TestYielder;

use crate::local_nb::{
    BufferedProducer as BufferedProducerLocalNb, BulkProducer as BulkProducerLocalNb,
    Producer as ProducerLocalNb,
};
use crate::sync::producer::FromBoxedSlice;
use crate::sync::{BufferedProducer, BulkProducer, Producer};

/// If you need to test code that works with arbitrary producers, use this one. You can choose which items it emits, which final or or error value it emits, the size of the slices it presents with `expose_slots`, and when to its async functions should yield instead of returning immediately. Beyond manual control, the [`Arbitrary`] implementation lets you test against various producer behaviours automatically.
///
/// Create new [`TestProducer`](crate::common::producer::TestProducer)s either via a [`TestProducerBuilder`] or via the implementation of [`Arbitrary`].
///
/// ```
/// use std::convert::Infallible;
/// use either::Either::*;  
/// use ufotofu::sync::producer::*;
/// use ufotofu::sync::*;
///
/// let mut pro: TestProducer<u8, u16, Infallible> = TestProducerBuilder::new(vec![1, 2, 3].into(), Ok(9999)).build();
/// assert_eq!(Ok(Left(1)), pro.produce());
/// assert_eq!(Ok(Left(2)), pro.produce());
/// assert_eq!(Ok(Left(3)), pro.produce());
/// assert_eq!(Ok(Right(9999)), pro.produce());
/// ```
pub struct TestProducer_<Item, Final, Error>(Invariant<TestProducer<Item, Final, Error>>);

impl<Item, Final, Error> TestProducer_<Item, Final, Error> {
    /// Obtain a slice of all regular items that will be produced in the future.
    ///
    /// ```
    /// use std::convert::Infallible;
    /// use either::Either::*;  
    /// use ufotofu::sync::producer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut pro: TestProducer<u8, (), ()> = TestProducerBuilder::new(vec![1, 2, 3, 4].into(), Ok(())).build();
    /// assert_eq!(Ok(Left(1)), pro.produce());
    /// assert_eq!(Ok(Left(2)), pro.produce());
    /// assert_eq!(&[3, 4], pro.remaining());
    /// ```
    pub fn remaining(&self) -> &[Item] {
        self.0.as_ref().remaining()
    }

    /// Consume the [`TestProducer`](crate::common::producer::TestProducer) and obtain ownership of all values that it has or would have produced, including the last one.
    ///
    /// ```
    /// use std::convert::Infallible;
    /// use either::Either::*;  
    /// use ufotofu::sync::producer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut pro: TestProducer<u8, (), u16> = TestProducerBuilder::new(vec![1, 2, 3, 4].into(), Err(999)).build();
    /// assert_eq!(Ok(Left(1)), pro.produce());
    /// assert_eq!(Ok(Left(2)), pro.produce());
    /// assert_eq!((vec![1, 2, 3, 4].into(), Some(Err(999))), pro.into_data());
    /// ```
    pub fn into_data(self) -> (Box<[Item]>, Option<Result<Final, Error>>) {
        self.0.into_inner().into_data()
    }

    /// Obtain a reference to the last value that this will emit, either a `Final` value, or an `Error` value.
    ///
    /// ```
    /// use std::convert::Infallible;
    /// use either::Either::*;  
    /// use ufotofu::sync::producer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut pro: TestProducer<u8, u16, Infallible> = TestProducerBuilder::new(vec![1, 2, 3].into(), Ok(9999)).build();
    /// assert_eq!(Ok(Left(1)), pro.produce());
    /// assert_eq!(Some(&Ok(9999)), pro.peek_last());
    /// ```
    pub fn peek_last(&self) -> Option<&Result<Final, Error>> {
        self.0.as_ref().peek_last()
    }

    /// Return whether the last value (a `Final` value or an `Error`) was already emitted.
    ///
    /// ```
    /// use std::convert::Infallible;
    /// use either::Either::*;  
    /// use ufotofu::sync::producer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut pro: TestProducer<u8, u16, Infallible> = TestProducerBuilder::new(vec![1].into(), Ok(9999)).build();
    /// assert_eq!(false, pro.did_already_emit_last());
    /// assert_eq!(Ok(Left(1)), pro.produce());
    /// assert_eq!(false, pro.did_already_emit_last());
    /// assert_eq!(Ok(Right(9999)), pro.produce());
    /// assert_eq!(true, pro.did_already_emit_last());
    /// ```
    pub fn did_already_emit_last(&self) -> bool {
        self.0.as_ref().did_already_emit_last()
    }
}

invarianted_impl_debug!(TestProducer_<Item: Debug, Final: Debug, Error: Debug>);

invarianted_impl_producer_sync_and_local_nb!(TestProducer_<Item: Copy + Default, Final, Error> Item Item; Final Final; Error Error);
invarianted_impl_buffered_producer_sync_and_local_nb!(TestProducer_<Item: Copy + Default, Final, Error>);
invarianted_impl_bulk_producer_sync_and_local_nb!(TestProducer_<Item: Copy + Default, Final, Error>);

impl<'a, Item: Arbitrary<'a>, Final: Arbitrary<'a>, Error: Arbitrary<'a>> Arbitrary<'a>
    for TestProducer_<Item, Final, Error>
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let items = Box::<[Item]>::arbitrary(u)?;
        let last = Result::<Final, Error>::arbitrary(u)?;
        let slot_sizes = Box::<[NonZeroUsize]>::arbitrary(u)?;
        let yield_pattern = Box::<[bool]>::arbitrary(u)?;

        let ret = TestProducerBuilder::new(items, last)
            .exposed_items_sizes(slot_sizes)
            .yield_pattern(yield_pattern)
            .build();
        Ok(ret)
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        size_hint::and_all(&[
            Error::size_hint(depth),
            usize::size_hint(depth),
            Box::<[NonZeroUsize]>::size_hint(depth),
            Box::<[bool]>::size_hint(depth),
        ])
    }
}

/// A [builder](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html) for [`TestProducer`](crate::common::producer::TestProducer).
///
/// ```
/// use std::convert::Infallible;
/// use either::Either::*;  
/// use ufotofu::sync::producer::*;
/// use ufotofu::sync::*;
///
/// let mut pro: TestProducer<u8, u16, Infallible> = TestProducerBuilder::new(vec![1, 2, 3].into(), Ok(9999)).build();
/// assert_eq!(Ok(Left(1)), pro.produce());
/// assert_eq!(Ok(Left(2)), pro.produce());
/// assert_eq!(Ok(Left(3)), pro.produce());
/// assert_eq!(Ok(Right(9999)), pro.produce());
/// ```
pub struct TestProducerBuilder<Item, Final, Error> {
    items: Box<[Item]>,
    last: Result<Final, Error>,
    exposed_items_sizes: Option<Box<[NonZeroUsize]>>,
    yield_pattern: Option<Box<[bool]>>,
}

impl<Item, Final, Error> TestProducerBuilder<Item, Final, Error> {
    /// Create a new [`TestProducerBuilder`].
    ///
    /// The resulting producer will succesfully produce the given `items` before emitting the given `last` value (either a final value or an error).
    ///
    /// ```
    /// use std::convert::Infallible;
    /// use either::Either::*;  
    /// use ufotofu::sync::producer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut pro: TestProducer<u8, u16, Infallible> = TestProducerBuilder::new(vec![1, 2, 3].into(), Ok(9999)).build();
    /// assert_eq!(Ok(Left(1)), pro.produce());
    /// assert_eq!(Ok(Left(2)), pro.produce());
    /// assert_eq!(Ok(Left(3)), pro.produce());
    /// assert_eq!(Ok(Right(9999)), pro.produce());
    /// ```
    pub fn new(items: Box<[Item]>, last: Result<Final, Error>) -> Self {
        TestProducerBuilder {
            items,
            last,
            exposed_items_sizes: None,
            yield_pattern: None,
        }
    }

    /// Set a pattern of upper bounds to the slice sizes that the [`BulkProducer::expose_items`] method returns. The consumer will cycle through the supplied upper bounds. Up to a size of 2048, the supplied sizes will also act as lower bounds, i.e., [`BulkProducer::expose_items`] will return slices of the exact sizes supplied here, unless it exceeds 2048.
    ///
    /// An empty slice will be ignored.
    ///
    /// Will be ignored by non-bulk producers.
    ///
    /// ```
    /// use std::convert::Infallible;
    /// use either::Either::*;  
    /// use ufotofu::sync::producer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut pro: TestProducer<u8, u16, Infallible> = TestProducerBuilder::new(vec![1, 2, 3, 4].into(), Ok(9999))
    ///     .exposed_items_sizes(vec![2.try_into().unwrap(), 1.try_into().unwrap()].into())
    ///     .build();
    /// assert_eq!(2, pro.expose_items().unwrap().unwrap_left().len());
    /// assert_eq!(1, pro.expose_items().unwrap().unwrap_left().len());
    /// assert_eq!(2, pro.expose_items().unwrap().unwrap_left().len());
    /// ```
    pub fn exposed_items_sizes(mut self, sizes: Box<[NonZeroUsize]>) -> Self {
        if sizes.len() > 0 {
            self.exposed_items_sizes = Some(sizes);
        } else {
            self.exposed_items_sizes = None;
        }

        self
    }

    /// Set a pattern of whether to immediately complete asynchronous action, or to yield back to the task executor first.
    ///
    /// If all booleans are `true`, a single `false` will be appended (otherwise, the producer would never complete its operations).
    ///
    /// Will be ignored by non-nonblocking consumers.
    pub fn yield_pattern(mut self, pattern: Box<[bool]>) -> Self {
        if pattern.iter().all(|b| *b) {
            // This also handles empty patterns.
            let mut pat = Vec::with_capacity(pattern.len() + 1);
            pat.extend_from_slice(&pattern);
            pat.push(false);
            self.yield_pattern = Some(pat.into_boxed_slice());
        } else {
            self.yield_pattern = Some(pattern);
        }

        self
    }

    /// Create a fully configured [`TestProducer`](crate::common::consumer::TestProducer).
    ///
    /// ```
    /// use std::convert::Infallible;
    /// use either::Either::*;  
    /// use ufotofu::sync::producer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut pro: TestProducer<u8, u16, Infallible> = TestProducerBuilder::new(vec![1, 2, 3].into(), Ok(9999)).build();
    /// assert_eq!(Ok(Left(1)), pro.produce());
    /// assert_eq!(Ok(Left(2)), pro.produce());
    /// assert_eq!(Ok(Left(3)), pro.produce());
    /// assert_eq!(Ok(Right(9999)), pro.produce());
    /// ```
    pub fn build(self) -> TestProducer_<Item, Final, Error> {
        TestProducer_(Invariant::new(TestProducer {
            inner: FromBoxedSlice::new(self.items),
            last: Some(self.last),
            exposed_items_sizes: self.exposed_items_sizes.map(|sizes| (sizes, 0)),
            yielder: self.yield_pattern.map(TestYielder::new),
        }))
    }
}

#[derive(Debug)]
struct TestProducer<Item, Final, Error> {
    inner: FromBoxedSlice<Item>,
    last: Option<Result<Final, Error>>,
    exposed_items_sizes: Option<(Box<[NonZeroUsize]>, usize /* current index*/)>,
    yielder: Option<TestYielder>,
}

impl<Item, Final, Error> TestProducer<Item, Final, Error> {
    fn remaining(&self) -> &[Item] {
        self.inner.remaining()
    }

    fn into_data(self) -> (Box<[Item]>, Option<Result<Final, Error>>) {
        (self.inner.into_inner(), self.last)
    }

    fn peek_last(&self) -> Option<&Result<Final, Error>> {
        self.last.as_ref()
    }

    pub fn did_already_emit_last(&self) -> bool {
        self.last.is_none()
    }

    async fn maybe_yield(&mut self) {
        match &mut self.yielder {
            None => (),
            Some(yielder) => yielder.maybe_yield().await,
        }
    }
}

impl<Item: Clone, Final, Error> Producer for TestProducer<Item, Final, Error> {
    type Item = Item;
    type Final = Final;
    type Error = Error;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        if self.inner.remaining().len() == 0 {
            return Ok(Right(self.last.take().unwrap()?));
        } else {
            return Ok(Left(
                Producer::produce(&mut self.inner).unwrap().unwrap_left(),
            ));
        }
    }
}

impl<Item: Clone, Final, Error> BufferedProducer for TestProducer<Item, Final, Error> {
    fn slurp(&mut self) -> Result<(), Self::Error> {
        if self.inner.remaining().len() == 0 && self.last.as_ref().unwrap().is_err() {
            let last_owned = self.last.take().unwrap();
            match last_owned {
                Ok(_) => unreachable!(),
                Err(err) => return Err(err),
            }
        }

        return Ok(());
    }
}

impl<Item, Final, Error> BulkProducer for TestProducer<Item, Final, Error>
where
    Item: Copy,
{
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        if self.inner.remaining().len() == 0 {
            let last_owned = self.last.take().unwrap();
            match last_owned {
                Ok(fin) => return Ok(Right(fin)),
                Err(err) => return Err(err),
            }
        } else {
            match self.exposed_items_sizes {
                None => {
                    return Ok(Left(
                        BulkProducer::expose_items(&mut self.inner)
                            .unwrap()
                            .unwrap_left(),
                    ));
                }
                Some((ref exposed_item_sizes, ref mut index)) => {
                    let max_len: usize = exposed_item_sizes[*index].into();
                    *index = (*index + 1) % exposed_item_sizes.len();

                    let inner_slots = BulkProducer::expose_items(&mut self.inner)
                        .unwrap()
                        .unwrap_left(); // may unwrap because Err<!>
                    let actual_len = min(inner_slots.len(), max_len);

                    Ok(Left(&inner_slots[..actual_len]))
                }
            }
        }
    }

    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        if self.inner.remaining().len() == 0 && self.last.as_ref().unwrap().is_err() {
            let last_owned = self.last.take().unwrap();
            match last_owned {
                Ok(_) => unreachable!(),
                Err(err) => return Err(err),
            }
        }

        return Ok(BulkProducer::consider_produced(&mut self.inner, amount).unwrap());
    }
}

impl<Item, Final, Error> ProducerLocalNb for TestProducer<Item, Final, Error>
where
    Item: Clone,
{
    type Item = Item;
    type Final = Final;
    type Error = Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.maybe_yield().await;
        Producer::produce(self)
    }
}

impl<Item, Final, Error> BufferedProducerLocalNb for TestProducer<Item, Final, Error>
where
    Item: Clone,
{
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        self.maybe_yield().await;
        BufferedProducer::slurp(self)
    }
}

impl<Item, Final, Error> BulkProducerLocalNb for TestProducer<Item, Final, Error>
where
    Item: Copy,
{
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'a,
    {
        self.maybe_yield().await;
        BulkProducer::expose_items(self)
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.maybe_yield().await;
        BulkProducer::consider_produced(self, amount)
    }
}
