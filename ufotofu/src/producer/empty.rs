use core::fmt::Debug;
use core::marker::PhantomData;

use either::Either::{self, Right};

use crate::producer::Invariant;
use crate::{BufferedProducer, BulkProducer, Producer};

invarianted_producer_outer_type!(
    /// Produces nothing but a single, final item.
    Empty_ Empty <I, F, E>
);

invarianted_impl_debug!(Empty_<I, F: Debug, E>);

impl<I, F, E> Empty_<I, F, E> {
    /// Creates a producer that produces a final item and nothing else.
    ///
    /// ```
    /// use either::Either::*;
    /// use ufotofu::producer::*;
    /// use ufotofu::*;
    ///
    /// let mut empty = Empty::<bool, u8, f32>::new(17);
    ///
    /// pollster::block_on(async {
    ///     assert_eq!(Ok(Right(17)), empty.produce().await);
    /// });
    /// ```
    pub fn new(fin: F) -> Empty_<I, F, E> {
        let invariant = Invariant::new(Empty {
            fin: Some(fin),
            phantom: PhantomData,
        });
        Empty_(invariant)
    }
}

invarianted_impl_producer!(Empty_<I, F, E> Item I;
    Final F;
    Error E
);
invarianted_impl_buffered_producer!(Empty_<I, F, E>);
invarianted_impl_bulk_producer!(Empty_<I, F, E>);

struct Empty<I, F, E> {
    fin: Option<F>,
    phantom: PhantomData<(I, E)>,
}

impl<I, F: Debug, E> Debug for Empty<I, F, E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Empty").field("fin", &self.fin).finish()
    }
}

impl<I, F, E> Producer for Empty<I, F, E> {
    type Item = I;
    type Final = F;
    type Error = E;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        Ok(Right(self.fin.take().expect(
            "Must not use a producer after it has emitted its final item.",
        )))
    }
}

impl<I, F, E> BufferedProducer for Empty<I, F, E> {
    async fn slurp(&mut self) -> Result<(), Self::Error> {
        // There are no effects to perform so we simply return.
        Ok(())
    }
}

impl<I, F, E> BulkProducer for Empty<I, F, E> {
    async fn expose_items<'b>(
        &'b mut self,
    ) -> Result<Either<&'b [Self::Item], Self::Final>, Self::Error>
    where
        Self::Item: 'b,
    {
        Ok(Right(self.fin.take().expect(
            "Must not use a producer after it has emitted its final item.",
        )))
    }

    async fn consider_produced(&mut self, _amount: usize) -> Result<(), Self::Error> {
        Ok(())
    }
}
