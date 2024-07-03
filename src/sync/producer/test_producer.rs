use arbitrary::Arbitrary;
use either::Either;
use either::Either::Left;
use either::Either::Right;

use crate::sync::producer::{FromVec, ProduceOperations, Scramble};
use crate::sync::{BufferedProducer, BulkProducer, Producer};

#[derive(Debug)]
pub struct TestProducer<Item, Final, Error> {
    inner: Scramble<FromVec<Item>, Item, (), !>,
    termination: Option<Either<Final, Error>>,
}

impl<Item, Final, Error> TestProducer<Item, Final, Error> {
    pub fn peek_slice(&self) -> &[Item] {
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
    fn producer_slots(&mut self) -> Result<Either<&[Self::Item], Self::Final>, Self::Error> {
        // Unwrapping is okay because the error is of never.
        match self.inner.producer_slots() {
         Ok(Left(slots)) => Ok(Left(slots)),
         Ok(Right(())) => match self.termination.take().expect("Must not call produce after any function of the producer returned a final item or error.") {
             Left(fin) => Ok(Right(fin)),
             Right(err) => Err(err),
         },
         Err(_) => unreachable!(),
        }
    }

    fn did_produce(&mut self, amount: usize) -> Result<(), Self::Error> {
        // Unwrapping is okay because the error is of never.
        Ok(self.inner.did_produce(amount).unwrap())
    }
}

impl<'a, Item, Final, Error> Arbitrary<'a> for TestProducer<Item, Final, Error>
where
    Item: Copy + Arbitrary<'a>,
    Final: Arbitrary<'a>,
    Error: Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let p: FromVec<Item> = FromVec::new(Arbitrary::arbitrary(u)?);
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
