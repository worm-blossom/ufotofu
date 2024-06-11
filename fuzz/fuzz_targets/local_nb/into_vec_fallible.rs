#![no_main]
#![feature(never_type)]

use libfuzzer_sys::fuzz_target;

use ufotofu::local_nb;
use ufotofu::local_nb::consumer::IntoVecFallible;
use ufotofu::local_nb::producer::SliceProducer;

fn fuzz_pipe(data: &[u8]) {
    smol::block_on(async {
        let mut o = SliceProducer::new(&data[..]);
        let mut i = IntoVecFallible::new();

        let _ = local_nb::pipe(&mut o, &mut i).await;

        assert_eq!(&i.into_vec(), &data[..]);
    })
}

fn fuzz_bulk_pipe(data: &[u8]) {
    smol::block_on(async {
        let mut o = SliceProducer::new(&data[..]);
        let mut i = IntoVecFallible::new();

        let _ = local_nb::bulk_pipe(&mut o, &mut i).await;

        assert_eq!(&i.into_vec(), &data[..]);
    })
}

fuzz_target!(|data: &[u8]| {
    fuzz_pipe(data);
    fuzz_bulk_pipe(data);
});
