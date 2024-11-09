#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::sync::{producer::TestProducer, BulkProducer, Producer};

fuzz_target!(|data: (TestProducer<u16, u16, u16>, usize)| {
    let (mut pro, len) = data;

    if len < 8192 {
        let mut control = pro.clone();

        let mut slice = vec![0; len];
        let mut control_slice = vec![0; len];

        assert_eq!(
            pro.bulk_overwrite_full_slice(&mut slice[..]),
            control.overwrite_full_slice(&mut control_slice[..])
        );
        assert_eq!(pro, control);
    }
});
