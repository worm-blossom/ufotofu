//! [TODO]
//!
//! Useful functionality for working with consumers, beyond the [`ConsumerExt`], [`BufferedConsumerExt`], and [`BulkConsumerExt`] traits.

use core::convert::Infallible;

use crate::errors::*;

/// A [`Consumer`] consumes a potentially infinite sequence, one item at a time.
///
/// The sequence consists of an arbitrary number of values of type [`Self::Item`], followed by
/// up to one value of type [`Self::Final`]. If you intend for the sequence to be infinite, use
/// [`Infallible`] for [`Self::Final`].
///
/// A consumer may signal an error of type [`Self::Error`] instead of consuming any item (whether repeated or final).
#[must_use = "consumers are lazy and do nothing unless produced into"]
pub trait Consumer {
    /// The sequence consumed by this consumer starts with *arbitrarily many* values of this type.
    type Item;
    /// The sequence consumed by this consumer ends with *up to one* value of this type.
    type Final;
    /// The type of errors the consumer can emit instead of doing its job.
    type Error;

    /// Attempts to consume the next item.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait returned an error,
    /// nor after [`close`](Consumer::close) was called.
    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error>;

    /// Attempts to consume the final value.
    ///
    /// After this function is called, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after [`close`](Consumer::close) was called.
    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error>;
}

impl<C: Consumer> Consumer for &mut C {
    type Item = C::Item;
    type Final = C::Final;
    type Error = C::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        (*self).consume(item).await
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        (*self).close(fin).await
    }
}

#[cfg(feature = "alloc")]
impl<C: Consumer> Consumer for alloc::boxed::Box<C> {
    type Item = C::Item;
    type Final = C::Final;
    type Error = C::Error;

    async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
        self.as_mut().consume(item).await
    }

    async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
        self.as_mut().close(fin).await
    }
}

impl Consumer for Infallible {
    type Item = Infallible;
    type Final = Infallible;
    type Error = Infallible;

    async fn consume(&mut self, _item: Self::Item) -> Result<(), Self::Error> {
        unreachable!()
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        unreachable!()
    }
}

/// Conversion into a [`Consumer`].
///
/// By implementing `IntoConsumer` for a type, you define how it will be
/// converted to a consumer.
pub trait IntoConsumer {
    /// The type of repeated items being consumed.
    type Item;

    /// The type of the final value being consumed.
    type Final;

    /// The type of errors the consumer may emit.
    type Error;

    type IntoConsumer: Consumer<Item = Self::Item, Final = Self::Final, Error = Self::Error>;

    /// Creates a consumer from a value.
    fn into_consumer(self) -> Self::IntoConsumer;
}

impl<C: Consumer> IntoConsumer for C {
    type Item = C::Item;
    type Final = C::Final;
    type Error = C::Error;
    type IntoConsumer = C;

    #[inline]
    fn into_consumer(self) -> C {
        self
    }
}

/// A [`Consumer`] that can delay performing side-effects when consuming items.
///
/// It must not delay performing side-effects when being closed. In other words,
/// calling [`close`](Consumer::close) should internally trigger flushing.
pub trait BufferedConsumer: Consumer {
    /// Forces the consumer to perform any side-effects that were delayed for previously consumed items.
    ///
    /// This function allows client code to force execution of the (potentially expensive)
    /// side-effects. In exchange, the consumer gains the freedom to delay the side-effects of
    /// [`consume`](Consumer::consume) to improve efficiency.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error,
    /// nor after [`close`](Consumer::close) was called.
    async fn flush(&mut self) -> Result<(), Self::Error>;
}

impl<C: BufferedConsumer> BufferedConsumer for &mut C {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        (*self).flush().await
    }
}

#[cfg(feature = "alloc")]
impl<C: BufferedConsumer> BufferedConsumer for alloc::boxed::Box<C> {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.as_mut().flush().await
    }
}

impl BufferedConsumer for Infallible {
    async fn flush(&mut self) -> Result<(), Self::Error> {
        unreachable!()
    }
}

/// Conversion into a [`BufferedConsumer`].
///
/// This trait is automatically implemented by implementing [`IntoConsumer`] with the associated consumer being a buffered consumer.
pub trait IntoBufferedConsumer: IntoConsumer<IntoConsumer: BufferedConsumer> {}

impl<C> IntoBufferedConsumer for C where C: IntoConsumer<IntoConsumer: BufferedConsumer> {}

/// A [`Consumer`] that is able to consume several items with a single function call, in order to
/// improve on the efficiency of the [`Consumer`] trait. Semantically, there must be no
/// difference between consuming items in bulk or one item at a time.
pub trait BulkConsumer: Consumer {
    /// Consumes a non-zero number of items by reading them from a given non-empty buffer, and returning how
    /// many items were consumed.
    ///
    /// After this function returns an error, no further functions of this trait may be invoked.
    ///
    /// #### Invariants
    ///
    /// Must not be called after any function of this trait has returned an error, nor after
    /// [`close`](Consumer::close) was called.
    ///
    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error>;
}

impl<C: BulkConsumer> BulkConsumer for &mut C {
    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        (*self).bulk_consume(buf).await
    }
}

#[cfg(feature = "alloc")]
impl<C: BulkConsumer> BulkConsumer for alloc::boxed::Box<C> {
    async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
        self.as_mut().bulk_consume(buf).await
    }
}

impl BulkConsumer for Infallible {
    async fn bulk_consume(&mut self, _buf: &[Self::Item]) -> Result<usize, Self::Error> {
        unreachable!()
    }
}

/// Conversion into a [`BulkConsumer`].
///
/// This trait is automatically implemented by implementing [`IntoConsumer`] with the associated consumer being a bulk consumer.
pub trait IntoBulkConsumer: IntoConsumer<IntoConsumer: BulkConsumer> {}

impl<C> IntoBulkConsumer for C where C: IntoConsumer<IntoConsumer: BulkConsumer> {}

// #[macro_use]
// mod macros;

// // mod into_slice;
// // pub use into_slice::IntoSlice_ as IntoSlice;

// #[cfg(feature = "alloc")]
// mod into_vec;
// #[cfg(feature = "alloc")]
// pub use into_vec::IntoVec_ as IntoVec;

// mod map_item;
// pub use map_item::MapItem;

// mod map_final;
// pub use map_final::MapFinal;

// mod map_error;
// pub use map_error::MapError;

// mod limit;
// pub use limit::Limit;

// #[cfg(feature = "compat")]
// mod writer;
// #[cfg(feature = "compat")]
// pub use writer::WriterToBulkConsumer;

// #[cfg(test)]
// mod invariant;
// #[cfg(not(test))]
// mod invariant_noop;
// #[cfg(test)]
// pub use invariant::Invariant;
// #[cfg(not(test))]
// pub use invariant_noop::Invariant;

// #[cfg(feature = "dev")]
// mod bulk_scrambler;
// #[cfg(feature = "dev")]
// pub use bulk_scrambler::{BulkConsumerOperation, BulkScrambler_ as BulkScrambler};

// #[cfg(all(feature = "dev", feature = "alloc"))]
// mod test_consumer;
// #[cfg(all(feature = "dev", feature = "alloc"))]
// pub use test_consumer::{TestConsumerBuilder, TestConsumer_ as TestConsumer};
