#![no_main]

use std::num::NonZeroUsize;

use libfuzzer_sys::fuzz_target;
use ufotofu::{consumer::TestConsumer, BulkConsumer, Consumer};

fuzz_target!(
    |data: (TestConsumer<u16, u16, u16>, Box<[u16]>, Box<[NonZeroUsize]>)| {
        pollster::block_on(async {
            let (mut con, items, slice_lengths) = data;
            let mut control = con.clone();

            // Pit individual consume calls against bulk consume calls, assert equality of the results and the consumer states.

            let mut consumed_so_far = 0;
            let mut slice_length_index = 0;

            while slice_length_index < slice_lengths.len() && items[consumed_so_far..].len() > 0 {
                let slice_length: usize = core::cmp::min(
                    slice_lengths[slice_length_index].into(),
                    items.len() - consumed_so_far,
                );

                if slice_length == 0 {
                    break;
                }

                match con
                    .bulk_consume(&items[consumed_so_far..consumed_so_far + slice_length])
                    .await
                {
                    Ok(num_consumed) => {
                        for _ in 0..num_consumed {
                            assert!(control.consume(items[consumed_so_far]).await.is_ok());
                            consumed_so_far += 1;
                        }
                    }
                    Err(err) => {
                        assert_eq!(
                            err,
                            control.consume(items[consumed_so_far]).await.unwrap_err()
                        );
                        break;
                    }
                }

                slice_length_index += 1;
                assert_eq!(con, control);
            }

            assert_eq!(con, control);
        });
    }
);
