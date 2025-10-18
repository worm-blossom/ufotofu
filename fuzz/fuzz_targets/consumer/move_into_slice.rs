#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{consumer::move_into_slice, prelude::*, queues::new_fixed};

// Generate a slice of input data, create a MoveIntoSlice consumer, write the data into it, and check that at the end the data slice has been overwritten with the input data. Also do an unscrambled test that things error at the expected time.

fuzz_target!(
    |data: (usize, Vec<BulkConsumerOperation>, Box<[u16]>, usize)| {
        pollster::block_on(async {
            let (mut scrambler_buffer_size, ops, input, mut slice_len) = data;
            scrambler_buffer_size = scrambler_buffer_size.clamp(1, 4096);
            slice_len = slice_len.clamp(0, 4096);

            let mut backing_slice = vec![17; slice_len];

            let mut con = move_into_slice(&mut backing_slice[..])
                .bulk_scrambled(new_fixed(scrambler_buffer_size), ops);

            let mut did_error = false;
            for (i, x) in input.iter().enumerate() {
                match con.consume(*x).await {
                    Ok(()) => { /* no-op */ }
                    Err(()) => {
                        did_error = true;
                        assert!(i >= slice_len);
                        break;
                    }
                }
            }

            if slice_len >= input.len() {
                assert_eq!(con.flush().await, Ok(()));
                assert_eq!(&backing_slice[..input.len()], &input[..]);
            } else if !did_error {
                assert_eq!(con.flush().await, Err(()));
            }

            // And a non-scrambled test which is less thorough but has no interference from the buffer of the scrambler.

            let mut backing_slice2 = vec![17; slice_len];
            let mut con2 = move_into_slice(&mut backing_slice2[..]);

            for (i, x) in input.iter().enumerate() {
                match con2.consume(*x).await {
                    Ok(()) => { /* no-op */ }
                    Err(()) => {
                        assert_eq!(i, slice_len);
                        break;
                    }
                }
            }

            let min_len = core::cmp::min(slice_len, input.len());
            assert_eq!(&backing_slice2[..min_len], &input[..min_len]);
        });
    }
);
