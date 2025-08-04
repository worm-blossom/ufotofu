use either::Either::{self, Right};

use futures::future::select;
use reusable_box_future::ReusableLocalBoxFuture;

use crate::Producer;

/// The [`Merge`] adaptor wraps two producers and interleaves their items, drops the first `Final` item, but forwards the first `Error`.
pub struct Merge<P1, P2, Item, Final, Error> {
    produce_future1:
        Option<ReusableLocalBoxFuture<(Result<Either<P1::Item, P1::Final>, P1::Error>, P1)>>,
    produce_future2:
        Option<ReusableLocalBoxFuture<(Result<Either<P2::Item, P2::Final>, P2::Error>, P2)>>,
}

impl<P1, P2, Item, Final, Error> Merge<P1, P2, Item, Final, Error>
where
    P1: Producer<Item = Item, Final = Final, Error = Error> + 'static,
    P2: Producer<Item = Item, Final = Final, Error = Error> + 'static,
    Item: 'static,
    Final: 'static,
    Error: 'static,
{
    /// Returns a producer which wraps two producers and interleaves their items, drops the first `Final` item, but forwards the first `Error`.
    pub fn new(mut p1: P1, mut p2: P2) -> Self {
        Merge {
            produce_future1: Some(ReusableLocalBoxFuture::new(async move {
                (p1.produce().await, p1)
            })),
            produce_future2: Some(ReusableLocalBoxFuture::new(async move {
                (p2.produce().await, p2)
            })),
        }
    }
}

impl<P1, P2, Item, Final, Error> Producer for Merge<P1, P2, Item, Final, Error>
where
    P1: Producer<Item = Item, Final = Final, Error = Error> + 'static,
    P2: Producer<Item = Item, Final = Final, Error = Error> + 'static,
    Item: 'static,
    Final: 'static,
    Error: 'static,
{
    type Item = Item;
    type Final = Final;
    type Error = Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        loop {
            match (self.produce_future1.as_mut(), self.produce_future2.as_mut()) {
                (None, None) => {
                    panic!("Must not call produce after a producer emitted its final item")
                }
                (Some(fut), None) => {
                    let (result, mut p1) = (&mut *fut).await;
                    fut.set(async move { (p1.produce().await, p1) });
                    return result;
                }
                (None, Some(fut)) => {
                    let (result, mut p2) = (&mut *fut).await;
                    fut.set(async move { (p2.produce().await, p2) });
                    return result;
                }
                (Some(fut1), Some(fut2)) => match select(&mut *fut1, &mut *fut2).await {
                    futures::future::Either::Left(((result, mut p1), _fut2)) => {
                        fut1.set(async move { (p1.produce().await, p1) });
                        if let Ok(Right(_fin)) = result {
                            self.produce_future1 = None;
                            // continue to next iteration of the loop, i.e., ignore this final value
                        } else {
                            return result;
                        }
                    }
                    futures::future::Either::Right(((result, mut p2), _fut1)) => {
                        fut2.set(async move { (p2.produce().await, p2) });
                        if let Ok(Right(_fin)) = result {
                            self.produce_future2 = None;
                            // continue to next iteration of the loop, i.e., ignore this final value
                        } else {
                            return result;
                        }
                    }
                },
            }
        }
    }
}
