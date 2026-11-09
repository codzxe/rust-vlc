//! The end-to-end virtual-time suite (card 2.D3 AC): video-engine's
//! REAL [`PresentationScheduler`] + [`PresentationQueue`] driving the
//! [`CanvasBridge`] over the [`MailboxTransport`], with a simulated
//! webview consumer draining the mailbox — all on ONE virtual
//! nanosecond timeline shared by the scheduler and the bridge's emit
//! clock (doc 03 §8: zero sleeps; end-to-end latency is measurable
//! in a single coordinate system, which is exactly what a desktop
//! run needs the protocol-v2 origin handshake for).
//!
//! Plus the honest-stub proof: the `lumen-desktop` binary reports the
//! ADR-004 pending state and exits with the documented code.

mod common;

use std::num::NonZeroUsize;
use std::process::Command;

use common::{Timeline, marked_request};
use desktop::CanvasBridge;
use desktop::mailbox::{LatencyAccountant, MailboxConfig};
use desktop::transport::MailboxTransport;
use media_types::time::Nanoseconds;
use renderer_wgpu::params::Extent2D;
use testkit::Must;
use video_engine::{
    EnqueueOutcome, PresentAction, PresentationQueue, PresentationScheduler, Presenter,
    SchedulerCfg,
};

/// The suite's frame extent (marker pixel at (5,5) — 64x48).
const EXTENT: Extent2D = Extent2D {
    width: 64,
    height: 48,
};

/// 30 fps content (doc 03 §4.10 e default) = an exact multiple of
/// the 60 Hz consumer tick (2×) — the nominal scenario's arithmetic
/// is exact by construction.
const FRAME_30: u64 = 33_333_334;

/// 60 fps content — the slow-consumer stressor.
const FRAME_60: u64 = 16_666_667;

/// The simulated 60 Hz webview consumer tick.
const CONSUMER_60: u64 = 16_666_667;

/// The simulated 20 Hz webview consumer tick (the stressor).
const CONSUMER_20: u64 = 50_000_000;

/// The simulated serialize+transport cost folded between the
/// scheduler's tick and the bridge's emit stamp (the number that
/// makes end-to-end latency non-trivial and EXACT).
const ENCODE_COST: u64 = 5_000_000;

/// The full engine→mailbox wiring over one timeline.
struct Sim {
    timeline: Timeline,
    frame_period: u64,
    scheduler: PresentationScheduler,
    queue: PresentationQueue,
    bridge: CanvasBridge<MailboxTransport>,
    next_consumer_tick: u64,
    consumer_period: u64,
    transport_latency: LatencyAccountant,
    slot_drift: LatencyAccountant,
    drawn_ids: Vec<u64>,
}

impl Sim {
    fn new(frame_period: u64, consumer_period: u64, capacity: usize) -> Sim {
        let timeline = Timeline::new();
        let cfg = SchedulerCfg::for_frame_duration(Nanoseconds(frame_period)).must();
        let scheduler = PresentationScheduler::new(cfg, timeline.media_clock());
        let queue = PresentationQueue::new(NonZeroUsize::new(8).must());
        let transport = MailboxTransport::new(MailboxConfig {
            capacity: NonZeroUsize::new(capacity).must(),
        })
        .must();
        let bridge = CanvasBridge::with_clock(transport, Box::new(timeline.emit_clock()));
        Sim {
            timeline,
            frame_period,
            scheduler,
            queue,
            bridge,
            next_consumer_tick: consumer_period,
            consumer_period,
            transport_latency: LatencyAccountant::default(),
            slot_drift: LatencyAccountant::default(),
            drawn_ids: Vec::new(),
        }
    }

    /// One production-shaped present cycle: push the frame, advance
    /// to its slot, tick, enqueue, fold the encode cost, drain to
    /// the bridge (the emit stamp), then run every consumer tick up
    /// to the next slot — ticks are fired at THEIR times: a tick
    /// before the emit completes drew nothing (the frame did not
    /// exist yet), so its slot on the schedule is consumed as a
    /// no-op, never back-dated.
    fn present_cycle(&mut self, slot: u64, pts_ns: i64, index: u8) {
        let request = marked_request(EXTENT, pts_ns, index);
        let frame = request.frame.clone();
        let _ = self.scheduler.push(frame);
        self.timeline.set_ns(slot);
        let outcome = self.scheduler.tick();
        let request = match &outcome.action {
            PresentAction::Present(request) | PresentAction::RepeatLast(request) => {
                Some(request.clone())
            }
            PresentAction::Background => None,
        };
        if let Some(request) = request {
            match self.queue.try_push(request) {
                EnqueueOutcome::Enqueued => {}
                refused => unreachable!("wide queue refused a push: {refused:?}"),
            }
        }
        // Consumer ticks up to and including the slot fire at their
        // own times (whatever was in the mailbox — the previous
        // frame — or nothing).
        while self.next_consumer_tick <= slot {
            self.timeline.set_ns(self.next_consumer_tick);
            self.draw_one();
            self.next_consumer_tick += self.consumer_period;
        }
        // The serialize+transport cost lands between the tick and
        // the emit stamp (virtual, exact).
        self.timeline.set_ns(slot + ENCODE_COST);
        let delivered = self.queue.drain_to(&mut self.bridge).must();
        assert_eq!(delivered, 1, "one tick delivers one request");
        // Ticks that fell inside the cost window drew nothing (the
        // frame was not emitted yet): consume them as no-ops.
        while self.next_consumer_tick < slot + ENCODE_COST {
            self.next_consumer_tick += self.consumer_period;
        }
        // Ticks from the emit to the next slot fire at their times.
        while self.next_consumer_tick < slot + self.frame_period {
            self.timeline.set_ns(self.next_consumer_tick);
            self.draw_one();
            self.next_consumer_tick += self.consumer_period;
        }
    }

    /// The consumer draws one frame (if the mailbox has one) and
    /// records both latency series.
    fn draw_one(&mut self) {
        let now = self.timeline.now_ns() as i64;
        if let Some(frame) = self.bridge.transport_mut().mailbox_mut().draw_next() {
            self.transport_latency
                .record(now - frame.header.emitted_ns as i64);
            self.slot_drift.record(now - frame.header.pts_ns);
            self.drawn_ids.push(frame.header.frame_id);
        }
    }

    /// Run consumer ticks until the mailbox is empty (end-of-stream
    /// drain).
    fn drain_all(&mut self) {
        while !self.bridge.transport().mailbox().is_empty() {
            self.timeline.advance_ns(self.consumer_period);
            self.draw_one();
        }
    }
}

/// Nominal: 30 fps content, 60 Hz consumer, capacity 2 — every frame
/// drawn in order with the EXACT end-to-end virtual latency
/// (emit at slot+5 ms, draw at the next 16.67 ms tick).
#[test]
fn nominal_pipeline_delivers_every_frame_with_exact_latency() {
    let mut sim = Sim::new(FRAME_30, CONSUMER_60, 2);
    for i in 0..60u64 {
        let slot = i * FRAME_30;
        sim.present_cycle(slot, slot as i64, (i % 256) as u8);
    }
    sim.drain_all();

    // Every frame drawn, in order, exactly once.
    assert_eq!(sim.drawn_ids, (0..60).collect::<Vec<_>>());

    // The exact end-to-end numbers (this timeline's arithmetic):
    // draw tick − emit = 16_666_667 − 5_000_000; slot drift =
    // draw tick − pts = one consumer tick.
    let latency = sim.transport_latency.summary();
    assert_eq!(latency.samples, 60);
    assert_eq!(latency.min_ns, Some(11_666_667));
    assert_eq!(latency.max_ns, Some(11_666_667));
    assert_eq!(latency.mean_ns, Some(11_666_667));
    let drift = sim.slot_drift.summary();
    assert_eq!(drift.samples, 60);
    assert_eq!(drift.min_ns, Some(16_666_667_i64));
    assert_eq!(drift.max_ns, Some(16_666_667_i64));

    // The mailbox's conservation identity holds at end of stream.
    let stats = sim.bridge.transport().mailbox().stats();
    assert_eq!(stats.offered, 60);
    assert_eq!(stats.drawn, 60);
    assert_eq!(stats.dropped_overflow, 0);
    assert_eq!(stats.dropped_stale, 0);
    assert!(stats.conserves());
    assert!(sim.bridge.transport().mailbox().is_empty());

    // And the bridge shipped exactly 60 records.
    assert_eq!(sim.bridge.stats().frames_shipped, 60);
}

/// Stress: 60 fps content, 20 Hz consumer, capacity 2 — latest-wins
/// drops are counted, order is never violated, the freshest frame
/// always survives, and latencies stay inside the structural bound.
#[test]
fn slow_consumer_latest_wins_with_structural_bounds() {
    let mut sim = Sim::new(FRAME_60, CONSUMER_20, 2);
    for i in 0..60u64 {
        let slot = i * FRAME_60;
        sim.present_cycle(slot, slot as i64, (i % 256) as u8);
    }
    sim.drain_all();

    let stats = sim.bridge.transport().mailbox().stats();
    // Conservation: every offered frame is drawn or dropped.
    assert_eq!(stats.offered, 60);
    assert!(stats.conserves(), "offered != drawn + drops: {stats:?}");
    assert!(
        stats.drawn < 60,
        "a 20 Hz consumer cannot draw 60 fps: {stats:?}"
    );
    assert!(
        stats.dropped_overflow > 0,
        "the stressor must overflow: {stats:?}"
    );

    // Order is never violated (strictly increasing ids).
    let mut sorted = sim.drawn_ids.clone();
    sorted.sort_unstable();
    assert_eq!(sim.drawn_ids, sorted, "draw order must be monotonic");
    // Latest-wins: the LAST drawn frame is the LAST presented one.
    assert_eq!(*sim.drawn_ids.last().must(), 59);

    // Structural latency bound: a frame waits at most capacity
    // producer periods (queued) + one consumer period (till the next
    // tick) after its emit.
    let latency = sim.transport_latency.summary();
    assert_eq!(latency.samples, stats.drawn);
    assert!(
        latency.min_ns.must() > 0,
        "no frame draws at its own emit instant"
    );
    assert!(
        latency.max_ns.must() <= (2 * FRAME_60 + CONSUMER_20) as i64,
        "latency {} ns exceeds the structural bound",
        latency.max_ns.must()
    );
}

/// Generation flush through the host signal: the command layer's
/// generation bump (a seek/stop class-A acceptance) flushes the
/// in-flight frames, counted as stale — the engine-side twin of the
/// scheduler's own purge.
#[test]
fn generation_bump_flushes_in_flight_frames() {
    use desktop::DesktopHost;
    use media_types::GenerationId;

    let timeline = Timeline::new();
    let transport = MailboxTransport::new(MailboxConfig {
        capacity: NonZeroUsize::new(8).must(),
    })
    .must();
    let mut bridge = CanvasBridge::with_clock(transport, Box::new(timeline.emit_clock()));

    // Three frames in flight (the consumer has not drawn yet).
    for i in 0..3u64 {
        let mut request = marked_request(EXTENT, (i * FRAME_30) as i64, i as u8);
        request.frame.generation = GenerationId::FIRST;
        bridge.present(&request).must();
    }
    assert_eq!(bridge.transport().mailbox().len(), 3);

    // The host sees a class-A acceptance at a NEW generation (a
    // seek, say) — the 1.D4 catalog's real Command/EventKind types.
    let mut host = DesktopHost::new();
    let id = host.next_request_id();
    let response = commands::CommandResponse::<commands::CommandResult> {
        request_id: id,
        session: Some(commands::SessionId(1)),
        generation: commands::Generation(2),
        result: Ok(commands::CommandOutcome::Accepted {
            session: Some(commands::SessionId(1)),
            generation: commands::Generation(2),
        }),
    };
    let observation = host.observe_response(&response);
    let (session, generation) = observation.generation_bump.must();
    assert_eq!(session, commands::SessionId(1));
    assert_eq!(generation, commands::Generation(2));

    // The wiring flushes on the signal.
    let flushed = bridge.transport_mut().mailbox_mut().flush_stale();
    assert_eq!(flushed, 3);
    let stats = bridge.transport().mailbox().stats();
    assert_eq!(stats.dropped_stale, 3);
    // 3 offered, 0 drawn, 3 stale: conserved.
    assert!(stats.conserves(), "{stats:?}");

    // Post-flush frames flow under the new generation.
    let mut request = marked_request(EXTENT, 1_000_000_000, 9);
    request.frame.generation = GenerationId::new(2);
    request.generation = GenerationId::new(2);
    bridge.present(&request).must();
    let drawn = bridge.transport_mut().mailbox_mut().draw_next().must();
    assert_eq!(drawn.header.frame_id, 3);
}

/// The honest stub: `lumen-desktop` reports the typed pending state
/// and exits with the documented code (never a fake window).
#[test]
fn stub_binary_reports_pending_state() {
    let output = Command::new(env!("CARGO_BIN_EXE_lumen-desktop"))
        .output()
        .must();
    assert_eq!(output.status.code(), Some(86), "WEBVIEW_PENDING exit code");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Issue #3 (4.H1 handoff c): the startup report CONSUMES the
    // product.toml [window] table — the build spec line is the proof.
    assert!(stdout.contains(
        "window build spec (product.toml [window]): 1280x720 (min 480x320)"
    ));
    assert!(stdout.contains("window geometry: remembered across runs"));
    assert!(stdout.contains("A (native wgpu surface)"));
    assert!(stdout.contains("B (canvas frame streaming)"));
    assert!(stdout.contains("A-or-B (prototype decides)"));
    assert!(stdout.contains("WebView2"));
    assert!(stdout.contains("WKWebView"));
    assert!(stdout.contains("WebKitGTK 4.1"));
    assert!(stdout.contains("LUF1"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ADR-004"));
}
