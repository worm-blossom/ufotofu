#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, queues::new_fixed};

// Check that a CloneFromOwnedSlice produces the same sequence as the IntoProducer of Vec for the same data.

fuzz_target!(|data: (Vec<u16>, usize, Vec<BulkProducerOperation>)| {
    pollster::block_on(async {
        let (slice_data, mut scrambler_buffer_size, ops) = data;
        scrambler_buffer_size = scrambler_buffer_size.clamp(1, 4096);

        let mut control = slice_data.clone().into_producer();
        let mut scrambled = producer::clone_from_owned_slice(slice_data)
            .bulk_scrambled(new_fixed(scrambler_buffer_size), ops);

        assert!(scrambled.equals(&mut control).await);
    });
});
