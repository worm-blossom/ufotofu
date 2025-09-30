use core::convert::Infallible;

use crate::prelude::*;

/// A producer created from an infinite [`Iterator`]. It exhibits unspecified behaviour if the wrapped iterator returns `None` from [`Iterator::next`].
///
/// <br/>Counterpart: none, because the standard library has no counterpart to iterators.
#[derive(Debug)]
pub struct InfiniteIteratorToProducer<I>(I);

/// Creates a producer that produces items from a wrapped infinite iterator.
///
/// Do not use this method to convert finite iterators to producers, see [`iterator_to_producer`](crate::producer::iterator_to_producer) for the appropriate alternative. A finite iterator is one whose `next` method can return `None`.
///
/// ```
/// # use ufotofu::prelude::*;
/// use producer::compat::infinite_iterator_to_producer;
/// # pollster::block_on(async {
///
/// let mut p = infinite_iterator_to_producer(core::iter::repeat(4));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Left(4));
/// // ...
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: none, because the standard library has no counterpart to iterators.
pub fn infinite_iterator_to_producer<T>(iter: T) -> InfiniteIteratorToProducer<T> {
    InfiniteIteratorToProducer(iter)
}

impl<I> InfiniteIteratorToProducer<I> {
    /// Retrieves the wrapped iterator.
    pub fn into_inner(self) -> I {
        self.0
    }
}

impl<I> Producer for InfiniteIteratorToProducer<I>
where
    I: Iterator,
{
    type Item = I::Item;

    type Final = Infallible;

    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.0.next() {
            Some(it) => Ok(Left(it)),
            None => panic!("A supposedly infinite iterator yielded None."),
        }
    }
}
