# AUDIT: crates/demux/src/avi/tests.rs#part4 (lines 1869-2522)
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-21 |
| Lines / functions | 654 audited (1869-2522) / 13 (12 tests + 1 helper; 4 shared helpers at 1815-1867 belong to part3/A07) |
| File verdict | REAL (for this chunk) |
| Reachable from shipped entry point? | Test code: cfg(test) (avi/mod.rs:1270-1271). Subject (`AviDemuxer`) YES — `apps/cli/src/containers/mod.rs:54` `"avi" => avi::open`; proven end-to-end by baseline `avi-firstvertical-open-session.txt` (full Idle→Playing→Stopped→Idle) and `mp4-smoke-bench-avi-mjpeg-pcm.txt` (60/60 frames both tracks) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `rg -n "^#\[test\]|^fn " crates/demux/src/avi/tests.rs` (chunk boundaries); baseline log grep for all 12 test names (all `ok`, lines 1629-1672); `sed -n` verification of every quote; python3 hex/arithmetic check (see F-001, budget formula); `rg -n "MAX_WALK_RESYNCS" crates/demux/src/avi` (=32, mod.rs:137). |

Chunk boundaries (noted per instructions): nominal part4 = 1876-2500; audited
the test-natural range 1869-2522 — `garbage_chunk_jump_past_bound_resyncs_via_idx1`
(1869) opens the range; `seek_does_not_move_the_read_position` ends at 2522;
helpers `smash_chunk_header`/`drain_with_damage`/`resync_body` (1815-1867) are
part3's (A07) and only referenced here; `lying_hdrl_size_does_not_swallow_the_movi`
(2528+) is part5's (A09).

## 1. One-paragraph verdict
This chunk is the W2-07 damage/resync suite plus the seek suite of the AVI
demuxer — the crate's flagship and the repo's one proven end-to-end lane. All
12 tests executed green in the baseline
(`orchrun-test-workspace-nofailfast.log.txt:1629-1672`). The damage tests are
arithmetically pinned to the byte: exact `(skipped, reason)` damage ledgers in
walk order, exact payload sequences, a 33-vs-32 budget proof whose expected
total (`66·G − 7272`) I re-derived independently and confirmed, and typed
terminals with error-kind matching. The seek tests pin keyframe at-or-below
semantics, linear-fallback rounding, position restoration (via a
position-recording stream wrapper), and out-of-range typing. Three S4 defects:
one factually wrong hex comment, one dead computation, and one ambiguous
seek-span-end fixture. No fakeness of any grade found.

## 2. What actually works (verified)
— 12/12 tests pass: baseline log lines 1629-1672 (suite totals 4534/7/49; none
   of the 7 failures in crates/demux).
— Budget arithmetic independently re-derived: 33·G + (33·G − 7656) + 32·12 =
   66·G − 7272 = 8,278,863,768 for G=125,437,440 — matches the assert at
   line 2143-2147 exactly; `MAX_WALK_RESYNCS = 32` confirmed at
   `crates/demux/src/avi/mod.rs:137`, and mod.rs:765 enforces it.
— Subject ships and plays: `apps/cli/src/containers/mod.rs:54` wires `"avi"`;
   baseline `avi-firstvertical-open-session.txt` (Playing lifecycle) and
   `mp4-smoke-bench-avi-mjpeg-pcm.txt` (v_frames=60 v_decoded=60 a_frames=60
   a_decoded=60).

## 3. Findings
### F-001 [S4]
Where: crates/demux/src/avi/tests.rs:1916
Code:
```
    // the walk itself would accept — the 0268 filing's garbage value.
    let garbage_size = 4_286_905_213u32; // 0xFFBEFFFF-ish, odd
```
What is wrong: 4_286_905_213 = 0xFF84FB7D (verified by computation), not
"0xFFBEFFFF-ish" — 0xFFBEFFFF is 4,290,707,455, a difference of 3,802,242
(~0x3A0000). The comment also claims this is "the 0268 filing's garbage
value", so either the filing's value is 0xFFBEFFFF and the test does not use
it, or the comment mislabels the constant.
Why it matters: the whole point of these tests is byte-shape traceability to
filed corpus damage ("the exact byte shapes W1-03 traced", line 1812-1814). A
comment that misidentifies the traced value breaks the traceability chain the
suite advertises; a future maintainer re-deriving the fixture from the filing
will believe the code is wrong (or "fix" the constant to match the comment and
silently change the odd/even pad arithmetic this test depends on at line 1926).
What correct looks like: `let garbage_size = 0xFF84_FB7Du32; // the 0268 filing's garbage value, odd` — the literal in hex, exactly.
Confidence: HIGH — Verification: computed with python3 (`hex(4_286_905_213)` → `0xff84fb7d`); quote verified with sed.

### F-002 [S4]
Where: crates/demux/src/avi/tests.rs:2155-2161
Code:
```
    let movi_list_start = built
        .chunk_positions
        .first()
        .copied()
        .unwrap_or(0)
        .saturating_sub(12);
    let _ = movi_list_start;
```
What is wrong: A computed value immediately discarded with `let _` — dead
scaffolding left in the test. The actual splice below uses `movi_offset`
(line 2165), not this variable.
Why it matters: noise in a suite whose value is precision; a reader must prove
the discarded computation has no effect (it doesn't — `built.bytes` is only
read after).
What correct looks like: delete lines 2155-2161.
Confidence: HIGH — Verification: read; whole test traced 2150-2187.

### F-003 [S4]
Where: crates/demux/src/avi/tests.rs:2272-2353 (assertion at 2340-2344)
Code:
```
    // The span end is the declared track end (start + length).
    match landing.end.timestamp() {
        Ok(ts) => assert_eq!(ts.ticks, 3),
        Err(_) => unreachable!("landing end present"),
    }
```
What is wrong: The fixture declares `video_stream()` with `length: 3`
(tests.rs:177) but builds FOUR movi chunks (ticks 0..3, lines 2277-2282). The
asserted end tick 3 is simultaneously "declared end (0+3)" and "the tick of
the last frame" — two different semantics that this under-declared fixture
conflates. With the declared length, the 100 ms landing span is [3,3]: a
zero-width span for a frame that exists, is a keyframe, and is returned as the
landing.
Why it matters: no test in this chunk pins the seek-span end when the
declaration is honest (length=4 → end=4?) or over-declared; a demuxer that
returned `end = landing.start` (always zero-width) or `end = last frame tick`
would pass this test as written. The ambiguity is invisible until a real file
with a lying strh length seeks.
What correct looks like: build the fixture with `video_stream_of(4)` (the
helper exists at line 2660) and assert `end.ticks == 4` — separating declared
end from last-frame tick; optionally add an over-declared variant.
Confidence: MEDIUM — Verification: read; `video_stream()` traced at 173-181, `video_stream_of` at 2660; seek landing semantics reasoned from the assertions only (implementation in avi/mod.rs is another agent's chunk — flagged in §9).

## 4. Per-function table (chunk-local; shared helpers `smash_chunk_header`, `DamageLedger`, `drain_with_damage`, `resync_body` at 1815-1867 are part3's)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `garbage_chunk_jump_past_bound_resyncs_via_idx1` | 1869-1909 | REAL | — | Exact 3-entry damage ledger in walk order; exact 5-payload sequence; EOS terminal. G=0x077A0600 comment verified correct. |
| `garbage_chunk_jump_past_bound_resyncs_by_scan_without_idx1` | 1911-1953 | REAL | S4 | Same shape minus idx1 (truncate + RIFF shrink via wrapping_neg patch); scan-flavored reason pinned. F-001 wrong hex comment. |
| `garbage_chunk_jump_without_landing_is_typed_never_silent_eos` | 1955-2012 | REAL | — | No-landing shape → `demux.avi.walk-resync-failed` + Corrupt kind; prefix payloads still delivered; damage events die with the terminal (documented), total observable via `damage_total()`. |
| `garbage_chunk_jump_inside_rec_hands_back_to_parent` | 2014-2077 | REAL | — | Hand-built `rec ` list file (not via `build`); parent-handback reason `movi-resync-parent` pinned; `r0`/`t0` tail delivered. |
| `resync_budget_exhaustion_is_the_typed_terminal_not_an_endless_walk` | 2079-2148 | REAL | — | 33 damaged jumps vs budget 32; tail unreachable; `damage_total() == 66G − 7272` (independently re-derived, exact); budget off-by-one would shift total by 12 — arithmetic pins it. |
| `missing_header_and_missing_movi_are_distinct_typed_errors` | 2150-2187 | REAL | S4 | Two spliced files; `demux.avi.no-header` vs `demux.avi.no-movi` distinct. F-002 dead computation. |
| `methods_before_start_are_typed_not_panics` | 2189-2204 | REAL | — | `next_event`/`seek` before start → `demux.not-started` (crate-generic code, not avi-prefixed — consistent with error.rs design). |
| `unusable_sources_are_rejected_at_construction` | 2206-2258 | REAL | — | Source-stream mock (mocks the SOURCE, not the subject); three typed construction refusals at stage Open: unknown-length / unseekable / empty. |
| `fn span_at_nanos` | 2264-2270 | REAL | — | ns-base TimeSpan builder; correct. |
| `seek_lands_on_last_keyframe_at_or_below_target` | 2272-2353 | REAL | S4 | Hand-built idx1 with frame 1 non-key; 0/45/65/100 ms landings pinned (0,0,2,3); landing base (1,30) pinned. F-003 span-end ambiguity. |
| `seek_without_idx1_walks_the_movi` | 2355-2385 | REAL | — | Linear fallback: 50 ms → tick 2 (tie-away-from-zero rounding, documented); seek is a pure query — post-seek drain still yields all 3 packets. |
| `seek_out_of_range_is_typed` | 2387-2426 | REAL | — | Past-end, negative, and UNKNOWN-span targets all `demux.avi.seek.out-of-range`. |
| `seek_does_not_move_the_read_position` | 2428-2522 | REAL | — | Position-recording `Tracked` wrapper (logs position at every op EXIT — catches read-driven drift, not just seek targets); before==after asserted. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| garbage_chunk_jump_past_bound_resyncs_via_idx1 | YES | Damage ledger equality (1888-1896) fails on any skipped-byte or reason drift; payload vec equality (1897-1907) fails on lost/reordered recovery. |
| …by_scan_without_idx1 | YES | Same, plus the scan-vs-idx1 reason string distinction (1938). |
| …without_landing… | YES | Mutation: silent EOS instead of typed terminal → `unreachable!` at 1989 fires; mutation: smooth the prefix → payload assert 1984 fails. |
| …inside_rec… | YES | Parent-handback reason + exact byte distances (2066-2074). |
| resync_budget_exhaustion… | YES | Budget ±1 landing shifts `damage_total()` by 12 → assert 2143 fails (the arithmetic IS the budget pin). |
| missing_header_and_missing_movi… | YES | Distinct codes asserted; a generic "bad-avi" collapse fails either assert. |
| methods_before_start… | YES | Panics fail via test panic; wrong code fails string equality. |
| unusable_sources… | YES | Three-construction loop with exact codes and stage. |
| seek_lands_on_last_keyframe… | YES (except span-end) | Landing ticks pinned incl. the non-keyframe skip at 45 ms; span END is the F-003 ambiguity. |
| seek_without_idx1_walks_the_movi | YES | Landing tick + post-seek full-drain count. |
| seek_out_of_range_is_typed | YES | Three exact refusals. |
| seek_does_not_move_the_read_position | YES | If seek leaves the stream elsewhere, `after != before` (2518-2521). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| AVI first vertical plays end-to-end | INDEX.md §"contrast: AVI first-vertical" | REAL | `avi-firstvertical-open-session.txt` + `mp4-smoke-bench-avi-mjpeg-pcm.txt` (60/60); this chunk's seek/damage units all pass in nofailfast log 1629-1672. |
| "bounded damage, never an endless skip (D-3)" (test comment 2086-2087 citing design doc) | crates/demux/src/avi/tests.rs:2086 + mod.rs:137 | REAL | `MAX_WALK_RESYNCS=32` enforced at mod.rs:765; budget test passes with the exact-count arithmetic. |
| ffmpeg avi_sync-equivalent scan | tests.rs:1914 comment | REAL-UNPROVEN | The scan behavior is pinned by the test (reason `movi-resync-scan`, landing at chunk 3), but ffmpeg-side equivalence is asserted by comment only — no corpus differential for the no-idx1 damaged lane in this chunk (the corpus lanes live in tests/avi_corpus.rs, another agent's scope). |

## 7. Dead / duplicated / unreachable code
- F-002's `movi_list_start` (2155-2161) — dead computation.
- `garbage_chunk_jump_inside_rec_hands_back_to_parent` hand-builds the AVI
  byte-by-byte instead of reusing `build()` — justified (needs a `rec ` list
  shape `build` cannot express); noted as intentional, not duplication.

## 8. Dependencies & flags
No feature gates; whole file is `#[cfg(test)]` (avi/mod.rs:1270-1271). Uses
`test_support::BytesStream`, `std::sync::{Arc, Mutex}` for the position log —
no unsafe, no external deps. Ran in baseline default selection.

## 9. Open questions for the next agent
- F-003: does `AviDemuxer::seek`'s landing span end come from declared strh
  length or actual content? Implementation (avi/mod.rs / index.rs, A07's
  chunks) should answer; if declared, a lying strh length directly skews player
  position/remaining-time UI.
- The no-landing terminal swallows queued `Recovered` events (documented at
  1998-2001): does the playback engine surface `damage_total()` anywhere
  user-visible, or is mid-walk damage invisible on the fatal path?
- `demux.not-started` (line 2193) is crate-generic, unlike every other
  `demux.avi.*` code in this chunk — intentional taxonomy or an oversight?
  (error.rs audit — file 6 of this agent — follows up.)
