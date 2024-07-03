#![no_main]

use core::mem::MaybeUninit;
use either::{Left, Right};
use libfuzzer_sys::fuzz_target;
use ufotofu::local_nb::producer::{bulk_pipe_into_slice_uninit, TestProducer};

fuzz_target!(|data: (TestProducer<u16, u8, u8>, usize)| {
    let (mut producer, length) = data;

    let clamped_length = length.clamp(0, 512);

    let mut v: Vec<u16> = vec![0; clamped_length];

    let producer_data = producer.peek_slice().to_vec();

    let producer_data_maybe_uninit =
        unsafe { &mut *(&mut v[0..clamped_length] as *mut [u16] as *mut [MaybeUninit<u16>]) };

    async {
        match bulk_pipe_into_slice_uninit(producer_data_maybe_uninit, &mut producer).await {
            Ok(_) => {
                // Were the bytes successfully piped?

                // refactor as assert_eq
                if producer_data[0..clamped_length] != v[0..clamped_length] {
                    panic!("Bytes piped into slice did not match!")
                }
            }
            Err(err) => {
                // let producer_data = producer.peek_slice();

                // If the filled property is not equal to the inner slice, panic.
                // assertion macro
                if err.filled != producer_data {
                    panic!("Filled was not the same as the producer slice.")
                }

                // Do we need to check if the error produced was the right kind? e.g. by inspecting the producer's termination field?
            }
        }
    };
});
