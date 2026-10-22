//! TEMPORARY PROBE — measure the native-seek accounting numbers.
use commands::catalog::types::SourceSpec;
use commands::envelope::RequestId;
use commands::{Command, PlaybackState, WireRequest};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::CliHost;

fn parent() -> SourceSpec {
    SourceSpec::File {
        path: format!(
            "{}/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi",
            env!("CARGO_MANIFEST_DIR")
        )
        .into(),
    }
}

fn dispatch_open(host: &mut CliHost, source: SourceSpec) {
    let response = lumen_dispatch(
        host,
        &WireRequest {
            request_id: RequestId(1),
            command: Command::SessionOpen {
                source,
                options: Default::default(),
            },
        },
    );
    assert!(response.result.is_ok());
    host.settle();
}

#[test]
fn probe_native_seek_accounting() {
    let mut host = CliHost::new();
    dispatch_open(&mut host, parent());
    let session_before = host.session_id();
    let gen_before = host.generation();
    let pre = host.headless_accounting().unwrap();
    println!("PRE-SEEK: {:?}", pre);
    let response = lumen_dispatch(
        &mut host,
        &WireRequest {
            request_id: RequestId(2),
            command: Command::PlaybackSeek {
                target: commands::catalog::types::SeekTarget::Percent { fraction: 0.5 },
            },
        },
    );
    assert!(response.result.is_ok());
    let mid = host.headless_accounting().unwrap();
    println!("POST-SEEK (pre-advance): {:?}", mid);
    println!("session {:?} -> {:?}, gen {:?} -> {:?}", session_before, host.session_id(), gen_before, host.generation());
    println!("state after seek: {:?}", host.state());
    println!("position after seek: {}", host.position_us());
    println!("trail: {:?}", host.state_history());
    for e in host.events().iter().rev().take(12) {
        println!("  event: {:?}", e.kind);
    }
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(host.state(), PlaybackState::Ended);
    let post = host.headless_accounting().unwrap();
    println!("POST-RUN: {:?}", post);
    println!("position at end: {}", host.position_us());
    let idle_opens = host.state_history().iter().filter(|(a, b)| *a == PlaybackState::Idle && *b == PlaybackState::Opening).count();
    println!("idle->opening count: {}", idle_opens);
}
