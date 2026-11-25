//! FX6-A02 — the ticker lane's stamps + receipt surface (FIX-PLAN §3
//! F1.2, wave-FX6 row FX6-A02: "10 ms ticker ships records.last()
//! only (app.rs:223-229); no drop-policy awareness, no frame counters
//! in the ticker lane").
//!
//! The unit under test: `lumen_desktop_tauri::app::ShipLog` — row
//! A02's half of the ticker lane, composed with the surfaces this wave
//! landed around it:
//!
//! * FX6-A03's bounded lane (`frame_channel::FrameLane`) carries the
//!   policy and the counters between the ticker's drain and the
//!   webview dispatch (offer/take_newest, drop-oldest,
//!   flush-on-rebind) — its own tests (`fx6_a03_frame_lane.rs`) pin
//!   the bound and the policy;
//! * FX6-A01's mailbox (apps/desktop/src/mailbox.rs) keeps the
//!   per-record timestamps through its drain — the stamps ride the
//!   LUF1 records the ticker drains (`drain_to`'s passthrough);
//! * THIS unit closes the row's remaining gap: the dispatched
//!   record's own wire stamps read and latched where the lane can see
//!   them (the timestamp passthrough the pre-FX6 ticker never
//!   performed), and the periodic produced-vs-dropped receipt (every
//!   128 ships, the app's stderr diagnostic convention) built over
//!   the bounded lane's counters.
//!
//! The tests drive the exact composition the ticker's dispatch half
//! runs (take_newest → note_dispatched → the receipt over
//! `lane.stats()`), against records built by the desktop crate's own
//! wire encoder — the same encoder the frame tap's drain uses, so the
//! stamp reads are proven against the encoder's real output, not a
//! hand-built byte mirror. The receipt's print cadence itself rides
//! the ticker thread (launch/hardware lanes observe it on stderr);
//! everything here is the pure half. CI-gated with the whole crate
//! (`tauri-check` installs webkit2gtk-4.1) — the same gate
//! `fx4_a04_audio_lane.rs` and `fx6_a03_frame_lane.rs` ride; nothing
//! here needs a live webview.

use desktop::wire::{encode, FormatCode, FrameFlags, FrameHeader};
use lumen_desktop_tauri::app::{RecordStamps, ShipLog};
use lumen_desktop_tauri::frame_channel::{FrameLane, FrameLaneStats};

/// One complete LUF1 record with the given stamps — built by the
/// desktop crate's own wire encoder (the same encoder the frame tap's
/// drain re-encodes through), so the stamp reads below are proven
/// against the encoder's real output: 1×1 RGBA, payload 4 bytes.
fn luf1_record(frame_id: u64, pts_ns: i64, emitted_ns: u64) -> Vec<u8> {
    let header = FrameHeader {
        format: FormatCode::Rgba8,
        flags: FrameFlags::default(),
        width: 1,
        height: 1,
        payload_len: 4,
        frame_id,
        pts_ns,
        emitted_ns,
    };
    match encode(&header, &[9u8, 9, 9, 255]) {
        Ok(record) => record,
        Err(error) => unreachable!("a hand-built 1x1 RGBA record always encodes: {error}"),
    }
}

/// Timestamps ride the shipped record (the audit's "no timestamps on
/// the shipped record" gap, closed at the read): the observer latches
/// exactly the stamps the record carries — the encoder's own
/// `frame_id`, `pts_ns`, `emitted_ns` — and re-latches on every
/// dispatch. Unreadable bytes latch `None`: a visible anomaly, never
/// a guess.
#[test]
fn fx6_a02_dispatched_record_stamps_ride_the_lane() {
    let mut log = ShipLog::default();
    assert_eq!(
        log.last_stamps(),
        None,
        "a fresh observer has latched nothing"
    );
    log.note_dispatched(&luf1_record(9, 90_000, 900));
    assert_eq!(
        log.last_stamps(),
        Some(RecordStamps {
            frame_id: 9,
            pts_ns: 90_000,
            emitted_ns: 900,
        }),
        "the stamps latched are the record's own"
    );
    // A later dispatch re-latches: the newest dispatch is the truth.
    log.note_dispatched(&luf1_record(10, 100_000, 1_000));
    assert_eq!(log.last_stamps().map(|stamps| stamps.frame_id), Some(10));
    // A truncated record and a non-LUF1 buffer both fail the read.
    log.note_dispatched(&[0x4Cu8, 0x55, 0x46, 0x31]);
    assert_eq!(log.last_stamps(), None, "a truncated record has no stamps");
    log.note_dispatched(&vec![0u8; 64]);
    assert_eq!(log.last_stamps(), None, "a non-LUF1 buffer has no stamps");
}

/// The counters increment as the mailbox's drain reports them: tick by
/// tick, the bounded lane's offered/shipped/dropped_* counters track
/// exactly what the drain yielded and the dispatch took (one record
/// per tick; the rest counted, never silent), and the conservation
/// identity holds after every tick.
#[test]
fn fx6_a02_lane_counters_increment_as_the_mailbox_reports() {
    let mut lane = FrameLane::new();
    let mut log = ShipLog::default();
    // Tick 1: the drain yields one record — it dispatches.
    lane.offer(luf1_record(1, 10_000, 100));
    let taken = match lane.take_newest() {
        Some(record) => record,
        None => unreachable!("the tick-1 record is queued"),
    };
    log.note_dispatched(&taken);
    let stats = lane.stats();
    assert_eq!(
        (stats.offered, stats.shipped, stats.dropped_superseded),
        (1, 1, 0)
    );
    assert!(lane.conserves());
    // Tick 2: the drain yields a burst of two — one dispatches, the
    // superseded one is counted (the pre-FX6 silent drop, now
    // arithmetic).
    lane.offer(luf1_record(2, 20_000, 200));
    lane.offer(luf1_record(3, 30_000, 300));
    let taken = match lane.take_newest() {
        Some(record) => record,
        None => unreachable!("the tick-2 burst is queued"),
    };
    log.note_dispatched(&taken);
    let stats = lane.stats();
    assert_eq!(
        (stats.offered, stats.shipped, stats.dropped_superseded),
        (3, 2, 1)
    );
    assert_eq!(stats.dropped_overflow, 0);
    assert_eq!(stats.dropped_stale, 0);
    assert!(lane.conserves());
    // Tick 3: the drain yields nothing — a counted no-op.
    assert!(
        lane.take_newest().is_none(),
        "an empty lane dispatches nothing"
    );
    let stats = lane.stats();
    assert_eq!((stats.offered, stats.shipped), (3, 2));
    // The stamps track the newest dispatched record, tick over tick.
    assert_eq!(log.last_stamps().map(|stamps| stamps.frame_id), Some(3));
}

/// The receipt — the app's stderr diagnostic convention — carries the
/// produced-vs-dropped counters AND the shipped record's stamps: what
/// the F1.2 hardware run reads off stderr, at the 128-ship cadence
/// the ticker prints it at. A capacity burst exercises both drop
/// buckets: the overflow drop (the lane was full) and the superseded
/// drop (the one-frame-per-tick coalesce).
#[test]
fn fx6_a02_receipt_carries_produced_dropped_and_shipped_stamps() {
    let mut lane = FrameLane::new();
    let mut log = ShipLog::default();
    lane.offer(luf1_record(7, 70_000, 700));
    lane.offer(luf1_record(8, 80_000, 800));
    lane.offer(luf1_record(9, 90_000, 900)); // capacity 2: id 7 drops by policy
    let taken = match lane.take_newest() {
        Some(record) => record,
        None => unreachable!("the burst is queued"),
    };
    log.note_dispatched(&taken);
    let receipt = log.receipt(&lane.stats());
    // offered 3, shipped 1, dropped 2 = overflow 1 + superseded 1.
    for needed in [
        "shipped 1",
        "produced 3",
        "dropped 2",
        "overflow 1",
        "superseded 1",
        "stale 0",
        "last: frame_id 9",
        "pts_ns 90000",
        "emitted_ns 900",
    ] {
        assert!(
            receipt.contains(needed),
            "the receipt must carry {needed:?}: {receipt}"
        );
    }
}

/// The receipt without latched stamps reports the counters alone — the
/// anomaly (an unreadable dispatched record) is visible as a missing
/// stamp line, never as a fabricated stamp.
#[test]
fn fx6_a02_receipt_reports_counters_alone_when_no_stamps_latched() {
    let mut log = ShipLog::default();
    let lane_stats = FrameLaneStats {
        offered: 4,
        shipped: 4,
        ..FrameLaneStats::default()
    };
    let receipt = log.receipt(&lane_stats);
    for needed in ["shipped 4", "produced 4", "dropped 0"] {
        assert!(
            receipt.contains(needed),
            "the receipt must carry {needed:?}: {receipt}"
        );
    }
    assert!(
        !receipt.contains("last:"),
        "no stamp line without a latched stamp: {receipt}"
    );
    // A garbage dispatch latches nothing; the counters stay the truth.
    log.note_dispatched(&vec![7u8; 64]);
    assert_eq!(log.last_stamps(), None);
}
