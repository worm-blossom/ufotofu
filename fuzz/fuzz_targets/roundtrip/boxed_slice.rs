#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, queues::new_fixed};

// Generate a random boxed slice, call into_producer, scramble that producer. Then use IntoConsumer on a new owned slice of equal size, scramble the consumer, and check that the original slice contents are being reconstructed.

fuzz_target!(|data: (
    Box<[u16]>,
    usize,
    Vec<BulkConsumerOperation>,
    usize,
    Vec<ProducerOperation>,
)| {
    pollster::block_on(async {
        let (slice, mut con_buffer_size, con_ops, mut pro_buffer_size, pro_ops) = data;
        con_buffer_size = con_buffer_size.clamp(1, 4096);
        pro_buffer_size = pro_buffer_size.clamp(1, 4096);

        let slice_copy = slice.clone();

        let mut pro = slice
            .into_producer()
            .scrambled(new_fixed(pro_buffer_size), pro_ops);

        let mut produced = vec![0; slice_copy.len()];
        assert_eq!(pro.overwrite_full_slice(&mut produced[..]).await, Ok(()));

        assert_eq!(&produced[..], &slice_copy[..]);

        let mut con = vec![0; slice_copy.len()]
            .into_boxed_slice()
            .into_consumer()
            .bulk_scrambled(new_fixed(con_buffer_size), con_ops);

        assert_eq!(con.consume_full_slice(&produced[..]).await, Ok(()));
        con.flush().await.unwrap();

        let consumed: Box<[u16]> = con.into_inner().into();

        assert_eq!(consumed, slice_copy);
    });
});
