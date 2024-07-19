#![no_main]

use either::{Left, Right};
use libfuzzer_sys::fuzz_target;
use ufotofu::sync::consumer::{pipe_from_slice, TestConsumer};
use ufotofu::sync::BufferedConsumer;

fuzz_target!(|data: (TestConsumer<u16, u8, u8>, Vec<u16>)| {
    let (mut consumer, mut items) = data;

    match pipe_from_slice(&mut items, &mut consumer) {
        Ok(_) => {
            // Were the bytes successfully piped?
            consumer.flush();

            assert_eq!(
                consumer.as_ref(),
                items.as_slice(),
                "Bytes piped into slice did not match!"
            );
        }
        Err(err) => {
            // let consumer_data = consumer.peek_slice();
            consumer.flush();

            // If the filled property is not equal to the inner slice, panic.
            // assertion macro
            if *consumer.as_ref() == items[0..err.consumed] {
                panic!("Consumed was not the same as the consumer slice.")
            }

            // Do we need to check if the error produced was the right kind? e.g. by inspecting the consumer's termination field?
        }
    }
});
