use core::mem::MaybeUninit;

use either::Either;

use crate::sync::{BulkProducer, Producer};

/// Information you get from `produce_n` or `bulk_produce_n` when the producer is unable to produce `N` items.
///
/// /// `N` is the number of items, `T` the type of items, `F` the `Final` type of the producer, and `E` the `Error` type of the producer.
#[derive(Debug)]
pub struct ProduceNError<const N: usize, T, F, E> {
    /// How many items the producer did produce successfully.
    /// Guaranteed to be strictly less than `N`.
    pub count: usize,
    /// An array whose first `count` slots are initialized and contain the items that
    /// the producer did produce successfully. The contents of the remaining slots are
    /// unspecified, they may or may not be initialized.
    pub items: [MaybeUninit<T>; N],
    /// The reason why the producer failed to produce more items.
    /// Either it produced its final Item of type `F` too soon, or it
    /// yielded an error of type `E`.
    pub reason: Either<F, E>,
}

/// Try to obtain exactly `N` items from a producer.
pub fn produce_n_static<const N: usize, T, P>(
    producer: &mut P,
) -> Result<[T; N], ProduceNError<N, T, P::Final, P::Error>>
where
    P: Producer<Item = T>,
{
    unimplemented!() // implement by using pipe_into_slice_uninit
}

/// Try to obtain exactly `N` items from a bulk producer. More efficient than `produce_n`.
pub fn bulk_produce_n_static<const N: usize, T, P>(
    producer: &mut P,
) -> Result<[T; N], ProduceNError<N, T, P::Final, P::Error>>
where
    T: Copy,
    P: BulkProducer<Item = T>,
{
    unimplemented!() // implement by using bulk_pipe_into_slice_uninit
}
