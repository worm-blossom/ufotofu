#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, queues::new_fixed};

// Tell the fuzzer to generate a random item to repreat, and the random data we need to create a scrambled version of `SeventeenTimes`.
fuzz_target!(|data: (u16, usize, Vec<BulkProducerOperation>)| {
    pollster::block_on(async {
        let (item, mut buffer_size, ops) = data;
        buffer_size = buffer_size.clamp(1, 4096);

        let pro = seventeen_times(item);
        let mut scrambled = pro.bulk_scrambled(new_fixed(buffer_size), ops);

        for _ in 0..17 {
            assert_eq!(scrambled.produce().await, Ok(Left(item)));
        }
        assert_eq!(scrambled.produce().await, Ok(Right(())));
    });
});

struct SeventeenTimes<T> {
    already_emitted: usize,
    items: [T; 17],
}

fn seventeen_times<T: Clone>(item: T) -> SeventeenTimes<T> {
    SeventeenTimes {
        already_emitted: 0,
        items: core::array::from_fn(|_| item.clone()),
    }
}

impl<T: Clone> Producer for SeventeenTimes<T> {
    type Item = T;
    type Final = ();
    type Error = Infallible;

    async fn produce(&mut self) -> Result<Either<Self::Item, Self::Final>, Self::Error> {
        if self.already_emitted == 17 {
            return Ok(Right(()));
        } else {
            self.already_emitted += 1;
            return Ok(Left(self.items[0].clone()));
        }
    }

    async fn slurp(&mut self) -> Result<(), Self::Error> {
        return Ok(());
    }
}

impl<T: Clone> BulkProducer for SeventeenTimes<T> {
    async fn expose_items<F, R>(&mut self, f: F) -> Result<Either<R, Self::Final>, Self::Error>
    where
        F: AsyncFnOnce(&[Self::Item]) -> (usize, R),
    {
        if self.already_emitted == 17 {
            return Ok(Right(()));
        } else {
            let (amount, ret) = f(&self.items[self.already_emitted..]).await;
            self.already_emitted += amount;
            return Ok(Left(ret));
        }
    }
}
