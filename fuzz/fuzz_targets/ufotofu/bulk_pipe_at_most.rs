#![no_main]

use libfuzzer_sys::fuzz_target;
use ufotofu::{bulk_pipe_at_most, consumer::TestConsumer, pipe_at_most, producer::TestProducer};

fuzz_target!(|data: (
    TestConsumer<u16, u16, u16>,
    TestProducer<u16, u16, u16>,
    usize
)| {
    pollster::block_on(async {
        let (mut con, mut pro, count) = data;
        let mut control_con = con.clone();
        let mut control_pro = pro.clone();

        assert_eq!(
            bulk_pipe_at_most(&mut pro, &mut con, count).await,
            pipe_at_most(&mut control_pro, &mut control_con, count).await,
        );
        assert_eq!(con, control_con);
        assert_eq!(pro, control_pro);
    });
});
