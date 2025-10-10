use core::convert::Infallible;

use crate::prelude::*;

/// A producer created from a finite [`Iterator`].
///
/// <br/>Counterpart: none, because the standard library has no counterpart to iterators.
#[derive(Debug, Clone)]
pub struct IteratorToProducer<I>(I);

/// Creates a producer that produces items from a wrapped iterator.
///
/// Do not use this method to convert infinite iterators to producers, see [`infinite_iterator_to_producer`](crate::producer::compat::infinite_iterator_to_producer) for the appropriate alternative. An inifinite iterator is one whose `next` method never returns `None`.
///
/// ```
/// # use ufotofu::prelude::*;
/// use producer::compat::iterator_to_producer;
/// # pollster::block_on(async {
///
/// let mut p = iterator_to_producer(vec![1, 2, 4].into_iter());
/// assert_eq!(p.produce().await?, Left(1));
/// assert_eq!(p.produce().await?, Left(2));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: none, because the standard library has no counterpart to iterators.
pub fn iterator_to_producer<T>(iter: T) -> IteratorToProducer<T> {
    IteratorToProducer(iter)
}

impl<I> IteratorToProducer<I> {
    /// Retrieves the wrapped iterator.
    pub fn into_inner(self) -> I {
        self.0
    }
}

impl<I> AsRef<I> for IteratorToProducer<I> {
    fn as_ref(&self) -> &I {
        &self.0
    }
}

impl<I> AsMut<I> for IteratorToProducer<I> {
    fn as_mut(&mut self) -> &mut I {
        &mut self.0
    }
}

impl<I> Producer for IteratorToProducer<I>
where
    I: Iterator,
{
    type Item = I::Item;

    type Final = ();

    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.0.next() {
            Some(it) => Ok(Left(it)),
            None => Ok(Right(())),
        }
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
