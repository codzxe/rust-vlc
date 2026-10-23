//! W6-05 allocation proofs and the measured audit of every
//! `SharedBytes` construction path (cards: memory-audit V-2 fix,
//! W5-19's `build` handoff, W5-14's range-flavor handoff).
//!
//! Three runtime proofs (D-6 — measured, not argued, with the testkit
//! counting allocator):
//!
//! 1. `clone` and view minting (`from_pool_range`/`slice`) allocate
//!    **nothing** — the Arc-bump-only contract of the hottest data
//!    structure (D-1);
//! 2. [`SharedBytes::build`] allocates the payload **once** and fills
//!    in place: exactly one payload-sized event plus the constant-size
//!    `Arc` header, and no post-fill copy (the pre-W6-05
//!    build-`Vec`-then-`from_vec` pattern paid a second payload-sized
//!    block — W5-19's measured double-copy);
//! 3. [`SharedBytes::from_vec`] moves the payload allocation in: its
//!    own cost is one constant-size header event, independent of
//!    payload length (V-2: the old `Arc<[u8]>` conversion allocated a
//!    second payload-sized block and copied every byte).
//!
//! The audit table (visible with `cargo test -p media-types --test
//! shared_bytes_alloc -- --nocapture`) measures the OLD core
//! operations side by side under the same allocator in the same
//! process — before/after under identical protocol, no numbers copied
//! from anywhere else. Allocation *counts and bytes* are what the
//! counting allocator sees; memcpy costs are structural facts of the
//! std conversions (`Arc::from(Vec)` copies, `Arc::from(&[u8])` copies,
//! `Arc::new(Vec)` moves) and are labelled as such, not measured here.
//! Counts are deterministic, not load-sensitive: no timing claims are
//! made in this file.
//!
//! Whole-binary serialization follows the house discipline (see
//! `testkit::alloc_count` module docs): the allocator is process-global
//! and counts everything, including a libtest harness thread spawn that
//! can land inside an open window — hence the settle pause plus the
//! single-replay backstop (the same foreign-noise discipline as
//! demux's `zero_alloc_read_chain`; a real defect reproduces on every
//! replay, harness noise does not survive the retry).

use media_types::packet::SharedBytes;
use std::sync::Arc;
use testkit::alloc_count::{AllocDelta, CountingGlobalAlloc, serialize_tests};

#[global_allocator]
static ALLOC: CountingGlobalAlloc = CountingGlobalAlloc::new();

/// Audit payload size: the demux slot-cap figure from the V-1
/// [PROPOSAL] (64 KiB) — the size class the spill path actually pays.
const N: usize = 65_536;

/// Let an in-flight sibling-thread spawn finish before a measured
/// window opens (the `zero_alloc_read_chain` diagnostic: harness
/// spawns landed 2–10 events inside open windows without this pause).
fn settle_foreign_spawns() {
    std::thread::sleep(std::time::Duration::from_millis(30));
}

fn run_once(section: &mut dyn FnMut()) -> AllocDelta {
    let guard = ALLOC.state().guard();
    section();
    guard.delta()
}

/// A deterministic measured section under the settle + single-replay
/// discipline: a window that fails `accept` is re-measured once; a
/// defect that fails BOTH replays is real (D-5: this hardens against
/// the environment, never against the code).
fn clean_delta_with(mut section: impl FnMut(), accept: impl Fn(&AllocDelta) -> bool) -> AllocDelta {
    settle_foreign_spawns();
    let first = run_once(&mut section);
    if accept(&first) {
        return first;
    }
    settle_foreign_spawns();
    let second = run_once(&mut section);
    assert!(
        accept(&second),
        "measured dirty on BOTH deterministic replays — a real allocation \
         defect (first {first:?}, second {second:?})"
    );
    second
}

/// The zero-allocation predicate (the same contract
/// `CountingGuard::assert_zero` enforces).
fn is_zero(delta: &AllocDelta) -> bool {
    delta.alloc_calls == 0 && delta.total_bytes == 0
}

/// Calibrate the constant-size `Arc` header event (`ArcInner` around a
/// `Vec<u8>`: 40 bytes on this 64-bit toolchain) so the exact-count
/// proofs are self-describing instead of pinning std layout internals.
fn arc_header_event() -> u64 {
    let delta = clean_delta_with(
        || {
            let arc = Arc::new(Vec::<u8>::new());
            drop(arc);
        },
        |d| d.alloc_calls == 1,
    );
    assert_eq!(delta.alloc_calls, 1, "Arc::new must be exactly one event");
    delta.total_bytes
}

#[test]
fn clone_is_an_arc_bump_only() {
    let _serial = serialize_tests();
    let sb = SharedBytes::from_vec(vec![0xAB; N]);
    let pool = Arc::new(vec![0x0Eu8; N]);
    let view = SharedBytes::from_pool_range(Arc::clone(&pool), 8, 4096);

    let delta = clean_delta_with(
        || {
            for _ in 0..1000 {
                let c = sb.clone();
                assert_eq!(c.len(), N);
                let v = view.clone();
                assert_eq!(v.len(), 4096);
            }
        },
        is_zero,
    );
    assert!(
        delta.alloc_calls == 0 && delta.total_bytes == 0,
        "1000 clones + 1000 view clones: {delta:?}"
    );
}

#[test]
fn view_minting_allocates_nothing() {
    let _serial = serialize_tests();
    let pool = Arc::new(vec![0x11u8; N]);
    let full = SharedBytes::from_pool_range(Arc::clone(&pool), 0, N);
    let warm = full.slice(0..64);
    drop(warm);

    let delta = clean_delta_with(
        || {
            for i in 0..1000 {
                // The (Arc, offset, len) constructor and the slicing
                // method: the demux V-1 upgrade path's per-packet cost.
                let a = SharedBytes::from_pool_range(Arc::clone(&pool), i, 512);
                assert_eq!(a.len(), 512);
                let b = a.slice(0..256);
                assert_eq!(b.len(), 256);
                drop(b);
                drop(a);
            }
        },
        is_zero,
    );
    assert!(
        delta.alloc_calls == 0 && delta.total_bytes == 0,
        "1000 view pairs: {delta:?}"
    );
}

#[test]
fn sole_owner_refills_in_place_without_allocating() {
    let _serial = serialize_tests();
    // The W5-19 slot-rotation shape: a handle whose allocation has no
    // other owner refills through get_mut (D-1 buffer recycling).
    let mut sole = SharedBytes::from_vec(vec![0x22u8; 256]);
    let delta = clean_delta_with(
        || {
            let bytes = sole
                .get_mut()
                .unwrap_or_else(|| unreachable!("sole owner must refill in place"));
            bytes[0] = 0xEE;
            assert_eq!(bytes.len(), 256);
        },
        is_zero,
    );
    assert_eq!(sole.as_slice()[0], 0xEE, "the in-place refill landed");
    assert!(
        delta.alloc_calls == 0 && delta.total_bytes == 0,
        "in-place refill: {delta:?}"
    );
}

#[test]
fn build_allocates_the_payload_once_and_fills_in_place() {
    let _serial = serialize_tests();
    let header_bytes = arc_header_event();
    // Content check outside the measured window.
    let probe = SharedBytes::build(N, |buf| {
        assert_eq!(buf.len(), N);
        for (i, b) in buf.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }
    });
    assert_eq!(probe.len(), N);
    assert_eq!(probe.as_slice()[0], 0);
    assert_eq!(probe.as_slice()[N - 1], ((N - 1) % 251) as u8);
    drop(probe);

    let delta = clean_delta_with(
        || {
            let sb = SharedBytes::build(N, |buf| buf.fill(0x5A));
            assert_eq!(sb.len(), N);
            drop(sb);
        },
        |d| d.alloc_calls == 2,
    );
    // Exactly one payload-sized event plus the constant header — never
    // a second payload-sized block, never a post-fill copy.
    assert_eq!(
        delta.alloc_calls, 2,
        "one payload block + one Arc header: {delta:?}"
    );
    assert_eq!(
        delta.total_bytes,
        N as u64 + header_bytes,
        "payload {N} + constant header {header_bytes}: {delta:?}"
    );

    // Zero-length build: just the header event (the empty Vec does not
    // allocate).
    let empty = clean_delta_with(
        || {
            let sb = SharedBytes::build(0, |buf| assert!(buf.is_empty()));
            assert!(sb.is_empty());
            drop(sb);
        },
        |d| d.alloc_calls == 1,
    );
    assert_eq!(empty.total_bytes, header_bytes, "header only: {empty:?}");
}

#[test]
fn from_vec_moves_the_payload_in_v2() {
    let _serial = serialize_tests();
    let header_bytes = arc_header_event();

    // Differential proof, per size: the caller pattern (fresh Vec +
    // wrap) costs exactly the Vec's own event plus the constant-size
    // header — from_vec's own contribution never scales with the
    // payload. (The Vec is prepared inside each section so the replay
    // discipline stays self-contained; its cost is measured separately
    // and subtracted.)
    for size in [1usize, 4096, N, 1 << 20] {
        let prep = clean_delta_with(
            || {
                let v = vec![0x33u8; size];
                drop(v);
            },
            |d| d.alloc_calls == 1,
        );
        let whole = clean_delta_with(
            || {
                let sb = SharedBytes::from_vec(vec![0x33u8; size]);
                assert_eq!(sb.len(), size);
                drop(sb);
            },
            |d| d.alloc_calls == 2,
        );
        assert_eq!(
            prep.alloc_calls + 1,
            whole.alloc_calls,
            "from_vec is exactly one event at size {size}"
        );
        assert_eq!(
            prep.total_bytes + header_bytes,
            whole.total_bytes,
            "from_vec adds only the constant header at size {size}: {whole:?} vs {prep:?}"
        );
    }
}

#[test]
fn audit_table_of_measured_costs() {
    let _serial = serialize_tests();
    let header_bytes = arc_header_event();

    // --- old core operations, replicated verbatim in this process ---
    // Old from_vec core: Vec -> Arc<[u8]> (allocates a second
    // payload-sized block; copies the bytes — std-documented, the copy
    // itself is not allocator-visible).
    let old_from_vec = clean_delta_with(
        || {
            let v = vec![0u8; N];
            let arc: Arc<[u8]> = v.into();
            assert_eq!(arc.len(), N);
            drop(arc);
        },
        |d| d.alloc_calls == 2,
    );
    assert!(
        old_from_vec.total_bytes >= 2 * N as u64,
        "the old core op paid two payload-sized blocks: {old_from_vec:?}"
    );

    static AUDIT_BYTES: [u8; 4096] = [0xAB; 4096];
    // Old from_static core: &[u8] -> Arc<[u8]> (allocates + copies —
    // the old "no allocation, no copy" doc was wrong).
    let old_from_static_full = clean_delta_with(
        || {
            let arc: Arc<[u8]> = Arc::from(&AUDIT_BYTES[..]);
            assert_eq!(arc.len(), 4096);
            drop(arc);
        },
        |d| d.alloc_calls == 1,
    );
    let old_from_static_empty = clean_delta_with(
        || {
            let empty: &[u8] = &[];
            let arc: Arc<[u8]> = Arc::from(empty);
            assert_eq!(arc.len(), 0);
            drop(arc);
        },
        |d| d.alloc_calls <= 2,
    );

    // --- the new paths, same protocol ---
    let new_from_vec = clean_delta_with(
        || {
            let sb = SharedBytes::from_vec(vec![0u8; N]);
            assert_eq!(sb.len(), N);
            drop(sb);
        },
        |d| d.alloc_calls == 2,
    );
    let new_from_static_full = clean_delta_with(
        || {
            let sb = SharedBytes::from_static(&AUDIT_BYTES[..]);
            assert_eq!(sb.len(), 4096);
            drop(sb);
        },
        |d| d.alloc_calls == 2,
    );
    let new_from_static_empty = clean_delta_with(
        || {
            let sb = SharedBytes::from_static(&[]);
            assert!(sb.is_empty());
            drop(sb);
        },
        |d| d.alloc_calls == 1,
    );

    // The printed audit record (raw data on disk via --nocapture);
    // copies are structural std facts, labelled not measured.
    println!(
        "SharedBytes allocation audit (payload N = {N}, static 4096, this process, pinned toolchain 1.99.1)"
    );
    println!(
        "    old from_vec core (Vec -> Arc<[u8]>):   {} events, {} bytes, + 1 payload memcpy (std-documented)",
        old_from_vec.alloc_calls, old_from_vec.total_bytes
    );
    println!(
        "    new from_vec (Arc::new(Vec), moves):    {} events, {} bytes, 0 payload copies",
        new_from_vec.alloc_calls, new_from_vec.total_bytes
    );
    println!(
        "    old from_static core (Arc::from slice): {} events, {} bytes, + 1 payload memcpy (std-documented)",
        old_from_static_full.alloc_calls, old_from_static_full.total_bytes
    );
    println!(
        "    new from_static (to_vec + Arc::new):    {} events, {} bytes, 1 payload copy (a static slice cannot be adopted in safe Rust)",
        new_from_static_full.alloc_calls, new_from_static_full.total_bytes
    );
    println!(
        "    old from_static empty:                  {} events, {} bytes",
        old_from_static_empty.alloc_calls, old_from_static_empty.total_bytes
    );
    println!(
        "    new from_static empty:                  {} events, {} bytes (no payload copy)",
        new_from_static_empty.alloc_calls, new_from_static_empty.total_bytes
    );
    println!("    Arc header calibration (Arc::new(Vec::new())): {header_bytes} bytes");
    println!(
        "    build(N, fill): 2 events = payload {N} + header {header_bytes}; zero-init memset; no post-fill copy (separate proof above)"
    );

    // The deltas that matter, asserted: the caller pattern keeps ONE
    // payload-sized block (the old core paid two).
    assert_eq!(new_from_vec.total_bytes, N as u64 + header_bytes);
    assert!(
        new_from_vec.total_bytes < old_from_vec.total_bytes,
        "V-2: the second payload-sized block is gone ({} -> {})",
        old_from_vec.total_bytes,
        new_from_vec.total_bytes
    );
    // from_static full: the same single payload copy as the old core,
    // one extra constant-size header event — the honest cost of the
    // new repr on a path whose only production callers pass b"".
    assert_eq!(
        new_from_static_full.alloc_calls,
        old_from_static_full.alloc_calls + 1
    );
}
