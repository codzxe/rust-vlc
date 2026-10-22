//! The D-1/D-6 measured proof for the M11 epoch-aware staleness
//! decision (ED-FIX-4): the drain-time check the output hub applies
//! per resident event —
//! [`DrainStaleness::is_stale_at_drain`](playback_engine::DrainStaleness::is_stale_at_drain)
//! — allocates **nothing**, measured under a supersede-then-push
//! storm and on the all-fresh common path.
//!
//! # Why a dedicated binary
//!
//! The counting allocator is process-global
//! (see [`testkit::alloc_count`]); this binary exists so the measured
//! windows are deterministic — exactly one test, whole-binary
//! serialization, every allocation inside a window is the code under
//! test's own (the `alloc_count_e2e.rs` discipline).
//!
//! # What is measured
//!
//! The storm window is the hub's drain shape itself — pop one
//! resident output, decide, count — over 10 000 undelivered events of
//! a superseded epoch (mixed kinds, seek terminals included: the
//! cross-epoch arm must classify them stale without the exemption)
//! with receipts interleaved. The construction of the storm (queue
//! pushes, event clones) happens **before** the guard: the drain is
//! the hot path, the push is setup. Bounded work is the claim, and
//! bounded work is what is measured: 10 001 decisions, zero
//! allocation events, zero bytes.

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

use std::collections::VecDeque;

use commands::envelope::{Generation, RequestId, SessionId};
use playback_engine::epoch::{GenerationPair, PipelineGeneration, SessionEpoch};
use playback_engine::event::EngineEventKind;
use playback_engine::session::{CommandReceipt, ReceiptResult, SessionCommandKind};
use playback_engine::{DrainStaleness, EngineEvent, EngineOutput};
use testkit::alloc_count::serialize_tests;

fn current_pair() -> GenerationPair {
    GenerationPair {
        epoch: SessionEpoch(2),
        pipeline: PipelineGeneration(0),
    }
}

fn resident_event(epoch: u64, counter: u64, seq: u64, kind: EngineEventKind) -> EngineOutput {
    EngineOutput::Event(EngineEvent {
        seq,
        session: SessionId(epoch),
        generation: Generation(counter),
        origin: None,
        kind,
    })
}

fn resident_receipt(request: u64, epoch: u64) -> EngineOutput {
    EngineOutput::Receipt(CommandReceipt {
        request: RequestId(request),
        command: SessionCommandKind::Play,
        result: ReceiptResult::Completed {
            session: SessionId(epoch),
        },
    })
}

fn storm_kind(seq: u64) -> EngineEventKind {
    match seq % 4 {
        0 => EngineEventKind::EndOfMedia,
        1 => EngineEventKind::TimeTick {
            position_ns: seq as i64,
            fraction: 0.5,
        },
        2 => EngineEventKind::Buffering { progress: 0.5 },
        _ => EngineEventKind::Seeked {
            requested: 0,
            actual: 0,
        },
    }
}

#[test]
fn the_epoch_staleness_decision_is_zero_allocation_under_storm() {
    let _serial = serialize_tests();
    let current = current_pair();

    // ---- Setup (outside every measured window): the storm and the
    // fresh population, in the hub's own queue shape. ----
    let mut storm: VecDeque<EngineOutput> = VecDeque::new();
    for seq in 0..10_000u64 {
        storm.push_back(resident_event(1, 3, seq, storm_kind(seq)));
        if seq % 1_000 == 0 {
            storm.push_back(resident_receipt(seq, 1));
        }
    }
    let mut fresh: VecDeque<EngineOutput> = VecDeque::new();
    for seq in 0..10_000u64 {
        fresh.push_back(resident_event(2, 0, seq, storm_kind(seq)));
    }

    // ---- Window 1: the supersede-then-push storm drain. ----
    let guard = ALLOC.state().guard();
    let mut suppressed_stale = 0u64;
    let mut receipts_delivered = 0u64;
    let mut events_delivered = 0u64;
    while let Some(output) = storm.pop_front() {
        match output {
            EngineOutput::Receipt(_) => receipts_delivered += 1,
            EngineOutput::Event(event) => {
                if event.is_stale_at_drain(&current) {
                    suppressed_stale = suppressed_stale.saturating_add(1);
                } else {
                    events_delivered += 1;
                }
            }
        }
    }
    guard.assert_zero("the epoch-staleness storm drain (10_001 decisions)");
    assert_eq!(
        suppressed_stale, 10_000,
        "every storm event stale by construction"
    );
    assert_eq!(receipts_delivered, 10, "receipts are never suppressed");
    assert_eq!(events_delivered, 0, "no superseded-epoch event survives");

    // ---- Window 2: the all-fresh common path (the live pair's own
    // events) — the decision must be just as flat when it passes
    // everything. ----
    let guard = ALLOC.state().guard();
    let mut delivered = 0u64;
    while let Some(output) = fresh.pop_front() {
        match output {
            EngineOutput::Receipt(_) => delivered += 1,
            EngineOutput::Event(event) => {
                if !event.is_stale_at_drain(&current) {
                    delivered += 1;
                }
            }
        }
    }
    guard.assert_zero("the all-fresh drain (10_000 decisions)");
    assert_eq!(delivered, 10_000, "the live pair's events all pass");
}
