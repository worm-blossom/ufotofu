#![no_main]
#![feature(never_type)]

use core::cmp::min;

use libfuzzer_sys::fuzz_target;

use ufotofu::sync;
use ufotofu::sync::consumer::{IntoVecError, IntoVecFallible};
use ufotofu::sync::producer::Cursor as ProducerCursor;

fn fuzz_pipe(data: Box<[u8]>) {
    let mut o = ProducerCursor::new(&data[..]);
    let mut i = IntoVecFallible::new();

    match sync::pipe::<_, _, IntoVecError>(&mut o, &mut i) {
        Ok(_) => {
            if &o.as_ref().len() > &i.as_ref().len() {
                panic!()
            }
        }
        Err(_) => {
            if &o.as_ref().len() <= &i.as_ref().len() {
                panic!()
            }
        }
    }

    let m = min(o.as_ref().len(), i.as_ref().len());
    assert_eq!(&i.as_ref()[..m], &o.as_ref()[..m]);
}

fn fuzz_bulk_pipe(data: Box<[u8]>) {
    let mut o = ProducerCursor::new(&data[..]);
    let mut i = IntoVecFallible::new();

    match sync::bulk_pipe::<_, _, IntoVecError>(&mut o, &mut i) {
        Ok(_) => {
            if &o.as_ref().len() > &i.as_ref().len() {
                panic!()
            }
        }
        Err(_) => {
            if &o.as_ref().len() <= &i.as_ref().len() {
                panic!()
            }
        }
    }

    let m = min(o.as_ref().len(), i.as_ref().len());
    assert_eq!(&i.as_ref()[..m], &o.as_ref()[..m]);
}

fuzz_target!(|data: (Box<[u8]>, Box<[u8]>)| {
    fuzz_pipe(data.0);
    fuzz_bulk_pipe(data.1);
});
