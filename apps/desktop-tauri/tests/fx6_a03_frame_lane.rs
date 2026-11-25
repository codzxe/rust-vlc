//! FX6-A03 — the tauri frame-lane bound (FIX-PLAN §3 F1.2; W14-A07's
//! unbounded-queue class, at FX4-A04's handoff-note site: the ship
//! side of `frame_channel.rs`).
//!
//! The object under test: [`FrameLane`] — the bounded, counted queue
//! `apps/desktop-tauri/src/frame_channel.rs` puts between the
//! ticker's drain and the webview dispatch (the queue FEEDING the
//! tauri Channel). Pure std, no webview: the bound and the drop
//! policy are provable headlessly, exactly like the mailbox seam
//! tests in apps/desktop/src/mailbox.rs (the upstream oracle).
//!
//! CI-gated with the whole crate: `webkit2gtk-4.1` is absent from
//! dev boxes, so the crate builds only in its dedicated lane (ci.yml
//! `tauri-check`, `cargo test --manifest-path
//! apps/desktop-tauri/Cargo.toml --locked --all-targets`) — the same
//! gate `tests/single_command.rs` and FX4-A04's audio-lane tests
//! ride. Nothing here needs a live webview; the tests exercise the
//! lane's own contract:
//!
//! * the stall bound — a dispatch that never takes (the stalled
//!   webview's shape) keeps the lane within its explicit capacity,
//!   counting every excess frame as a policy drop, never
//!   accumulating;
//! * latest-wins — the record that survives a burst is the NEWEST
//!   (bytewise), and the shipped record is the offered bytes
//!   VERBATIM (the LUF1 header's `pts_ns`/`emitted_ns` pass
//!   through; the lane never fabricates);
//! * conservation — every offered record is shipped or dropped,
//!   exactly once, or still queued;
//! * the bind-generation flush — a reloaded webview's lane is
//!   emptied wholesale, counted, never shipped stale.

use std::num::NonZeroUsize;

use lumen_desktop_tauri::frame_channel::{FrameLane, FRAME_LANE_CAPACITY};

/// One complete LUF1 record, hand-built on the wire layout
/// (apps/desktop/src/wire.rs's own table): 48-byte header — magic,
/// header_len, version, format, flags, reserved, width, height,
/// payload_len, `frame_id` @ 24, `pts_ns` @ 32, `emitted_ns` @ 40 —
/// plus a 1×1 RGBA payload (4 bytes). The timestamps ride INSIDE the
/// record bytes: the lane's contract is to pass them through
/// verbatim, never to read or fabricate them.
fn luf1_record(frame_id: u64, pts_ns: i64) -> Vec<u8> {
    let mut record = Vec::with_capacity(48 + 4);
    record.extend_from_slice(&[0x4C, 0x55, 0x46, 0x31]); // magic "LUF1"
    record.extend_from_slice(&48u16.to_le_bytes()); // header_len
    record.extend_from_slice(&1u16.to_le_bytes()); // version
    record.push(1); // format: Rgba8
    record.push(0); // flags: a fresh present
    record.extend_from_slice(&0u16.to_le_bytes()); // reserved
    record.extend_from_slice(&1u32.to_le_bytes()); // width
    record.extend_from_slice(&1u32.to_le_bytes()); // height
    record.extend_from_slice(&4u32.to_le_bytes()); // payload_len
    record.extend_from_slice(&frame_id.to_le_bytes()); // frame_id @ 24
    record.extend_from_slice(&pts_ns.to_le_bytes()); // pts_ns @ 32
    record.extend_from_slice(&(frame_id * 1_000).to_le_bytes()); // emitted_ns @ 40
    record.push(frame_id as u8); // payload: the frame's own byte
    record.push(0);
    record.push(0);
    record.push(0xFF);
    assert_eq!(record.len(), 52, "48-byte header + 4-byte payload");
    record
}

/// The stall bound (the W14-A07 class, made falsifiable): a dispatch
/// that never takes — the stalled webview's shape — keeps the lane
/// within its explicit capacity while the ticker keeps offering.
/// Every excess frame is a COUNTED policy drop; the lane's retained
/// memory is capacity × record size (a constant — 2 records ≈ 7.4 MB
/// at 1280×720 RGBA, see [`FRAME_LANE_CAPACITY`]'s doc), never a
/// function of the stall's duration.
#[test]
fn fx6_a03_stalled_dispatch_stays_bounded_and_counts_drops() {
    let mut lane = FrameLane::new();
    assert_eq!(lane.queued(), 0);
    assert!(!lane.is_full());
    // A 3×-capacity burst with NO dispatch (the stalled consumer):
    let offered = 6 * FRAME_LANE_CAPACITY.get() as u64;
    for i in 0..offered {
        lane.offer(luf1_record(i, i * 1_000));
        assert!(
            lane.queued() <= FRAME_LANE_CAPACITY.get(),
            "occupancy must never cross the explicit capacity (offer {i})"
        );
    }
    let stats = lane.stats();
    assert_eq!(stats.offered, offered);
    assert_eq!(
        stats.dropped_overflow,
        offered - FRAME_LANE_CAPACITY.get() as u64,
        "the excess over capacity is dropped by policy, counted"
    );
    assert!(lane.is_full());
    assert!(lane.conserves());
    // Latest-wins under the stall: the survivor handed to the
    // (eventual) dispatch is the NEWEST burst member, bytewise —
    // carrying its own timestamps, not the lane's.
    let survivor = match lane.take_newest() {
        Some(record) => record,
        None => unreachable!("a full lane must hand the dispatch a record"),
    };
    assert_eq!(survivor, luf1_record(offered - 1, (offered - 1) * 1_000));
    assert_eq!(lane.queued(), 0);
    assert!(lane.conserves());
}

/// The one-frame-per-tick contract, counted: the dispatch ships the
/// NEWEST record and the superseded older records are COUNTED drops
/// (the old ticker's `records.last()` discarded them silently — the
/// audit's "no frame counters in the ticker lane" gap).
#[test]
fn fx6_a03_take_newest_ships_newest_counts_superseded() {
    let mut lane = FrameLane::new();
    for i in 0..2u64 {
        lane.offer(luf1_record(i, i * 1_000));
    }
    let shipped = match lane.take_newest() {
        Some(record) => record,
        None => unreachable!("the lane held two records"),
    };
    // Bytewise identity: the shipped record IS the newest offered
    // record — header, timestamps and payload verbatim.
    assert_eq!(shipped, luf1_record(1, 1_000));
    // The lane is empty after a dispatch (one frame per tick; the
    // next tick's records start fresh, no zombie leftovers).
    assert_eq!(lane.queued(), 0);
    assert!(lane.take_newest().is_none());
    let stats = lane.stats();
    assert_eq!(stats.offered, 2);
    assert_eq!(stats.shipped, 1);
    assert_eq!(stats.dropped_superseded, 1);
    assert_eq!(stats.dropped_overflow, 0);
    assert!(lane.conserves());
}

/// Timestamp passthrough (FIX-PLAN §3 F1.2's "timestamp passthrough
/// — no more `records.last()` silently dropping intermediate
/// frames"): the shipped bytes are the offered bytes — the `pts_ns`
/// laid at header offset 32 rides untouched, two frames with
/// different slots never blur into one another, and the lane ships
/// records without ever writing them.
#[test]
fn fx6_a03_shipped_records_carry_upstream_timestamps_verbatim() {
    let mut lane = FrameLane::new();
    let first = luf1_record(10, 1_234_567);
    let second = luf1_record(11, 1_234_600);
    lane.offer(first.clone());
    lane.offer(second.clone());
    let shipped = match lane.take_newest() {
        Some(record) => record,
        None => unreachable!("the lane held two records"),
    };
    assert_eq!(shipped, second, "the newest record ships bytewise");
    assert_ne!(
        shipped, first,
        "the two slots stay distinct — no fabricated or blurred record"
    );
    // The pts_ns field (offset 32, 8 bytes little-endian) is the
    // UPSTREAM value, not a lane invention:
    let pts_bytes: [u8; 8] = match shipped[32..40].try_into() {
        Ok(bytes) => bytes,
        Err(_) => unreachable!("offset 32..40 of a 52-byte record is exactly 8 bytes"),
    };
    assert_eq!(i64::from_le_bytes(pts_bytes), 1_234_600);
    let emitted_bytes: [u8; 8] = match shipped[40..48].try_into() {
        Ok(bytes) => bytes,
        Err(_) => unreachable!("offset 40..48 of a 52-byte record is exactly 8 bytes"),
    };
    assert_eq!(u64::from_le_bytes(emitted_bytes), 11_000);
}

/// Conservation under mixed traffic: bursts, dispatch ticks, an
/// empty tick, a refilling burst — the identity holds after EVERY
/// step (every offered record shipped, dropped, or still queued,
/// exactly once), and occupancy never crosses the capacity.
#[test]
fn fx6_a03_conservation_holds_under_mixed_traffic() {
    let three = match NonZeroUsize::new(3) {
        Some(value) => value,
        None => unreachable!("3 is non-zero"),
    };
    let mut lane = FrameLane::with_capacity(three);
    for tick in 0..24u64 {
        let burst = tick % 4;
        for k in 0..burst {
            let id = tick * 4 + k;
            lane.offer(luf1_record(id, id * 333));
        }
        if tick % 2 == 0 {
            if let Some(record) = lane.take_newest() {
                assert_eq!(record.len(), 52, "a complete record, never a fragment");
            }
        }
        assert!(
            lane.queued() <= three.get(),
            "occupancy within capacity on tick {tick}"
        );
        assert!(
            lane.conserves(),
            "conservation must hold after every tick (tick {tick}: {:?})",
            lane.stats()
        );
    }
    // Drain to empty: the pure identity then holds exactly.
    while lane.queued() > 0 {
        assert!(lane.take_newest().is_some());
    }
    let stats = lane.stats();
    assert!(stats.offered > 0, "the traffic was non-empty");
    assert!(stats.shipped > 0, "the dispatches did happen");
    assert!(lane.conserves());
    assert_eq!(lane.queued(), 0);
}

/// The bind-generation flush (the mailbox's own `flush_stale`
/// semantics, mirrored at the ship side): a reloaded webview's
/// re-bind empties the lane wholesale, counted once — nothing from
/// the previous page ships stale.
#[test]
fn fx6_a03_rebind_flushes_pending_records_counted() {
    let mut lane = FrameLane::new();
    for i in 0..2u64 {
        lane.offer(luf1_record(i, i * 1_000));
    }
    assert_eq!(lane.flush_stale(), 2, "both queued records drop wholesale");
    assert_eq!(lane.queued(), 0);
    assert_eq!(
        lane.flush_stale(),
        0,
        "a second flush drops nothing (wholesale, not cumulative)"
    );
    let stats = lane.stats();
    assert_eq!(stats.dropped_stale, 2);
    assert!(lane.conserves());
    // Post-flush frames flow normally (the fresh page's records).
    lane.offer(luf1_record(50, 50_000));
    let shipped = match lane.take_newest() {
        Some(record) => record,
        None => unreachable!("the fresh record is queued"),
    };
    assert_eq!(shipped, luf1_record(50, 50_000));
    assert!(lane.conserves());
}
