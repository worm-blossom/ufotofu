#![no_main]

use std::num::NonZeroUsize;

use either::Either::{Left, Right};
use libfuzzer_sys::fuzz_target;
use ufotofu::local_nb::{producer::TestProducer, BulkProducer, Producer};

fuzz_target!(|data: (TestProducer<u16, u16, u16>, Box<[NonZeroUsize]>)| {
    smol::block_on(async {
        let (mut pro, slice_lengths) = data;
        let mut control = pro.clone();

        let num_items = pro.remaining().len();

        let mut produced = vec![0; num_items];
        let mut produced_control = vec![];

        // Pit individual produce calls against bulk produce calls, assert equality of the results and the producer states.

        let mut produced_so_far = 0;
        let mut slice_length_index = 0;

        while slice_length_index < slice_lengths.len() {
            let slice_length: usize = core::cmp::min(
                slice_lengths[slice_length_index].into(),
                num_items - produced_so_far,
            );

            if slice_length == 0 {
                break;
            }

            match pro.bulk_produce(&mut produced[produced_so_far..]).await {
                Ok(Left(num_consumed)) => {
                    for _ in 0..num_consumed {
                        let control_item = control.produce().await.unwrap().unwrap_left();
                        produced_control.push(control_item);
                        produced_so_far += 1;
                    }
                }
                Ok(Right(fin)) => {
                    assert_eq!(fin, control.produce().await.unwrap().unwrap_right());
                    break;
                }
                Err(err) => {
                    assert_eq!(err, control.produce().await.unwrap_err());
                    break;
                }
            }

            slice_length_index += 1;
            assert_eq!(&produced[..produced_so_far], &produced_control[..]);
            assert_eq!(pro, control);
        }

        assert_eq!(&produced[..produced_so_far], &produced_control[..]);
        assert_eq!(pro, control);
    });
});
