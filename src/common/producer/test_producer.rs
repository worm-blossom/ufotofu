use core::fmt::Debug;

use arbitrary::Arbitrary;
use either::Either;
use either::Either::Left;
use either::Either::Right;

use crate::sync::producer::{FromBoxedSlice, ProduceOperations, Scramble};
use crate::sync::{BufferedProducer, BulkProducer, Producer};

invarianted_producer_outer_type!(
    /// Create via its `Arbitrary` implementation. Proper constructors and documentation will be added in the next release =S
    TestProducer_ TestProducer <Item, Final, Error>
);

invarianted_impl_debug!(TestProducer_<Item: Debug, Final: Debug, Error: Debug>);

impl<Item, Final, Error> TestProducer_<Item, Final, Error> {
    pub fn peek_slice(&self) -> &[Item] {
        self.0.as_ref().peek_slice()
    }

    /*
    pub fn peek_termination(&self) -> Either<Final, Error> {

    }
    */
}

invarianted_impl_producer_sync_and_local_nb!(TestProducer_<Item: Copy, Final, Error> Item Item;
    Final Final;
    Error Error
);
invarianted_impl_buffered_producer_sync_and_local_nb!(TestProducer_<Item: Copy, Final, Error>);
invarianted_impl_bulk_producer_sync_and_local_nb!(TestProducer_<Item: Copy, Final, Error>);

#[derive(Debug)]
struct TestProducer<Item, Final, Error> {
    inner: Scramble<FromBoxedSlice<Item>, Item, (), !>,
    termination: Option<Either<Final, Error>>,
}

impl<Item, Final, Error> TestProducer<Item, Final, Error> {
    fn peek_slice(&self) -> &[Item] {
        self.inner.as_ref().as_ref()
    }

    /*
    pub fn peek_termination(&self) -> Either<Final, Error> {

    }
    */
}

impl<Item, Final, Error> Producer for TestProducer<Item, Final, Error>
where
    Item: Copy,
{
    type Item = Item;
    type Final = Final;
    type Error = Error;

    fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.inner.produce() {
            Ok(Left(item)) => Ok(Left(item)),
            Ok(Right(())) => match self.termination.take().expect("Must not call produce after any function of the producer returned a final item or error.") {
                Left(fin) => Ok(Right(fin)),
                Right(err) => Err(err),
            },
            Err(_) => unreachable!(),
        }
    }
}

impl<Item, Final, Error> BufferedProducer for TestProducer<Item, Final, Error>
where
    Item: Copy,
{
    fn slurp(&mut self) -> Result<(), Self::Error> {
        // Unwrapping is okay because the error is of never.
        self.inner.slurp().unwrap();
        Ok(())
    }
}

impl<Item, Final, Error> BulkProducer for TestProducer<Item, Final, Error>
where
    Item: Copy,
{
    fn expose_items(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        // Unwrapping is okay because the error is of never.
        match self.inner.expose_items() {
         Ok(Left(slots)) => Ok(Left(slots)),
         Ok(Right(())) => match self.termination.take().expect("Must not call produce after any function of the producer returned a final item or error.") {
             Left(fin) => Ok(Right(fin)),
             Right(err) => Err(err),
         },
         Err(_) => unreachable!(),
        }
    }

    fn consider_produced(&mut self, amount: usize) -> Result<(), Self::Error> {
        // Unwrapping is okay because the error is of never.
        self.inner.consider_produced(amount).unwrap();
        Ok(())
    }
}

sync_producer_as_local_nb!(TestProducer<Item: Copy, Final, Error>);
sync_buffered_producer_as_local_nb!(TestProducer<Item: Copy, Final, Error>);
sync_bulk_producer_as_local_nb!(TestProducer<Item: Copy, Final, Error>);

impl<'a, Item, Final, Error> Arbitrary<'a> for TestProducer<Item, Final, Error>
where
    Item: Copy + Arbitrary<'a>,
    Final: Arbitrary<'a>,
    Error: Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let p: FromBoxedSlice<Item> = FromBoxedSlice::new(Arbitrary::arbitrary(u)?);
        let ops: ProduceOperations = Arbitrary::arbitrary(u)?;
        let capacity: usize = Arbitrary::arbitrary(u)?;

        let term: Either<Final, Error> = if Arbitrary::arbitrary(u)? {
            Left(Arbitrary::arbitrary(u)?)
        } else {
            Right(Arbitrary::arbitrary(u)?)
        };

        Ok(TestProducer {
            inner: Scramble::new(p, ops, capacity.clamp(128, 512)),
            termination: Some(term),
        })
    }
}
