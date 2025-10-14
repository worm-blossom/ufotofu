use core::cmp::min;

use alloc::{boxed::Box, vec::Vec};

use arbitrary::{size_hint, Arbitrary};
use derive_builder::Builder;

use crate::{consumer::compat, prelude::*, test_yielder::TestYielder};

/// Returns a [`TestConsumerBuilder`] for building a consumer with fully configurable observable behaviour.
///
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut c = build_test_consumer::<u32, (), char>()
///     .err('z')
///     .consumptions_until_error(2)
///     .build().unwrap();
///
/// c.consume(1).await?;
/// c.consume(2).await?;
/// assert_eq!(c.consume(4).await, Err('z'));
///
/// assert_eq!(c.as_slice(), &[1, 2]);
/// assert_eq!(c.peek_final(), None);
/// # Result::<(), char>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [`producer::build_test_producer`] function.
pub fn build_test_consumer<Item, Final, Error>() -> TestConsumerBuilder<Item, Final, Error>
where
    Item: Clone,
    Final: Clone,
    Error: Clone,
{
    TestConsumerBuilder::create_empty()
}

impl<Item, Final, Error> TestConsumerBuilder<Item, Final, Error> {
    /// Configures the number of item slots the built [`TestConsumer`] will expose on each call to `expose_slots`; the built consumer will cycle through this vec of sizes.
    ///
    /// Entries of `0` will be ignored. If all entries are zero, a single `usize::MAX` is used as the pattern.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut c = build_test_consumer::<u32, (), char>()
    ///     .err('z')
    ///     .consumptions_until_error(999)
    ///     .exposed_slots_sizes(vec![1, 2])
    ///     .build().unwrap();
    ///
    /// // The pattern starts with `1`, so one slots is exposed.
    /// c.expose_slots(async |items| {
    ///     assert_eq!(items.len(), 1);
    ///     (0, ()) // Report back that zero items should be considered consumed.
    /// }).await?;
    ///
    /// // The pattern continues with `2`, so two slots are exposed.
    /// c.expose_slots(async |items| {
    ///     assert_eq!(items.len(), 2);
    ///     (0, ()) // Report back that zero items should be considered consumed.
    /// }).await?;
    ///
    /// // The pattern loops back to its start, so one item is exposed.
    /// c.expose_slots(async |items| {
    ///     assert_eq!(items.len(), 1);
    ///     (0, ()) // Report back that zero items should be considered consumed.
    /// }).await?;
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
    ///
    /// If you do not call this method, the built consumer will expose unspecified sizes of item slots strictly less than 65536.
    ///
    /// <br/>Counterpart: the [`TestProducerBuilder::exposed_items_sizes`](producer::TestProducerBuilder::exposed_items_sizes) method.
    pub fn exposed_slots_sizes<VALUE: Into<Vec<usize>>>(&mut self, value: VALUE) -> &mut Self {
        let mut the_sizes: Vec<usize> = value.into().into_iter().filter(|size| *size > 0).collect();

        if the_sizes.is_empty() {
            the_sizes.push(65535);
        }

        let new = self;
        new.exposed_slots_sizes = Some(the_sizes.into_boxed_slice());
        new
    }

    /// Sets a pattern to control whether the built [`TestConsumer`] will immediately complete asynchronous methods, or whether it will yield back to the task executor first.
    ///
    /// If you do not call this method, the built consumer will complete all its methods immediately without unnecessary yielding.
    ///
    /// If all booleans are `true`, a single `false` is automatically appended (otherwise, the producer would always yield and never progress).
    ///
    /// <br/>Counterpart: the [`TestProducerBuilder::yield_pattern`](producer::TestProducerBuilder::yield_pattern) method.
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
    /// Configures the error which the built [`TestConsumer`] will emit.
    ///
    /// See [`TestConsumerBuilder::consumptions_until_error`] for how to set *when* the error will be emitted.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut c = build_test_consumer::<u32, (), char>()
    ///     .err('z')
    ///     .consumptions_until_error(2)
    ///     .build().unwrap();
    ///
    /// c.consume(1).await?;
    /// c.consume(2).await?;
    /// assert_eq!(c.consume(4).await, Err('z'));
    ///
    /// assert_eq!(c.as_slice(), &[1, 2]);
    /// assert_eq!(c.peek_final(), None);
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
    #[builder(setter(strip_option))]
    err: Option<Error>,
    /// Configures after how many consumed items the built [`TestConsumer`] will emit [its error](TestConsumerBuilder::err).
    ///
    /// See [`TestConsumerBuilder::err`] for how to set *which* error will be emitted.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut c = build_test_consumer::<u32, (), char>()
    ///     .err('z')
    ///     .consumptions_until_error(2)
    ///     .build().unwrap();
    ///
    /// c.consume(1).await?;
    /// c.consume(2).await?;
    /// assert_eq!(c.consume(4).await, Err('z'));
    ///
    /// assert_eq!(c.as_slice(), &[1, 2]);
    /// assert_eq!(c.peek_final(), None);
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
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
    /// Returns the regular items the consumer has already produced.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut c = build_test_consumer::<u32, (), char>()
    ///     .err('z')
    ///     .consumptions_until_error(2)
    ///     .build().unwrap();
    ///
    /// assert_eq!(c.as_slice(), &[]);
    /// c.consume(1).await?;
    /// assert_eq!(c.as_slice(), &[1]);
    /// c.consume(2).await?;
    /// assert_eq!(c.as_slice(), &[1, 2]);
    /// assert_eq!(c.consume(4).await, Err('z'));
    /// assert_eq!(c.as_slice(), &[1, 2]);
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`TestProducer::as_slice`](producer::TestProducer::as_slice) method.
    pub fn as_slice(&self) -> &[Item] {
        self.inner.as_slice()
    }

    /// Returns a reference to the final value the consumer has been closed with, or `None` if it has not yet been closed.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut c = build_test_consumer::<u32, f32, char>()
    ///     .err('z')
    ///     .consumptions_until_error(2)
    ///     .build().unwrap();
    ///
    /// assert_eq!(c.peek_final(), None);
    /// c.consume(1).await?;
    /// assert_eq!(c.peek_final(), None);
    /// c.close(5.2).await?;
    /// assert_eq!(c.peek_final(), Some(&5.2));
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`TestProducer::peek_last`](producer::TestProducer::peek_last) method (when it returns an `Ok` value).
    pub fn peek_final(&self) -> Option<&Final> {
        self.fin.as_ref()
    }

    /// Returns whether the consumer has been closed already.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut c = build_test_consumer::<u32, f32, char>()
    ///     .err('z')
    ///     .consumptions_until_error(2)
    ///     .build().unwrap();
    ///
    /// assert_eq!(c.is_closed(), false);
    /// c.consume(1).await?;
    /// assert_eq!(c.is_closed(), false);
    /// c.close(5.2).await?;
    /// assert_eq!(c.is_closed(), true);
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`TestProducer::did_already_emit_last`](producer::TestProducer::did_already_emit_last) method.
    pub fn is_closed(&self) -> bool {
        self.fin.is_some()
    }

    /// Drops the consumer and returns ownership of the items it has consumed and the final value it has been closed with (if any).
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut c = build_test_consumer::<u32, f32, char>()
    ///     .err('z')
    ///     .consumptions_until_error(2)
    ///     .build().unwrap();
    ///
    /// c.consume(1).await?;
    /// c.close(5.2).await?;
    ///
    /// let (items, fin) = c.into_consumed();
    /// assert_eq!(items, vec![1]);
    /// assert_eq!(fin, Some(5.2));
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`TestProducer::into_not_yet_produced`](producer::TestProducer::into_not_yet_produced) method.
    pub fn into_consumed(self) -> (Vec<Item>, Option<Final>) {
        (self.inner.into(), self.fin)
    }

    /// Returns a reference to the error the consumer will eventually emit, or `None` if it has been emitted already.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut c = build_test_consumer::<u32, (), char>()
    ///     .err('z')
    ///     .consumptions_until_error(2)
    ///     .build().unwrap();
    ///
    /// assert_eq!(c.peek_error(), Some(&'z'));
    /// c.consume(1).await?;
    /// assert_eq!(c.peek_error(), Some(&'z'));
    /// c.consume(2).await?;
    /// assert_eq!(c.peek_error(), Some(&'z'));
    /// assert_eq!(c.consume(4).await, Err('z'));
    /// assert_eq!(c.peek_error(), None);
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`TestProducer::peek_last`](producer::TestProducer::peek_last) method (when it returns an `Ok` value).
    pub fn peek_error(&self) -> Option<&Error> {
        self.err.as_ref()
    }

    /// Returns whether the consumer has emitted its error already.
    ///
    /// ```
    /// use ufotofu::prelude::*;
    /// # pollster::block_on(async{
    /// let mut c = build_test_consumer::<u32, (), char>()
    ///     .err('z')
    ///     .consumptions_until_error(2)
    ///     .build().unwrap();
    ///
    /// assert_eq!(c.did_already_error(), false);
    /// c.consume(1).await?;
    /// assert_eq!(c.did_already_error(), false);
    /// c.consume(2).await?;
    /// assert_eq!(c.did_already_error(), false);
    /// assert_eq!(c.consume(4).await, Err('z'));
    /// assert_eq!(c.did_already_error(), true);
    /// # Result::<(), char>::Ok(())
    /// # });
    /// ```
    ///
    /// <br/>Counterpart: the [`TestProducer::peek_last`](producer::TestProducer::peek_last) method (when it returns an `Ok` value).
    pub fn did_already_error(&self) -> bool {
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

/// Generates almost completely random [`TestConsumer`], the only exception is that no individual slot_size is greater than 65535 (because arbitrarily large slot_sizes yield in running out of memory on bulk consumption).
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
        let mut slot_sizes = Vec::<usize>::arbitrary(u)?;
        let yield_pattern = Vec::<bool>::arbitrary(u)?;

        for n in slot_sizes.iter_mut() {
            *n %= 65536;
        }

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
/// ```
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut c1 = build_test_consumer::<u32, (), char>()
///     .err('z')
///     .consumptions_until_error(2)
///     .build().unwrap();
///
/// let mut c2 = build_test_consumer::<u32, (), char>()
///     .err('z')
///     .consumptions_until_error(999)
///     .build().unwrap();
///
/// assert_eq!(c1 == c2, true);
/// c1.consume(1).await?;
/// assert_eq!(c1 == c2, false);
/// c2.consume(1).await?;
/// assert_eq!(c1 == c2, true);
/// # Result::<(), char>::Ok(())
/// # });
/// ```
impl<Item: PartialEq, Final: PartialEq, Error: PartialEq> PartialEq
    for TestConsumer<Item, Final, Error>
{
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice() && self.peek_final() == other.peek_final()
    }
}

impl<Item: Eq, Final: Eq, Error: Eq> Eq for TestConsumer<Item, Final, Error> {}
