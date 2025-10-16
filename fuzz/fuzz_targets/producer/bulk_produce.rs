#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::prelude::*;

// Generate a TestProducer, clone it, and then check that a bulk_produce call agrees with the corresponding individual produce calls on the clone.

fuzz_target!(|data: (TestProducer<u16, u16, u16>, usize)| {
    pollster::block_on(async {
        let (mut pro, mut slice_len) = data;
        slice_len = slice_len.clamp(0, 2048);
        let mut control = pro.clone();

        let mut slice = vec![17; slice_len];

        match pro.bulk_produce(&mut slice[..]).await {
            Ok(Left(amount)) => {
                assert!(slice_len == 0 || amount > 0);

                for i in 0..amount {
                    assert_eq!(control.produce().await, Ok(Left(slice[i])));
                }
                if amount < slice_len {
                    assert!(slice[amount] == 17);
                }
            }
            Ok(Right(fin)) => {
                assert_eq!(control.produce().await, Ok(Right(fin)));
                if slice_len > 0 {
                    slice[0] = 17;
                }
            }
            Err(err) => {
                assert_eq!(control.produce().await, Err(err));
                if slice_len > 0 {
                    slice[0] = 17;
                }
            }
        }
    });
});
