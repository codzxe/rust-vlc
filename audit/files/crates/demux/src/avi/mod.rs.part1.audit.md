# AUDIT: crates/demux/src/avi/mod.rs#part1 (chunk: lines 1-635 of 1271)
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-21 |
| Lines / functions | 635 audited / 16 units (4 consts, 4 structs, 11 fns/methods incl. doctest; `step_movi_inner` starts at 455 and continues past the chunk boundary into part2/A07 at 636) |
| File verdict (chunk) | REAL |
| Reachable from shipped entry point? | YES — `apps/cli/src/containers/avi.rs:40` `AviDemuxer::new(stream)` (lumen CLI AVI lane) and `apps/web/src/open.rs:251`; executed in baseline: avi-firstvertical-open-session.txt full lifecycle (Buffering→Playing→Stopped→Idle), decode-bench `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` on tests/fixtures/generated/valid/avi_mjpeg_pcm.avi (INDEX.md:78), rc-smoke 6/6. Doctest at mod.rs:208 ran green (orchrun-test-workspace-nofailfast.log.txt:6367). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 4 S4: 1 |
| Commands run | `Read` full 1-769; `rg -n "AviDemuxer" apps/ crates/` (callers: cli containers/avi.rs:40, web open.rs:251, demux tests, walk_bench example); `rg -n "avi|Doc-tests demux" orchrun-test-workspace-nofailfast.log.txt` (doctest `avi::AviDemuxer (line 208) ... ok`, avi tests green); `sed -n` on every quoted range (11-13, 123-137, 169-181, 461-477, 494-515, 517-528, 554-574, 628-637); `rg -n "fn ticks_for_sample_size" -A 10 index.rs→header.rs:329` (advance None-unreachability proof); `sed -n 575,634p riff.rs` (PayloadSlots::fill contract); `rg -n "fn new" riff.rs:219` + `sed -n 230,290p riff.rs` (SourceReader::new rejects unseekable/empty — matches doc); symbol-usage counts for all riff imports (all used); `rg -l walk_bench audit/runs/baseline/` (no example run committed). |

## 1. One-paragraph verdict
This is real demuxer code, not a stub: the `movi` walk reads chunk headers,
gates deliverability through the lane table, clamps lying sizes three ways
(in-bound full / past-EOF prefix / past-bound clamp), routes every skip
through damage accounting, and assembles packets with ladder-computed PTS.
The doctest is a genuine 114-line end-to-end construction that ran green in
the baseline, and the shipped CLI lane exercises this exact code. The
findings are quality-level: one dead-defensive branch whose failure mode is
silent payload loss, one observability gap (never-declared-stream chunks
vanish with zero damage counting), one terminal-behavior inconsistency
(truncation is fatal at a clean boundary but prefix-delivering mid-chunk),
and a self-referential conformance story — every executed AVI byte in the
evidence base came out of the repo's own fixture writer.

## 2. What actually works (verified)
— Doctest `crates/demux/src/avi/mod.rs - avi::AviDemuxer (line 208) ... ok` →
orchrun-test-workspace-nofailfast.log.txt:6367 (new → start → 2 tracks →
first movi chunk is a keyframe Packet).
— AVI first-vertical lane through this demuxer executed: INDEX.md:71-78
(avi-firstvertical-open-session.txt lifecycle + 60/60 frames both tracks).
— Overflow tracing: `next` uses saturating_add (489-492); `advance` clamps
ticks with saturating_add (index.rs:213) and `ticks_for_sample_size` clamps
`whole.min(i64::MAX as u64)` (header.rs:335); no panic path found in the
chunk (no unwrap/expect/index/slice-div; the one modulo at 634 is guarded by
`sample_size.max(1)` and only reached when `unaligned` ⟹ sample_size > 0).
— Loop-progress tracing: every branch either advances `cursor.pos`
(`resume > pos` since resume = pos+8+size+pad or bound ≥ pos+8) or returns;
the `ends`-pop path sets `pos.max(end)` with end ≥ pos. No infinite loop
shape found.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/avi/mod.rs:628-631
Code:
```rust
                    let Some(step) = cursor.ladders.advance(stream, delivered) else {
                        cursor.pos = resume;
                        continue;
                    };
```
What is wrong: Dead-defensive branch with a silent-data-loss failure mode.
`advance` (index.rs:203-215) returns `None` only for an ordinal outside the
lane table or a `None` lane; the walk already gated on
`cursor.ladders.has_lane(stream)` at line 505, and `has_lane` (index.rs:196-198)
is exactly `lanes.get(stream).is_some_and(Option::is_some)` — so once the
gate passes, `advance` cannot return `None`. The branch is unreachable, but
if the invariant ever broke (lane table mutated mid-walk, an ordinal remap),
a fully-read payload would be dropped with **no damage note** — no
`damage.note`, no `surface_damage`, nothing.
Why it matters: The module's own doctrine (lines 40-55, 700-704) is "never a
silent EOS / never forfeit playable bytes silently"; this branch is the one
place a deliverable payload can vanish with zero observability. Defensive
code that fails silently is worse than a panic here — it would hide a real
bug during exactly the damage-recovery scenarios this file exists for.
What correct looks like: Either `debug_assert!(…)` documenting the
invariant, or route the else through `damage.note(delivered,
"lane-vanished-mid-walk")` before continuing.
Confidence: HIGH — Verification: read of mod.rs:505 + index.rs:196-215
(has_lane ⟹ advance Some, mechanically).

### F-002 [S3]
Where: crates/demux/src/avi/mod.rs:11-13
Code:
```rust
//! byte layout it parses is exactly the one the in-repo fixture
//! writer emits (`tools/xtask/src/cmd/fixtures.rs`) — the generated
//! corpus and its expectation ledgers are the conformance truth.
```
What is wrong: The declared conformance truth is self-referential: the demuxer
is tested against files the repo's own writer emits. Every executed AVI byte
in the baseline evidence is the generated fixture (INDEX.md:78
`tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`); `audit/runs/baseline/samples/`
contains only MP4s (ffmpeg-generated). No externally-produced AVI (a real
encoder/capture file with JUNK padding, non-standard `idx1` `dwOffset` bases,
`rec ` nesting from capture tools) appears in any committed run. `rg -l
walk_bench audit/runs/baseline/` → no example run committed either.
Why it matters: Self-generated corpora systematically miss the byte shapes
real writers produce (odd-size padding variants, 'AVI LIST' forms at
non-standard alignment, idx1 offsets relative to file start vs movi start).
The demuxer may be exactly right or exactly fixture-shaped — the evidence
base cannot distinguish.
What correct looks like: At least one ffmpeg-/VLC-produced AVI in the corpus
with an expectation ledger, or a documented statement that v0 conformance is
deliberately limited to the in-repo writer's output shape.
Confidence: HIGH — Verification: read of module doc + INDEX.md:78 + `ls
audit/runs/baseline/samples/` (MP4s only) + baseline grep (no walk_bench,
no external AVI).
Confidence note: the code itself parses spec-shaped structures (avih/strh/
strf/movi/idx1 layouts match the AVI 1.0 spec as far as traced); the finding
is about the evidence base, not a traced defect.

### F-003 [S3]
Where: crates/demux/src/avi/mod.rs:494-515
Code:
```rust
                    // The deliverability gate: one indexed lane lookup
                    // (W5-14's D-3 cleanup — the pre-lane walk scanned
                    // the stream table per chunk). A packet chunk of a
                    // stream with no lane — codec-filtered or never
                    // declared — is skipped silently: the parse-time
                    // skip already counted the filtered ones, and the
                    // idx1 entries of a filtered stream skip silently
                    // for the same reason (W5-14, the `vprp`
                    // rationale, D-9).
                    if !cursor.ladders.has_lane(stream) {
```
What is wrong: The silent-skip rationale covers **codec-filtered** streams
(counted at strl parse time) — but a movi chunk of a **never-declared**
ordinal (e.g. a `99dc` chunk in a file whose hdrl declares streams 0-1) also
takes this branch and is skipped with **zero damage accounting**: no
`damage.note` in this branch, and parse-time counting never saw the ordinal
(no strl exists). Contrast: the `idx1` side *does* count unknown ordinals
(index.rs `dropped_unknown_stream` → "idx1-entry-unknown-stream"), and the
`Other` arm counts "unknown-chunk-in-movi". So a file without an idx1
(class-B noidx0) loses undeclared-stream chunks with no observable trace.
Why it matters: Damage accounting is this module's core observability
promise; the no-index file variant is exactly the one where the movi walk is
the only witness. A user playing a damaged mux gets no signal that whole
chunks vanished.
What correct looks like: Count never-declared ordinals here (one
`damage.note(size, "movi-chunk-undeclared-stream")`), or route unknown
ordinals to the `Other` arm's accounting.
Confidence: HIGH — Verification: read of 494-515 (no note call in branch) +
index.rs idx1 counting traced; cross-checked `filtered`/`has_lane` semantics
in index.rs:196-198 and header lane construction.
Note: severity held at S3 (not S2) because the skipped content is by
definition undeliverable media (no decoder lane exists for it) — the defect
is missing observability, not lost playable bytes.

### F-004 [S3]
Where: crates/demux/src/avi/mod.rs:467-476
Code:
```rust
                if region.effective_end < region.declared_end {
                    // The movi was clamped (RIFF extent ∩ stream) but
                    // declared more: the container is truncated, and
                    // the walk says so instead of a silent empty EOF.
                    return Err(
                        self.fail(error::truncated(region.declared_end, region.effective_end))
                    );
                }
                self.eos = true;
                return Ok(None);
```
What is wrong: Terminal behavior depends on where the truncation cut lands:
a chunk whose **payload** crosses the physical end delivers its real prefix
with a damage note and ends `Ok` (EOS) — lines 560-565, 596-601, 668-672; a
cut that lands at a clean chunk boundary or straddles a **chunk header**
(pos+8 > bound, line 462) makes the walk end in a **sticky fatal**
`truncated` error *after every playable byte was already delivered*. Same
physical condition (file shorter than the movi declared), two different
engine-visible outcomes (EndOfStream vs Error state).
Why it matters: A partially downloaded AVI that plays fine to the available
end will land the engine in Error in one case and Stop in the other,
depending on a single byte's position. The module doc (52-55) presents this
as deliberate ("a walk that cannot even read a chunk header ... reports the
declared end, never a silent EOF") — but "reports the declared end" after
full delivery is a state-choice, not a necessity, and it is inconsistent
with the mid-chunk cut's containment.
What correct looks like: Both cut shapes end the walk identically: deliver
what exists, emit `Recovered`/truncation damage, then EOS — reserving the
typed fatal for a truncation detected **before** any payload delivery
(e.g. movi entirely missing).
Confidence: MEDIUM — Verification: read + trace of both paths; no test in
part1's range pins the clean-boundary variant (tests live in tests.rs —
A07/A10's chunks; flagged for their attention in §9).
Note: this is a documented design choice, so S3 (drift/quality), not S2.

### F-005 [S4]
Where: crates/demux/src/avi/mod.rs:123-126 and 340-348
Code:
```rust
pub const MAX_CHUNK_PAYLOAD: u64 = 32 * 1024 * 1024;
```
What is wrong: The payload ceiling (32 MiB) is 512× the pool's default
capacity (2 × 64 KiB = 128 KiB, lines 345-347); any chunk between 64 KiB and
32 MiB takes the pool's exact-size spill path — a fresh allocation of the
full file-controlled size per packet (`pool.fill(want, …)`, line 575-577;
riff.rs:588+ spill). Bounded per packet and documented as a PROPOSAL, but a
stream of ~32 MiB chunks (a corrupt or adversarial file that stays under the
ceiling) allocates and frees 32 MiB per chunk with no aggregate backstop
beyond the engine's queue caps.
Why it matters: File-controlled allocation amplitude; also the ceiling is a
flat constant where a per-demuxer budget would be the safer shape.
What correct looks like: A running spill-bytes counter with a typed
ResourceLimit terminal, or lower ceiling (MJPEG frames are KBs per the
constant's own rationale).
Confidence: HIGH — Verification: read of const + pool field docs + fill
contract in riff.rs:577-596. (The pool itself is A04's chunk — cross-ref.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-92 | REAL | — | design narrative; claims audited §6; references in-repo wave/bug IDs (D-018, W5-14, W1-03) not independently verifiable here |
| `use`/mod decls | 93-121 | REAL | — | all 10 imported riff symbols used (counts verified); `pub mod fuzz`/`test_support` reachable (fuzz tests green in baseline log:1590-1594) |
| `MAX_CHUNK_PAYLOAD` | 123-126 | REAL | S4 | F-005 |
| `RESYNC_SCAN_BUDGET` | 128-132 | REAL | — | 1 MiB per attempt; used by resync (part2) |
| `MAX_WALK_RESYNCS` | 134-137 | REAL | — | 32 attempts; typed terminal on spend (part2) |
| `MAX_LIST_DEPTH` | 139-142 | REAL | — | 8; guard at 676 is fatal `bad_structure` (part2 range) — deep nesting kills walk vs damage-contained unknown chunks; noted for A07 |
| `MoviRegion` | 144-155 | REAL | — | declared vs effective end split drives F-004 |
| `StickyShape::of`/`mint` | 169-181 | REAL | — | error replay by shape; drops source chain (documented 157-161) |
| `WalkCursor` | 183-196 | REAL | — | pos/ends/ladders/index_cursor/resyncs; no panic paths |
| `AviDemuxer` (struct + doctest) | 198-349 | REAL | — | doctest ran green (baseline :6367); 114-line hand-built AVI is a real construction; would fail under header-parse, track-count, or first-packet mutations |
| `AviDemuxer::new` | 360-379 | REAL | — | delegates rejection (unseekable/empty/unknown length) to SourceReader::new — verified riff.rs:219-237 does exactly that |
| `main_header` | 382-384 | REAL | — | returns zeroed header pre-start (documented) |
| `stream_format` | 389-394 | REAL | — | O(n) find per call; decoder-config seam — callers: apps/cli/src/bench.rs harvest path |
| `damage_total` | 398-400 | REAL | — | accessor |
| `fail` | 403-406 | REAL | — | sets sticky + returns; no double-count |
| `surface_damage` | 409-414 | REAL | — | mem::take → pending queue; total preserved in DamageLog |
| `emit` | 422-428 | REAL | — | damage-first ordering; order-preserving; no drop path |
| `step_movi` | 439-452 | REAL | — | cursor take/put-back incl. error freeze; `movi: None` defensive → typed no_movi |
| `step_movi_inner` (455-635 portion) | 455-635 | REAL | S3 ×3 | F-001 (628), F-003 (494-515), F-004 (467-476); bound/EOF arithmetic traced overflow-free; `size as usize` safe (≤32 MiB check at 517 precedes); continues into part2 (A07) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| doctest mod.rs:208-320 | YES | builds a real 2-track AVI by hand, asserts `tracks.len()==2` and first event is a keyframe Packet; breaks under any header-parse, lane-gate, or packet-assembly mutation. Executed green in baseline (:6367). |
| (avi::fuzz::tests ×4, baseline :1590-1594) | YES (as survival tests) | they prove no-panic on adversarial size fields/scrambled inputs — mutation of the clamp paths to unwrap/panic would fail them; they do NOT prove byte correctness (survival only) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Packets are extracted by walking LIST movi chunk by chunk — index or no index" | mod.rs:17-22 | REAL | step_movi_inner walks movi; index used only for keyframe_for/seek (keyframe_for at 875, part2) |
| "Declared sizes are hints, recognition is authority… clamped to parent's and stream's" | mod.rs:23-28 | REAL | three-way want computation 554-574 + rec clamp 684-691 |
| "A lying chunk size is contained, never fatal on its own" | mod.rs:29-30 | REAL (with F-004 caveat) | payload paths deliver prefix + damage; the fatal truncation is a lying LIST size, not chunk size — consistent as worded |
| "zero allocation in the steady state" (pool, 340-347) | mod.rs:340-347 | REAL-UNPROVEN | fill() pass-1 serves from pooled arenas (riff.rs:597-628 traced); no committed run of the alloc-counting example (walk_bench) in baseline — UNVERIFIED by execution |
| "Timing is the AVI ladder, not per-chunk stamps… format-canonical" | mod.rs:73-77 | REAL | Ladders/advance + ticks_for_sample_size (index.rs:203-215, header.rs:329-338) traced; video=1 tick/chunk, audio=whole-samples |
| "the generated corpus and its expectation ledgers are the conformance truth" | mod.rs:11-13 | SHALLOW (evidence base) | F-002 |
| "first vertical" AVI MJPEG+PCM lane works | doc 03 §7 per mod.rs:1-2 | REAL | INDEX.md:71-78 (open-session lifecycle + 60/60 frames) |

## 7. Dead / duplicated / unreachable code
- F-001's else-branch (628-631): unreachable by the has_lane/advance
  invariant — the only dead-defensive shape in the chunk.
- No duplication found within the chunk; `main_header`/`stream_format`/
  `damage_total` are thin accessors, each with callers (bench.rs, part2).

## 8. Dependencies & flags
- Uses `media_types` (error/time/ids), `plugin_api` (Demuxer/SourceStream/
  DemuxEvent) — all workspace crates; no feature gates in this chunk
  (no `#[cfg]` other than the doctest's none); nothing gated behind
  non-shipped features.
- `pub mod fuzz` and `pub mod test_support` are compiled unconditionally
  (test-support seam in production build — deliberate, noted in W3 audits of
  other crates; same pattern here, `test_support.rs` is A05's file).

## 9. Open questions for the next agent
- A07 (mod.rs#part2): does any test pin the **clean-boundary truncation**
  fatal (F-004) vs the mid-chunk prefix path? Does `next_event` drain
  `pending` before calling step_movi (emit's packet-waiting-one-call depends
  on it)?
- A07: `resync_after_clamp` at 756+ — verify `next - bound` can't underflow
  at every call site (part1's three call sites all check `next > bound`
  first; verify part2's).
- A07: MAX_LIST_DEPTH guard (676) is fatal `bad_structure` — decide whether
  deep-nesting should be damage-contained like everything else.
- A04 (riff.rs): pool spill aggregate budget (F-005 cross-ref); zero-alloc
  claim unexecuted (walk_bench never run in baseline).
- A10/A06 (avi/tests.rs): confirm coverage exists for never-declared-stream
  movi chunks (F-003) — I found no test in part1's range (tests are in
  tests.rs chunks).
