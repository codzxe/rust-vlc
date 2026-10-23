//! Temporary probe — measure the scripted source's read pattern (to
//! be deleted before commit).

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::Command;
use testkit::alloc_count::serialize_tests;

use crate::host::{dispatch, ScenarioHost};

#[test]
fn probe_read_pattern() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    let open = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::Url {
                url: "script://avi-parent".to_string(),
            },
            options: OpenOptions::default(),
        },
    );
    assert!(open.result.is_ok(), "{:?}", open.result);
    let (r0, b0) = host.script().unwrap().read_stats();
    eprintln!("AFTER OPEN: reads={r0} bytes={b0} state={:?}", host.state());
    host.settle();
    let (r1, b1) = host.script().unwrap().read_stats();
    eprintln!("AFTER SETTLE: reads={r1} bytes={b1} state={:?}", host.state());
    for i in 0..40 {
        host.pump_pipeline();
        if i % 8 == 0 {
            let (r, b) = host.script().unwrap().read_stats();
            eprintln!(
                "pump {i}: reads={r} bytes={b} state={:?} frames={}",
                host.state(),
                host.sink().map(|s| s.frames_accepted()).unwrap()
            );
        }
    }
    let (r2, b2) = host.script().unwrap().read_stats();
    eprintln!("AFTER 40 PUMPS: reads={r2} bytes={b2} state={:?}", host.state());
    let _ = dispatch(&mut host, Command::SessionClose);
}
