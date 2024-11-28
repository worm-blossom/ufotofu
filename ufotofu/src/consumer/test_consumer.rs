use core::cmp::min;
use core::fmt::Debug;
use core::num::NonZeroUsize;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{boxed::Box, vec::Vec};
#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec};

use arbitrary::{size_hint, Arbitrary};
use wrapper::Wrapper;

use crate::consumer::IntoVec;
use crate::consumer::Invariant;
use crate::test_yielder::TestYielder;
use crate::{BufferedConsumer, BulkConsumer, Consumer};

#[derive(Clone)]
/// If you need to test code that works with arbitrary consumers, use this one. You can choose which error it should emit, when it emits its error, the size of the slices it presents with `expose_slots`, and when to its async functions should yield instead of returning immediately. Beyond manual control, the [`Arbitrary`] implementation lets you test against various consumer behaviours automatically.
///
/// Create new [`TestConsumer`](crate::common::consumer::TestConsumer)s either via a [`TestConsumerBuilder`] or via the implementation of [`Arbitrary`].
///
/// ```
/// use ufotofu::sync::consumer::*;
/// use ufotofu::sync::*;
///
/// let mut con: TestConsumer<u8, (), u16> = TestConsumerBuilder::new(404, 2).build();
/// assert_eq!(Ok(()), con.consume(4));
/// assert_eq!(Ok(()), con.consume(7));
/// assert_eq!(Err(404), con.consume(99)); // Configured to fail after two operations.
/// assert_eq!(&[4, 7], con.consumed());
/// ```
pub struct TestConsumer_<Item, Final, Error>(Invariant<TestConsumer<Item, Final, Error>>);

impl<Item, Final, Error> TestConsumer_<Item, Final, Error> {
    /// Obtain a slice of all items that have been consumed so far.
    ///
    /// ```
    /// use ufotofu::sync::consumer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut con: TestConsumer<u8, (), ()> = TestConsumerBuilder::new((), 999).build();
    /// assert_eq!(Ok(()), con.consume(4));
    /// assert_eq!(Ok(()), con.consume(7));
    /// assert_eq!(&[4, 7], con.consumed());
    /// ```
    pub fn consumed(&self) -> &[Item] {
        self.0.as_ref().consumed()
    }

    /// Obtain a reference to the final item that was consumed, or `None` if the consumer had not been closed so far.
    ///
    /// ```
    /// use ufotofu::sync::consumer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut con: TestConsumer<u8, u8, ()> = TestConsumerBuilder::new((), 999).build();
    /// assert_eq!(Ok(()), con.consume(4));
    /// assert_eq!(None, con.final_consumed());
    /// assert_eq!(Ok(()), con.close(17));
    /// assert_eq!(Some(&17), con.final_consumed());
    /// ```
    pub fn final_consumed(&self) -> Option<&Final> {
        self.0.as_ref().final_consumed()
    }

    /// Consume the [`TestConsumer`](crate::common::consumer::TestConsumer) and obtain ownership of all items that were consumed, including the final one (if any).
    ///
    /// ```
    /// use ufotofu::sync::consumer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut con: TestConsumer<u8, u8, ()> = TestConsumerBuilder::new((), 999).build();
    /// assert_eq!(Ok(()), con.consume(4));
    /// assert_eq!(None, con.final_consumed());
    /// assert_eq!(Ok(()), con.close(17));
    /// assert_eq!((vec![4], Some(17)), con.into_consumed());
    /// ```
    pub fn into_consumed(self) -> (Vec<Item>, Option<Final>) {
        self.0.into_inner().into_consumed()
    }

    /// Obtain a reference to the error that this will eventually emit, or `None` if the error was already emitted.
    ///
    /// ```
    /// use ufotofu::sync::consumer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut con: TestConsumer<u8, (), u16> = TestConsumerBuilder::new(404, 1).build();
    /// assert_eq!(Ok(()), con.consume(4));
    /// assert_eq!(Some(&404), con.peek_error());
    /// assert_eq!(Err(404), con.consume(99)); // Configured to fail after one operation.
    /// assert_eq!(None, con.peek_error());
    /// ```
    pub fn peek_error(&self) -> Option<&Error> {
        self.0.as_ref().peek_error()
    }

    /// Return whether an error was already emitted.
    ///
    /// ```
    /// use ufotofu::sync::consumer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut con: TestConsumer<u8, (), u16> = TestConsumerBuilder::new(404, 1).build();
    /// assert_eq!(Ok(()), con.consume(4));
    /// assert_eq!(false, con.did_error());
    /// assert_eq!(Err(404), con.consume(99)); // Configured to fail after one operation.
    /// assert_eq!(true, con.did_error());
    /// ```
    pub fn did_error(&self) -> bool {
        self.0.as_ref().did_error()
    }
}

invarianted_impl_debug!(TestConsumer_<Item: Debug, Final: Debug, Error: Debug>);

invarianted_impl_consumer!(TestConsumer_<Item: Copy + Default, Final, Error> Item Item; Final Final; Error Error);
invarianted_impl_buffered_consumer!(TestConsumer_<Item: Copy + Default, Final, Error>);
invarianted_impl_bulk_consumer!(TestConsumer_<Item: Copy + Default, Final, Error>);

impl<'a, Item: Arbitrary<'a>, Final: Arbitrary<'a>, Error: Arbitrary<'a>> Arbitrary<'a>
    for TestConsumer_<Item, Final, Error>
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let err = Error::arbitrary(u)?;
        let operations_until_error = usize::arbitrary(u)?;
        let slot_sizes = Box::<[NonZeroUsize]>::arbitrary(u)?;
        let yield_pattern = Box::<[bool]>::arbitrary(u)?;

        let ret = TestConsumerBuilder::new(err, operations_until_error)
            .exposed_slot_sizes(slot_sizes)
            .yield_pattern(yield_pattern)
            .build::<Item, Final>();
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

/// This implementation considers only the consumed items, the consumed final value, and the stored error value.
impl<Item: PartialEq, Final: PartialEq, Error: PartialEq> PartialEq
    for TestConsumer_<Item, Final, Error>
{
    fn eq(&self, other: &Self) -> bool {
        return self.0.as_ref().inner.as_ref() == other.0.as_ref().inner.as_ref()
            && self.0.as_ref().fin == other.0.as_ref().fin
            && self.0.as_ref().error == other.0.as_ref().error;
    }
}

impl<Item: Eq, Final: Eq, Error: Eq> Eq for TestConsumer_<Item, Final, Error> {}

/// A [builder](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html) for [`TestConsumer`](crate::common::consumer::TestConsumer).
///
/// ```
/// use ufotofu::sync::consumer::*;
/// use ufotofu::sync::*;
///
/// let mut con: TestConsumer<u8, (), u16> = TestConsumerBuilder::new(404, 2).build();
/// assert_eq!(Ok(()), con.consume(4));
/// assert_eq!(Ok(()), con.consume(7));
/// assert_eq!(Err(404), con.consume(99)); // Configured to fail after two operations.
/// assert_eq!(&[4, 7], con.consumed());
/// ```
pub struct TestConsumerBuilder<Error> {
    error: Error,
    consumptions_until_error: usize,
    exposed_slot_sizes: Option<Box<[NonZeroUsize]>>,
    yield_pattern: Option<Box<[bool]>>,
}

impl<Error> TestConsumerBuilder<Error> {
    /// Create a new [`TestConsumerBuilder`].
    ///
    /// The resulting consumer will succesfully receive `consumptions_until_error` many items before emitting the error `error` any further method call.
    ///
    /// ```
    /// use ufotofu::sync::consumer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut con: TestConsumer<u8, (), u16> = TestConsumerBuilder::new(404, 2).build();
    /// assert_eq!(Ok(()), con.consume(4));
    /// assert_eq!(Ok(()), con.consume(7));
    /// assert_eq!(Err(404), con.consume(99)); // Configured to fail after two consumptions.
    /// assert_eq!(&[4, 7], con.consumed());
    /// ```
    pub fn new(error: Error, consumptions_until_error: usize) -> TestConsumerBuilder<Error> {
        TestConsumerBuilder {
            error,
            consumptions_until_error,
            exposed_slot_sizes: None,
            yield_pattern: None,
        }
    }

    /// Set a pattern of upper bounds to the slice sizes that the [`BulkConsumer::expose_slots`] method returns. The consumer will cycle through the supplied upper bounds. Up to a size of 2048, the supplied sizes will also act as lower bounds, i.e., [`BulkConsumer::expose_slots`] will return slices of the exact sizes supplied here, unless it exceeds 2048.
    ///
    /// An empty slice will be ignored.
    ///
    /// Will be ignored by non-bulk consumers.
    ///
    /// ```
    /// use ufotofu::sync::consumer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut con: TestConsumer<u8, (), ()> = TestConsumerBuilder::new((), 999)
    ///     .exposed_slot_sizes(vec![76.try_into().unwrap(), 1.try_into().unwrap()].into())
    ///     .build();
    /// assert_eq!(76, con.expose_slots().unwrap().len());
    /// assert_eq!(1, con.expose_slots().unwrap().len());
    /// assert_eq!(76, con.expose_slots().unwrap().len());
    /// ```
    pub fn exposed_slot_sizes(mut self, sizes: Box<[NonZeroUsize]>) -> Self {
        if sizes.len() > 0 {
            self.exposed_slot_sizes = Some(sizes);
        } else {
            self.exposed_slot_sizes = None;
        }

        self
    }

    /// Set a pattern of whether to immediately complete asynchronous action, or to yield back to the task executor first.
    ///
    /// If all booleans are `true`, a single `false` will be appended (otherwise, the consumer would never complete its operations).
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

    /// Create a fully configured [`TestConsumer`](crate::common::consumer::TestConsumer).
    ///
    /// ```
    /// use ufotofu::sync::consumer::*;
    /// use ufotofu::sync::*;
    ///
    /// let mut con: TestConsumer<u8, (), u16> = TestConsumerBuilder::new(404, 2).build();
    /// assert_eq!(Ok(()), con.consume(4));
    /// assert_eq!(Ok(()), con.consume(7));
    /// assert_eq!(Err(404), con.consume(99)); // Configured to fail after two operations.
    /// assert_eq!(&[4, 7], con.consumed());
    /// ```
    pub fn build<Item, Final>(self) -> TestConsumer_<Item, Final, Error> {
        TestConsumer_(Invariant::new(TestConsumer {
            inner: IntoVec::new(),
            fin: None,
            error: Some(self.error),
            consumptions_until_error: self.consumptions_until_error,
            exposed_slot_sizes: self.exposed_slot_sizes.map(|sizes| (sizes, 0)),
            yielder: self.yield_pattern.map(TestYielder::new),
        }))
    }
}

#[derive(Clone)]
struct TestConsumer<Item, Final, Error> {
    inner: IntoVec<Item>,
    fin: Option<Final>,
    error: Option<Error>, // An option so we can `take` the error to emit it.
    consumptions_until_error: usize,
    exposed_slot_sizes: Option<(Box<[NonZeroUsize]>, usize /* current index*/)>,
    yielder: Option<TestYielder>,
}

impl<Item, Final, Error> TestConsumer<Item, Final, Error> {
    fn consumed(&self) -> &[Item] {
        self.inner.as_ref()
    }

    fn final_consumed(&self) -> Option<&Final> {
        self.fin.as_ref()
    }

    fn into_consumed(self) -> (Vec<Item>, Option<Final>) {
        (self.inner.into_inner(), self.fin)
    }

    fn peek_error(&self) -> Option<&Error> {
        self.error.as_ref()
    }

    pub fn did_error(&self) -> bool {
        self.error.is_none()
    }

    fn check_error(&mut self) -> Result<(), Error> {
        if self.consumptions_until_error == 0 {
            Err(self.error.take().unwrap()) // Can unwrap because the invariant wrapper panics before unwrapping can be reached.
        } else {
            Ok(())
        }
    }

    async fn maybe_yield(&mut self) {
        match &mut self.yielder {
            None => (),
            Some(yielder) => yielder.maybe_yield().await,
        }
    }
}

impl<Item: Debug, Final: Debug, Error: Debug> Debug for TestConsumer<Item, Final, Error> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TestConsumer")
            .field("inner", &self.inner)
            .field("final", &self.fin) // "final" is a nicer output than "fin"
            .field("error", &self.error)
            .field("operations_until_error", &self.consumptions_until_error)
            .field("exposed_slot_sizes", &self.exposed_slot_sizes)
            .field("yielder", &self.yielder)
            .finish()
    }
}

impl<Item: Default, Final, Error> Consumer for TestConsumer<Item, Final, Error> {
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

        Consumer::close(&mut self.inner, ()).await.unwrap(); // may unwrap because Err<!>
        Ok(())
    }
}

impl<Item: Default, Final, Error> BufferedConsumer for TestConsumer<Item, Final, Error> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.maybe_yield().await;
        self.check_error()?;

        BufferedConsumer::flush(&mut self.inner).await.unwrap(); // may unwrap because Err<!>
        Ok(())
    }
}

impl<Item, Final, Error> BulkConsumer for TestConsumer<Item, Final, Error>
where
    Item: Copy + Default,
{
    async fn expose_slots<'a>(&'a mut self) -> Result<&'a mut [Self::Item], Self::Error>
    where
        Self::Item: 'a,
    {
        self.maybe_yield().await;
        self.check_error()?;

        let max_len: usize = match self.exposed_slot_sizes {
            None => usize::MAX,
            Some((ref exposed_slot_sizes, ref mut index)) => {
                let max_len: usize = exposed_slot_sizes[*index].into();
                *index = (*index + 1) % exposed_slot_sizes.len();
                max_len
            }
        };

        let min_len = min(max_len, 2048);

        while self.inner.remaining_slots() < min_len {
            self.inner.make_space_even_if_not_needed();
        }

        let inner_slots = BulkConsumer::expose_slots(&mut self.inner).await.unwrap(); // may unwrap because Err<!>
        let actual_len = min(
            min(inner_slots.len(), max_len),
            self.consumptions_until_error,
        );

        Ok(&mut inner_slots[..actual_len])
    }

    async fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.maybe_yield().await;
        self.check_error()?;

        BulkConsumer::consume_slots(&mut self.inner, amount)
            .await
            .unwrap(); // may unwrap because Err<!>
        self.consumptions_until_error -= amount;
        Ok(())
    }
}
