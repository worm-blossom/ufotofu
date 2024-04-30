#![no_main]
#![feature(never_type)]

use libfuzzer_sys::fuzz_target;

use ufotofu::sync;
use ufotofu::sync::consumer::{IntoVecError, IntoVecFallible};
use ufotofu::sync::producer::Cursor as ProducerCursor;

fn fuzz_pipe(data: &[u8]) {
    let mut o = ProducerCursor::new(&data[..]);
    let mut i = IntoVecFallible::new();

    let _ = sync::pipe::<_, _, IntoVecError>(&mut o, &mut i);

    assert_eq!(&i.into_vec(), &data[..]);
}

fn fuzz_bulk_pipe(data: &[u8]) {
    let mut o = ProducerCursor::new(&data[..]);
    let mut i = IntoVecFallible::new();

    let _ = sync::bulk_pipe::<_, _, IntoVecError>(&mut o, &mut i);

    assert_eq!(&i.into_vec(), &data[..]);
}

fuzz_target!(|data: &[u8]| {
    fuzz_pipe(data);
    fuzz_bulk_pipe(data);
});
