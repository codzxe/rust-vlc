# AUDIT: crates/demux/tests/zero_alloc_read_chain.rs
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Lines / functions | 1128 / 22 (10 #[test] + 12 helpers/impls) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test binary) — wired: runs under `cargo test -p demux`, verified live (exit 0) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `./target/debug/deps/zero_alloc_read_chain-4e2fc9d9594bfc37 --test-threads=1` -> 10 passed, exit 0; single-filter run `header_peek_loop_allocates_nothing_when_warm` -> 1 passed, exit 0; sed quote verifications (all byte-exact) |

## 1. One-paragraph verdict
This is a real, executed zero-allocation proof suite. The counting allocator
(`testkit::alloc_count::CountingGlobalAlloc`, verified to forward to `System`
and count alloc/realloc events) is installed process-global; all 10 tests
passed live on the prebuilt binary. The tests measure genuine hot paths
(`SourceReader::peek_chunk_header_into`, `read_prefix_into`, `PayloadSlots::fill`,
the full `AviDemuxer` walk through `next_event`) with content assertions
(fill-byte checks, track ordinals, payload sizes) — not tautologies. Defects
found are hygiene-level only: one dead clamp, one dead variable, one brittle
magic constant, and a retry-once noise-tolerance argument that only holds for
deterministic sections (which these are, in-memory/single-threaded).

## 2. What actually works (verified)
— verified by `./target/debug/deps/zero_alloc_read_chain-4e2fc9d9594bfc37 --test-threads=1`
-> "test result: ok. 10 passed; 0 failed" exit 0 (whole binary, includes the
end-to-end `real_movi_walk_allocates_nothing_per_packet` and the census test).
Cross-checked against audit/runs/baseline/cargo-test-workspace.txt:2450-2465
(same 10 tests, ok). The counting instrument itself is real:
crates/testkit/src/alloc_count.rs:237-273 (`unsafe impl GlobalAlloc` delegating
to `System`, counting on the way through) — read, verified.

## 3. Findings
F-001 [S4]
Where: crates/demux/tests/zero_alloc_read_chain.rs:410-415
Code:
```
                let want = 64 + (index as usize % 512);
                let at = (index % 32) * 64;
                let handle = pool
                    .fill(want, |out| reader.read_prefix_into(out, at))
                    .must();
                assert_eq!(handle.len(), want.min(2048));
```
What is wrong: PAYLOADS=128, so `index % 512 == index` and `want` spans only
64..=191. `want.min(2048)` is a dead clamp — the 2048 arm can never engage, so
the assertion implies a truncation case is covered when it is not. Why it
matters: the test name promises "varying size payloads"; a reader auditing the
assert would believe sizes up to 2048 were exercised. What correct looks like:
either `want = 64 + (index % 512)` with PAYLOADS > 512, or assert
`handle.len() == want` plainly.
Confidence: HIGH — Verification: read + arithmetic.

F-002 [S4]
Where: crates/demux/tests/zero_alloc_read_chain.rs:573
Code:
```
    let _ = movi_list_pos;
```
What is wrong: dead variable — `movi_list_pos` is computed at line 554 and
never used; the `let _` silences the unused warning instead of deleting it.
Why it matters: copy-paste residue from the sibling builder `avi_bytes_av`
(which has no equivalent); noise in a fixture builder whose byte arithmetic
must be trusted. What correct looks like: delete both lines.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/tests/zero_alloc_read_chain.rs:1076-1081
Code:
```
    /// The open-phase `read_into` calls for this fixture (measured:
    /// RIFF head 12 B, `LIST hdrl` peek + form, `avih` peek + 56 B,
    /// `LIST strl` peek + form, `strh` peek + 56 B, `strf` peek +
    /// 40 B, `LIST movi` peek + form — 14 small asks, none over
    /// 56 B, none per-payload).
    const OPEN_READS: u64 = 14;
```
What is wrong: the census assert at line 1116-1122 pins the exact total
`OPEN_READS + CHUNKS*2`. Any benign change to the open phase's read pattern
(an added peek, a reordered discovery) breaks the test even when the pinned
contract (one payload-class ask per frame) still holds; conversely the open
phase is free to regress as long as it stays under "14 by name". Why it
matters: over-pinned canaries rot into #[ignore] or constant-bumps. The
payload-class assertion (big_asks == CHUNKS, max_ask == PAYLOAD) is the honest
core; the total-count assert is incidental coupling. What correct looks like:
assert the payload-class contract plus an upper bound on small asks (e.g.
open small asks <= 32), not an exact census.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/demux/tests/zero_alloc_read_chain.rs:69-80
Code:
```
    settle_foreign_spawns();
    let first = section();
    if accepts(&first) {
        return first;
    }
    settle_foreign_spawns();
    let second = section();
    assert!(
        accepts(&second),
```
What is wrong: retry-once tolerance. A defect that dirties a window with
probability p per run passes with probability 1−p²; the module doc's "no false
pass exists" argument holds only under the determinism assumption it states
(in-memory bytes, no clocks, one thread). That assumption is true for these
sections today, but it is an argument, not an enforced invariant — nothing
prevents a future section with a clock or thread from inheriting
`clean_replay`'s tolerance. Why it matters: the tolerance is documented
thoroughly (an observed real-suite flake is cited), so this is a hygiene note,
not a hole: flag the invariant, e.g. debug_assert the section is replay-stable
by always running twice in CI-fail mode.
Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| settle_foreign_spawns | 44-46 | REAL | — | 30 ms sleep; noise dampener, live-verified indirectly |
| clean_replay | 62-81 | REAL | S4 (F-004) | retry-once; determinism assumption documented not enforced |
| is_zero | 85-87 | REAL | — | matches CountingGuard::assert_zero predicate (alloc_count.rs:194-204) |
| BorrowMemSource::new | 98-105 | REAL | — | honest in-memory source |
| BorrowMemSource::uri/size/supports_seek | 109-119 | REAL | — | trivially correct |
| BorrowMemSource::read | 121-126 | REAL | — | allocates (to_vec) — fine, only used via Vec path |
| BorrowMemSource::read_into | 128-134 | REAL | — | memcpy, no alloc; positions checked (pos+len can't overflow: test data ≤ ~9.5 MB) |
| BorrowMemSource::seek | 136-166 | REAL | — | checked_add, typed error on out-of-range; start-overflow via `to > len` rejected |
| BorrowMemSource::position | 168-170 | REAL | — | correct |
| movi_layout | 175-187 | REAL | — | 8-byte header + payload + WORD pad; stride arithmetic consistent with both consumers |
| test header_peek_loop_allocates_nothing_when_warm | 192-235 | REAL | — | 16 warm + 1000 measured peeks; asserts fourcc+size each iteration; would fail if peek allocated or mis-parsed |
| test pooled_movi_walk_allocates_nothing_when_warm | 241-297 | REAL | — | fill-byte check per chunk; pool warm-state asserted (resident==max, default 2×16MiB per riff.rs:539-540) |
| test borrow_reads_allocate_nothing | 302-334 | REAL | — | 500×(exact 8B + prefix 2048B); prefix re-reads pos 0 so no EOF issue |
| test vec_twins_cost_exactly_one_allocation_per_call | 341-379 | REAL | — | pins 2×CALLS exactly — a strict regression canary on the Vec path |
| test varying_size_payloads_pack_into_the_arena_zero_alloc | 390-429 | REAL | S4 (F-001) | zero-alloc claim verified live; dead min clamp |
| test over_ceiling_spill_is_exactly_two_allocs_per_payload | 437-476 | REAL | — | tight ceiling 1024; pins 2 events/payload + resident_slots==1 |
| avi_bytes | 493-588 | REAL | S4 (F-002) | builds valid RIFF/AVI (live-demuxed by 4 tests); dead movi_list_pos |
| test real_movi_walk_allocates_nothing_per_packet | 597-666 | REAL | — | public Demuxer contract, 50 warm + 1000 measured, fill-byte per packet, EOS drain after window |
| avi_bytes_av | 672-799 | REAL | — | two-stream MJPG+PCM, strh 56 B each (verified arithmetic), idx1 interleave |
| test two_stream_interleaved_walk_allocates_nothing_per_packet | 808-884 | REAL | — | checks track ordinal 0/1, per-stream fill bytes, 550 pairs ≥ 1050 packets |
| test idx1_parse_allocates_per_block_not_per_entry | 892-977 | REAL | — | pins growth ≤ +4 allocs from 256→1024 entries and large < 64; damage.total==0 |
| SeamStats::record / CountingSource impl | 1008-1050 | REAL | — | atomic counters; read_into interception is total (all calls counted) |
| test big_payload_reads_are_one_source_call_per_frame | 1071-1128 | REAL | S4 (F-003) | 1.5 MiB frames; big_asks==6, max_ask==PAYLOAD verified live; OPEN_READS brittleness |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| header_peek_loop... | YES | each iteration asserts fourcc + size; zero-alloc window measured with the real global allocator |
| pooled_movi_walk... | YES | fill-byte asserted per chunk — a mis-targeted read fails byte check, an allocation fails delta |
| borrow_reads... | YES | asserts got==2048 per read |
| vec_twins... | YES | exact count 512 allocs; ±1 breaks it |
| varying_size... | YES | zero-alloc window; len asserted per payload |
| over_ceiling_spill... | YES | exact 2/payload + resident_slots==1 |
| real_movi_walk... | YES | would catch per-packet allocs AND wrong bytes (fill check per packet) |
| two_stream_interleaved... | YES | adds track-ordinal + per-stream fill checks |
| idx1_parse... | YES | entry count, damage.total==0, growth bound; per-entry regression (1024 allocs) blows the <64 bound |
| big_payload_reads... | YES for chopping (24 asks/frame → big_asks 144≠6); NO for open-phase regressions below 14 (only exact-census protected) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the movi-walk-shaped sequence ... allocates nothing in the steady state, measured with the testkit counting allocator (runtime evidence, D-6 — not syntactic green)" | module doc lines 1-5 | REAL | live run: 10/10 pass incl. end-to-end walk; instrument verified at testkit/src/alloc_count.rs:237-273 |
| "one real-suite flake, `real_movi_walk` at 2 events / 99 B" (settle rationale) | lines 41-43 | UNVERIFIED (historical observation; cannot reproduce noise) | settle+replay ran clean 10/10 today |

## 7. Dead / duplicated / unreachable code
- `movi_list_pos` (F-002) — dead.
- `want.min(2048)` clamp (F-001) — dead arm.
- `avi_bytes` vs `avi_bytes_av` — ~90 lines of duplicated RIFF/avih/strh building
  with divergent style (loop vs line-by-line); acceptable test fixture duplication,
  noted.

## 8. Dependencies & flags
Test-only: testkit (dev), plugin_api, media_types, demux. No feature gates;
`#[global_allocator]` requires the dedicated test binary (tests/ dir = one
binary per file, satisfied). No unsafe in this file.

## 9. Open questions for the next agent
- riff.rs itself (SourceReader/PayloadSlots internals) is assigned elsewhere in
  this wave or the next — F-001's arena-capacity growth behavior (16 MiB
  default ceiling per arena, lazily grown) deserves a bounds check there: a
  1.5 MiB frame fits under 16 MiB, but two concurrent >8 MiB frames would
  spill — untested territory.
- The doc references "the input crate's `large_read_bypasses_the_window` proof"
  (line 996-997) — that test lives outside this file; verify its existence when
  auditing the input crate.
