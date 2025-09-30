use core::convert::Infallible;
use core::fmt::Debug;
use core::marker::PhantomData;

use crate::prelude::*;

/// A (bulk) consumer that always closes succesfully but cannot consume anything.
///
/// <br/>Counterpart: the [`producer::Empty`] type.
#[derive(Debug, Clone, Copy)]

pub struct Full<T>(PhantomData<T>);

/// Creates a consumer that can be closed but cannot consume anything.
///
/// ```
/// # use ufotofu::prelude::*;
/// use consumer::full;
/// # pollster::block_on(async {
///
/// let mut c = full();
/// c.close(17).await?;
/// # Result::<(), Infallible>::Ok(())
/// # });
/// ```
///
/// <br/>Counterpart: the [producer::empty] function.
pub fn full<T>() -> Full<T> {
    Full(PhantomData)
}

impl<T> Consumer for Full<T> {
    type Item = Infallible;
    type Final = T;
    type Error = Infallible;

    async fn consume(&mut self, _item: Self::Item) -> Result<(), Self::Error> {
        unreachable!()
    }

    async fn close(&mut self, _fin: Self::Final) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T> BulkConsumer for Full<T> {
    async fn bulk_consume(&mut self, _buf: &[Self::Item]) -> Result<usize, Self::Error> {
        panic!("Must not call bulk_consume with an empty buffer") // impossible to create a non-empty buffer of Infallibles
    }
}
