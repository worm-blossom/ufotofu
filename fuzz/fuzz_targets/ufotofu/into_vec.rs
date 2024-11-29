#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{consumer::IntoVec, BufferedConsumer, Consumer};

fuzz_target!(|data: (Box<[u8]>, usize)| {
    pollster::block_on(async {
        let (input, capacity) = data;
        let capacity = core::cmp::min(capacity, 2048);

        let mut into_vec = IntoVec::with_capacity(capacity);
        // TODO scramble the consume operations

        for item in &input {
            assert!(into_vec.consume(*item).await.is_ok());
        }
        assert!(into_vec.flush().await.is_ok()); // To flush intermediate buffers of the scrambler.

        let collected = into_vec.into_vec();
        assert!(collected.capacity() >= capacity);
        assert_eq!(&input[..], &collected[..]);
    });
});
