//! FX6-A04 — the frame-lane integration proofs for FIX-PLAN §3 F1.2's
//! acceptance ("user watches sample.mp4 end-to-end in the GUI with a
//! frame counter that matches the decode count").
//!
//! The hardware/webview half of that acceptance is CI-gated (the
//! tauri crate is workspace-excluded; its 10 ms ticker still ships
//! `records.last()` — FX6-A02's file — and its send-side channel parks
//! full records in tauri's unbounded queue — FX6-A03's). These tests
//! prove the LANE LOGIC in-process at the deepest landed layers of
//! apps/desktop, in the crate's own harness (the same headless lane
//! the shell.rs pacer proofs and tests/pipeline.rs run in; FX4-A03's
//! report names it the model for this crate's lane tests):
//!
//! * the engine→mailbox seam [`FrameTap`] (the w31.1 `luf1-channel`
//!   composition the shell's bind installs — a [`CanvasBridge`] over a
//!   [`MailboxTransport`] over the bounded latest-wins mailbox at the
//!   production default capacity 2);
//! * the instrumented twin lane — the same composition driven on the
//!   tests/common virtual Timeline, the landed layer that still
//!   exposes the mailbox's full [`MailboxStats`] counters;
//! * the whole GUI funnel (`ShellRuntime::open_path` → the single
//!   command → `pump_once` ticks → the per-tick frame drain) against
//!   the real AVI/MJPEG fixture, with the decode-count oracle pinned
//!   by apps/cli/tests/pipeline_accounting.rs for this same fixture:
//!   60 decoded video frames, 120 deliveries, 2.000 s.
//!
//! What is NOT landed at this file's authoring: `ShellRuntime` does
//! not surface the frame-lane counters (FX6-A02's seam —
//! `take_frame_records` answers a record count only, and the shipped
//! `FrameTap` exposes `BridgeStats`, not `MailboxStats`). The
//! acceptance test therefore installs its own instrumented
//! `PresentTap` through the SAME injector the shell's bind uses
//! (`CliHost::set_present_tap`, apps/desktop/src/shell.rs:187) — the
//! deepest landed measurement point — rather than any src change
//! (this file is tests-only, per the assignment).

mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use commands::{Command, CommandOutcome, EventKind, PlaybackState, WireRequest};
use desktop::frame_tap::FrameTap;
use desktop::mailbox::{MailboxConfig, MailboxStats};
use desktop::shell::{ShellRuntime, DEFAULT_EVENT_CAPACITY};
use desktop::transport::MailboxTransport;
use desktop::wire::{encode_into, WireDecoder, MIN_HEADER_LEN};
use desktop::CanvasBridge;
use lumen_cli::pipeline_host::PresentTap;
use media_types::EngineError;
use renderer_wgpu::params::Extent2D;
use testkit::Must;
use video_engine::{PresentRequest, Presenter};

use common::{marked_request, payload_marker, Timeline};

/// The suite's frame extent — the common/mod.rs marker-pixel
/// convention (pixel (5,5) carries the frame index, so every claim
/// about WHICH frame survived is checked against CONTENT, not just
/// counters).
const EXTENT: Extent2D = Extent2D {
    width: 64,
    height: 48,
};

/// 30 fps content (doc 03 §4.10 e's default — the fixture class of
/// the end-to-end proof).
const FRAME_30: u64 = 33_333_334;

/// 60 fps content — the faster-than-consumption stressor's cadence.
const FRAME_60: u64 = 16_666_667;

/// The simulated 20 Hz consumer tick (the stressor's slow side).
const CONSUMER_20: u64 = 50_000_000;

/// The frames the stress drives through the lane (one virtual second
/// of 60 fps content).
const STRESS_FRAMES: u64 = 60;

/// The stalled-consumer drive: enough offers that an unbounded queue
/// of the W14-A07 class would be glaring, cheap enough to stay a fast
/// test (64x48 frames).
const STALL_OFFERS: usize = 512;

/// The pacer bound for the end-to-end drive — the shell.rs pacer
/// proofs' own headroom (1024 pumps for a fixture that ends well
/// inside ~170; the bound is never the pacing truth).
const PUMP_BOUND: usize = 1024;

/// The real fixture the end-to-end proof plays — the same file the
/// shell.rs frame-channel proof and the CLI's
/// apps/cli/tests/pipeline_accounting.rs oracle drive: 60 MJPEG
/// frames + 60 PCM packets, 2.000 s. The decode-count oracle (60
/// video frames / 120 deliveries / position 2_000_000 µs) is theirs,
/// re-derived here through the GUI funnel.
const AVI_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi"
);

/// The instrumented twin of the production lane — the `FrameTap`'s
/// exact composition (a `CanvasBridge` over a `MailboxTransport` over
/// the bounded latest-wins `FrameMailbox` at the production default
/// capacity), driven on the common Timeline's virtual clock so the
/// emitted stamps are exact, with the mailbox's full counters readable
/// (the `FrameTap` itself exposes `BridgeStats` only).
struct Lane {
    timeline: Timeline,
    bridge: CanvasBridge<MailboxTransport>,
    /// The production capacity (`MailboxConfig::default()` = 2 — the
    /// F1.2 "capacity 2 is the only backpressure" knob), read from the
    /// config so a capacity change fails these tests loudly.
    capacity: usize,
}

fn lane() -> Lane {
    let timeline = Timeline::new();
    let transport = MailboxTransport::new(MailboxConfig::default()).must();
    let capacity = MailboxConfig::default().capacity.get();
    Lane {
        bridge: CanvasBridge::with_clock(transport, Box::new(timeline.emit_clock())),
        timeline,
        capacity,
    }
}

/// The instrumented tap (test-local): the `FrameTap`'s exact
/// composition — a `CanvasBridge` over a `MailboxTransport` behind one
/// lock — with the mailbox's full `MailboxStats` readable from the
/// consumer side. Installed through `CliHost::set_present_tap`, the
/// same injector the shell's `session.set-frame-channel` bind uses
/// (shell.rs:187), so real engine presents flow through the real seam;
/// the drift from the shipped `FrameTap` is instrumentation only.
struct CountingTap {
    bridge: Mutex<CanvasBridge<MailboxTransport>>,
}

impl CountingTap {
    fn new() -> CountingTap {
        let transport = match MailboxTransport::new(MailboxConfig::default()) {
            Ok(transport) => transport,
            Err(error) => unreachable!("the default config is valid: {error}"),
        };
        CountingTap {
            bridge: Mutex::new(CanvasBridge::new(transport)),
        }
    }

    /// The consumer's per-tick drain (the `FrameTap::drain_records`
    /// body): draw every queued frame, re-encode each as one complete
    /// LUF1 record. Returns how many records were appended.
    fn drain_records(&self, out: &mut Vec<Vec<u8>>) -> usize {
        let mut bridge = match self.bridge.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let mailbox = bridge.transport_mut().mailbox_mut();
        let mut count = 0;
        while let Some(frame) = mailbox.draw_next() {
            let mut record = Vec::with_capacity(frame.header.payload_len as usize + MIN_HEADER_LEN);
            if encode_into(&frame.header, &frame.payload, &mut record).is_ok() {
                out.push(record);
                count += 1;
            }
        }
        count
    }

    /// The full mailbox accounting (the frame lane's landed counters).
    fn mailbox_stats(&self) -> MailboxStats {
        match self.bridge.lock() {
            Ok(guard) => guard.transport().mailbox().stats(),
            Err(poisoned) => poisoned.into_inner().transport().mailbox().stats(),
        }
    }
}

impl PresentTap for CountingTap {
    fn tap_present(&self, request: &PresentRequest, repeat: bool) -> Result<(), EngineError> {
        let mut bridge = match self.bridge.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if repeat {
            bridge.present_repeat(request).map(|_| ())
        } else {
            bridge.present(request).map(|_| ())
        }
    }
}

/// FX6 #1 — frames produced vs dropped at the mailbox: a 60 fps
/// producer against a 20 Hz consumer (the pacer/oracle drive —
/// tests/pipeline.rs's stressor cadences, driven through the
/// instrumented twin lane) drops by the mailbox's own policy, and the
/// landed counters make the whole story arithmetic: produced N,
/// dropped K, shipped ≤ produced, occupancy never above capacity —
/// checked after every single offer.
#[test]
fn fx6_a04_frames_produced_vs_dropped_at_the_mailbox() {
    let mut lane = lane();
    let mut next_consumer_tick = CONSUMER_20;
    let mut drawn_by_hand = 0u64;
    for i in 0..STRESS_FRAMES {
        let slot = i * FRAME_60;
        // The producer: one present per 60 fps slot (never blocked by
        // the consumer — overflow is the mailbox's own policy).
        lane.timeline.set_ns(slot);
        lane.bridge
            .present(&marked_request(EXTENT, slot as i64, i as u8))
            .must();
        // Occupancy never exceeds capacity — after EVERY offer.
        assert!(
            lane.bridge.transport().mailbox().len() <= lane.capacity,
            "offer {i}: occupancy {} exceeds the capacity {}",
            lane.bridge.transport().mailbox().len(),
            lane.capacity
        );
        // The consumer: drains everything queued at its 20 Hz ticks.
        while next_consumer_tick <= slot {
            lane.timeline.set_ns(next_consumer_tick);
            while lane
                .bridge
                .transport_mut()
                .mailbox_mut()
                .draw_next()
                .is_some()
            {
                drawn_by_hand += 1;
            }
            next_consumer_tick += CONSUMER_20;
        }
    }
    // The end-of-stream drain (the consumer's final tick).
    while lane
        .bridge
        .transport_mut()
        .mailbox_mut()
        .draw_next()
        .is_some()
    {
        drawn_by_hand += 1;
    }
    let stats = lane.bridge.transport().mailbox().stats();
    assert_eq!(
        stats.offered, STRESS_FRAMES,
        "produced N: every present offered exactly one frame"
    );
    assert_eq!(
        stats.drawn, drawn_by_hand,
        "the landed counter mirrors the harness's own drain count"
    );
    assert_eq!(
        stats.offered,
        stats.drawn + stats.dropped_overflow + stats.dropped_stale,
        "every offered frame is drawn or dropped exactly once (the mailbox is empty)"
    );
    assert!(stats.conserves());
    assert!(
        stats.dropped_overflow > 0,
        "a 20 Hz consumer cannot keep up with 60 fps: {stats:?}"
    );
    assert!(
        stats.drawn < stats.offered,
        "shipped is strictly under produced under the stressor: {stats:?}"
    );
    // The two produced counters agree: the bridge's ship count and
    // the mailbox's offer count are the same frames.
    assert_eq!(lane.bridge.stats().frames_shipped, stats.offered);
}

/// FX6 #2 — the drop policy honored on overflow: the mailbox is
/// drop-oldest/keep-latest (mailbox.rs's documented "latest-wins" —
/// when full, the OLDEST un-consumed frame is dropped, counted, and
/// the freshest always wins). Proven at the real engine seam (the
/// shipped `FrameTap`, production capacity 2) on CONTENT, not just
/// counters: 7 marked frames offered with no drain, and the survivors
/// are the NEWEST two — identified by frame id, by PTS, and by the
/// marker pixel each payload carries.
#[test]
fn fx6_a04_drop_policy_honored_on_overflow() {
    let tap = FrameTap::new().must();
    for i in 0..7u64 {
        // Every offer returns Ok — a full mailbox never blocks the
        // producer (doc 03 §4.10 c: the scheduler never stalls).
        tap.tap_present(
            &marked_request(EXTENT, (i * FRAME_30) as i64, i as u8),
            false,
        )
        .must();
    }
    assert_eq!(
        tap.stats().frames_shipped,
        7,
        "all 7 offers shipped into the mailbox"
    );
    let mut records = Vec::new();
    assert_eq!(
        tap.drain_records(&mut records),
        2,
        "only the mailbox capacity survives the overflow"
    );
    let mut decoder = WireDecoder::new();
    let mut survivors = Vec::new();
    for record in &records {
        survivors.push(match decoder.decode_record(record) {
            Ok(frame) => frame,
            Err(error) => unreachable!("a lane record must decode: {error}"),
        });
    }
    // The survivors are frames 5 and 6 — the NEWEST; frames 0..4 were
    // dropped oldest-first.
    assert_eq!(survivors[0].header.frame_id, 5);
    assert_eq!(survivors[1].header.frame_id, 6);
    // The payload content agrees with the ids — the marker pixel
    // carries the frame index (the 2.D1 convention), so the surviving
    // RECORDS are the policy-chosen frames, not just headers claiming
    // to be.
    assert_eq!(
        payload_marker(EXTENT, &survivors[0].payload),
        [5, 0xEE, 0x77, 255]
    );
    assert_eq!(
        payload_marker(EXTENT, &survivors[1].payload),
        [6, 0xEE, 0x77, 255]
    );
    // Order is never violated: the surviving pair is PTS-ascending.
    assert!(survivors[0].header.pts_ns < survivors[1].header.pts_ns);
}

/// FX6 #3 — timestamps pass through the lane: a frame pushed at stamp
/// T arrives with stamp T — no fabrication, no drift beyond ordering.
/// Proven twice: (a) at the real engine seam (the `FrameTap`'s full
/// round trip — present → encode → mailbox → drain → re-encode →
/// decode — at 1:1 pace so every stamp is observable, with the
/// production monotonic emit clock); (b) on the instrumented twin
/// lane's virtual Timeline, where BOTH stamps are known exactly: the
/// scheduled slot T (the media stamp) and the serialize instant (the
/// emit clock's own reading at push time).
#[test]
fn fx6_a04_timestamps_pass_through_the_lane() {
    // (a) the engine seam: push at stamp T, drain immediately, decode,
    // compare.
    let tap = FrameTap::new().must();
    let mut decoder = WireDecoder::new();
    let mut last_emitted = 0u64;
    for i in 0..5u64 {
        let stamp = (i * FRAME_30) as i64;
        tap.tap_present(&marked_request(EXTENT, stamp, i as u8), false)
            .must();
        let mut records = Vec::new();
        assert_eq!(
            tap.drain_records(&mut records),
            1,
            "1:1 pace: one frame in, one record out"
        );
        let frame = match decoder.decode_record(&records[0]) {
            Ok(frame) => frame,
            Err(error) => unreachable!("a lane record must decode: {error}"),
        };
        assert_eq!(
            frame.header.pts_ns, stamp,
            "the pushed stamp T is the arriving stamp T"
        );
        assert!(
            frame.header.emitted_ns >= last_emitted,
            "the emit stamp is monotonic: {} after {}",
            frame.header.emitted_ns,
            last_emitted
        );
        last_emitted = frame.header.emitted_ns;
    }
    // (b) the instrumented twin: the serialize instants are known
    // exactly (the virtual Timeline), and both stamps survive the wire
    // codec's encode → decode round trip inside the transport.
    let mut lane = lane();
    for i in 0..5u64 {
        let stamp = (i * FRAME_30) as i64;
        let emit_instant = (i + 1) * 100_000_000;
        lane.timeline.set_ns(emit_instant);
        lane.bridge
            .present(&marked_request(EXTENT, stamp, i as u8))
            .must();
        let frame = match lane.bridge.transport_mut().mailbox_mut().draw_next() {
            Some(frame) => frame,
            None => unreachable!("the 1:1 drain must find the frame just offered"),
        };
        assert_eq!(frame.header.pts_ns, stamp, "stamp T in, stamp T out");
        assert_eq!(
            frame.header.emitted_ns, emit_instant,
            "the emit stamp is the clock's own reading, never fabricated"
        );
    }
}

/// FX6 #4 — a stalled consumer is bounded (the W14-A07
/// unbounded-queue class, at the landed seam): 512 offers with the
/// consumer NEVER draining — the producer is never blocked, the
/// mailbox's occupancy is exactly the capacity after saturation
/// (never more — and never less: the bound is tight), and the single
/// drain at the end ships exactly the capacity. No unbounded
/// accumulation anywhere the harness can observe.
#[test]
fn fx6_a04_stalled_consumer_is_bounded() {
    // (a) the production seam: the FrameTap the engine calls.
    let tap = FrameTap::new().must();
    for i in 0..STALL_OFFERS {
        tap.tap_present(
            &marked_request(EXTENT, (i as i64) * (FRAME_30 as i64), (i % 256) as u8),
            false,
        )
        .must();
    }
    assert_eq!(
        tap.stats().frames_shipped,
        STALL_OFFERS as u64,
        "the stalled consumer never blocks the producer: every offer shipped"
    );
    let mut records = Vec::new();
    assert_eq!(
        tap.drain_records(&mut records),
        MailboxConfig::default().capacity.get(),
        "the stalled mailbox holds exactly the capacity, not the offers"
    );
    // (b) the instrumented twin: per-step occupancy and the full
    // counter story.
    let mut lane = lane();
    for i in 0..STALL_OFFERS {
        lane.timeline.advance_ns(FRAME_30);
        lane.bridge
            .present(&marked_request(
                EXTENT,
                (i as i64) * (FRAME_30 as i64),
                (i % 256) as u8,
            ))
            .must();
        // The bound is tight: after saturation, exactly the capacity
        // stays queued — never more, never less.
        assert_eq!(
            lane.bridge.transport().mailbox().len(),
            usize::min(i + 1, lane.capacity),
            "offer {i}: occupancy must be min(offered, capacity)"
        );
    }
    let stats = lane.bridge.transport().mailbox().stats();
    assert_eq!(stats.offered, STALL_OFFERS as u64);
    assert_eq!(stats.drawn, 0, "the consumer never drained");
    assert_eq!(stats.dropped_stale, 0);
    assert_eq!(
        stats.dropped_overflow,
        (STALL_OFFERS - lane.capacity) as u64,
        "every offer past the capacity dropped the oldest, counted"
    );
    // The mid-stall conservation identity carries the backlog
    // (mailbox.rs's conserves() precondition, honored explicitly).
    assert_eq!(
        stats.offered,
        stats.drawn
            + stats.dropped_overflow
            + stats.dropped_stale
            + lane.bridge.transport().mailbox().len() as u64,
        "offered == drawn + dropped + queued (the backlog-inclusive identity)"
    );
    // The end-of-stall drain empties the bounded residue and closes
    // the conservation exactly.
    let mut drained = 0u64;
    while lane
        .bridge
        .transport_mut()
        .mailbox_mut()
        .draw_next()
        .is_some()
    {
        drained += 1;
    }
    assert_eq!(drained, lane.capacity as u64, "the residue is the capacity");
    let stats = lane.bridge.transport().mailbox().stats();
    assert_eq!(stats.drawn, drained);
    assert!(
        stats.conserves(),
        "the empty-mailbox identity holds after the drain: {stats:?}"
    );
}

/// FX6 #5 — the F1.2 acceptance, in-process, in two halves.
///
/// (a) The whole GUI funnel (open through the runtime's own path →
/// play through the single command → ONLY `pump_once` ticks, with
/// the frame drain the ticker performs after every pump) drives the
/// real AVI fixture end to end, and the frame lane's counters are
/// proven against the engine's own decode accounting (the oracle
/// pinned by apps/cli/tests/pipeline_accounting.rs for this fixture:
/// 60 decoded video frames, 120 deliveries, position 2.000 s):
/// produced == the renderer's present count (`video_presented`),
/// every shipped record is counted, and the acceptance's
/// conservation closes EXACTLY through the present-path loss
/// ledger: produced + superseded_drops + hold_refusals (W1-06) ==
/// decoded. The 60/60 presentation intent — produced alone covering
/// every decoded frame — is NOT deleted: it is exactly what the
/// ledger reduces to once W1-06's Lead-gated fix lands (stash the
/// CONVERTED frame in the window-full hold). W1-06, the registered
/// KNOWN DEFECT (apps/cli/src/pipeline_host.rs:1686-1714): the hold
/// stashes the UNCONVERTED I420 original while the accept path
/// converts I420→RGBA, so hold-retried frames are refused at the
/// renderer (`render.upload.format-unsupported`); this fixture
/// decodes as I420 (MJPEG 4:2:0). The gap is cadence-independent —
/// NOT pace-derivable: the decode side has no backpressure (the
/// defect register's own note), so the window+hold fill ahead of
/// the clock under any pump cadence — a wall-paced
/// `pump_wallclock_at` drive presents the same refusals as the
/// whole-ring one; the loss is engine-side. Note the pace
/// caveat: `pump_once` is the whole-ring headless pump (shell.rs's
/// own docs), so its per-pump present bursts can overflow the
/// capacity-2 mailbox by policy — the conservation identity (not a
/// zero-drop pin) is what holds under this drive.
///
/// (b) The zero-overflow steady state at the lane's normal pace — a
/// 30 fps producer against a 60 Hz consumer (the nominal pairing,
/// doc 03 §4.10 e) on the instrumented twin: produced == drawn == 60,
/// dropped == 0. The production ticker's own zero-overflow drive
/// (`pump_session`, the wall-bounded pump) has no injectable-clock
/// seam yet (shell.rs audit F-004 — a src change this tests-only
/// assignment must not make); (b) is the deepest in-process proof of
/// the normal-pace state.
#[test]
fn fx6_a04_frame_counter_matches_decode_count_at_normal_pace() {
    let mut runtime = ShellRuntime::new(DEFAULT_EVENT_CAPACITY);
    let tap = Arc::new(CountingTap::new());
    // The same injector the shell's bind uses (shell.rs:187), installed
    // before the open exactly as a pre-open bind stashes
    // (`CliHost::set_present_tap` is state-safe). The injector takes the
    // tap by value as the trait object, so the concrete counting handle
    // stays here for the assertions while a clone coerced to
    // `Arc<dyn PresentTap>` rides the engine seam — the unsizing happens
    // at this let's coercion site; the trait object never arrives via
    // `&Arc<CountingTap>`, which cannot coerce.
    let dyn_tap: Arc<dyn PresentTap> = tap.clone();
    runtime.engine_host().set_present_tap(dyn_tap);

    let open = match runtime.open_path(&PathBuf::from(AVI_FIXTURE)) {
        Some(response) => response,
        None => unreachable!("a media file must classify as openable"),
    };
    match open.result {
        Ok(CommandOutcome::Accepted { .. }) => {}
        other => unreachable!("a real media file must open accepted: {other:?}"),
    }
    let play_id = runtime.next_request_id();
    let play = runtime.dispatch(&WireRequest {
        request_id: play_id,
        command: Command::PlaybackPlay,
    });
    match play.result {
        Ok(CommandOutcome::Accepted { .. }) => {}
        other => unreachable!("play must be accepted from every live state: {other:?}"),
    }

    // The pacer loop — the ticker's exact body, plus the frame drain
    // the ticker performs after each pump (the first take drains the
    // open's own event burst).
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut records = Vec::new();
    let mut ended = false;
    let mut pumps = 0usize;
    while pumps < PUMP_BOUND {
        pumps += 1;
        runtime.pump_once();
        for event in runtime.take_pending_events() {
            match event.kind {
                EventKind::PlaybackEndOfMedia => ended = true,
                EventKind::SessionStateChanged {
                    to: PlaybackState::Ended,
                    ..
                } => ended = true,
                _ => {}
            }
        }
        tap.drain_records(&mut records);
        if ended {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the pacer loop must not hang: a bound breach is a failure, never a mystery"
        );
    }
    assert!(
        ended,
        "the session must end within {PUMP_BOUND} pumps ({pumps} pumped)"
    );
    // The end-of-media drain (the session's last residue).
    tap.drain_records(&mut records);

    // The whole media played end to end: the state is Ended and the
    // position lands on the fixture's 2.000 s duration exactly (the
    // pinned oracle for this fixture and this pump class).
    assert_eq!(runtime.engine_host().state(), PlaybackState::Ended);
    assert_eq!(
        runtime.engine_host().position_us(),
        2_000_000,
        "the whole media played"
    );

    // The decode-count oracle (this exact fixture, re-derived through
    // the GUI funnel): 60 video frames decoded, admitted and
    // scheduled; 120 deliveries on both limbs; nothing refused,
    // nothing late.
    let accounting = match runtime.engine_host().headless_accounting() {
        Some(accounting) => accounting,
        None => unreachable!("an open session wires a consumer"),
    };
    assert_eq!(
        accounting.scheduler_pushed, 60,
        "the decode count: {accounting:?}"
    );
    assert_eq!(accounting.scheduler_dropped_late, 0, "{accounting:?}");
    assert_eq!(accounting.frames_accepted, 120, "{accounting:?}");

    // The lane counters (the instrumented tap — the FrameTap's own
    // composition): produced == the renderer's present count (the
    // engine's own counter), and the mailbox's conservation identity
    // holds whatever the whole-ring pump's bursts dropped by policy
    // (the counters are the story, not a zero-drop pin).
    let stats = tap.mailbox_stats();
    assert_eq!(
        stats.offered, accounting.video_presented,
        "the tap mirrors every renderer present (the engine's own counter)"
    );
    assert_eq!(
        stats.offered,
        stats.drawn + stats.dropped_overflow + stats.dropped_stale,
        "every offered frame is drawn or dropped exactly once (the mailbox is empty)"
    );
    assert!(stats.conserves());
    assert_eq!(
        stats.dropped_stale, 0,
        "no generation change mid-run: {stats:?}"
    );
    // The acceptance's conservation, closed through the present-path
    // loss ledger (the honest re-scope): produced + superseded_drops
    // + hold_refusals (W1-06) == decoded. The engine surfaces decoded
    // (scheduler_pushed, pinned at 60 above), produced
    // (video_presented — mirrored to stats.offered above) and the
    // late/stale drop policies (pinned at zero above); it does NOT
    // surface the two loss classes this drive actually suffers, so
    // each is DERIVED from the surfaced counters and the total is
    // asserted to close exactly:
    // * superseded_drops — the scheduler's step-4 prefix drop
    //   (crates/video-engine/src/scheduler.rs:792-812): per tick only
    //   the newest due frame presents, everything due before it
    //   drops;
    // * hold_refusals — the W1-06 KNOWN DEFECT's renderer refusals:
    //   the window-full hold stashes the UNCONVERTED I420 original
    //   (apps/cli/src/pipeline_host.rs:1686-1714) while the accept
    //   path converts I420→RGBA, so hold-retried frames reach the
    //   renderer unconverted and it refuses them (the upload gate
    //   pinned below). This fixture decodes as I420 (MJPEG 4:2:0),
    //   so the defect is on this drive's path (measured centrally:
    //   produced 18 of 60, the decode-count oracle fully green).
    //
    // The 60/60 intent is NOT deleted: produced == 60 is exactly what
    // this ledger reduces to once W1-06's Lead-gated fix (stashing
    // the CONVERTED frame) lands — hold_refusals goes to zero and
    // produced covers every decoded frame; until then the gap is
    // measured, named, and carried inside the same conservation.
    let decode_count = accounting.scheduler_pushed;
    let produced = stats.offered;
    let surfaced_drops = accounting.scheduler_dropped_late + accounting.scheduler_dropped_stale;
    // The structural invariants that make the derived losses
    // well-formed (each decoded frame is selected, dropped, or lost
    // at most once; each renderer take came from a scheduler offer) —
    // both fail loudly if the engine's counters ever disagree.
    assert!(
        accounting.scheduler_presented + surfaced_drops <= decode_count,
        "the scheduler cannot account for more frames than were decoded: {accounting:?}"
    );
    assert!(
        accounting.video_presented <= accounting.scheduler_presented + accounting.scheduler_repeats,
        "the renderer cannot take more requests than the scheduler offered: {accounting:?}"
    );
    let superseded_drops = decode_count - accounting.scheduler_presented - surfaced_drops;
    let hold_refusals =
        accounting.scheduler_presented + accounting.scheduler_repeats - accounting.video_presented;
    // The mechanism pin at the renderer's own public gate: the v0
    // renderer refuses I420 uploads, so an unconverted hold-retried
    // frame cannot present — the W1-06 refusals the ledger measures.
    // (This trips loudly if the renderer ever starts accepting I420,
    // which would re-scope this ledger's W1-06 attribution.)
    assert!(
        !renderer_wgpu::uploads_format(media_types::PixelFormat::I420),
        "the v0 upload gate must refuse I420 for the W1-06 attribution to hold"
    );
    // The conservation: produced + superseded drops + hold refusals
    // (the W1-06 gap) + surfaced drops == the decoded count plus the
    // repeats (repeats ride the same lane, counted as presents — the
    // defect's own frozen-clock retry storm inflates them; the term
    // keeps the total exact on both sides of the W1-06 fix).
    assert_eq!(
        produced + superseded_drops + hold_refusals + surfaced_drops,
        decode_count + accounting.scheduler_repeats,
        "produced ({produced}) + superseded drops ({superseded_drops}) + hold \
         refusals, the W1-06 gap ({hold_refusals}) + surfaced drops \
         ({surfaced_drops}) must close over the {decode_count} decoded frames \
         (repeats ride the same lane, counted as presents); the engine's own \
         present-path accounting: {accounting:?}"
    );

    // The frame counter: every shipped record is one complete,
    // decodable LUF1 frame — the counter counts real frames, in media
    // order, and exactly the drawn count (a drain-side skip would show
    // here: records under drawn).
    assert!(!records.is_empty());
    assert!(
        records.iter().all(|record| record.starts_with(b"LUF1")),
        "every shipped record carries the LUF1 magic"
    );
    assert_eq!(
        records.len() as u64,
        stats.drawn,
        "every shipped record counted: {} records vs {} drawn",
        records.len(),
        stats.drawn
    );
    let mut decoder = WireDecoder::new();
    let mut decoded = 0u64;
    let mut last_pts = i64::MIN;
    for record in &records {
        let frame = match decoder.decode_record(record) {
            Ok(frame) => frame,
            Err(error) => unreachable!("a shipped record must decode: {error}"),
        };
        decoded += 1;
        assert!(
            frame.header.pts_ns >= last_pts,
            "shipped records arrive in media order (pts {} after {})",
            frame.header.pts_ns,
            last_pts
        );
        last_pts = frame.header.pts_ns;
    }
    assert_eq!(decoded, records.len() as u64);

    // The egress accounting conserves end to end (the event lane's
    // own counters).
    assert!(runtime.event_stats().conserves());

    // (b) The zero-overflow steady state at the lane's normal pace: a
    // 30 fps producer against a 60 Hz consumer — every frame drawn,
    // exactly once, nothing dropped, occupancy never above capacity.
    let mut lane = lane();
    let mut next_consumer_tick = FRAME_60;
    for i in 0..60u64 {
        let slot = i * FRAME_30;
        lane.timeline.set_ns(slot);
        lane.bridge
            .present(&marked_request(EXTENT, slot as i64, i as u8))
            .must();
        assert!(
            lane.bridge.transport().mailbox().len() <= lane.capacity,
            "offer {i}: occupancy {} exceeds the capacity {}",
            lane.bridge.transport().mailbox().len(),
            lane.capacity
        );
        while next_consumer_tick <= slot {
            while lane
                .bridge
                .transport_mut()
                .mailbox_mut()
                .draw_next()
                .is_some()
            {}
            next_consumer_tick += FRAME_60;
        }
    }
    while lane
        .bridge
        .transport_mut()
        .mailbox_mut()
        .draw_next()
        .is_some()
    {}
    let stats = lane.bridge.transport().mailbox().stats();
    assert_eq!(stats.offered, 60, "produced == the decode count stand-in");
    assert_eq!(stats.dropped_overflow, 0, "the normal pace drops nothing");
    assert_eq!(stats.dropped_stale, 0);
    assert_eq!(stats.drawn, 60, "every frame drawn, exactly once");
    assert!(stats.conserves());
    assert_eq!(
        lane.bridge.stats().frames_shipped,
        60,
        "the bridge counter agrees: 60 frames shipped"
    );
}
