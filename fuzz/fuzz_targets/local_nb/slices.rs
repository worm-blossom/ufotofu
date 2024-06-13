#![no_main]

use core::cmp::min;

use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::{arbitrary, arbitrary::Arbitrary};

use ufotofu::local_nb;
use ufotofu::local_nb::consumer::SliceConsumer;
use ufotofu::local_nb::producer::SliceProducer;

#[derive(Debug, Clone, Arbitrary)]
struct TestData {
    input_buf: Box<[u8]>,
    output_buf: Box<[u8]>,
    input_start: usize,
    input_end: usize,
    output_start: usize,
    output_end: usize,
}

// Filter out any data conditions which would violate invariants that must be
// upheld when calling our functions.
fn data_is_invalid(data: TestData) -> bool {
    if data.input_start > data.input_end {
        return true;
    } else if data.input_end > data.input_buf.len() {
        return true;
    } else if data.output_start > data.output_end {
        return true;
    } else if data.output_end > data.output_buf.len() {
        return true;
    } else {
        false
    }
}

fn fuzz_pipe(mut data: TestData) {
    smol::block_on(async {
        if data_is_invalid(data.clone()) {
            return;
        }

        let mut o = SliceProducer::new(&data.input_buf[data.input_start..data.input_end]);
        let mut i = SliceConsumer::new(&mut data.output_buf[data.output_start..data.output_end]);

        match local_nb::pipe(&mut o, &mut i).await {
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
    })
}

fn fuzz_bulk_pipe(mut data: TestData) {
    smol::block_on(async {
        if data_is_invalid(data.clone()) {
            return;
        }

        let mut o = SliceProducer::new(&data.input_buf[data.input_start..data.input_end]);
        let mut i = SliceConsumer::new(&mut data.output_buf[data.output_start..data.output_end]);

        match local_nb::bulk_pipe(&mut o, &mut i).await {
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
    })
}

fuzz_target!(|data_origin: (TestData, TestData)| {
    let data = data_origin.0.clone();
    fuzz_pipe(data);

    let data = data_origin.1.clone();
    fuzz_bulk_pipe(data);
});
