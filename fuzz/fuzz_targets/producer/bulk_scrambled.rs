#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, queues::new_fixed};

// Generate a TestProducer, turn it into a bulk scrambler, and test that the resulting producer produces the same sequence as the original producer.

fuzz_target!(|data: (
    TestProducer<u16, u16, u16>,
    usize,
    Vec<BulkProducerOperation>
)| {
    pollster::block_on(async {
        let (pro, mut buffer_size, ops) = data;
        buffer_size = buffer_size.clamp(1, 4096);

        let mut original = pro.clone();
        let mut scrambled = pro.bulk_scrambled(new_fixed(buffer_size), ops);

        assert!(scrambled.equals(&mut original).await);
    });
});
