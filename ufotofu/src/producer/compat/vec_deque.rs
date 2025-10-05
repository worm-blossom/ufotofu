//! Producer functionality for [`VecDeque`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `VecDeque<T>`,
//! - an [`IntoProducer`] impl for `&VecDeque<T>`, and
//! - an [`IntoProducer`] impl for `&mut VecDeque<T>`.
//!
//! <br/>Counterpart: the [`consumer::compat::vec_deque`] module.

use std::collections::VecDeque;

use crate::{
    prelude::*,
    producer::compat::{iterator_to_producer, IteratorToProducer},
};

/// The producer of the [`IntoProducer`] impl of `VecDeque<T>`.
///
/// ```
/// use std::collections::VecDeque;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let deq: VecDeque<_> = vec![1, 2, 4].into_iter().collect();
/// let mut p = deq.into_producer();
///
/// assert_eq!(p.produce().await?, Left(1));
/// assert_eq!(p.produce().await?, Left(2));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`consumer::compat::vec_deque::IntoConsumer`].
pub struct IntoProducer<T>(IteratorToProducer<<VecDeque<T> as IntoIterator>::IntoIter>);

impl<T> Producer for IntoProducer<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T> crate::IntoProducer for VecDeque<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducer<T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducer(iterator_to_producer(
            <VecDeque<T> as IntoIterator>::into_iter(self),
        ))
    }
}

/// The producer of the [`IntoProducer`] impl of `&VecDeque<T>`.
///
/// ```
/// use std::collections::VecDeque;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let deq: VecDeque<_> = vec![1, 2, 4].into_iter().collect();
/// let mut p = (&deq).into_producer();
///
/// assert_eq!(p.produce().await?, Left(&1));
/// assert_eq!(p.produce().await?, Left(&2));
/// assert_eq!(p.produce().await?, Left(&4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [`consumer::compat::vec_deque::IntoConsumerMut`].
pub struct IntoProducerRef<'s, T>(IteratorToProducer<<&'s VecDeque<T> as IntoIterator>::IntoIter>);

impl<'s, T> Producer for IntoProducerRef<'s, T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'s, T> crate::IntoProducer for &'s VecDeque<T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerRef<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerRef(iterator_to_producer(self.iter()))
    }
}

/// The producer of the [`IntoProducer`] impl of `&mut VecDeque<T>`.
///
/// ```
/// use std::collections::VecDeque;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut deq: VecDeque<_> = vec![1, 2, 4].into_iter().collect();
/// let mut p = (&mut deq).into_producer();
///
/// assert_eq!(p.produce().await?, Left(&mut 1));
/// let mut mutable_ref = p.produce().await?.unwrap_left();
/// *mutable_ref = 17;
/// assert_eq!(p.produce().await?, Left(&mut 4));
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(deq, vec![1, 17, 4].into_iter().collect::<VecDeque<_>>());
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: none, because you cannot consume into a collection behind an immutable reference.
pub struct IntoProducerMut<'s, T>(
    IteratorToProducer<<&'s mut VecDeque<T> as IntoIterator>::IntoIter>,
);

impl<'s, T> Producer for IntoProducerMut<'s, T> {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'s, T> crate::IntoProducer for &'s mut VecDeque<T> {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerMut<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerMut(iterator_to_producer(self.iter_mut()))
    }
}
