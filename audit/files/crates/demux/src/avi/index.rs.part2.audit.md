# AUDIT: crates/demux/src/avi/index.rs (part 2 of 2 — lines 465-929)
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-21 |
| Lines / functions | 465 (of 929) / 11 (parse_idx1 tail, probe_offset_base, search_index, scan_movi, ScanCap/SEEK_SCAN_CAP, ScanLanding, scan_movi_with, scan_movi_inner, 3 test items) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → containers/mod.rs "avi" arm → `AviDemuxer` → avi/mod.rs:978,979,1070,1078,1193 call replay_ladders/Ladders::with_ticks/search_index/scan_movi_with/parse_idx1; baseline proof: avi-firstvertical-open-session.txt full lifecycle + mp4-smoke-bench-avi-mjpeg-pcm.txt 60/60 frames decoded) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |

## 1. One-paragraph verdict
Part 2 is the seek engine of the one demuxer lane the shipped binary actually exercises
end-to-end (AVI MJPEG+PCM first vertical), and it is built to a standard the rest of the
repo should copy: bounded block-batched idx1 parsing with clamped declared sizes, stable
file-order re-sort with damage-noted reordering, FFmpeg-style offset-base disambiguation,
a budgeted linear scan whose cap arithmetic is exactly as documented ("finding the
landing exactly at the cap is success" — verified by reading the check placement), and
integration tests in avi/tests.rs that pin the absolute-offset variant, the too-slow
refusal, and the position-restore contract (all `ok` in baseline). Defects: the
position-restore swallows the seek-back error while its own doc promises restoration
"on every path"; the unbudgeted `scan_movi` has zero callers (dead public API next to
the budgeted twin the demuxer actually uses); plus a comment that contradicts the code
below it and a negligible byte-budget undercount.

## 2. What actually works (verified)
- Both local tests pass in baseline: `avi::index::tests::out_of_order_idx1_parses_to_the_file_ordered_table`
  and `avi::index::tests::search_index_finds_last_keyframe_at_or_below` —
  audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:1594-1595.
- Integration coverage of this part's functions (all `ok`, same log, lines 1617-1665):
  `absolute_idx1_offsets_are_autodetected_and_seek` (probe_offset_base),
  `seek_lands_on_last_keyframe_at_or_below_target` (search_index),
  `seek_without_idx1_walks_the_movi` + `slow_linear_scan_is_bounded_and_typed`
  (scan_movi_with — asserts `demux.avi.seek.too-slow` + ResourceLimit kind + D4
  position/playable after refusal, read at tests.rs:3039-3075),
  `seek_does_not_move_the_read_position` (the restore contract).
- Budget arithmetic verified by reading scan_movi_inner:694-707 — the cap is checked
  after adding the current chunk's bytes, so a landing found exactly at
  `bytes == max_bytes` / `packets == max_packets` succeeds; only walking PAST the cap
  refuses. Matches the doc comment.
- Loop-termination audit of scan_movi_inner: every arm advances `pos` by ≥ 8
  (packet/other: `pos = next ≥ pos+8`; RecList: `pos += 12`), and `rec ` nesting is
  bounded by `super::MAX_LIST_DEPTH` (index.rs:732-734) — no infinite loop, no
  unbounded stack.
- Out-of-order repair is real: `raw.sort_by_key(|entry| entry.chunk_pos)` (index.rs:431,
  part-1 boundary) before the ladder pass, stable for duplicates, damage-counted at
  index.rs:471-473 — and the local regression test would fail without it (stored order
  C,A,B would yield pts (24→0, 4→1, 14→2), failing the `(4,0),(14,1),(24,2)` assert).

## 3. Findings

### F-001 [S3] Position-restore contract swallows the seek-back error
Where: crates/demux/src/avi/index.rs:642-654
Code:
```rust
    let saved = reader.position();
    let outcome = scan_movi_inner(
        reader,
        streams,
        stream,
        movi_data_start,
        movi_data_end,
        target_ticks,
        cap,
    );
    let _ = reader.seek_to(saved);
    outcome
```
What is wrong: `scan_movi_with`'s doc (index.rs:631-632) promises "The reader's position
is restored on every path", and the module doc (index.rs:562-564) repeats it ("the
landed `Demuxer::seek` contract forbids moving it"). The restore is `let _ =
reader.seek_to(saved);` — if the seek-back fails, the reader stays where the scan left
it AND the failure is discarded. The scan then returns `Ok(ScanLanding)` while having
silently violated the contract the whole function exists to uphold.
Why it matters: A source whose seek errors (e.g., a flaky network-backed SourceStream)
leaves the demuxer's read position mid-`movi` after a "successful" seek — the next
`next_event` reads garbage and the failure surfaces as container corruption instead of
the real I/O error. The `seek_does_not_move_the_read_position` test (avi/tests.rs:2429)
passes because it uses a mem stream whose seek cannot fail — the error path is
unpinnable there and unpinned here.
What correct looks like: Propagate: `reader.seek_to(saved)?` after capturing `outcome`,
or at minimum damage-log the failed restore so it is observable.
Confidence: HIGH — Verification: sed -n '642,654p' (quote verified); read the doc
comments at 562-564 and 631-632; rg confirmed the only caller path is
avi/mod.rs:1078.

### F-002 [S3] `scan_movi` — the unbudgeted public scan — has zero callers
Where: crates/demux/src/avi/index.rs:575-593
Code:
```rust
pub fn scan_movi(
    reader: &mut SourceReader,
    streams: &[StreamDesc],
    stream: u32,
    movi_data_start: u64,
    movi_data_end: u64,
    target_ticks: i64,
) -> Result<Option<Landing>, EngineError> {
    let state = scan_movi_with(
```
What is wrong: rg across crates/, apps/, tests/ finds no caller of `scan_movi` — the
demuxer's seek path uses `scan_movi_with(..., Some(SEEK_SCAN_CAP))` (avi/mod.rs:974,
1078). `scan_movi` is `pub` in the `pub mod index` (avi/mod.rs:107) and exists only as
an unbudgeted pass-through: `scan_movi_with(..., None)` — an unbounded whole-`movi`
walk, the exact shape `SEEK_SCAN_CAP` exists to prevent.
Why it matters: Dead public API that bypasses the §6 budget if anyone ever wires it;
survives because it is `pub`, so the dead-code lint never fires. The slow_linear test's
comment even defends it ("The pure query stays unbounded (the landed 2.B2 behavior)")
— defending code nothing calls.
What correct looks like: Delete it, or make it crate-private and used by the pure-query
path if one exists (none found).
Confidence: HIGH — Verification: rg "scan_movi\b" crates/ apps/ tests/ → definition +
doc references only; mod.rs:974 shows the production call is the capped variant.

### F-003 [S4] Comment contradicts the code: physical-EOF break does NOT discard the landing
Where: crates/demux/src/avi/index.rs:683-687
Code:
```rust
        if pos + 8 > length {
            // The scan hit the physical end mid-list: no landing at or
            // below the target was found before the damage.
            break;
        }
```
What is wrong: `break` exits to `Ok(ScanLanding { landing: best, .. })` (index.rs:748-751)
— `best` holds the last in-range landing found BEFORE the damage and is returned. The
comment claims "no landing … was found before the damage", which is false whenever
`best.is_some()`.
Why it matters: The behavior (salvage the last in-range landing at physical EOF) is
reasonable; the comment tells the next maintainer the opposite of what happens.
What correct looks like: Comment: "physical end mid-list: return the last in-range
landing found so far (best may be Some)".
Confidence: HIGH — Verification: sed -n '683,687p' + read of the return at 748-751.

### F-004 [S4] Byte budget undercounts the pad byte
Where: crates/demux/src/avi/index.rs:693
Code:
```rust
        bytes = bytes.saturating_add(8 + u64::from(header.size));
```
What is wrong: `next` (index.rs:689-692) advances by 8 + size + pad, but the `bytes`
budget counts only 8 + size. Every odd-sized chunk undercounts by one byte — up to 1000
bytes on a 2 MiB cap across a full 1000-packet scan.
Why it matters: Negligible for the cap's purpose (a heuristic budget), but the budget
and the walk disagree about what a byte of parse work is.
What correct looks like: Count `8 + size + pad` to match the actual walk.
Confidence: HIGH — Verification: sed -n '688,693p' (both lines read).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_idx1` (tail; fn spans 298-479, boundary at 464) | 460-479 | REAL | — | Coalesced per-reason damage (one event, 16 B/entry — the anti-event-storm design, module docs 22-27); `entries.is_empty() → Ok(None)` (linear fallback). Overflow-safe (`dropped_*  * 16` on ≤524 288 entries). |
| `probe_offset_base` | 489-528 | REAL | — | Relative-first tie rule (spec default), absolute → base 0 + damage note; region + fourcc double-check per candidate; stack buffer, no alloc. Heuristic weakness (coincidental absolute match on a corrupt first entry of a relative file) is region-guarded (absolute needs rel ≥ movi_data_start) — same disambiguation FFmpeg uses. Pinned by avi/tests.rs:1450. |
| `search_index` | 536-556 | REAL | — | Last-keyframe-at-or-below; early break valid (per-stream pts monotone by sort+ladder construction). Unit-tested locally. |
| `scan_movi` | 575-593 | DEAD | S3 | F-002: zero callers; unbudgeted pass-through. |
| `ScanCap` / `SEEK_SCAN_CAP` | 595-609 | REAL | — | 1000 packets / 2 MiB; consumed at mod.rs:974. |
| `ScanLanding` | 611-620 | REAL | — | Data struct. |
| `scan_movi_with` | 633-654 | REAL | S3 | F-001: swallowed restore error; otherwise correct save/restore shape. |
| `scan_movi_inner` | 658-752 | REAL | S4 | Budget placement verified correct-at-cap; rec-list clamped to parent bound (parity with walk, D-3); MAX_LIST_DEPTH guard; progress guaranteed every iteration; F-003 comment, F-004 pad undercount. Snapshot-before-advance re-anchor semantics correct (landing chunk re-emits its pts). |
| tests: `video_stream_desc` | 763-780 | REAL | — | 30 fps MJPEG one-stream table, honest shape. |
| tests: `out_of_order_idx1_parses_to_the_file_ordered_table` | 789-869 | REAL | — | Regression for M16 family 1: asserts file-order table, search landing, and the 32-byte damage event. Mutation: removing the sort fails the first assert. |
| tests: `search_index_finds_last_keyframe_at_or_below` | 871-928 | REAL | — | 6 lanes incl. non-keyframe skip, other-stream isolation, before-first None. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| out_of_order_idx1_parses_to_the_file_ordered_table | YES | Concrete (pos, pts) tuple assert fails under stored-order timing; damage-event assert fails if the reorder note is dropped. |
| search_index_finds_last_keyframe_at_or_below | YES | 6 distinct behavioral lanes asserted on a hand-built table. |
| (integration, avi/tests.rs) absolute_idx1_offsets_are_autodetected_and_seek | YES | Asserts the Recovered event, Fast seekability, landing tick 1, and 3 drained packets (tests.rs:1480-1500). |
| (integration) slow_linear_scan_is_bounded_and_typed | YES | 1002 packets vs 1000 cap → asserts too-slow code + ResourceLimit kind + still-playable D4 (tests.rs:3039-3075, read). |
| (integration) seek_does_not_move_the_read_position | PARTIAL | Pins the happy-path restore only — the seek-back FAILURE path (F-001) cannot fail on a mem stream and is unpinned. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The reader's read position is restored before returning" | index.rs:562-564 | REAL (happy path) / UNPINNED (error path) | F-001: restore is best-effort with the error swallowed. |
| "finding the landing exactly at the cap is success" | index.rs:701-702 | REAL | Check placement read (694-707): cap fires only strictly past. |
| "one damage event per reason … the count rides the event's byte total" | index.rs:22-27 | REAL | Code at 460-473; pinned by the out-of-order test's 32-byte assert. |
| "Relative wins ties (spec default)" | index.rs:66-67 | REAL | probe_offset_base:519-527 tries relative first. |
| "The pure query stays unbounded (the landed 2.B2 behavior)" | avi/tests.rs:3042 (slow_linear comment) | REAL-but-dead | scan_movi exists unbounded — with zero callers (F-002). |

## 7. Dead / duplicated / unreachable code
- `scan_movi` (F-002): dead public function.
- Nothing else: every other item has the mod.rs call sites listed in the header.

## 8. Dependencies & flags
- No feature gates; `pub mod index` unconditional (avi/mod.rs:107); shipped binary
  reaches it through the avi arm. Std + workspace crates; no unsafe. `#[allow(clippy::too_many_arguments)]`
  on parse_idx1 and scan_movi_inner — arg-count pragmas on genuinely wide parse
  signatures, documented; not hiding a real lint.

## 9. Open questions for the next agent
- avi/mod.rs (whose file is it in wave 4?): `locate()` at :974 passes SEEK_SCAN_CAP —
  confirm the pure-query path (no cap) truly no longer exists anywhere (F-002 depends
  on it).
- Can a SourceStream implementation fail `seek_to` in practice? (input crate's file
  source: presumably infallible — if so F-001's blast radius is future source types.)
