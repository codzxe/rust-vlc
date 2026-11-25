//! FX7-A01 — the W1-06 regression proof at the pipeline level: the
//! window-full hold must stash the CONVERTED frame, so every decoded
//! frame of an I420-bearing media becomes presentable.
//!
//! The defect (registered W1-06, closed by this wave): the hold at
//! `apps/cli/src/pipeline_host.rs` stashed the UNCONVERTED original
//! while the accept path converts I420→RGBA, so hold-retried frames
//! reached the renderer in a format it refuses
//! (`render.upload.format-unsupported` — `uploads_format` accepts only
//! Rgba8/Bgra8). The committed first-vertical transcript for this same
//! fixture pinned the loss: `scheduler_presented == 60` (the scheduler
//! selected EVERY decoded frame) against `video_presented == 16` —
//! the whole 60→16 loss sits between the scheduler and the renderer,
//! in the hold-retried refusals. The FX6 central run measured 18/60
//! (16 RGBA presents plus ≤2 repeats) — the root cause of the
//! fx6_a04 acceptance's 18/60 shortfall (FX6-R3/R4's diagnosis).
//!
//! This fixture decodes as I420: MJPEG 4:2:0 (Y 2×2, Cb/Cr 1×1 —
//! FX6-R3's SOF0 parse), mapped to `PixelFormat::I420` by the codec,
//! and its decode burst exceeds the 16-slot scheduler window, so the
//! hold arm is on this drive's path — the exact shape W1-06 named.
//! After the fix the hold stashes the converted frame, the renderer
//! takes every scheduler selection, and `video_presented` covers
//! every decoded frame — the pipeline-level 60/60 that FX6-R4's
//! loss-ledger note said awaits this fix (the desktop fx6_a04
//! acceptance's 60/60 flip is FX7-A02's row).

use commands::catalog::types::SourceSpec;
use commands::envelope::RequestId;
use commands::{Command, PlaybackState, WireRequest};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::CliHost;
use testkit::Must;

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
    match response.result {
        Ok(_) => {}
        Err(error) => unreachable!("open failed: {error}"),
    }
    host.settle();
}

/// One whole-media run's accounting (the run's own health pinned:
/// `Ended` at the exact 2.000 s duration).
fn run_to_end() -> lumen_cli::host::HeadlessAccounting {
    let mut host = CliHost::new();
    dispatch_open(&mut host, parent());
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    if host.state() != PlaybackState::Ended {
        unreachable!("the run should have ended, state: {:?}", host.state());
    }
    let position = host.position_us();
    if position != 2_000_000 {
        unreachable!("the whole media plays out, position: {position}");
    }
    host.headless_accounting().must()
}

/// THE W1-06 regression: a 4:2:0/I420-bearing media driven through the
/// whole pipeline — every decoded frame becomes presentable. The
/// decode-count oracle first (the same one
/// apps/cli/tests/pipeline_accounting.rs pins for this fixture: 60
/// video frames + 60 audio packets delivered, all 60 admitted to the
/// scheduling window, nothing refused, nothing late), then the
/// present-path closure the defect broke:
///
/// * `video_presented == scheduler_presented + scheduler_repeats` —
///   every request the scheduler selected (fresh or repeat) was taken
///   by the renderer: ZERO hold refusals. This is exactly the residual
///   FX6-R4's loss-ledger derived as `hold_refusals`; under the defect
///   it measured 60 + repeats − 18 > 0.
/// * `video_presented >= 60` — every decoded frame presentable (the
///   60/60 intent at the pipeline level).
/// * `staging_frames == video_presented` and one full-frame RGBA copy
///   per present — the presents went through the upload path as RGBA,
///   the converted vocabulary, never the I420 the renderer refuses.
///
/// The mechanism pin makes the regression non-tautological: the v0
/// renderer still refuses I420 uploads, so `video_presented` covering
/// all 60 can ONLY hold because the hold stashed CONVERTED frames.
#[test]
fn fx7_a01_hold_presents_every_decoded_i420_frame() {
    let accounting = run_to_end();

    // The decode-count oracle (unchanged by the fix — decode, admission
    // and refusal accounting are not the present path).
    assert_eq!(accounting.frames_accepted, 120, "{accounting:?}");
    assert_eq!(accounting.scheduler_pushed, 60, "{accounting:?}");
    assert_eq!(accounting.scheduler_dropped_late, 0, "{accounting:?}");
    assert_eq!(accounting.video_refused, 0, "{accounting:?}");
    assert_eq!(accounting.frames_refused, 0, "{accounting:?}");
    assert!(
        accounting.scheduler_presented + accounting.scheduler_repeats >= 60,
        "at least every frame selected once: {accounting:?}"
    );
    // The mechanism pin: the renderer's own public upload gate refuses
    // I420 — an unconverted hold-retried frame cannot present.
    assert!(
        !renderer_wgpu::uploads_format(media_types::PixelFormat::I420),
        "the v0 upload gate is RGBA-only (uploads_format(I420) is true)"
    );
    // The W1-06 closure: every scheduler selection and repeat reached
    // the renderer — the hold-refusal residual is exactly zero.
    assert_eq!(
        accounting.video_presented,
        accounting.scheduler_presented + accounting.scheduler_repeats,
        "zero hold refusals: every scheduler selection was taken by the \
         renderer (the FX6-R4 hold_refusals residual): {accounting:?}"
    );
    // Every decoded frame presentable — the 60/60 the defect cut to 18.
    assert!(
        accounting.video_presented >= 60,
        "every decoded frame became presentable: {accounting:?}"
    );
    // The presents were RGBA uploads, one full-frame copy each — the
    // converted vocabulary, at the fixture's own 320×240 extent.
    assert_eq!(
        accounting.staging_frames, accounting.video_presented,
        "1:1 staged uploads per present: {accounting:?}"
    );
    assert_eq!(
        accounting.staging_bytes,
        accounting.staging_frames * 320 * 240 * 4,
        "one full-frame RGBA copy per present: {accounting:?}"
    );
    assert_eq!(
        accounting.staging_copies, accounting.staging_frames,
        "{accounting:?}"
    );
}

/// Determinism of the fixed present path: two whole-media runs agree
/// on the accounting (the same oracle class
/// `pipeline_accounting.rs::whole_media_runs_are_deterministic` pins
/// for the run's trail/position/frame bytes).
#[test]
fn fx7_a01_presentable_run_is_deterministic() {
    let first = run_to_end();
    let second = run_to_end();
    assert_eq!(first, second, "two runs' accounting must agree");
}
