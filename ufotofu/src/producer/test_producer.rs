use core::cmp::min;

use alloc::{boxed::Box, vec::Vec};

use arbitrary::{size_hint, Arbitrary};
use derive_builder::Builder;

use crate::{
    prelude::*,
    producer::{clone_from_owned_slice, CloneFromOwnedSlice},
    test_yielder::TestYielder,
};

/// Returns a [`TestProducerBuilder`] for building a producer with fully configurable observable behaviour.
///
/// See the [fuzz-testing tutorial](crate::fuzz_testing_tutorial) for typical usage.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut p = build_test_producer::<u32, char, Infallible>()
///     .items(vec![1, 2, 4])
///     .fin('z')
///     .build().unwrap();
///
/// assert_eq!(p.produce().await?, Left(1));
/// assert_eq!(p.produce().await?, Left(2));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Right('z'));
///                 
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [`consumer::build_test_consumer`] function.
pub fn build_test_producer<Item, Final, Error>() -> TestProducerBuilder<Item, Final, Error>
where
    Item: Clone,
    Final: Clone,
    Error: Clone,
{
    TestProducerBuilder::create_empty()
}

impl<Item, Final, Error> TestProducerBuilder<Item, Final, Error> {
    /// Configures the built [`TestProducer`] to emit the given items before its last value.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, char, Infallible>()
    ///     .items(vec![1, 2, 4])
    ///     .fin('z')
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.produce().await?, Left(1));
    /// assert_eq!(p.produce().await?, Left(2));
    /// assert_eq!(p.produce().await?, Left(4));
    /// assert_eq!(p.produce().await?, Right('z'));
    ///
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// If you do not call this method, the built producer will emit zero items (i.e., it will immediately yield its final value or error).
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, char, Infallible>()
    ///     .fin('z')
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.produce().await?, Right('z'));
    ///
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn items<VALUE: Into<Vec<Item>>>(&mut self, value: VALUE) -> &mut Self {
        let new = self;
        new.inner = Some(clone_from_owned_slice(value.into()));
        new
    }

    /// Configures the built [`TestProducer`] to emit the given final value after its regular items.
    ///
    /// Calling `builder.fin(fin)` is equivalent to calling `builder.last(Ok(fin))`.
    ///
    /// Building will fail if you called neither [`TestProducerBuilder::last`] nor [`TestProducerBuilder::fin`] nor [`TestProducerBuilder::err`].
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, char, Infallible>()
    ///     .items(vec![1])
    ///     .fin('z')
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.produce().await?, Left(1));
    /// assert_eq!(p.produce().await?, Right('z'));
    ///
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    pub fn fin<VALUE: Into<Final>>(&mut self, value: VALUE) -> &mut Self {
        let new = self;
        new.last = Some(Some(Ok(value.into())));
        new
    }

    /// Configures the built [`TestProducer`] to emit the given error after its regular items.
    ///
    /// Calling `builder.err(err)` is equivalent to calling `builder.last(Err(err))`.
    ///
    /// Building will fail if you called neither [`TestProducerBuilder::last`] nor [`TestProducerBuilder::fin`] nor [`TestProducerBuilder::err`].
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, Infallible, char>()
    ///     .items(vec![1])
    ///     .err('z')
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.produce().await?, Left(1));
    /// assert_eq!(p.produce().await, Err('z'));
    ///
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`TestConsumerBuilder::err`](consumer::TestConsumerBuilder::err) method.
    pub fn err<VALUE: Into<Error>>(&mut self, value: VALUE) -> &mut Self {
        let new = self;
        new.last = Some(Some(Err(value.into())));
        new
    }

    /// Configures the number of items the built [`TestProducer`] will expose on each call to `expose_items`; the built producer will cycle through this vec of sizes. The exposed slices will be shorter when not enough items remain to be produced.
    ///
    /// Entries of `0` will be ignored. If all entries are zero, a single `usize::MAX` is used as the pattern.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, char, Infallible>()
    ///     .items(vec![1, 2, 4])
    ///     .fin('z')
    ///     .exposed_items_sizes(vec![1, 2])
    ///     .build().unwrap();
    ///
    /// // Three items remain, the pattern starts with `1`, so one item is exposed.
    /// p.expose_items(async |items| {
    ///     assert_eq!(items.len(), 1);
    ///     (0, ()) // Report back that zero items were produced.
    /// }).await?;
    ///
    /// // Three items remain, the pattern continues with `2`, so two items are exposed.
    /// p.expose_items(async |items| {
    ///     assert_eq!(items.len(), 2);
    ///     (0, ()) // Report back that zero items were produced.
    /// }).await?;
    ///
    /// // Three items remain, the pattern loops back to its start, so one item is exposed.
    /// p.expose_items(async |items| {
    ///     assert_eq!(items.len(), 1);
    ///     (1, ()) // Report back that one item was produced, but whatever, the example ends here.
    /// }).await?;
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// If you do not call this method, the built producer will always expose all remaining items.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, char, Infallible>()
    ///     .items(vec![1, 2, 4])
    ///     .fin('z')
    ///     .build().unwrap();
    ///
    /// // Three items remain, all are exposed.
    /// p.expose_items(async |items| {
    ///     assert_eq!(items.len(), 3);
    ///     (1, ()) // Report back that one item was produced.
    /// }).await?;
    ///
    /// // Two items remain, all are exposed.
    /// p.expose_items(async |items| {
    ///     assert_eq!(items.len(), 2);
    ///     (1, ()) // Report back that one item was produced, but whatever, the example ends here.
    /// }).await?;
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`TestConsumerBuilder::exposed_slots_sizes`](consumer::TestConsumerBuilder::exposed_slots_sizes) method.
    pub fn exposed_items_sizes<VALUE: Into<Vec<usize>>>(&mut self, value: VALUE) -> &mut Self {
        let mut the_sizes: Vec<usize> = value.into().into_iter().filter(|size| *size > 0).collect();

        if the_sizes.is_empty() {
            the_sizes.push(usize::MAX);
        }

        let new = self;
        new.exposed_items_sizes = Some(the_sizes.into_boxed_slice());
        new
    }

    /// Sets a pattern to control whether the built [`TestProducer`] will immediately complete asynchronous methods, or whether it will yield back to the task executor first.
    ///
    /// If you do not call this method, the built producer will complete all its methods immediately without unnecessary yielding.
    ///
    /// If all booleans are `true`, a single `false` is automatically appended (otherwise, the producer would always yield and never progress).
    ///
    /// <br/>Counterpart: the [`TestConsumerBuilder::yield_pattern`](consumer::TestConsumerBuilder::yield_pattern) method.
    pub fn yield_pattern<VALUE: Into<Vec<bool>>>(&mut self, value: VALUE) -> &mut Self {
        let new = self;
        new.yielder = Some(TestYielder::new(value.into().into_boxed_slice()));
        new
    }
}

#[derive(Debug, Clone, Builder)]
#[builder(no_std)]
/// A producer with fully configurable observable behaviour, intended for testing other code.
///
/// See the [fuzz-testing tutorial](crate::fuzz_testing_tutorial) for typical usage.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut p = build_test_producer::<u32, char, Infallible>()
///     .items(vec![1, 2, 4])
///     .fin('z')
///     .build().unwrap();
///
/// assert_eq!(p.produce().await?, Left(1));
/// assert_eq!(p.produce().await?, Left(2));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Right('z'));
///                 
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [`TestConsumer`](consumer::TestConsumer) type.
pub struct TestProducer<Item, Final, Error> {
    #[builder(default = "clone_from_owned_slice(alloc::vec![])")]
    #[builder(setter(custom))]
    inner: CloneFromOwnedSlice<Vec<Item>, Item>,
    /// Configures the built [`TestProducer`] to emit the given last value after its regular items, either as a final value (when called with an `Ok`) or as an error (when called with an `Err`).
    ///
    /// Building will fail if you called neither [`TestProducerBuilder::last`] nor [`TestProducerBuilder::fin`] nor [`TestProducerBuilder::err`].
    ///
    /// Example with an `Ok` value.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, char, Infallible>()
    ///     .items(vec![1])
    ///     .last(Ok('z'))
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.produce().await?, Left(1));
    /// assert_eq!(p.produce().await?, Right('z'));
    ///
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// Example with an `Err` value.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, Infallible, char>()
    ///     .items(vec![1])
    ///     .err('z')
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.produce().await?, Left(1));
    /// assert_eq!(p.produce().await, Err('z'));
    ///
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
    #[builder(setter(strip_option))]
    last: Option<Result<Final, Error>>,
    #[builder(default = "alloc::vec![usize::MAX].into_boxed_slice()")]
    #[builder(setter(custom))]
    exposed_items_sizes: Box<[usize]>,
    #[builder(setter(skip))]
    exposed_items_sizes_index: usize,
    #[builder(default)]
    #[builder(setter(custom))]
    yielder: TestYielder,
}

impl<Item, Final, Error> TestProducer<Item, Final, Error> {
    /// Returns the regular items the producer will still produce.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, char, Infallible>()
    ///     .items(vec![1, 2, 4])
    ///     .fin('z')
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.as_slice(), &[1, 2, 4]);
    /// assert_eq!(p.produce().await?, Left(1));
    /// assert_eq!(p.as_slice(), &[2, 4]);
    /// assert_eq!(p.produce().await?, Left(2));
    /// assert_eq!(p.as_slice(), &[4]);
    /// assert_eq!(p.produce().await?, Left(4));
    /// assert_eq!(p.as_slice(), &[]);
    /// assert_eq!(p.produce().await?, Right('z'));
    /// assert_eq!(p.as_slice(), &[]);              
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`TestConsumer::as_slice`](consumer::TestConsumer::as_slice) method.
    pub fn as_slice(&self) -> &[Item] {
        self.inner.remaining()
    }

    /// Returns the regular items the producer has already produced.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, char, Infallible>()
    ///     .items(vec![1, 2, 4])
    ///     .fin('z')
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.already_produced(), &[]);
    /// assert_eq!(p.produce().await?, Left(1));
    /// assert_eq!(p.already_produced(), &[1]);
    /// assert_eq!(p.produce().await?, Left(2));
    /// assert_eq!(p.already_produced(), &[1, 2]);
    /// assert_eq!(p.produce().await?, Left(4));
    /// assert_eq!(p.already_produced(), &[1, 2, 4]);
    /// assert_eq!(p.produce().await?, Right('z'));
    /// assert_eq!(p.already_produced(), &[1,2, 4]);              
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: none. The counterpart on [`TestConsumer`](consumer::TestConsumer) would be a method returning a slice of all items which have not yet been consumed but will be. Good luck implementing that.
    pub fn already_produced(&self) -> &[Item] {
        self.inner.produced()
    }

    /// Returns a reference to the last value the producer will produce, or `None` if it has already been produced.
    ///
    /// An example returning a `Some(Ok(_))` (because the producer was configured to return a *final* value):
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, char, Infallible>()
    ///     .fin('z')
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.peek_last(), Some(&Ok('z')));
    /// assert_eq!(p.produce().await?, Right('z'));
    /// assert_eq!(p.peek_last(), None);           
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// An example returning a `Some(Err(_))` (because the producer was configured to return an *error*):
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, Infallible, char>()
    ///     .err('z')
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.peek_last(), Some(&Err('z')));
    /// assert_eq!(p.produce().await, Err('z'));
    /// assert_eq!(p.peek_last(), None);           
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterparts: the [`TestConsumer::peek_final`](consumer::TestConsumer::peek_final) and [`TestConsumer::peek_error`](consumer::TestConsumer::peek_error) methods.
    pub fn peek_last(&self) -> Option<&Result<Final, Error>> {
        self.last.as_ref()
    }

    /// Returns whether the producer has produced its last value already.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, char, Infallible>()
    ///     .fin('z')
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.did_already_emit_last(), false);
    /// assert_eq!(p.produce().await?, Right('z'));
    /// assert_eq!(p.did_already_emit_last(), true);          
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`TestConsumer::is_closed`](consumer::TestConsumer::is_closed) method if this emitted its final value, the [`TestConsumer::did_already_error`](consumer::TestConsumer::did_already_error) method if this emitted an error.
    pub fn did_already_emit_last(&self) -> bool {
        self.last.is_none()
    }

    /// Drops the producer and returns ownership of the items it has not yet produced, and its final value or error (if it has not yet been emitted).
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut p = build_test_producer::<u32, char, Infallible>()
    ///     .items(vec![1, 2, 4])
    ///     .fin('z')
    ///     .build().unwrap();
    ///
    /// assert_eq!(p.produce().await?, Left(1));
    ///
    /// let (items, last) = p.into_not_yet_produced();
    ///
    /// assert_eq!(items, vec![2, 4]);
    /// assert_eq!(last, Some(Ok('z')));
    /// # Result::<(), Infallible>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`TestConsumer::into_consumed`](consumer::TestConsumer::into_consumed) method.
    pub fn into_not_yet_produced(self) -> (Vec<Item>, Option<Result<Final, Error>>) {
        let amount_produced = self.inner.produced().len();
        let mut remaining_items = self.inner.into_inner();
        remaining_items.drain(0..amount_produced);
        (remaining_items, self.last)
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

                let max_len: usize =
                    self.exposed_items_sizes[self.exposed_items_sizes_index].into();
                self.exposed_items_sizes_index =
                    (self.exposed_items_sizes_index + 1) % self.exposed_items_sizes.len();

                f(&inner_items[..min(inner_items_len, max_len)]).await
            })
            .await
        {
            Ok(Left(yay)) => Ok(Left(yay)),
            Ok(Right(())) => Ok(Right(self.last.take().expect("Must not call `expose_items` after the TestProducer already emitted its last value.")?)),
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

/// This implementation considers only the values that have will still be emitted (regular, final, and error). That is, two `TestProducer`s are considered equal if they will produce equal sequences of values, irrespective of the details of how many items they will expose with future  `expose_items` calls, and irrespective of the pattern in which the async methods yield. In particular, it also does not matter which values they have produced prior to checking for equality.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut p1 = build_test_producer::<u32, char, Infallible>()
///     .items(vec![1, 2, 4])
///     .fin('z')
///     .build().unwrap();
///
/// let mut p2 = build_test_producer::<u32, char, Infallible>()
///     .items(vec![2, 4])
///     .fin('z')
///     .build().unwrap();
///
/// assert_eq!(p1 == p2, false);
/// assert_eq!(p1.produce().await?, Left(1));
/// assert_eq!(p1 == p2, true);
/// assert_eq!(p1.produce().await?, Left(2));
/// assert_eq!(p1 == p2, false);
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`PartialEq`] impl of [`TestConsumer`](consumer::TestConsumer).
impl<Item: PartialEq, Final: PartialEq, Error: PartialEq> PartialEq
    for TestProducer<Item, Final, Error>
{
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice() && self.peek_last() == other.peek_last()
    }
}

impl<Item: Eq, Final: Eq, Error: Eq> Eq for TestProducer<Item, Final, Error> {}
