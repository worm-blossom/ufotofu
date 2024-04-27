#![no_main]
#![feature(never_type)]

use libfuzzer_sys::fuzz_target;

use ufotofu::sync;
use ufotofu::sync::consumer::IntoVec;
use ufotofu::sync::producer::Cursor as ProducerCursor;

fn fuzz_pipe(data: Box<[u8]>) {
    let mut o = ProducerCursor::new(&data[..]);
    let mut i = IntoVec::new();

    let _ = sync::pipe::<_, _, !>(&mut o, &mut i);

    assert_eq!(&i.into_vec(), &data[..]);
}

fn fuzz_bulk_pipe(data: Box<[u8]>) {
    let mut o = ProducerCursor::new(&data[..]);
    let mut i = IntoVec::new();

    let _ = sync::bulk_pipe::<_, _, !>(&mut o, &mut i);

    assert_eq!(&i.into_vec(), &data[..]);
}

fuzz_target!(|data: (Box<[u8]>, Box<[u8]>)| {
    fuzz_pipe(data.0);
    fuzz_bulk_pipe(data.1);
});
