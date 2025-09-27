use core::convert::Infallible;
use core::fmt::Debug;

use either::Either;

use crate::prelude::*;

/// A (bulk) producer that immediately yields a predetermined final value.
///
/// <br/>Counterpart: the [TODO] type.
#[derive(Debug, Clone, Copy)]

pub struct Empty<T>(Option<T>);

/// Creates a producer that returns a final value and nothing else.
///
/// ```
/// # use ufotofu::prelude::*;
/// use ufotofu::producer::empty;
/// # pollster::block_on(async {
///
/// let mut p = empty(17);
/// assert_eq!(p.produce().await?, Right(17));
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [TODO] function.
pub fn empty<T>(fin: T) -> Empty<T> {
    Empty(Some(fin))
}

impl<T> Producer for Empty<T> {
    type Item = Infallible;
    type Final = T;
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        Ok(Right(self.0.take().expect(
            "Must not call produce after having yielded the final value",
        )))
    }
}

impl<T> BulkProducer for Empty<T> {
    async fn bulk_produce(
        &mut self,
        buf: &mut [Self::Item],
    ) -> Result<Either<usize, Self::Final>, Self::Error> {
        debug_assert_ne!(
            buf.len(),
            0,
            "Must not call bulk_produce with an empty buffer."
        );

        Ok(Right(self.0.take().expect(
            "Must not call produce after having yielded the final value",
        )))
    }
}
