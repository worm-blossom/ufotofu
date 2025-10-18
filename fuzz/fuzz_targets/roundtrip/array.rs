#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, queues::new_fixed};

// Generate a random array, call into_producer, scramble that producer. Then use IntoConsumer on a new array, scramble the consumer, and check that the original array contents are being reconstructed

fuzz_target!(|data: (
    [u16; 12],
    usize,
    Vec<BulkConsumerOperation>,
    usize,
    Vec<BulkProducerOperation>,
)| {
    pollster::block_on(async {
        let (arr, mut con_buffer_size, con_ops, mut pro_buffer_size, pro_ops) = data;
        con_buffer_size = con_buffer_size.clamp(1, 4096);
        pro_buffer_size = pro_buffer_size.clamp(1, 4096);

        let arr_copy = arr;

        let mut pro = arr
            .into_producer()
            .bulk_scrambled(new_fixed(pro_buffer_size), pro_ops);

        let mut produced = [0; 12];
        assert_eq!(pro.overwrite_full_slice(&mut produced[..]).await, Ok(()));

        assert_eq!(produced, arr_copy);

        let mut con = [0; 12]
            .into_consumer()
            .bulk_scrambled(new_fixed(con_buffer_size), con_ops);

        assert_eq!(con.consume_full_slice(&produced[..]).await, Ok(()));
        con.flush().await.unwrap();

        let consumed: [u16; 12] = con.into_inner().into();

        assert_eq!(consumed, arr_copy);
    });
});
