use core::convert::Infallible;

use either::Either::{self, *};

use crate::{IntoProducer, Producer};

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "std")]
use std::{
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque},
    ffi::OsStr,
    path::{Path, PathBuf},
};

/// Use a finite [`Iterator`] as a [`Producer`] of final type `()` and error type [`Infallible`].
///
/// Do not use this wrapper for infinite iterators, use the [`InfiniteIteratorAsProducer`] wrapper instead (which has a final type of [`Infallible`]).
pub struct IteratorAsProducer<I>(I);

impl<I> IteratorAsProducer<I> {
    /// Wrap a finite iterator to use it as a producer.
    pub fn new(iter: I) -> Self {
        Self(iter)
    }
}

impl<I> Producer for IteratorAsProducer<I>
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
}

/// Use an infinite [`Iterator`] as a [`Producer`] of final type [`Infallible`] and error type [`Infallible`].
///
/// Do not use this wrapper for finite iterators, use the [`IteratorAsProducer`] wrapper instead (which has a final type of `()`).
pub struct InfiniteIteratorAsProducer<I>(I);

impl<I> InfiniteIteratorAsProducer<I> {
    /// Wrap an infinite iterator to use it as a producer.
    pub fn new(iter: I) -> Self {
        Self(iter)
    }
}

impl<I> Producer for InfiniteIteratorAsProducer<I>
where
    I: Iterator,
{
    type Item = I::Item;

    type Final = Infallible;

    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.0.next() {
            Some(it) => Ok(Left(it)),
            None => panic!("Expected an inifinte iterator, but the iterator returend None"),
        }
    }
}

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! implementIntoProducerForIntoIteratorType {
        ($outer:ident $(< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)? ; $item:ty) => {
            impl $(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?
                $crate::producer::IntoProducer
            for $outer
                $(< $( $lt ),+ >)?
            {
                type Item = $item;
                type Final = ();
                type Error = Infallible;
                type IntoProducer = $crate::producer::IteratorAsProducer<<$outer $(< $( $lt ),+ >)? as core::iter::IntoIterator>::IntoIter>;

                fn into_producer(self) -> Self::IntoProducer {
                    $crate::producer::IteratorAsProducer::new(self.into_iter())
                }
            }
        }
    }

    #[macro_export]
    macro_rules! infiniteImplementIntoProducerForIntoIteratorType {
        ($outer:ident $(< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)? ; $item:ty) => {
            impl $(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?
                $crate::producer::IntoProducer
            for $outer
                $(< $( $lt ),+ >)?
            {
                type Item = $item;
                type Final = Infallible;
                type Error = Infallible;
                type IntoProducer = $crate::producer::InfiniteIteratorAsProducer<<$outer $(< $( $lt ),+ >)? as core::iter::IntoIterator>::IntoIter>;

                fn into_producer(self) -> Self::IntoProducer {
                    $crate::producer::InfiniteIteratorAsProducer::new(self.into_iter())
                }
            }
        }
    }
}

////////////
// Slices //
////////////

// Slice

impl<'a, T> IntoProducer for &'a [T] {
    type Item = &'a T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a [T] as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

impl<'a, T> IntoProducer for &'a mut [T] {
    type Item = &'a mut T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a mut [T] as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

// Boxed Slice

#[cfg(feature = "alloc")]
impl<T> IntoProducer for Box<[T]> {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<Box<[T]> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(<Box<[T]> as IntoIterator>::into_iter(self))
    }
}

#[cfg(feature = "alloc")]
impl<'a, T> IntoProducer for &'a Box<[T]> {
    type Item = &'a T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a Box<[T]> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

#[cfg(feature = "alloc")]
impl<'a, T> IntoProducer for &'a mut Box<[T]> {
    type Item = &'a mut T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a mut Box<[T]> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

// Array

impl<T, const N: usize> IntoProducer for [T; N] {
    type Item = T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<[T; N] as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

impl<'a, T, const N: usize> IntoProducer for &'a [T; N] {
    type Item = &'a T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a [T; N] as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

impl<'a, T, const N: usize> IntoProducer for &'a mut [T; N] {
    type Item = &'a mut T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a mut [T; N] as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

/////////////////
// Zero-Or-One //
/////////////////

// Option

implementIntoProducerForIntoIteratorType!(Option<T>; T);

impl<'a, T> IntoProducer for &'a Option<T> {
    type Item = &'a T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a Option<T> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

impl<'a, T> IntoProducer for &'a mut Option<T> {
    type Item = &'a mut T;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a mut Option<T> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

// Result

impl<T, E> IntoProducer for Result<T, E> {
    type Item = T;
    type Final = ();
    type Error = E;
    type IntoProducer = ResultProducer<T, E>;

    fn into_producer(self) -> Self::IntoProducer {
        ResultProducer(Some(self))
    }
}

pub struct ResultProducer<T, E>(Option<Result<T, E>>);

impl<T, E> Producer for ResultProducer<T, E> {
    type Item = T;

    type Final = ();

    type Error = E;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.0.take() {
            Some(Ok(it)) => Ok(Left(it)),
            Some(Err(err)) => Err(err),
            None => Ok(Right(())),
        }
    }
}

impl<'a, T, E> IntoProducer for &'a Result<T, E> {
    type Item = &'a T;
    type Final = ();
    type Error = &'a E;
    type IntoProducer = ResultProducerRef<'a, T, E>;

    fn into_producer(self) -> Self::IntoProducer {
        ResultProducerRef(Some(self))
    }
}

pub struct ResultProducerRef<'a, T, E>(Option<&'a Result<T, E>>);

impl<'a, T, E> Producer for ResultProducerRef<'a, T, E> {
    type Item = &'a T;

    type Final = ();

    type Error = &'a E;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.0.take() {
            Some(Ok(it)) => Ok(Left(it)),
            Some(Err(err)) => Err(err),
            None => Ok(Right(())),
        }
    }
}

impl<'a, T, E> IntoProducer for &'a mut Result<T, E> {
    type Item = &'a mut T;
    type Final = ();
    type Error = &'a mut E;
    type IntoProducer = ResultProducerMut<'a, T, E>;

    fn into_producer(self) -> Self::IntoProducer {
        ResultProducerMut(Some(self))
    }
}

pub struct ResultProducerMut<'a, T, E>(Option<&'a mut Result<T, E>>);

impl<'a, T, E> Producer for ResultProducerMut<'a, T, E> {
    type Item = &'a mut T;

    type Final = ();

    type Error = &'a mut E;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match self.0.take() {
            Some(Ok(it)) => Ok(Left(it)),
            Some(Err(err)) => Err(err),
            None => Ok(Right(())),
        }
    }
}

/////////////////
// Collections //
/////////////////

// Vec

#[cfg(feature = "alloc")]
implementIntoProducerForIntoIteratorType!(Vec<T>; T);

#[cfg(feature = "alloc")]
impl<'a, K> IntoProducer for &'a Vec<K> {
    type Item = &'a K;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a Vec<K> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

#[cfg(feature = "alloc")]
impl<'a, K> IntoProducer for &'a mut Vec<K> {
    type Item = &'a mut K;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a mut Vec<K> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

// VecDeque

#[cfg(feature = "std")]
implementIntoProducerForIntoIteratorType!(VecDeque<T>; T);

#[cfg(feature = "std")]
impl<'a, K> IntoProducer for &'a VecDeque<K> {
    type Item = &'a K;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a VecDeque<K> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

#[cfg(feature = "std")]
impl<'a, K> IntoProducer for &'a mut VecDeque<K> {
    type Item = &'a mut K;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a mut VecDeque<K> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

// Linked List

#[cfg(feature = "std")]
implementIntoProducerForIntoIteratorType!(LinkedList<T>; T);

#[cfg(feature = "std")]
impl<'a, K> IntoProducer for &'a LinkedList<K> {
    type Item = &'a K;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a LinkedList<K> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

#[cfg(feature = "std")]
impl<'a, K> IntoProducer for &'a mut LinkedList<K> {
    type Item = &'a mut K;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a mut LinkedList<K> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

// BinaryHeap

#[cfg(feature = "std")]
implementIntoProducerForIntoIteratorType!(BinaryHeap<T>; T);

#[cfg(feature = "std")]
impl<'a, K> IntoProducer for &'a BinaryHeap<K> {
    type Item = &'a K;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a BinaryHeap<K> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

// BTreeSet

#[cfg(feature = "std")]
implementIntoProducerForIntoIteratorType!(BTreeSet<T>; T);

#[cfg(feature = "std")]
impl<'a, K> IntoProducer for &'a BTreeSet<K> {
    type Item = &'a K;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a BTreeSet<K> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

// BTreeMap

#[cfg(feature = "std")]
implementIntoProducerForIntoIteratorType!(BTreeMap<K, V>; (K, V));

#[cfg(feature = "std")]
impl<'a, K, V> IntoProducer for &'a BTreeMap<K, V> {
    type Item = (&'a K, &'a V);
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a BTreeMap<K, V> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

#[cfg(feature = "std")]
impl<'a, K, V> IntoProducer for &'a mut BTreeMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a mut BTreeMap<K, V> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

// HashSet

#[cfg(feature = "std")]
implementIntoProducerForIntoIteratorType!(HashSet<T, S>; T);

#[cfg(feature = "std")]
impl<'a, K, S> IntoProducer for &'a HashSet<K, S> {
    type Item = &'a K;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a HashSet<K, S> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

// HashMap

#[cfg(feature = "std")]
implementIntoProducerForIntoIteratorType!(HashMap<K, V, S>; (K, V));

#[cfg(feature = "std")]
impl<'a, K, V, S> IntoProducer for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a HashMap<K, V, S> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

#[cfg(feature = "std")]
impl<'a, K, V, S> IntoProducer for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a mut HashMap<K, V, S> as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

///////////////////////////
// Misc Standard Library //
///////////////////////////

#[cfg(feature = "std")]
impl<'a> IntoProducer for &'a Path {
    type Item = &'a OsStr;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a Path as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}

#[cfg(feature = "std")]
impl<'a> IntoProducer for &'a PathBuf {
    type Item = &'a OsStr;
    type Final = ();
    type Error = Infallible;
    type IntoProducer = IteratorAsProducer<<&'a PathBuf as IntoIterator>::IntoIter>;

    fn into_producer(self) -> Self::IntoProducer {
        IteratorAsProducer::new(self.into_iter())
    }
}
