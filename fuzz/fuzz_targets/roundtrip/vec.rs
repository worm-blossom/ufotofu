#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, queues::new_fixed};

// Generate a random vec, call into_producer, scramble that producer. Then use IntoConsumer on the default vec, scramble the consumer, and check that the original vec is being reconstructed.

fuzz_target!(|data: (
    Vec<u16>,
    usize,
    Vec<BulkConsumerOperation>,
    usize,
    Vec<ProducerOperation>,
)| {
    pollster::block_on(async {
        let (input_vec, mut con_buffer_size, con_ops, mut pro_buffer_size, pro_ops) = data;
        con_buffer_size = con_buffer_size.clamp(1, 4096);
        pro_buffer_size = pro_buffer_size.clamp(1, 4096);

        let vec_copy = input_vec.clone();

        let mut pro = input_vec
            .into_producer()
            .scrambled(new_fixed(pro_buffer_size), pro_ops);

        let mut produced = vec![0; vec_copy.len()];
        assert_eq!(pro.overwrite_full_slice(&mut produced[..]).await, Ok(()));

        assert_eq!(&produced[..], &vec_copy[..]);

        let mut con = vec![]
            .into_consumer()
            .bulk_scrambled(new_fixed(con_buffer_size), con_ops);

        assert_eq!(con.consume_full_slice(&produced[..]).await, Ok(()));
        con.flush().await.unwrap();

        let consumed: Vec<u16> = con.into_inner().into();

        assert_eq!(consumed, vec_copy);
    });
});
