use core::cmp::min;
use core::fmt::Debug;
use core::marker::PhantomData;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

use arbitrary::{Arbitrary, Error as ArbitraryError, Unstructured};
use either::Either;
use ufotofu_queues::{Fixed, Queue};

use wrapper::Wrapper;

use crate::common::producer::Invariant;
use crate::local_nb::{
    BufferedProducer as BufferedProducerLocalNb, BulkProducer as BulkProducerLocalNb,
    Producer as ProducerLocalNb,
};
use crate::sync::{BufferedProducer, BulkProducer, Producer};

/// Operations which may be called against a producer.
#[derive(Debug, PartialEq, Eq, Arbitrary, Clone)]
pub enum ProduceOperation {
    Produce,
    ProducerSlots(usize),
    Slurp,
}

/// A sequence of heap-allocated operations for determining the method call patterns that a `Scramble` wrapper subjects its inner producer to. Intended to be [generated by a fuzzer](https://rust-fuzz.github.io/book/cargo-fuzz/structure-aware-fuzzing.html).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProduceOperations(Box<[ProduceOperation]>);

impl ProduceOperations {
    /// Checks that the operations contain at least one non-slurp operation
    /// and that no more than 256 different operations are being allocated.
    pub fn new(operations: Box<[ProduceOperation]>) -> Option<Self> {
        let mut found_non_slurp = false;
        for op in operations.iter() {
            if *op != ProduceOperation::Slurp {
                found_non_slurp = true;
                break;
            }
        }

        if found_non_slurp && operations.len() <= 256 {
            Some(ProduceOperations(operations))
        } else {
            None
        }
    }
}

impl<'a> Arbitrary<'a> for ProduceOperations {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, ArbitraryError> {
        match Self::new(Arbitrary::arbitrary(u)?) {
            Some(ops) => Ok(ops),
            None => Err(ArbitraryError::IncorrectFormat),
        }
    }

    #[inline]
    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <Box<[ProduceOperation]> as Arbitrary<'a>>::size_hint(depth)
    }
}

/// A `Producer` wrapper that scrambles the methods that get called on a wrapped producer, without changing the observable semantics.
///
/// To be used in fuzz testing: when you want to test a `Producer` you implemented, test a scrambled version of that producer instead, to exercise many different method call patterns even if the test itself only performs a simplistic method call pattern.
pub struct Scramble_<C, T, F, E>(Invariant<Scramble<C, T, F, E>>);

impl<C, T: Default, F, E> Scramble_<C, T, F, E> {
    /// Create a new wrapper around `inner` that exercises the producer trait methods of `inner` by cycling through the given `operations`. To provide this functionality, the wrapper must allocate an internal buffer of items, `capacity` sets the size of that buffer. Larger values allow for more bizarre method call patterns, smaller values consume less space (surprise!).
    pub fn new(inner: C, operations: ProduceOperations, capacity: usize) -> Self {
        Scramble_(Invariant::new(Scramble::new(inner, operations, capacity)))
    }
}

invarianted_impl_debug!(Scramble_<C: Debug, T: Debug, F: Debug, E: Debug>);

invarianted_impl_as_ref!(Scramble_<C, T, F, E>; C);
invarianted_impl_as_mut!(Scramble_<C, T, F, E>; C);
invarianted_impl_wrapper!(Scramble_<C, T, F, E>; C);

impl<C, T, F, E> Producer for Scramble_<C, T, F, E>
where
    C: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    type Item = T;
    type Final = F;
    type Error = E;

    invarianted_producer_methods!();
}

impl<C, T, F, E> BufferedProducer for Scramble_<C, T, F, E>
where
    C: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    invarianted_buffered_producer_methods!();
}

impl<C, T, F, E> BulkProducer for Scramble_<C, T, F, E>
where
    C: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    invarianted_bulk_producer_methods!();
}

impl<C, T, F, E> ProducerLocalNb for Scramble_<C, T, F, E>
where
    C: BulkProducerLocalNb<Item = T, Final = F, Error = E>,
    T: Copy,
{
    type Item = T;
    type Final = F;
    type Error = E;

    invarianted_producer_methods_local_nb!();
}

impl<C, T, F, E> BufferedProducerLocalNb for Scramble_<C, T, F, E>
where
    C: BulkProducerLocalNb<Item = T, Final = F, Error = E>,
    T: Copy,
{
    invarianted_buffered_producer_methods_local_nb!();
}

impl<C, T, F, E> BulkProducerLocalNb for Scramble_<C, T, F, E>
where
    C: BulkProducerLocalNb<Item = T, Final = F, Error = E>,
    T: Copy,
{
    invarianted_bulk_producer_methods_local_nb!();
}

struct Scramble<P, T, F, E> {
    /// An implementer of the `Producer` traits.
    /// The `Producer` that we wrap. All producer operations on the `Scramble`
    /// will be transformed into semantically equivalent scrambled operations,
    /// and then forwarded to the `inner` producer.
    inner: P,
    /// A fixed capacity queue of items. We store items here before forwarding
    /// them to the `inner` producer. Intermediate storage is necessary so that
    /// we can arbitrarily scramble operations before forwarding.
    buffer: Fixed<T>,
    /// A final value which may or may not have been returned from the inner producer.
    final_val: Option<F>,
    /// The instructions on how to scramble producer operations. We cycle
    /// through these round-robin.
    operations: Box<[ProduceOperation]>,
    /// The next operation to call on the `inner` producer once we need to empty
    /// our item queue.
    operations_index: usize,
    /// Satisfy the type checker, no useful semantics.
    phantom: PhantomData<E>,
}

impl<P: core::fmt::Debug, T: core::fmt::Debug, F: core::fmt::Debug, E> core::fmt::Debug
    for Scramble<P, T, F, E>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Does not print the PhantomData.
        f.debug_struct("Scramble")
            .field("inner", &self.inner)
            .field("buffer", &self.buffer)
            .field("final_val", &self.final_val)
            .field("operations", &self.operations)
            .field("operations_index", &self.operations_index)
            .finish()
    }
}

impl<P, T: Default, F, E> Scramble<P, T, F, E> {
    /// Create a new wrapper around `inner` that exercises the producer trait methods of `inner` by cycling through the given `operations`. To provide this functionality, the wrapper must allocate an internal buffer of items, `capacity` sets the size of that buffer. Larger values allow for more bizarre method call patterns, smaller values consume less space (surprise!).
    fn new(inner: P, operations: ProduceOperations, capacity: usize) -> Self {
        Scramble::<P, T, F, E> {
            inner,
            buffer: Fixed::new(capacity),
            final_val: None,
            operations: operations.0,
            operations_index: 0,
            phantom: PhantomData,
        }
    }
}
impl<P, T, F, E> Scramble<P, T, F, E> {
    fn advance_operations_index(&mut self) {
        self.operations_index = (self.operations_index + 1) % self.operations.len();
    }
}

impl<P, T, F, E> AsRef<P> for Scramble<P, T, F, E> {
    fn as_ref(&self) -> &P {
        &self.inner
    }
}

impl<P, T, F, E> AsMut<P> for Scramble<P, T, F, E> {
    fn as_mut(&mut self) -> &mut P {
        &mut self.inner
    }
}

impl<P, T, F, E> Wrapper<P> for Scramble<P, T, F, E> {
    fn into_inner(self) -> P {
        self.inner
    }
}

// Operates by draining its `queue` and `final_val` and filling them via the
// `operations` when they become empty.
impl<P, T, F, E> Producer for Scramble<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    type Item = T;
    type Final = F;
    type Error = E;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        // First we aim to fill the queue by performing operations.
        //
        // While the final value has not been set and the queue is not full, perform operations.
        // If the final value is returned from an operation, set the final value.
        while self.final_val.is_none() && self.buffer.len() < self.buffer.capacity() {
            if let Some(final_val) = self.perform_operation()? {
                self.final_val = Some(final_val)
            }
        }

        // Return the final value if the queue is empty and the value
        // was previously returned from an operation.
        if self.buffer.len() == 0 {
            if let Some(final_val) = self.final_val.take() {
                return Ok(Either::Right(final_val));
            }
        }

        // Now that the queue has been filled, dequeue an item.
        let item = self
            .buffer
            .dequeue()
            .expect("queue should have been filled by performing operations");

        Ok(Either::Left(item))
    }
}

impl<P, T, F, E> BufferedProducer for Scramble<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn slurp(&mut self) -> Result<(), Self::Error> {
        if self.final_val.is_some() {
            Ok(())
        } else {
            // Slurp the inner producer.
            self.inner.slurp()?;

            // Perform operations until the queue is full.
            while self.final_val.is_none() && self.buffer.len() < self.buffer.capacity() {
                if let Some(final_val) = self.perform_operation()? {
                    // Set the final value if returned from an operation.
                    self.final_val = Some(final_val);
                }
            }

            Ok(())
        }
    }
}

impl<P, T, F, E> BulkProducer for Scramble<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        // While the final value has not been set and the queue is not full, perform operations.
        // If the final value is returned from an operation, set the final value.
        while self.final_val.is_none() && self.buffer.len() < self.buffer.capacity() {
            if let Some(final_val) = self.perform_operation()? {
                self.final_val = Some(final_val)
            }
        }

        // Return the final value if the queue is empty and the value
        // was previously returned from an operation.
        if self.buffer.len() == 0 {
            if let Some(final_val) = self.final_val.take() {
                return Ok(Either::Right(final_val));
            }
        }

        // Return readable slots.
        let slots = self
            .buffer
            .expose_items()
            .expect("queue should contain items after being filled by performing operations");

        Ok(Either::Left(slots))
    }

    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.buffer.consider_dequeued(amount);

        Ok(())
    }
}

impl<P, T, F, E> Scramble<P, T, F, E>
where
    P: BulkProducer<Item = T, Final = F, Error = E>,
    T: Copy,
{
    fn perform_operation(&mut self) -> Result<Option<F>, E> {
        debug_assert!(self.buffer.len() < self.buffer.capacity());

        match self.operations[self.operations_index] {
            // Attempt to produce an item from the inner producer.
            ProduceOperation::Produce => match self.inner.produce()? {
                // If an item was produced, attempt to add it to the queue.
                Either::Left(item) => {
                    // Return value should always be `None`, due to the `debug_assert`
                    // check for available queue capacity, so we ignore the result.
                    let _ = self.buffer.enqueue(item);
                }
                // If the final value was produced, return it.
                Either::Right(final_val) => return Ok(Some(final_val)),
            },
            // Attempt to expose slots from the inner producer.
            ProduceOperation::ProducerSlots(n) => {
                let n = if n == 0 { 1 } else { n }; // TODO make `n` a NonZeroUsize instead
                match self.inner.expose_items()? {
                    Either::Left(slots) => {
                        // Set an upper bound on the slice of slots by comparing the number of available
                        // inner slots and the number provided by the `ProducerSlots` operation and taking
                        // the lowest value.
                        let slots_len = slots.len();
                        let available_slots = &slots[..min(slots_len, n)];

                        // Enqueue items into the inner producer.
                        let amount = self.buffer.bulk_enqueue(available_slots);

                        // Report the amount of items produced.
                        self.inner.consider_produced(amount)?;
                    }
                    // If the final value was produced, return it.
                    Either::Right(final_val) => return Ok(Some(final_val)),
                }
            }
            ProduceOperation::Slurp => {
                // Slurp the inner producer.
                self.inner.slurp()?;
            }
        }

        // Update the operations index.
        self.advance_operations_index();

        Ok(None)
    }
}

// Operates by draining its `queue` and `final_val` and filling them via the
// `operations` when they become empty.
impl<P, T, F, E> ProducerLocalNb for Scramble<P, T, F, E>
where
    P: BulkProducerLocalNb<Item = T, Final = F, Error = E>,
    T: Copy,
{
    type Item = T;
    type Final = F;
    type Error = E;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        // First we aim to fill the queue by performing operations.
        //
        // While the final value has not been set and the queue is not full, perform operations.
        // If the final value is returned from an operation, set the final value.
        while self.final_val.is_none() && self.buffer.len() < self.buffer.capacity() {
            if let Some(final_val) = self.perform_operation_local_nb().await? {
                self.final_val = Some(final_val)
            }
        }

        // Return the final value if the queue is empty and the value
        // was previously returned from an operation.
        if self.buffer.len() == 0 {
            if let Some(final_val) = self.final_val.take() {
                return Ok(Either::Right(final_val));
            }
        }

        // Now that the queue has been filled, dequeue an item.
        let item = self
            .buffer
            .dequeue()
            .expect("queue should have been filled by performing operations");

        Ok(Either::Left(item))
    }
}

impl<P, T, F, E> BufferedProducerLocalNb for Scramble<P, T, F, E>
where
    P: BulkProducerLocalNb<Item = T, Final = F, Error = E>,
    T: Copy,
{
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        if self.final_val.is_some() {
            Ok(())
        } else {
            // Slurp the inner producer.
            self.inner.slurp().await?;

            // Perform operations until the queue is full.
            while self.final_val.is_none() && self.buffer.len() < self.buffer.capacity() {
                if let Some(final_val) = self.perform_operation_local_nb().await? {
                    // Set the final value if returned from an operation.
                    self.final_val = Some(final_val);
                }
            }

            Ok(())
        }
    }
}

impl<P, T, F, E> BulkProducerLocalNb for Scramble<P, T, F, E>
where
    P: BulkProducerLocalNb<Item = T, Final = F, Error = E>,
    T: Copy,
{
    async fn expose_items<'a>(
        &'a mut self,
    ) -> Result<Either<&'a [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'a,
    {
        // While the final value has not been set and the queue is not full, perform operations.
        // If the final value is returned from an operation, set the final value.
        while self.final_val.is_none() && self.buffer.len() < self.buffer.capacity() {
            if let Some(final_val) = self.perform_operation_local_nb().await? {
                self.final_val = Some(final_val)
            }
        }

        // Return the final value if the queue is empty and the value
        // was previously returned from an operation.
        if self.buffer.len() == 0 {
            if let Some(final_val) = self.final_val.take() {
                return Ok(Either::Right(final_val));
            }
        }

        // Return readable slots.
        let slots = self
            .buffer
            .expose_items()
            .expect("queue should contain items after being filled by performing operations");

        Ok(Either::Left(slots))
    }

    async fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        self.buffer.consider_dequeued(amount);

        Ok(())
    }
}

impl<P, T, F, E> Scramble<P, T, F, E>
where
    P: BulkProducerLocalNb<Item = T, Final = F, Error = E>,
    T: Copy,
{
    async fn perform_operation_local_nb(&mut self) -> Result<Option<F>, E> {
        debug_assert!(self.buffer.len() < self.buffer.capacity());

        match self.operations[self.operations_index] {
            // Attempt to produce an item from the inner producer.
            ProduceOperation::Produce => match self.inner.produce().await? {
                // If an item was produced, attempt to add it to the queue.
                Either::Left(item) => {
                    // Return value should always be `None`, due to the `debug_assert`
                    // check for available queue capacity, so we ignore the result.
                    let _ = self.buffer.enqueue(item);
                }
                // If the final value was produced, return it.
                Either::Right(final_val) => return Ok(Some(final_val)),
            },
            // Attempt to expose slots from the inner producer.
            ProduceOperation::ProducerSlots(n) => {
                let n = if n == 0 { 1 } else { n }; // TODO make `n` a NonZeroUsize instead
                match self.inner.expose_items().await? {
                    Either::Left(slots) => {
                        // Set an upper bound on the slice of slots by comparing the number of available
                        // inner slots and the number provided by the `ProducerSlots` operation and taking
                        // the lowest value.
                        let slots_len = slots.len();
                        let available_slots = &slots[..min(slots_len, n)];

                        // Enqueue items into the inner producer.
                        let amount = self.buffer.bulk_enqueue(available_slots);

                        // Report the amount of items produced.
                        self.inner.consider_produced(amount).await?;
                    }
                    // If the final value was produced, return it.
                    Either::Right(final_val) => return Ok(Some(final_val)),
                }
            }
            ProduceOperation::Slurp => {
                // Slurp the inner producer.
                self.inner.slurp().await?;
            }
        }

        // Update the operations index.
        self.advance_operations_index();

        Ok(None)
    }
}
