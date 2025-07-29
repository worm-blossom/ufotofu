use alloc::boxed::Box;
use core::{future::Future, pin::Pin};

use either::Either::{self, Right};

use futures::future::select;

use crate::Producer;

/// The [`Merge`] adaptor wraps two producers and interleaves their items, drops the first `Final` item, but forwards the first `Error`.
pub struct Merge<P1: Producer, P2: Producer> {
    produce_future1:
        Option<Pin<Box<dyn Future<Output = (Result<Either<P1::Item, P1::Final>, P1::Error>, P1)>>>>,
    produce_future2:
        Option<Pin<Box<dyn Future<Output = (Result<Either<P2::Item, P2::Final>, P2::Error>, P2)>>>>,
}

impl<P1: Producer + 'static, P2: Producer + 'static> Merge<P1, P2> {
    /// Returns a producer which wraps two producers and interleaves their items, drops the first `Final` item, but forwards the first `Error`.
    pub fn new(mut p1: P1, mut p2: P2) -> Self {
        Merge {
            produce_future1: Some(Box::pin(async move { (p1.produce().await, p1) })),
            produce_future2: Some(Box::pin(async move { (p2.produce().await, p2) })),
        }
    }
}

impl<
        P1: Producer<Item = Item, Final = Final, Error = Error> + 'static,
        P2: Producer<Item = Item, Final = Final, Error = Error> + 'static,
        Item: 'static,
        Final: 'static,
        Error: 'static,
    > Producer for Merge<P1, P2>
{
    type Item = Item;
    type Final = Final;
    type Error = Error;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        match (self.produce_future1.take(), self.produce_future2.take()) {
            (None, None) => panic!("Must not call produce after a producer emitted its final item"),
            (Some(fut), None) => {
                let (result, mut p1) = fut.await;
                self.produce_future1 = Some(Box::pin(async move { (p1.produce().await, p1) }));
                return result;
            }
            (None, Some(fut)) => {
                let (result, mut p2) = fut.await;
                self.produce_future2 = Some(Box::pin(async move { (p2.produce().await, p2) }));
                return result;
            }
            (Some(fut1), Some(fut2)) => match select(fut1, fut2).await {
                futures::future::Either::Left(((result, mut p1), fut2)) => {
                    self.produce_future1 = Some(Box::pin(async move { (p1.produce().await, p1) }));
                    self.produce_future2 = Some(Box::pin(fut2));
                    if let Ok(Right(_fin)) = result {
                        return self.produce().await;
                    } else {
                        return result;
                    }
                }
                futures::future::Either::Right(((result, mut p2), fut1)) => {
                    self.produce_future2 = Some(Box::pin(async move { (p2.produce().await, p2) }));
                    self.produce_future1 = Some(Box::pin(fut1));
                    if let Ok(Right(_fin)) = result {
                        return self.produce().await;
                    } else {
                        return result;
                    }
                }
            },
        }
    }
}

// impl<B, P: BufferedProducer, F: FnMut(P::Item) -> B> BufferedProducer for Merge<P, F> {
//     async fn slurp(&mut self) -> Result<(), Self::Error> {
//         self.inner.slurp().await
//     }
// }

// use alloc::boxed::Box;
// use core::{future::Future, pin::Pin};

// use either::Either::{self, Left, Right};

// use futures::future::select;

// use crate::{maybe_future::MaybeFuture, BufferedProducer, Producer};

// /// The [`Merge`] adaptor wraps two producers and interleaves their items, drops the first `Final` item, but forwards the first `Error`.
// pub struct Merge<'p, P1: Producer, P2: Producer> {
//     p1: &'p mut P1,
//     p2: &'p mut P2,
//     produce_future1:
//         Option<Pin<Box<dyn Future<Output = Result<Either<P1::Item, P1::Final>, P1::Error>> + 'p>>>,
//     produce_future2:
//         Option<Pin<Box<dyn Future<Output = Result<Either<P2::Item, P2::Final>, P2::Error>> + 'p>>>,
// }

// // impl<P: core::fmt::Debug, F> core::fmt::Debug for Merge<P, F> {
// //     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
// //         f.debug_struct("Merge")
// //             .field("inner", &self.inner)
// //             .finish()
// //     }
// // }

// impl<'p, P1: Producer, P2: Producer> Merge<'p, P1, P2> {
//     /// Returns a producer which wraps two producers and interleaves their items, drops the first `Final` item, but forwards the first `Error`.
//     pub fn new(p1: &'p mut P1, p2: &'p mut P2) -> Self {
//         Merge {
//             produce_future1: Some(Box::pin(p1.produce())),
//             produce_future2: Some(Box::pin(p2.produce())),
//             p1: p1,
//             p2: p2,
//         }
//     }
// }

// impl<
//         'p,
//         P1: Producer<Item = Item, Final = Final, Error = Error>,
//         P2: Producer<Item = Item, Final = Final, Error = Error>,
//         Item,
//         Final,
//         Error,
//     > Producer for Merge<'p, P1, P2>
// {
//     type Item = Item;
//     type Final = Final;
//     type Error = Error;

//     async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
//         match (&mut self.produce_future1, &mut self.produce_future2) {
//             (None, None) => panic!("Must call produce after a producer emitted its final item"),
//             (Some(fut), None) | (None, Some(fut)) => fut.await,
//             (Some(fut1), Some(fut2)) => {
//                 todo!()
//             }
//         }
//     }
// }

// // impl<B, P: BufferedProducer, F: FnMut(P::Item) -> B> BufferedProducer for Merge<P, F> {
// //     async fn slurp(&mut self) -> Result<(), Self::Error> {
// //         self.inner.slurp().await
// //     }
// // }
