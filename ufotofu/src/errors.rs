#[cfg(not(feature = "std"))]
use core::error::Error;
use core::fmt::{Debug, Display};
use either::Either;
#[cfg(feature = "std")]
use std::error::Error;

/// Everything that can go wrong when [piping](crate::pipe) a producer into a consumer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PipeError<ProducerError, ConsumerError> {
    /// The producer emitted an error.
    Producer(ProducerError),
    /// The consumer emitted an error when consuming an `Item`.
    Consumer(ConsumerError),
}

impl<ProducerError, ConsumerError> Display for PipeError<ProducerError, ConsumerError> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipeError::Producer(_) => {
                write!(
                    f,
                    "Failed to pipe a producer into a consumer, because the producer emitted an error",
                )
            }
            PipeError::Consumer(_) => {
                write!(
                    f,
                    "Failed to pipe a producer into a consumer, because the consumer emitted an error",
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl<ProducerError, ConsumerError> Error for PipeError<ProducerError, ConsumerError>
where
    ProducerError: 'static + Error,
    ConsumerError: 'static + Error,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PipeError::Producer(err) => Some(err),
            PipeError::Consumer(err) => Some(err),
        }
    }
}

/// An error emitted when a consumer is tasked to consume at least some number of items, but it could only consume a lower number of items.
///
/// `E` is the [`Error`](crate::Consumer::Error) type of the consumer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsumeAtLeastError<E> {
    /// The number of items that were consumed.
    pub count: usize,
    /// Why did the consumer stop accepting items?
    pub reason: E,
}

#[cfg(feature = "std")]
impl<E> Error for ConsumeAtLeastError<E>
where
    E: 'static + Error,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.reason)
    }
}

impl<E> Display for ConsumeAtLeastError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The consumer failed to consume sufficiently many items, it only consumed {} items",
            self.count
        )
    }
}

impl<E> ConsumeAtLeastError<E> {
    /// Consumes `self` and returns `self.reason`, effectively discarding `self.count`.
    pub fn into_reason(self) -> E {
        self.reason
    }
}

/// An error emitted when a producer is tasked to produce at least some number of items, but it could only produce a lower number of items.
///
/// `F` is the [`Final`](crate::Producer::Final) type of the consumer, `E` is the [`Error`](crate::Producer::Error) type of the producer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProduceAtLeastError<F, E> {
    /// How many items were produced.
    pub count: usize,
    /// Did producing enough items fail because the producer reached its final item, or because it yielded an error?
    pub reason: Either<F, E>,
}

#[cfg(feature = "std")]
impl<F, E> Error for ProduceAtLeastError<F, E>
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

impl<F, E> Display for ProduceAtLeastError<F, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.reason {
            Either::Left(_) => {
                write!(f, "The producer was unable to produce sufficiently many items due to emitting its final item; it stopped after producing {} items", self.count)
            }
            Either::Right(_) => {
                write!(f, "The producer was unable to produce sufficiently many items due to an error; it stopped after producing {} items", self.count)
            }
        }
    }
}

impl<F, E> ProduceAtLeastError<F, E> {
    /// Consumes `self` and returns `self.reason`, effectively discarding `self.count`.
    pub fn into_reason(self) -> Either<F, E> {
        self.reason
    }
}
