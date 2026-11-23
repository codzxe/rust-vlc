# AUDIT: crates/media-types/tests/shared_bytes_alloc.rs
| Field | Value |
|---|---|
| Auditor | W02-A09 |
| Date | 2025-06-27 |
| Lines / functions | 390 / 8 helpers + 6 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test binary) — but it tests `media_types::packet::SharedBytes`, which IS shipped-path: used by apps/cli/{pipeline_host,demux_host,containers/*}, apps/desktop/{bridge,frame_tap}, apps/web/src/render/present.rs (rg, 12+ production files) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p media-types --test shared_bytes_alloc -- --nocapture` → 6 passed, 0 failed, exit 0; audit table captured (see §2) |

## 1. One-paragraph verdict

These are real measured allocation proofs, not argued ones: a process-global
counting allocator (testkit), a settle-plus-single-replay discipline against
harness noise, calibrated constant header sizes instead of hard-coded std
layout numbers, and a differential old-vs-new audit that runs both cores in
the same process. I executed the binary myself — all 6 tests pass in 0.66 s
and the printed audit table matches the module's claims (old from_vec core
131088 bytes for a 64 KiB payload vs new 65576 = payload + 40-byte header).
The only defects are discipline-level: the replay design provably cannot
catch one-time lazy/global allocations (steady-state proofs only), and the
exact-count assertions are coupled to allocator/std behavior beyond the
calibrated header.

## 2. What actually works (verified)

— `cargo test -p media-types --test shared_bytes_alloc -- --nocapture` →
exit 0, `6 passed; 0 failed`, and the audit table printed:
```
old from_vec core (Vec -> Arc<[u8]>):   2 events, 131088 bytes, + 1 payload memcpy (std-documented)
new from_vec (Arc::new(Vec), moves):    2 events, 65576 bytes, 0 payload copies
old from_static core (Arc::from slice): 1 events, 4112 bytes, + 1 payload memcpy (std-documented)
new from_static (to_vec + Arc::new):    2 events, 4136 bytes, 1 payload copy (...)
Arc header calibration (Arc::new(Vec::new())): 40 bytes
build(N, fill): 2 events = payload 65536 + header 40; zero-init memset; no post-fill copy
```
This is measured evidence for every allocation claim in the module header
(:8-19): clone/view minting = 0 events (tests 1-2), build = 1 payload + 1
header (test 4), from_vec = constant header only (test 5, differential at
sizes 1/4096/64KiB/1MiB).
— Production linkage verified: `SharedBytes` consumed by the shipped hosts'
packet paths (rg: apps/cli/src/pipeline_host.rs, demux_host.rs, containers/
{mp4,mkv,ts,wav,ogg,ps}.rs, apps/desktop/src/{bridge,frame_tap}.rs,
apps/web/src/render/present.rs).
— The claim "a path whose only production callers pass `b\"\"`"
(:385 re from_static) verified: every `from_static` call site outside
media-types is inside `#[cfg(test)]` code (crates/plugin-api/src/codec.rs:163
sits in a `#[test]` fn, confirmed by read).

## 3. Findings

F-001 [S4]
Where: crates/media-types/tests/shared_bytes_alloc.rs:64-82 (and warm-up at :134-135)
Code:
```rust
fn clean_delta_with(mut section: impl FnMut(), accept: impl Fn(&AllocDelta) -> bool) -> AllocDelta {
    settle_foreign_spawns();
    let first = run_once(&mut section);
    if accept(&first) {
        return first;
    }
    settle_foreign_spawns();
    let second = run_once(&mut section);
```
What is wrong: the single-replay backstop hardens against environment noise
but provably cannot catch a ONE-TIME allocation (lazy static init, first-call
memoization inside `SharedBytes`): the first window records it, the replay is
clean because the process-global state is now initialized, and the test
passes. `view_minting_allocates_nothing` additionally warms `full.slice`
before measuring (:134-135), deliberately excluding first-call cost.
Why it matters: the proofs are steady-state proofs; the module header's
"allocate **nothing**" (:8-9) reads broader than what is proven.
What correct looks like: state the steady-state scope in the contract line,
or add one cold-process proof (fresh test binary per cold case).
Confidence: HIGH — Verification: read + ran (0.66 s run, discipline observed).
Mitigating: for a hot-path container, steady-state is the economically
relevant claim, and the D-5 rationale is documented at :64-67.

F-002 [S4]
Where: crates/media-types/tests/shared_bytes_alloc.rs:203,207-215,241-256
Code:
```rust
        |d| d.alloc_calls == 2,
    );
    // Exactly one payload-sized event plus the constant header — never
    // a second payload-sized block, never a post-fill copy.
    assert_eq!(
        delta.alloc_calls, 2,
```
What is wrong: exact allocator-event counts (`== 2`, `== 1`) are pinned to
std's `Arc`/`Vec` behavior; only the 40-byte header is calibrated at runtime.
A std change that merges or splits allocation events (e.g., an allocator
fast path, `Vec`'s alloc on zero len) breaks the suite without any defect in
`SharedBytes`.
Why it matters: CI flakiness-to-come vs genuine signal; the file itself
notes the toolchain pin ("pinned toolchain 1.99.1", :343).
What correct looks like: predicate on `total_bytes` deltas (already
calibrated) plus an upper bound on events, or keep exact counts but document
them as toolchain-pinned invariants (partially done).
Confidence: MEDIUM — Verification: read; current toolchain passes (ran).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ALLOC` (global allocator) | 44-45 | REAL | — | testkit CountingGlobalAlloc, process-global; tests serialized via `serialize_tests()` |
| `settle_foreign_spawns` | 54-56 | REAL | — | 30 ms sleep vs harness-thread noise; documented diagnostic |
| `run_once` | 58-62 | REAL | — | guard/delta pair |
| `clean_delta_with` | 64-82 | REAL | S4 | single-replay discipline masks one-time allocations (F-001) |
| `is_zero` | 84-88 | REAL | — | same predicate as CountingGuard::assert_zero |
| `arc_header_event` | 93-103 | REAL | — | runtime calibration of the 40-byte Arc header — good practice |
| `clone_is_an_arc_bump_only` | 105-127 | REAL | — | 1000+1000 clones, 0 events; ran, passed |
| `view_minting_allocates_nothing` | 129-156 | REAL | — | 1000 from_pool_range+slice pairs, 0 events; warmed first (F-001) |
| `sole_owner_refills_in_place_without_allocating` | 158-179 | REAL | — | get_mut path, 0 events + landed-content check |
| `build_allocates_the_payload_once_and_fills_in_place` | 181-228 | REAL | — | exact 2 events = N + 40; zero-len case = header only; content verified outside window |
| `from_vec_moves_the_payload_in_v2` | 230-268 | REAL | — | differential prep-vs-whole at 4 sizes; header-only delta asserted |
| `audit_table_of_measured_costs` | 270-390 | REAL | — | old/new cores side by side, printed + asserted; ran with --nocapture, table captured |

## 5. Test honesty

| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| clone_is_an_arc_bump_only | YES | a deep-copying clone dirties BOTH replays → assert :76 fires (steady-state scope, F-001 caveat) |
| view_minting_allocates_nothing | YES | allocating view ctor dirties both replays |
| sole_owner_refills_in_place_without_allocating | YES | cloning get_mut dirties both replays; content assert :174 catches a no-op refill |
| build_allocates_the_payload_once_and_fills_in_place | YES | a post-fill copy or double block breaks `== 2` events / `N + header` bytes on both replays; probe content assert :193-194 catches unfilled buffer |
| from_vec_moves_the_payload_in_v2 | YES | a copying from_vec adds payload bytes: `prep + header != whole` at every size |
| audit_table_of_measured_costs | YES | comparative asserts :376-389 (new < old, exact header arithmetic) |

No tautologies; no goldens generated by the code under test (the "old core"
lines replicate std operations, not SharedBytes, for comparison).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "clone and view minting allocate nothing — the Arc-bump-only contract (D-1)" | :8-10 | REAL | ran: 0 events over 2000 clone ops (steady-state; F-001 scope note) |
| "build allocates the payload once and fills in place… no post-fill copy" | :11-15 | REAL | ran: 2 events = 65536 + 40 bytes, exact |
| "from_vec… own cost is one constant-size header event, independent of payload length (V-2)" | :16-19 | REAL | ran: differential at 1/4096/64KiB/1MiB, header-only delta |
| "old from_static 'no allocation, no copy' doc was wrong" | :294-295 | REAL | measured: Arc::from slice = 1 event 4112 bytes + 1 memcpy (std-documented) |
| "memcpy costs are structural facts… labelled as such, not measured here" | :26-28 | REAL | the table labels copies as std-documented, allocator only counts events/bytes |

## 7. Dead / duplicated / unreachable code

None. Every helper is used; `old_from_static_empty`'s loose `<= 2` accept
(:311) is print-only context, never asserted on — acceptable.

## 8. Dependencies & flags

- dev-deps: `testkit` (alloc_count) — correctly dev-only per boundaries.toml
  `dev_only = ["testkit"]`; `media-types` itself has no new deps.
- No feature gates; runs in the standard `cargo test -p media-types` lane
  (baseline cargo-test-workspace.txt includes this binary).

## 9. Open questions for the next agent

1. Is there any cold-path consumer that mints `SharedBytes` views ONCE per
   process where a hidden first-call allocation would matter? (F-001 scope.)
2. `testkit::alloc_count` itself is unaudited by me — the counting
   allocator's accuracy underpins every number here (assignee of
   crates/testkit/src/alloc_count.rs should cross-check the guard/delta
   arithmetic).
