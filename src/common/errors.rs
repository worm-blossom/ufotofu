use core::error::Error;
use core::fmt::{Debug, Display};
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

impl<E> Error for ConsumeFullSliceError<E>
where
    E: 'static + Error,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.reason)
    }
}

impl<E> Display for ConsumeFullSliceError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The consumer failed to consume the complete slice, and only consumed {} items",
            self.consumed
        )
    }
}

/// Information you get from the `pipe_into_slice` family of functions when the producer is unable to fill the complete slice.
///
/// `'a` is the lifetime of the slice, `T` the type of items of the slice, `F` the `Final` type of the producer, and `E` the `Error` type of the producer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverwriteFullSliceError<F, E> {
    /// How many items of the slice *were* overwritten successfully. Guaranteed to be strictly less than the length of the original slice.
    pub overwritten: usize,
    /// Did completely filling the slice fail because the producer reached its final item, or because it yielded an error?
    pub reason: Either<F, E>,
}

impl<F, E> Error for OverwriteFullSliceError<F, E>
where
    F: Debug,
    E: 'static + Error,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.reason {
            Either::Left(_) => None,
            Either::Right(err) => Some(err),
        }
    }
}

impl<F, E> Display for OverwriteFullSliceError<F, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.reason {
            Either::Left(_) => {
                write!(f, "The producer was unable to fill the whole slice due to being finalised, and stopped after overwriting {} items", self.overwritten)
            }
            Either::Right(_) => {
                write!(f, "The producer was unable to fill the whole slice due to an error, and stopped after overwriting {} items", self.overwritten)
            }
        }
    }
}
