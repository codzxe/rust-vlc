mod common;

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::{Command, PlaybackState};
use first_vertical::harness::{SessionHarness, run_pumps};

#[test]
fn debug_one() {
    let rel = std::env::var("DBG_REL").unwrap_or_else(|_| "dirty/lie-023-0041-avi-video.mp4".into());
    let path = std::path::Path::new("/home/z/my-project/corpus-big").join(&rel);
    let name: &'static str = Box::leak(rel.clone().into_boxed_str());
    let mut harness = SessionHarness::new(name);
    let open = harness.dispatch(
        "session.open",
        Command::SessionOpen { source: SourceSpec::File { path }, options: OpenOptions::default() },
    );
    println!("open: {:?}", open.result.as_ref().map(|_| "ok").map_err(|e| e.to_string()));
    harness.settle();
    println!("after settle: {:?} pos {}", harness.state(), harness.position_us());
    for round in 0..40 {
        harness.advance(run_pumps());
        let s = harness.state();
        println!("round {round}: {:?} pos {} accounting {:?}", s, harness.position_us(), harness.accounting().map(|a| (a.frames_accepted, a.video_presented, a.scheduler_pushed, a.scheduler_presented)));
        if matches!(s, PlaybackState::Ended | PlaybackState::Error) { break; }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    println!("state history: {:?}", harness.state_history());
    let transcript = harness.finish("scratch").transcript_bytes();
    // print the last 40 lines
    let lines: Vec<&str> = transcript.lines().collect();
    for line in lines.iter().rev().take(40).rev() { println!("{line}"); }
}
