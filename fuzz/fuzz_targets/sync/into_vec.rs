#![no_main]
#![feature(never_type)]

use libfuzzer_sys::fuzz_target;

use ufotofu::sync;
use ufotofu::sync::consumer::IntoVec;
use ufotofu::sync::producer::SliceProducer;

fn fuzz_pipe(data: &[u8]) {
    let mut o = SliceProducer::new(&data[..]);
    let mut i = IntoVec::new();

    let _ = sync::pipe(&mut o, &mut i);

    assert_eq!(&i.into_vec(), &data[..]);
}

fn fuzz_bulk_pipe(data: &[u8]) {
    let mut o = SliceProducer::new(&data[..]);
    let mut i = IntoVec::new();

    let _ = sync::bulk_pipe(&mut o, &mut i);

    assert_eq!(&i.into_vec(), &data[..]);
}

fuzz_target!(|data: &[u8]| {
    fuzz_pipe(data);
    fuzz_bulk_pipe(data);
});
