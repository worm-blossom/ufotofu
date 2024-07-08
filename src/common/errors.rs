use either::Either;

/// Information you get from the `consume_full_slice` family of methods when the consumer is unable to consume the complete slice.
///
/// `E` is the `Error` type of the consumer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsumeFullSliceError<E> {
    /// The number of items that were consumed.
    pub consumed: usize,
    /// Why did the consumer stop accepting items?
    pub reason: E,
}
// TODO impl Error

/// Information you get from the `pipe_into_slice` family of functions when the producer is unable to fill the complete slice.
///
/// `'a` is the lifetime of the slice, `T` the type of items of the slice, `F` the `Final` type of the producer, and `E` the `Error` type of the producer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverwriteFullSliceError<'a, T, F, E> {
    /// The prefix of the argument slice that *was* filled successfully. The length of this is guaranteed to be strictly less than the length of the original slice.
    pub filled: &'a [T],
    /// Did completely filling the slice fail because the producer reached its final item, or because it yielded an error?
    pub reason: Either<F, E>,
}
// TODO impl Error
