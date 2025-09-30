//! Producer functionality for [`LinkedList`].
//!
//! Specifically, the module provides
//!
//! - an [`IntoProducer`] impl for `LinkedList<T>`,
//! - an [`IntoProducer`] impl for `&LinkedList<T>`, and
//! - an [`IntoProducer`] impl for `&mut LinkedList<T>`.
//!
//! <br/>Counterpart: the [`consumer::compat::vec`] module.

use std::collections::LinkedList;

use crate::{
    prelude::*,
    producer::compat::{iterator_to_producer, IteratorToProducer},
};

/// The producer of the [`IntoProducer`] impl of `LinkedList<T>`.
///
/// ```
/// use std::collections::LinkedList;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let list: LinkedList<_> = vec![1, 2, 4].into_iter().collect();
/// let mut p = list.into_producer();
///
/// assert_eq!(p.produce().await?, Left(1));
/// assert_eq!(p.produce().await?, Left(2));
/// assert_eq!(p.produce().await?, Left(4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducer<T>(IteratorToProducer<<LinkedList<T> as IntoIterator>::IntoIter>);

impl<T> Producer for IntoProducer<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<T> crate::IntoProducer for LinkedList<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducer<T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducer(iterator_to_producer(
            <LinkedList<T> as IntoIterator>::into_iter(self),
        ))
    }
}

/// The producer of the [`IntoProducer`] impl of `&LinkedList<T>`.
///
/// ```
/// use std::collections::LinkedList;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let list: LinkedList<_> = vec![1, 2, 4].into_iter().collect();
/// let mut p = (&list).into_producer();
///
/// assert_eq!(p.produce().await?, Left(&1));
/// assert_eq!(p.produce().await?, Left(&2));
/// assert_eq!(p.produce().await?, Left(&4));
/// assert_eq!(p.produce().await?, Right(()));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducerRef<'s, T>(
    IteratorToProducer<<&'s LinkedList<T> as IntoIterator>::IntoIter>,
);

impl<'s, T> Producer for IntoProducerRef<'s, T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, T> crate::IntoProducer for &'s LinkedList<T> {
    type Item = &'s T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerRef<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerRef(iterator_to_producer(self.into_iter()))
    }
}

/// The producer of the [`IntoProducer`] impl of `&mut LinkedList<T>`.
///
/// ```
/// use std::collections::LinkedList;
/// use ufotofu::prelude::*;
/// # pollster::block_on(async{
/// let mut list: LinkedList<_> = vec![1, 2, 4].into_iter().collect();
/// let mut p = (&mut list).into_producer();
///
/// assert_eq!(p.produce().await?, Left(&mut 1));
/// let mut mutable_ref = p.produce().await?.unwrap_left();
/// *mutable_ref = 17;
/// assert_eq!(p.produce().await?, Left(&mut 4));
/// assert_eq!(p.produce().await?, Right(()));
/// assert_eq!(list, vec![1, 17, 4].into_iter().collect::<LinkedList<_>>());
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: [TODO].
pub struct IntoProducerMut<'s, T>(
    IteratorToProducer<<&'s mut LinkedList<T> as IntoIterator>::IntoIter>,
);

impl<'s, T> Producer for IntoProducerMut<'s, T> {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        self.0.produce().await
    }
}

impl<'s, T> crate::IntoProducer for &'s mut LinkedList<T> {
    type Item = &'s mut T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IntoProducerMut<'s, T>;

    fn into_producer(self) -> Self::IntoProducer {
        IntoProducerMut(iterator_to_producer(self.into_iter()))
    }
}
