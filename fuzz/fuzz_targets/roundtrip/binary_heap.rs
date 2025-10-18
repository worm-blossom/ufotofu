#![no_main]

use std::collections::BinaryHeap;

use libfuzzer_sys::fuzz_target;
use ufotofu::{prelude::*, queues::new_fixed};

// Generate a random binary heap, call into_producer, scramble that producer. Then use IntoConsumer on the default binary heap, scramble the consumer, and check that the original heap is being reconstructed.

fuzz_target!(|data: (
    BinaryHeap<u16>,
    usize,
    Vec<ConsumerOperation>,
    usize,
    Vec<ProducerOperation>,
)| {
    pollster::block_on(async {
        let (input, mut con_buffer_size, con_ops, mut pro_buffer_size, pro_ops) = data;
        con_buffer_size = con_buffer_size.clamp(1, 4096);
        pro_buffer_size = pro_buffer_size.clamp(1, 4096);

        let mut input_copy = input.clone();

        let mut pro = input
            .into_producer()
            .scrambled(new_fixed(pro_buffer_size), pro_ops);

        let mut produced = vec![0; input_copy.len()];
        assert_eq!(pro.overwrite_full_slice(&mut produced[..]).await, Ok(()));

        let mut con = BinaryHeap::default()
            .into_consumer()
            .scrambled(new_fixed(con_buffer_size), con_ops);

        assert_eq!(con.consume_full_slice(&produced[..]).await, Ok(()));
        con.flush().await.unwrap();

        let mut consumed: BinaryHeap<u16> = con.into_inner().into();

        // assert_eq!(consumed, input_copy);

        loop {
            let (res, control_res) = (consumed.pop(), input_copy.pop());
            assert_eq!(res, control_res);

            if res.is_none() {
                break;
            }
        }
    });
});
