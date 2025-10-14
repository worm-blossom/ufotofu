#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::prelude::*;

// Tell the fuzzer to generate `TestProducers` with `u16` Items and `()` as Final and Error.
fuzz_target!(|data: TestProducer<u16, (), ()>| {
    let mut p = data;

    // Obtain the number of items that will be produced.
    // (`p.as_slice()` returns a slice of all items which are yet to be produced.)
    let len = p.as_slice().len();

    // The `pollster` crate lets you run async code in a sync closure.
    pollster::block_on(async {
        // Call our function on the TestProducer, crash if it behaves incorrectly.
        assert_eq!(bulk_count(&mut p).await, len);
    });
});

async fn bulk_count<P: BulkProducer>(p: &mut P) -> usize {
    let mut counted = 0;

    loop {
        match p
            .expose_items(async |slots| {
                counted += slots.len();
                (slots.len(), ())
            })
            .await
        {
            Ok(Left(())) => { /* no-op, continue counting */ }
            _ => return counted,
        }
    }
}
