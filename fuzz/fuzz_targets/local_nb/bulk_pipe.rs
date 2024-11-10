#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::local_nb::{bulk_pipe, consumer::TestConsumer, pipe, producer::TestProducer};

fuzz_target!(
    |data: (TestConsumer<u16, u16, u16>, TestProducer<u16, u16, u16>)| {
        smol::block_on(async {
            let (mut con, mut pro) = data;
            let mut control_con = con.clone();
            let mut control_pro = pro.clone();

            assert_eq!(
                bulk_pipe(&mut pro, &mut con).await,
                pipe(&mut control_pro, &mut control_con).await,
            );
            assert_eq!(con, control_con);
            assert_eq!(pro, control_pro);
        });
    }
);
