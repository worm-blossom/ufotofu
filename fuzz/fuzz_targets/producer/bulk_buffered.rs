#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, queues::new_fixed};

// Generate a TestProducer, bulk_buffer it, then turn it into a bulk scrambler, and test that the resulting producer produces the same sequence as the original producer.

fuzz_target!(|data: (
    TestProducer<u16, u16, u16>,
    usize,
    usize,
    Vec<BulkProducerOperation>
)| {
    pollster::block_on(async {
        let (pro, mut buffered_buffer_size, mut scrambler_buffer_size, ops) = data;
        buffered_buffer_size = buffered_buffer_size.clamp(1, 4096);
        scrambler_buffer_size = scrambler_buffer_size.clamp(1, 4096);

        let mut original = pro.clone();
        let mut scrambled = pro
            .bulk_buffered(new_fixed(buffered_buffer_size))
            .bulk_scrambled(new_fixed(scrambler_buffer_size), ops);

        assert!(scrambled.equals(&mut original).await);
    });
});
