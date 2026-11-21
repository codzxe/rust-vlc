# AUDIT: crates/demux/src/avi/mod.rs#part2 (lines 636–1271)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-11-21 |
| Lines / functions | 636 (of 1271; part2) / 18 units |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/host.rs:890 open_session` → `apps/cli/src/demux_host.rs:362 open_source` → `apps/cli/src/containers/mod.rs:54 "avi" => avi::open` → `apps/cli/src/containers/avi.rs:40 AviDemuxer::new` → `Demuxer::{start,next_event,seekability,reanchor}` (this range); engine pipeline drives next_event/reanchor — `crates/playback-engine/src/pipeline.rs:1499`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | `rg -n "AviDemuxer|avi::"` (callers: cli containers/demux_host only; playback-engine calls only `seekability`/`reanchor`); `rg -n "\.seek\(" crates/playback-engine/src/*.rs` (0 hits — engine never calls the seek query); `sed -n` on every quoted range; `rg -n "SEEK_SCAN_CAP|scan_movi_with" crates/demux/src/avi/index.rs` (cap semantics); `rg -n "AVI_FLIP_EXTENT"` tools/xtask/src/cmd/fixtures.rs (=64); baseline log greps below |

## 1. One-paragraph verdict
This range is the seek/reanchor engine, the resync ladder, and the top-level
container parse of the AVI demuxer — and it is genuinely implemented, genuinely
executed (doctest at :208 ok and 85 `test avi::` unit tests ok in
`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:6362-6372,1489-1955`;
end-to-end AVI lifecycle in `audit/runs/baseline/avi-firstvertical-open-session.txt`;
60/60 frames in `audit/runs/baseline/mp4-smoke-bench-avi-mjpeg-pcm.txt`). The
movi-walk damage containment (resync ladder, idx1-informed landing, bounded
signature scan) is real and carefully budgeted. The defects found are at the
edges the careful design did not extend to: the top-level chunk walk trusts
declared sizes with no recovery (one corrupted size before the `movi` bricks the
file with `no_movi`), the seek *query* is uncapped where the acting `reanchor`
is budgeted, and the stickiness policy is inconsistent between `next_event`
(permanently bricks the demuxer on a pre-`start` call) and `locate` (non-sticky
for the same violation).

## 2. What actually works (verified)
- Doctest `avi::AviDemuxer` (line 208) builds a 2-track AVI, starts, asserts
  first packet keyframe — **ok** in baseline: `orchrun-test-workspace-nofailfast.log.txt:6368`
  (`test crates/demux/src/avi/mod.rs - avi::AviDemuxer (line 208) ... ok`).
- demux crate unit suite: `running 462 tests … test result: ok. 462 passed; 0
  failed` (same log :1489-1955), 85 of them `test avi::`.
- End-to-end AVI first vertical through the shipped `lumen` binary:
  `avi-firstvertical-open-session.txt` (Buffering → Playing → Stopped → Idle,
  EXIT=0) and `mp4-smoke-bench-avi-mjpeg-pcm.txt` (`v_frames=60 v_decoded=60
  a_frames=60 a_decoded=60`).
- `SourceReader::new` rejects unknown-length/unseekable/empty sources
  (`crates/demux/src/avi/riff.rs:219-237`), which makes the `seekability`
  comment's "the scan fallback is always possible" claim sound — verified by read.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/avi/mod.rs:1215-1218 (and the fatal tail 1230-1238)
Code:
```rust
            } else {
                // JUNK and unknown top-level chunks: skipped.
                pos = next;
            }
```
What is wrong: The top-level walk trusts a non-LIST/non-idx1 chunk's declared
size verbatim — `pos = next` with no clamp, no damage note, no resync. The
in-movi walk got a three-rung recovery ladder (`resync_after_clamp` :756-804)
and `hdrl` got recognition-based resume (`pos = hdr.resume_at` :1163), but a
single corrupted size field on a top-level `JUNK` (or unknown) chunk that sits
*before* the `movi` jumps the cursor past the movi, the loop ends, and
`parse_container` returns `error::no_movi()` (:1237) — a fatal open failure.
The same holds for a *shrunk* RIFF size: `bound` (:1125) silently cuts the walk
with no damage note (only the grows-past-stream direction is noted :1126-1129).
Why it matters: The module's own design principle (mod.rs:29 "**A lying chunk
size is contained, never fatal on its own** (D-3/D-9)") is not extended to the
top level; one bit-flip in a size field between `hdrl` and `movi` makes the
whole file unopenable, where the same corruption inside `movi` recovers with
damage notes. The corpus cannot catch this: `AVI_FLIP_EXTENT = 64`
(tools/xtask/src/cmd/fixtures.rs:2428) only flips within RIFF size + form magic
+ the `LIST hdrl` start, and `garbage0` only appends trailing bytes — no
fixture corrupts a top-level size between hdrl and movi.
What correct looks like: Apply the class-d treatment at the top level too: when
a skipped top-level chunk's `next` exceeds `bound`, note damage and resync
(next recognized top-level chunk by signature scan); or at minimum count
"bytes the walk forfeited past bound" as damage so the failure mode is loud and
testable, and add a fixture class for top-level size corruption.
Confidence: HIGH — Verification: read (control flow traced end-to-end; corpus
extent verified in fixtures.rs; no fixture exercises this).

### F-002 [S3]
Where: crates/demux/src/avi/mod.rs:947-954
Code:
```rust
    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // Pure query: the index search never touches the source; the
        // no-index linear scan restores the read position before
        // returning (the landed trait's "does not move the read
        // position").
        let (landing, facts, _) = self.locate(target, None)?;
        Ok(self.landing_span(landing, facts))
    }
```
What is wrong: The seek *query* passes `scan_cap: None`, so for a no-index
file `locate` (:1074-1088) runs `index::scan_movi_with(..., cap: None)` — an
UNCAPPED full-movi linear scan (index.rs:703-707 only enforces the budget when
`cap` is `Some`). The acting half `reanchor` (:974) is budget-bounded by
`index::SEEK_SCAN_CAP` (1000 packets / 2 MiB, index.rs:606-609). The comment
even admits the query scans ("the no-index linear scan restores the read
position"). Reachable today only at open time with `--start-time`
(`apps/cli/src/containers/avi.rs:58 demuxer.seek(target)`); the engine itself
never calls the demuxer seek query (rg over `crates/playback-engine/src` → 0
hits for `.seek(`).
Why it matters: A multi-gigabyte `noidx` AVI opened with `--start-time` pays
one full-file positional read pass at open — unbounded work the reanchor path
explicitly refuses (`scan_too_slow`). The two halves of the same seek flow
(answer "where would we land" vs "land there") have unbounded vs budgeted cost
for identical inputs.
What correct looks like: Pass a (larger, but finite) query cap in `seek` — or
return the landing lazily/optionally when the scan would exceed a query budget
— so the query's cost class matches the acting path's.
Confidence: HIGH — Verification: read + rg (callers traced; cap semantics read
in index.rs:703-707).

### F-003 [S3]
Where: crates/demux/src/avi/mod.rs:921-923 vs 1034-1036
Code:
```rust
        if !self.started {
            return Err(self.fail(error::not_started()));
        }
```
vs `locate`:
```rust
        if !self.started {
            return Err(error::not_started());
        }
```
What is wrong: Same contract violation (using the demuxer before `start`),
two opposite state consequences. `next_event` routes the error through
`self.fail()` (:403-406), which sets `self.sticky` — and `start()`
(:896-898) refuses forever after (`if let Some(shape) = &self.sticky { return
Err(shape.mint()); }`). One out-of-order `next_event` call permanently bricks
the demuxer; even a subsequent correct `start()` can never succeed. `locate`
returns the identical `not_started` error non-sticky — the demuxer stays
usable.
Why it matters: The stickiness policy is supposed to freeze *walk damage
sites* ("the walk cursor frozen at the damage site", :915-916) — a pre-start
call has no damage site. A future caller ordering bug (e.g. an engine that
polls events before start on an error path) turns a recoverable mistake into
an unrecoverable one, and the asymmetry between the two methods means the
failure mode depends on which method tripped first.
What correct looks like: Not-started is a usage error, not container damage:
return it non-sticky in both paths (drop `self.fail` at :922), or make it
sticky in both — one policy, consistently applied.
Confidence: HIGH — Verification: read (fail/sticky interplay traced through
:403-406, :892-911, :1034-1036).

### F-004 [S4]
Where: crates/demux/src/avi/mod.rs:692
Code:
```rust
                    cursor.ends.push(end);
                    cursor.pos = pos + 8 + 4;
```
What is wrong: The `rec ` arm unconditionally skips 4 bytes for the form
FourCC, even when the declared list size is < 4 (extent `[pos+8, pos+8+size)`
contains no form type). For size 0–3 the walk resumes at `pos+12`, which is
past the declared extent — up to 4 bytes of the *next real chunk* are skipped
with no damage note (the `rec-extends-past-list-bound` note :686-689 only
fires when clamped by the parent bound). The misaligned walk then misreads the
next header; recovery happens only if the resulting garbage triggers the
`Other` arm's damage+resync path.
Why it matters: The damage contract ("never silent", every lost byte counted)
is breached in a narrow corruption case; the accounting a consumer sees
undercounts the loss by up to 4 bytes and the resync path may attribute the
loss to a different reason string.
What correct looks like: `cursor.pos = (pos + 8 + 4).max(pos + 8 +
u64::from(header.size) + riff::pad_for(u64::from(header.size)))` — never
resume before the declared extent ends — plus a damage note when the form
FourCC itself is cut.
Confidence: MEDIUM — Verification: read (arithmetic traced; no fixture emits a
sub-4-byte rec list, so untested territory).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `step_movi_inner` Packet-arm tail (ladder step → packet emit) | 622-674 | REAL | — | pts/dts mirror (no B-frames, documented :651-652); `duration` degrades to None with damage note on unrepresentable nanos (:640-647); `hit_eof` sets eos AFTER returning the prefix packet (:668-672) — terminal containment correct; `generation: GenerationId::FIRST` is a placeholder the engine re-stamps (:661-665, honest comment). |
| `step_movi_inner` `RecList` arm | 675-693 | REAL | S4 (F-004) | Depth-bounded (`MAX_LIST_DEPTH` 8 → typed `bad_structure`); clamp damage only for parent-bound crossing; sub-4-byte size mis-resume (F-004). |
| `step_movi_inner` `Other` arm | 694-710 | REAL | — | damage-counted skip; jump-past-bound → `resync_after_clamp` (class-d recovery) — the walk's own containment works here. |
| `resync_after_clamp` | 756-804 | REAL | — | `next - bound` cannot underflow (caller checked `next > bound` :697, :506, :519); attempt budget `MAX_WALK_RESYNCS`=32 → typed terminal; idx1 landing requires `landing + 8 <= bound`; every rung damage-noted. Sound. |
| `resync_terminal` | 812-817 | REAL | — | physically-short movi keeps truncation flavor; in-extent → `walk_resync_failed`. Matches doc. |
| `scan_for_landing` | 831-852 | REAL | — | `READ_STEP_MAX` (64 KiB) chunked reads; straddle-tail resume arithmetic verified by hand for block lens 8/9/10 (no candidate skipped); steps by 2 from `pos` — parity-odd `pos` (only reachable via a corrupt idx1 entry) would scan odd positions only; self-heals on the next resync rung (noted, not a finding). |
| `is_landing_header` | 861-870 | REAL | — | acceptance = walk's own Packet gate + extent check; `saturating_add` throughout. |
| `keyframe_for` | 875-888 | REAL | — | monotone `index_cursor` advance; default-true fallback is v0-correct (MJPEG/PCM all-keyframe, mod.rs:78-80). |
| `Demuxer::start` | 892-911 | REAL | — | idempotent via `self.tracks` cache; parse errors sticky via `StickyShape`. |
| `Demuxer::next_event` | 913-945 | REAL | S3 (F-003) | pending-before-EOS ordering correct (:924-941, damage-record-first honored at walk end). |
| `Demuxer::seek` | 947-954 | REAL | S3 (F-002) | pure query, position restored by `scan_movi_with` (index.rs:642-653) — but uncapped scan (F-002). |
| `Demuxer::seekability` | 956-969 | REAL | — | constructor preconditions verified (riff.rs:219-237 rejects unknown-length/unseekable/empty) — comment's claim holds. |
| `Demuxer::reanchor` | 971-1008 | REAL | — | compute-then-commit: `locate` before any mutation; ladders replayed (indexed) or scan-captured; `pending.clear()` + `eos=false` documented as intended; mid-`rec` landing documented v0 limit (:989-993). |
| `SeekFacts` | 1012-1017 | REAL | — | plain data. |
| `locate` | 1029-1091 | REAL | S3 (F-003) | validation order sound; range check only upper-bounds (`ticks < 0` + `> end_ticks`); non-sticky not_started (F-003). |
| `landing_span` | 1095-1104 | REAL | — | start=landing pts, end=declared end / Missing. |
| `parse_container` | 1107-1267 | REAL | S3 (F-001) | head classification (truncation vs not-avi) correct; hdrl recognition-resume correct; movi clamp damage-counted; duplicate movi/idx1 damage-noted; idx1-before-movi skipped honestly; top-level non-LIST sizes trusted (F-001); `pos + 8` cannot overflow (next is saturating and bounded by `bound` ≤ file length). |
| `mod tests;` | 1270-1271 | REAL | — | 3128-line suite audited as separate chunks (A10/A08/A07/A06/A09). |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| Doctest `AviDemuxer` (:208-319) | YES | Builds a real 2-track AVI byte-by-byte, starts, asserts 2 tracks and first packet keyframe; passed in baseline (:6368 of nofailfast log). Mutation thought: if `start` returned constant `Ok(vec![])`, `assert_eq!(tracks.len(), 2)` fails. |
| demux unit suite (avi:: 85 tests) | YES (per names traced) | Executed 462/462 ok in baseline (nofailfast :1489-1955); per-test honesty assessed in the avi/tests.rs chunk audits. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Seek is a pure query; re-anchor acts… the no-index re-anchor scan is budget-bounded (doc 03 §6's SlowLinear cap)" | mod.rs:59-72 module doc | REAL | reanchor :974 uses `SEEK_SCAN_CAP` (index.rs:606-609); but the *query* is uncapped (F-002) — the doc does not claim the query is capped, so no FALSE, just the asymmetry. |
| "A lying chunk size is contained, never fatal on its own (D-3/D-9)" | mod.rs:29-36 | SHALLOW (scoped) | True inside the movi walk (resync ladder :756-804); false at the top level (F-001: fatal `no_movi` on one corrupted top-level size). |
| container-avi row: "seek-to-time … 43-test unit suite + fuzz harness" | docs/feature-matrix.toml:194-204 | REAL-UNPROVEN / stale evidence string | seek machinery real & unit-tested; "43-test" understates the current 85 `test avi::` (baseline :1489-1955) — evidence-string drift; and seek-to-time end-to-end is exactly what `journey_seek` FAILS on in baseline (`orchrun-test-journey-seek.log.txt`: "the decode accounting keeps the pre-seek deliveries — observed 90, expected 154") — an engine-side accounting defect, not disproving this file's seek math. |
| container-avi row: status windows=implemented, macos=implemented | docs/feature-matrix.toml:200 | REAL-UNPROVEN | Row's own notes admit "ALL evidence to date is Linux-only, mock-labeled, CI 3-OS PENDING"; no windows/macos run exists in baseline (macos crosscheck covered platform-macos only). Pure-Rust no-OS-branches claim verified by read; execution unproven. |
| container-avi notes: "3.A1 seek reanchor … landed on wip/3a1-seek, merge to main pending" | docs/feature-matrix.toml:204 notes | STALE/FALSE-as-of-audit | `reanchor` is present in main @ e83e5bd3 (this file :971-1008) — the "merge pending" note is out of date. |

## 7. Dead / duplicated / unreachable code
None found in this range. `Demuxer::seek` (query) has exactly one production
caller (`apps/cli/src/containers/avi.rs:58`, open-time `--start-time`); the
`SelectDemuxer` forwarding (`apps/cli/src/demux_host.rs:593`) is currently
dead in production (the engine never calls `.seek` on a demuxer) but is the
trait's contract surface — noted, not flagged.

## 8. Dependencies & flags
No feature gates in this file (plain `use` of workspace crates:
media_types, plugin_api, crate::avi::{header,index,riff,error}). No unsafe.
No new deps. `#[cfg(test)] mod tests;` only.

## 9. Open questions for the next agent
- avi/tests.rs#part3 (mine, next): does any test cover a top-level size lie
  between hdrl and movi (F-001) or a sub-4-byte rec list (F-004)? Expected: no.
- Engine-side: `journey_seek`'s "keeps pre-seek deliveries — observed 90,
  expected 154" — is the demuxer's `reanchor` pending-queue purge (:1004)
  implicated, or is it a decode-side accounting issue? (Baseline footer
  attributes it to decode accounting; demux-side purge looks correct.)
- The `generation: GenerationId::FIRST` placeholder (:665): verify the engine
  truly re-stamps every packet (comment claims doc 03 §5 checkpoint 3).
