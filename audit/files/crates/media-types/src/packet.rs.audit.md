# AUDIT: crates/media-types/src/packet.rs
| Field | Value |
|---|---|
| Auditor | W02-A10 |
| Date | 2026-11-24 |
| Lines / functions | 589 / 15 prod fns (4 impls + 15 tests + 1 helper) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli (lumen-cli) Cargo.toml depends on media-types; apps/cli/src/pipeline_host.rs:2 `use media_types::packet::SharedBytes;`; demux (lumen-cli dep) uses `SharedBytes` in avi/riff.rs:595,623,647,663 and every format module; packet-queue `BoundedQueue<Packet>` is the playback-engine queue stage (crates/playback-engine/Cargo.toml:39-40). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p media-types --lib packet::` -> 15 passed, exit 0; `cargo test -p media-types --test shared_bytes_alloc` -> 6 passed, exit 0; rg for callers (from_static/from_vec/from_pool_range/build/get_mut); sed quote verification (all quotes below re-printed verbatim by sed). |

## 1. One-paragraph verdict
This file is real and tight. `SharedBytes` is an `(Arc<Vec<u8>>, Range<usize>)` view type whose five constructors each maintain the documented `0 <= start <= end <= buf.len()` invariant (asserted in `from_pool_range`:144-153 and `slice`:170-182, structural in the other three); `slice`'s additions cannot overflow given that invariant (`start + range.end <= self.range.end <= buf.len()`). Every allocation claim in the 40-line doc-comment is backed by a counting-global-allocator integration test that I ran and that passed: clone/view-minting allocate nothing, `build` costs exactly payload+header events, `from_vec` adds only the constant header. The `from_static` doc admits it copies — honest, and I verified the "only production shape is empty slices" claim against riff.rs. Tests are hostile to the code (panic-path coverage via catch_unwind, pointer-identity sharing checks, Arc::strong_count accounting, content-vs-identity hash equality). Remaining defects are cosmetic.

## 2. What actually works (verified)
- All 15 in-file unit tests pass — `cargo test -p media-types --lib packet::` -> `test result: ok. 15 passed; 0 failed; 0 ignored`.
- All 6 allocation proofs pass — `cargo test -p media-types --test shared_bytes_alloc` -> `test result: ok. 6 passed; 0 failed; 0 ignored` (clone_is_an_arc_bump_only, view_minting_allocates_nothing, build_allocates_the_payload_once_and_fills_in_place, from_vec_moves_the_payload_in_v2, sole_owner_refills_in_place_without_allocating, audit_table_of_measured_costs).
- The counting allocator backing those proofs (crates/testkit/src/alloc_count.rs:237-274) is a real `GlobalAlloc` forwarding to `System` with atomic counters, not a mock.
- `get_mut` recycle path is used in production slot rotation (e.g. crates/codecs/src/video/mjpeg/decoder.rs:882, audio-engine/src/pipeline.rs:424); `SharedBytes::build`'s only production caller is codecs/src/video/dnx/decoder.rs:583.

## 3. Findings
F-001 [S3]
Where: crates/media-types/tests/shared_bytes_alloc.rs:68-82 (evidence file for doc claims at packet.rs:32-33, 59, 107-108)
Code:
```
    settle_foreign_spawns();
    let second = run_once(&mut section);
    assert!(
        accept(&second),
        "measured dirty on BOTH deterministic replays — a real allocation \
         defect (first {first:?}, second {second:?})"
    );
    second
```
What is wrong: the retry-once discipline accepts the *second* window whenever the first is dirty, so a defect that allocates only on first invocation (lazy interior state warmed by the first measured call) is systematically masked: window 1 fails, window 2 passes, test returns green. Why it matters: packet.rs's doc-comments cite these tests as "proved by the counting allocator"; the proof is sound only for stateless paths. What correct looks like: warm-up call *outside* the measured window (as `view_minting_allocates_nothing` does with its `warm` slice) and fail on first dirty window, reserving the retry for harness-spawn noise only with a distinguishable signature. Mitigation in practice: `SharedBytes` methods are stateless, so a first-call-only allocation is not constructible for this type today — the hole is in the instrument pattern 30+ workspace tests copy, not in this type. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/media-types/src/packet.rs:384
Code:
```
        assert_eq!(std::mem::size_of::<SharedBytes>(), 24); // 64-bit pin
```
What is wrong: hard 24-byte pin; on any non-64-bit target (the repo ships a wasm32 web app, apps/web) this test aborts the whole test binary for that target. cfg(test) keeps it out of shipped builds, so blast radius is cross-target test runs only. What correct looks like: assert only the structural identity on line 381 (`size_of::<Arc<Vec<u8>>>() + 2 * size_of::<usize>()`), which is target-independent, and gate the 24 pin on `cfg(target_pointer_width = "64")`. Confidence: HIGH — Verification: read (test passed on this 64-bit host).
F-003 [S4]
Where: crates/media-types/src/packet.rs:252
Code:
```
        let hex: Vec<String> = cap.iter().map(|b| format!("{b:02x}")).collect();
```
What is wrong: the Debug preview allocates a `Vec` of up to 8 heap `String`s plus the joined result per formatting call — 9+ heap events to print ≤8 bytes. Cold path (Debug only), bounded, and the cap itself is correct behavior; this is hygiene, not a perf defect. What correct looks like: `f.write_str` + a two-nibble hex pair per byte, zero allocation. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedBytes::from_vec | 63-69 | REAL | — | moves Vec behind Arc; no copy (verified by from_vec_moves_the_payload_in_v2). |
| SharedBytes::from_static | 83-89 | REAL | — | copies via to_vec; doc admits it; prod callers pass b"" (riff.rs:595). |
| SharedBytes::build | 109-119 | REAL | — | zero-init + fill; 2 alloc events verified; prod caller dnx decoder:583. |
| SharedBytes::from_pool_range | 144-154 | REAL | — | checked_add assert kills overflow+OOR; panics documented, never clamps. |
| SharedBytes::slice | 170-182 | REAL | — | containment assert; addition overflow impossible under invariant. |
| SharedBytes::as_slice | 185-187 | REAL | — | range-exact. |
| SharedBytes::len / is_empty | 190-197 | REAL | — | trivial. |
| SharedBytes::get_mut | 213-216 | REAL | — | Arc::get_mut sound; used by prod slot rotation. |
| Deref::deref | 219-225 | REAL | — | range semantics, agrees with as_slice. |
| PartialEq::eq / Hash::hash | 230-244 | REAL | — | content-based, mutually consistent (test equality_and_hash_are_content_not_identity). |
| Debug::fmt | 246-255 | REAL | — | F-003 (alloc per call); cap at 8 view bytes correct. |
| AsRef::as_ref | 257-261 | REAL | — | delegates. |
| Packet::is_stale | 292-294 | REAL | — | strict past comparison; future-gen note is data, not bug. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| shared_bytes_clone_shares_without_copying | YES | asserts pointer equality of slices — a copying clone fails it. |
| out_of_range_construction_panics_never_clamps | YES | catch_unwind on 4 distinct panic paths + untouched-survivors check; a clamping impl fails. |
| views_keep_the_allocation_alive_past_the_original_handle | YES | strong_count arithmetic + use-after-owner-drop read; an eager-copy impl fails the pointer/count assertions. |
| get_mut_refuses_to_write_behind_live_views | YES | pins the safety contract (None while a view lives). |
| size_of_recorded_before_and_after | YES (on 64-bit) | layout pin; see F-002 for the portability cost. |
| equality_and_hash_are_content_not_identity | YES | cross-allocation equality + hash-consistency + inequality over same allocation. |
| build_fills_in_place / from_static_copies_content_faithfully / debug_shows_the_view_not_the_pool / range_views_expose_only_their_range / subslicing_composes_relative_to_the_view / pool_owner_can_reclaim_through_try_unwrap / packet_staleness_follows_generation_order / packet_fields_roundtrip_through_clone / shared_bytes_debug_is_capped | YES | each asserts observable content/pointer/panic behavior, not format. |
| (cross-file) shared_bytes_alloc::* | YES with the F-001 caveat | real counting allocator; retry-once can mask first-call-only defects (not constructible for this stateless type). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "from_vec ... the bytes are not copied" | packet.rs:54-55 | REAL | alloc test: whole == prep + 1 event, header bytes only (ran, passed). |
| "build allocates the payload once ... two events" | packet.rs:106-108 | REAL | build test asserts alloc_calls==2, total==N+header (ran, passed). |
| "from_pool_range and slice mint O(1) sub-views: ... no allocation, no copy" | packet.rs:29-31 | REAL | view_minting_allocates_nothing: 1000 pairs, 0 events (ran, passed). |
| "clone is the refcount bump only (proved by the counting allocator...)" | packet.rs:32-33 | REAL | clone_is_an_arc_bump_only (ran, passed); instrument verified real. |
| "from_static ... copies ... the only production shape (demux riff.rs zero-size payloads)" | packet.rs:80-82 | REAL | riff.rs:595 `Ok(SharedBytes::from_static(b""))`; all other from_static callers are cfg(test) (rg verified). |
| "SharedBytes = 24 ... on 64-bit" | packet.rs:365-375 | REAL | test passes on this 64-bit host. |

## 7. Dead / duplicated / unreachable code
None. Every public method has production callers (rg: from_vec/from_pool_range across demux+audio-output+audio-engine; build -> dnx; get_mut -> decoder slot rotations; slice/len/Deref ubiquitous). No #[allow] abuse, no todo!/unimplemented! in this file.

## 8. Dependencies & flags
std-only (std::hash, std::ops, std::sync::Arc). No feature gates, no unsafe, no external crates. Cargo.toml of media-types not feature-gated (crate is an unconditional dependency of lumen-cli).

## 9. Open questions for the next agent
- crates/codecs/src/video/dnx/decoder.rs:587 `*next = (count + 1) % FRAME_SLOTS;` after `slots.push` looks like an off-by-one in slot rotation (should arguably be `(idx+1) % count` or `count % FRAME_SLOTS`) — belongs to the dnx auditor, flagged from here.
- The `clean_delta_with` retry pattern (F-001) is copied across the workspace's zero-alloc tests; a workspace-wide assessment of which copied tests measure stateful paths would close the evidence hole.
