# AUDIT: crates/demux/src/avi/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-23 |
| Lines / functions | 1271 / 24 (+1 doctest, 4 constants) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/containers/mod.rs:54 `"avi" => avi::open(stream, opts)` → apps/cli/src/containers/avi.rs:40 `AviDemuxer::new` → `start`/`next_event` (engine drives next_event at crates/playback-engine/src/pipeline.rs:2141) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 4 |
| Commands run | sed/rg/od/python3 reads (see below); baseline citations: audit/runs/baseline/rc-smoke.txt:130-131, cargo-test-workspace.txt:2472,2466-2478 |

## 1. One-paragraph verdict
This is a genuinely implemented AVI 1.0 demuxer core: RIFF walk, hdrl/strl
discovery, idx1 seek table, bounded movi packet walk with a documented
damage/resync ladder, pool-backed payload reads, and a compute-then-commit
seek/reanchor pair. Executed end-to-end proof exists in the orchestrator
baseline (shipped CLI decoded the AVI fixture 60/60 video + 60/60 audio
frames). The walk is overflow-safe (saturating arithmetic on every parsed
extent), allocation from header fields is capped (32 MiB payload ceiling,
idx1 entry cap 524,288 in index.rs:84/327-328, pool bound verified in
riff.rs:683-691), and the two index accesses are provably in-bounds.
Defects found are doc-vs-code falsehoods (a 256× understated memory bound),
a state-machine trap (next_event-before-start permanently bricks a later
start), and a small degenerate-`rec` misalignment — none reachable as
misbehavior through the shipped engine's driving order.

## 2. What actually works (verified)
- Shipped-CLI AVI decode: audit/runs/baseline/rc-smoke.txt:130-131 —
  `BENCH file=tests/fixtures/generated/valid/avi_mjpeg_pcm.avi ... v_frames=60
  v_decoded=60 a_frames=60 a_decoded=60 wall_s=0.008` and
  `headless-decode: PASS` (exit 0 overall per baseline §13).
- Doctest for `AviDemuxer` (this file, line 208) compiled and passed:
  audit/runs/baseline/cargo-test-workspace.txt:2472
  `test crates/demux/src/avi/mod.rs - avi::AviDemuxer (line 208) ... ok`.
- AVI unit/corpus/fuzz suites pass: cargo-test-workspace.txt:1698-2312
  (e.g. `noidx0_demuxes_fully_via_the_movi_walk` :2254,
  `avi_seed_loop_survives` :2278).
- Quoting hazard handled: raw bytes of lines 301/315 verified via `od -c`
  and python byte reads (`b[hdrl_start ...` / `b[movi_size_pos ...`) — the
  doctest builder is valid Rust; a display-layer artifact in my shell
  initially mangled `[h`/`[m` sequences; all quotes below were re-verified
  with `sed -n 'A,Bp'`.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/avi/mod.rs:344-347
Code:
```
    /// path (varying/oversized/busy-slot shapes). The pool itself is
    /// bounded: at most `max_slots × slot_cap` bytes (default
    /// 2 × 64 KiB = 128 KiB per demuxer) — the D-1 payload budget on
    /// top of the queues' own byte caps.
```
What is wrong: the documented pool default is false. The real default is
`PayloadSlots::new(2, 16 * 1024 * 1024)` (riff.rs:688-690) = 2 × 16 MiB =
32 MiB per demuxer — 256× the documented 128 KiB. riff.rs's own "Slot-cap
policy (7-c)" docs say the 64 KiB figure is the *pre-7-c* policy; this
field doc was never updated. Why it matters: any memory-budget reasoning
built on the mod.rs comment is wrong by 256×; the doc is the D-1 budget
claim reviewers cite. What correct looks like: "default 2 × 16 MiB = 32 MiB
per demuxer". Confidence: HIGH — Verification: read (riff.rs:683-691
cross-checked).

F-002 [S3]
Where: crates/demux/src/avi/mod.rs:921-923 and 896-898
Code:
```
        if !self.started {
            return Err(self.fail(error::not_started()));
        }
```
```
        if let Some(shape) = &self.sticky {
            return Err(shape.mint());
        }
```
What is wrong: `next_event()` before `start()` routes the not-started error
through `self.fail`, which installs it as *sticky* (line 403-406). `start()`
checks sticky before parsing (896-898), so a single out-of-order
`next_event()` call permanently bricks the demuxer — a subsequent valid
`start()` replays `demux.not-started` forever. Inconsistent with `locate()`,
which returns the same `not_started` *non-sticky* (1034-1036), so
`seek()`-before-`start()` is recoverable but `next_event()`-before-`start()`
is not — same misuse, opposite outcomes. Why it matters: a misuse-shaped
recovery hole in a public trait impl; the trait contract
(plugin-api/src/demux.rs:29-35) does not declare this permanence. The
existing test `methods_before_start_are_typed_not_panics`
(avi/tests.rs:2190-2204) asserts the error code but never retries `start()`,
so it cannot catch this. What correct looks like: not-started is a
query-state error, not a fatal-parse error — return it without installing
sticky. Confidence: HIGH — Verification: read (control flow trivial);
executable repro not run (cargo-build-budget).

F-003 [S3]
Where: crates/demux/src/avi/mod.rs:684-693 (with 461-466)
Code:
```
                    let rec_declared_end = pos + 8 + u64::from(header.size);
                    let end = rec_declared_end.min(bound);
                    ...
                    cursor.ends.push(end);
                    cursor.pos = pos + 8 + 4;
```
```
                if let Some(end) = cursor.ends.pop() {
                    cursor.pos = cursor.pos.max(end);
                    continue;
                }
```
What is wrong: a `rec ` list with declared size < 4 (its form FourCC does
not fit) pushes `end = pos+8+size < pos+12` and sets `cursor.pos = pos+12`;
the next loop pass cannot fit a header (`pos+20 > end`), pops the list, and
`cursor.pos.max(end)` keeps `pos+12` — the parent walk resumes up to 4 bytes
*inside* the following chunk's header, with no damage note for the skipped
bytes. A crafted file (rec size 0, then a chunk whose bytes at +4..+8 spell
`00dc` + a size) misparses subsequent chunk headers. Why it matters:
adversarial misalignment that is silent (uncounted); impact bounded — the
misread header is either garbage (damage-counted skip) or a bogus packet.
What correct looks like: on pop, resume at `end` (the list's own extent),
or note damage for `pos - end` when `pos > end`. Confidence: HIGH —
Verification: read (arithmetic traced); not executed.

F-004 [S4]
Where: crates/demux/src/avi/mod.rs:675-693
Code:
```
                MoviChunkKind::RecList => {
                    if cursor.ends.len() >= MAX_LIST_DEPTH {
                        return Err(self.fail(error::bad_structure(pos)));
                    }
```
What is wrong: every `LIST` inside `movi` is descended into as a record
group; the arm never checks the form FourCC is `rec ` (riff.rs:118-121
classifies any LIST as RecList "transparent descent"). ffmpeg's AVI demuxer
checks the form type. A `LIST JUNK`-style list inside movi is walked
chunk-by-chunk instead of skipped whole. Bounded and damage-counted on
garbage; spec-lax, undocumented at the descent site. Confidence: MEDIUM —
Verification: read.

F-005 [S4]
Where: crates/demux/src/avi/mod.rs:913-919 (with 924)
Code:
```
    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if let Some(shape) = &self.sticky {
            ...
            return Err(shape.mint());
```
What is wrong: sticky is checked before the pending queue is drained, so
`Recovered` damage events queued just before a fatal walk error (e.g. the
`Other`-arm notes at 695-709 that precede `resync_after_clamp`'s terminal)
can never be observed by the caller — the observability contract
("damage-record-first") silently loses the final records exactly when they
matter most (diagnosing the terminal). Confidence: MEDIUM — Verification:
read.

F-006 [S4]
Where: crates/demux/src/avi/mod.rs:884-887
Code:
```
        match table.entries.get(cursor.index_cursor) {
            Some(entry) if entry.chunk_pos == chunk_pos && entry.stream == stream => entry.keyframe,
            _ => true,
        }
```
What is wrong: for interleaved files, same-position entries for a different
stream (standard: `00dc`@P and `01wb`@P) make the cursor stop on the first
entry; if it names the other stream, the arm falls to `_ => true` and the
index's keyframe flag for *this* stream's chunk at the same position is
never consulted. No v0 impact (MJPEG/PCM are all-keyframe); wrong
keyframe truth for any future non-all-keyframe codec. Confidence: MEDIUM —
Verification: read.

F-007 [S4]
Where: crates/demux/src/avi/mod.rs:208-320
What is wrong: the `AviDemuxer` doc example embeds 112 lines of hidden
fixture-builder code (`# fn avi_demo_bytes()`); compile-fragile
documentation (one edit to the fixture writer's layout silently breaks the
doctest), and the docs claim "exactly the one the in-repo fixture writer
emits" (line 11-13) while being a hand-copied duplicate of it — a
copy-paste maintenance hazard. It does pass (baseline :2472).
Confidence: HIGH — Verification: read + baseline run.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MAX_CHUNK_PAYLOAD/RESYNC_SCAN_BUDGET/MAX_WALK_RESYNCS/MAX_LIST_DEPTH | 123-142 | REAL | — | caps; values reasonable, documented |
| MoviRegion (struct) | 144-155 | REAL | — | declared vs effective end distinction is sound |
| StickyShape::of / mint | 169-181 | REAL | — | shape replay; loses source chain after first return (documented :157-161) |
| WalkCursor (struct) | 183-196 | REAL | — | resync budget resets per re-anchor (documented) |
| AviDemuxer::new | 360-379 | REAL | — | defers all I/O to start; SourceReader rejects unseekable/unknown-length/empty |
| main_header / stream_format / damage_total | 382-400 | REAL | — | trivial accessors; stream_format is the decoder-config seam |
| fail | 403-406 | REAL | S3 | installs sticky — misused for not_started (F-002) |
| surface_damage | 409-414 | REAL | — | drains damage log into pending |
| emit | 422-428 | REAL | — | damage-record-first ordering; correct |
| step_movi | 439-452 | REAL | — | take/put-back cursor; frozen-at-damage on error |
| step_movi_inner | 455-713 | REAL | S3 | core walk; saturating arithmetic throughout; payload ceiling gate; three-path clamp logic correct (traced); degenerate-rec misalignment F-003 |
| resync_after_clamp | 756-804 | REAL | — | 3-step ladder; `next - bound` and `landing - pos` underflow-safe (callers guarantee next>bound; partition_point guarantees landing>pos) |
| resync_terminal | 812-817 | REAL | — | truncation vs resync-failed flavor split |
| scan_for_landing | 831-852 | REAL | — | WORD-aligned scan; straddle-tail math verified even/odd block lengths; budget-bounded |
| is_landing_header | 861-870 | REAL | — | accepts only walk-acceptable headers; `bytes[0..3]` provably in-bounds (8-byte slices) |
| keyframe_for | 875-888 | REAL | S4 | F-006 same-position other-stream entry |
| Demuxer::start | 892-911 | REAL | S3 | idempotent via tracks cache; sticky gate (F-002) |
| Demuxer::next_event | 913-945 | REAL | S3/S4 | ordering sticky→started→pending→eos→walk; F-002, F-005 |
| Demuxer::seek | 947-954 | REAL | — | pure query; verified it only calls locate + landing_span (no source mutation) |
| Demuxer::seekability | 956-969 | REAL | — | honest None / Fast / SlowLinear |
| Demuxer::reanchor | 971-1008 | REAL | — | compute-then-commit; clears pending, revives eos; mid-rec landing documented v0 limit (:989-993) |
| locate | 1029-1091 | REAL | — | not-started NOT sticky here (inconsistency, F-002); range check via strh length |
| landing_span | 1095-1104 | REAL | — | Missing end when length undeclared |
| parse_container | 1107-1267 | REAL | — | RIFF magic classification, clamped top-level walk, hdrl/movi/idx1 discovery; recognition-over-declared-size resume; no overflow (u32 sizes + real positions) |
| doctest avi::AviDemuxer | 208-320 | REAL | S4 | passes (baseline :2472); F-007 hygiene |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| avi::tests (mod tests, 3122 loc; run in baseline :1840-1861) | YES for the covered behaviors | corpus + adversarial fixtures, per-frame hashes; not my file but they drive this file's walk |
| methods_before_start_are_typed_not_panics (tests.rs:2190) | NO for F-002 | asserts error code only; never retries start() — the bricking escapes |
| doctest (line 208) | YES weakly | executes new→start→next_event incl. keyframe assert; would fail if header parse regressed; hidden builder is a duplicate of the fixture writer |
| avi_seed_loop_survives (baseline :2278) | YES | 1.74s seeded fuzz loop over the walk |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The pool itself is bounded ... default 2 × 64 KiB = 128 KiB per demuxer" | mod.rs:344-347 | FALSE | riff.rs:688-690 `PayloadSlots::new(2, 16 * 1024 * 1024)` |
| "seek ... does not move the read position" | mod.rs:59-61, plugin-api demuxer doc | REAL | seek (947-954) calls only locate (index search or position-restoring scan) + landing_span |
| "A lying chunk size is contained, never fatal on its own" | mod.rs:29-36 | REAL | 554-610 clamped-prefix paths traced; damage-counted |
| "idx1 is the seek table ... when absent seek falls back to a position-restoring linear scan" | mod.rs:17-22 | REAL | locate 1068-1089; scan_movi_with in index.rs:633 |
| "resync ... never a silent EOS-with-playable-tail" | mod.rs:37-55 | REAL | resync_after_clamp 756-804 + resync_terminal 812-817 |

## 7. Dead / duplicated / unreachable code
None found in this file. All 24 functions are reachable: the Demuxer trait
impl via the engine; `reanchor` via pipeline seek; accessors via CLI
containers/avi.rs (harvest_facts uses main_header/stream_format);
`fuzz`/`test_support` submodules are separate files (audited separately).

## 8. Dependencies & flags
- std-only crate (demux Cargo.toml: "C5 discipline: std-only, zero external
  dependencies. No unsafe, no unwrap/expect/panic/todo"). Verified: zero
  `unsafe`, zero unwrap/expect/panic in this file (rg scan).
- No feature gates in this file; shipped release lane reaches it (rc-smoke
  ran `first-vertical:avi-mjpeg-pcm` features, baseline :127).

## 9. Open questions for the next agent
- index.rs `Ladders::advance`/`replay_ladders` tick arithmetic and
  `scan_movi_with` budget semantics are trusted here — audit file for
  crates/demux/src/avi/index.rs should confirm no panic paths.
- header.rs `parse_hdrl` clamping and `StreamFormat` trust (WAVEFORMATEX
  channel-count allocations) — the streams table mod.rs trusts.
- riff.rs `PayloadSlots::fill` spill admission (spill buffers below
  slot_cap enter the pool — verify residency can't exceed max_slots).
