#![no_main]

use std::num::NonZeroUsize;

use either::Either::{Left, Right};
use libfuzzer_sys::fuzz_target;
use ufotofu::{
    producer::{Merge, TestProducerBuilder},
    Producer,
};

fuzz_target!(|data: (
    Vec<u16>,
    Result<i16, i16>,
    Box<[NonZeroUsize]>,
    Box<[bool]>,
    Vec<u16>,
    Result<i16, i16>,
    Box<[NonZeroUsize]>,
    Box<[bool]>,
)| {
    pollster::block_on(async {
        let (
            p1_items,
            p1_last,
            p1_item_sizes,
            p1_yield_pattern,
            p2_items,
            p2_last,
            p2_item_sizes,
            p2_yield_pattern,
        ) = data;

        let p1_items_clone = p1_items.clone();
        let p2_items_clone = p2_items.clone();

        let p1_items: Vec<(u16, bool)> = p1_items.into_iter().map(|item| (item, true)).collect();
        let p2_items: Vec<(u16, bool)> = p2_items.into_iter().map(|item| (item, false)).collect();

        let p1 = TestProducerBuilder::new(p1_items.into_boxed_slice(), p1_last)
            .exposed_items_sizes(p1_item_sizes)
            .yield_pattern(p1_yield_pattern)
            .build();
        let p2 = TestProducerBuilder::new(p2_items.into_boxed_slice(), p2_last)
            .exposed_items_sizes(p2_item_sizes)
            .yield_pattern(p2_yield_pattern)
            .build();

        let mut merged = Merge::new(p1, p2);

        let mut offset_1 = 0;
        let mut offset_2 = 0;

        loop {
            match merged.produce().await {
                Ok(Left((item, from_first_producer))) => {
                    if from_first_producer {
                        assert_eq!(p1_items_clone[offset_1], item);
                        offset_1 += 1;
                    } else {
                        assert_eq!(p2_items_clone[offset_2], item);
                        offset_2 += 1;
                    }
                }
                Ok(Right(fin)) => {
                    assert_eq!(offset_1, p1_items_clone.len());
                    assert_eq!(offset_2, p2_items_clone.len());
                    assert!(Ok(fin) == p1_last || Ok(fin) == p2_last);
                    return;
                }
                Err(err) => {
                    assert!(Err(err) == p1_last || Err(err) == p2_last);
                    return;
                }
            }
        }
    });
});
