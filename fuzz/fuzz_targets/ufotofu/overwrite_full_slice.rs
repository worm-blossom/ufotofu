#![no_main]

use either::Either::{Left, Right};
use libfuzzer_sys::fuzz_target;
use ufotofu::{producer::TestProducer, ProduceAtLeastError, Producer};

fuzz_target!(|data: (TestProducer<u16, u16, u16>, usize)| {
    pollster::block_on(async {
        let (mut pro, len) = data;
        if len < 8192 {
            let mut slice = vec![0; len];

            let expected_items = pro.remaining().to_vec();
            let expected_last = *pro.peek_last().unwrap();

            match pro.overwrite_full_slice(&mut slice[..]).await {
                Ok(()) => {
                    assert!(!pro.did_already_emit_last());
                    assert_eq!(&slice[..], &expected_items[..len]);
                }
                Err(ProduceAtLeastError {
                    count,
                    reason,
                }) => {
                    assert!(pro.did_already_emit_last());

                    match expected_last {
                        Ok(fin) => assert_eq!(reason, Left(fin)),
                        Err(err) => assert_eq!(reason, Right(err)),
                    }

                    assert_eq!(&slice[..count], &expected_items[..count]);
                }
            }
        }
    });
});
