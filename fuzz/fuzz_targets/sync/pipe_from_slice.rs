#![no_main]

use either::{Left, Right};
use libfuzzer_sys::fuzz_target;
use ufotofu::sync::consumer::{pipe_from_slice, TestConsumer};

fuzz_target!(|data: (TestConsumer<u16, u8, u8>, Vec<u16>)| {
    let (mut producer, mut items) = data;

    println!("{:?}", producer.as_ref());

    match pipe_from_slice(&mut items, &mut producer) {
        Ok(_) => {
            // Were the bytes successfully piped?

            // refactor as assert_eq
            if producer.as_ref() != items.as_slice() {
                panic!("Bytes piped into slice did not match!")
            }
        }
        Err(err) => {
            // let producer_data = producer.peek_slice();

            // If the filled property is not equal to the inner slice, panic.
            // assertion macro
            if *producer.as_ref() == items[0..err.consumed] {
                panic!("Consumed was not the same as the producer slice.")
            }

            // Do we need to check if the error produced was the right kind? e.g. by inspecting the producer's termination field?
        }
    }
});
