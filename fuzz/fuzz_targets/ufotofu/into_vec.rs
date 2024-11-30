#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{
    consumer::{BulkConsumerOperation, BulkScrambler, IntoVec},
    BufferedConsumer, Consumer,
};

fuzz_target!(|data: (Box<[u8]>, usize, Vec<BulkConsumerOperation>)| {
    pollster::block_on(async {
        let (input, capacity, ops) = data;
        let capacity = core::cmp::min(capacity, 2048);

        let mut into_vec = BulkScrambler::new(IntoVec::with_capacity(capacity), ops);

        for item in &input {
            assert!(into_vec.consume(*item).await.is_ok());
        }
        assert!(into_vec.flush().await.is_ok()); // To flush intermediate buffers of the scrambler.

        let collected = into_vec.into_inner().into_vec();
        assert!(collected.capacity() >= capacity);
        assert_eq!(&input[..], &collected[..]);
    });
});
