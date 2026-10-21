# Engineering-Directives Audit — 2026-10-22 (wave 1)

Five adversarial audit agents (ED-AUDIT-1..5) audited main @ e072caf against
the binding directives (docs/engineering-directives.md, D-1..D-10). Method:
read-only, full-file reads, file:line evidence. This register is the durable
record; owning fix cards are registered in the worklog ED-AUDIT entry.

## Verdicts

| Audit | Scope | Verdict |
|---|---|---|
| ED-AUDIT-1 | D-1/D-8 memory, boundedness, zero-copy, fast-stream serialization | **VIOLATING** |
| ED-AUDIT-2 | D-2/D-6/D-7 algorithms, mechanical sympathy, ceremony, deferred debt | **VIOLATING** |
| ED-AUDIT-3 | D-3/D-9 dirty-world parsing, graceful recovery | **PARTIAL** |
| ED-AUDIT-4 | D-4/D-8 lifecycle, state isolation, concurrency, clock drift | **VIOLATING** |
| ED-AUDIT-5 | D-5/D-10 testing provenance, adversarial coverage, oracles | **VIOLATING** |

The clean anchors (verified, credited): `render_into` RT callback (alloc-free,
lock-free, bounded), SPSC ring, `BoundedQueue` (3-axis caps, weigh outside
lock), PCM zero-copy native path, packet-queue invariants Q1-Q4, 3.A1 seek
flush (decoder instances recycled in place, D4 rollback real, generation
discipline end-to-end within an epoch), audio pause = flag not device drop,
desktop wire = binary LUF1 (no JSON on frame path), zero input-reachable
panics in all scoped ingest code, `#[ignore]` inventory exactly 4 (no creep),
invariant-weakening scan clean (no commented-out asserts, no widened
tolerances, no env-gated tests), D-7 TODO scan clean (zero TODO/todo! in
engine src).

## CRITICAL violations (fix first)

| # | Directive | file:line | violation | Fix card |
|---|---|---|---|---|
| C1 | D-2/D-10 | codecs/src/video/mjpeg/idct.rs:79-103 | Naive i64 matrix-multiply IDCT, no zero-AC fast path; ~50M i64 MACs per 1080p frame — 4K60 freezes | ED-FIX-1 |
| C2 | D-2 | codecs/src/video/mjpeg/color.rs:98-133 | Per-pixel hardware division (6/pixel, loop-invariant `sy` recomputed), branch-laden per-pixel loop, i64 color math | ED-FIX-1 |
| C3 | D-2 | codecs/src/video/mjpeg/huffman.rs:85-105 + bitreader.rs:101-122 | Per-bit table walk, no LUT, bit-at-a-time accumulator | ED-FIX-1 |
| C4 | D-1 | codecs/src/video/mjpeg/decoder.rs:259-266 etc. | Stateless decode: 3 fresh multi-MB planes + 4 Huffman LUT rebuilds + crop intermediates per frame (~190 MB/s churn at 1080p60) | ED-FIX-1 |
| C5 | D-1 | renderer-wgpu/src/staging.rs:148,174 + backend.rs:105-115 | Fresh full-frame Vec per present (staging `mem::take` frees it; compositor `with_capacity` per composite, 4-byte writes); doc claims "reusable buffer" — false | ED-FIX-2 |
| C6 | D-1 | audio-engine/src/convert.rs:100-124 + pipeline.rs:223-237 | `planes.clear()` drops inner Vecs → fresh allocs per frame; `mem::take(out_bytes)` → quantum output reallocs; `refs` Vec per mix. Doc 03 §4.8(c) "never allocates per frame" violated by its own code | ED-FIX-2 |
| C7 | D-1 | apps/desktop transport/wire (audit-1 #6) | New WireDecoder per frame + full-frame `to_vec` + O(n) `drain` — 2 allocs + 2 copies per frame on the frame path | ED-FIX-2 |
| C8 | D-4/D-7 | apps/cli/src/host.rs:714-800 | CLI seek = stop→close→**file reopen**: destroys session epoch, audio device + 131k-frame ring (undrained tail), clock (re-anchored at requested not actual), decoders, renderer surface, all observability. Engine-native `SessionHandle::seek` exists (c79ec88) but no host calls it. UNOWNED card | ED-FIX-4 |

## MAJOR violations

| # | Directive | file:line | violation | Fix card |
|---|---|---|---|---|
| M1 | D-1 | playback-engine/src/session.rs:357-370,532 | Receipts pinned past the 1024 outbox cap (unbounded); `std::sync::mpsc` inbox unbounded; `push` O(n) `position+remove` at capacity | ED-FIX-4 (with 3.B4) |
| M2 | D-1 | audio-output/src/null.rs:162-167 | `write_bytes` fresh `Vec<f32>` per quantum + extra full copy | ED-FIX-2 |
| M3 | D-1 | video-engine/src/scheduler.rs:375,392-395 | Fresh `drops.late`/`superseded` Vecs every dropping tick | ED-FIX-2 |
| M4 | D-1/D-2 | demux/src/avi/riff.rs:246-289 + input/buffered.rs:179-186 | Per-chunk: 8-byte header heap alloc, 2 payload-sized allocs + 2 copies; unbatched movi walk does 2 redundant seeks per packet; idx1 parsed 16 bytes/read (audit-2 #7-#10: linear idx1 scan + O(N×streams) ladder replay per seek) | ED-FIX-3 (read chain: 3.B3) |
| M5 | D-3/D-9 | demux/src/avi/header.rs:522-526 | **Negative biHeight** (bottom-up DIB, ubiquitous) → whole-file abort | ED-FIX-3 |
| M6 | D-3/D-9 | header.rs:429-564 | One degenerate `strl` (dwScale=0, short strh/strf, channels=0) kills ALL streams — file refuses to play | ED-FIX-3 |
| M7 | D-3/D-9 | demux/src/avi/mod.rs:461-467 | Lying chunk size past EOF → sticky fatal session teardown; past-movi-bound → silent tail loss with zero damage events | ED-FIX-3 |
| M8 | D-3/D-9 | demux/src/avi/index.rs:193-217 | Absolute (non-movi-relative) idx1 offsets: every entry dropped + up to 524k unaggregated damage events queued (~8k ticks of event storm); idx1 entry-count cap checked before available-bytes clamp | ED-FIX-3 |
| M9 | D-3 | codecs mjpeg mod.rs:262 + pipeline.rs:787-795 | ONE corrupt JPEG frame mid-stream → session Error+teardown (stateless codec; PCM sibling does `Recovered` for the same class) | ED-FIX-4 |
| M10 | D-4 | playback-engine pipeline.rs:639-643 + video-engine scheduler | Seek pre-roll frames delivered while clock still frozen at OLD position — a host ticking the scheduler during pre-roll presents pre-completion frames (masked today only by a CLI accident) | ED-FIX-4 (3.C1) |
| M11 | D-4 | playback-engine session.rs:382-403 vs epoch.rs:19-23 | OutputHub staleness compares bare pipeline counters, ignores session epoch — the exact trap epoch.rs's doc names; masked by host-side filters | ED-FIX-4 (with 3.B4) |
| M12 | D-8 | media-clock/src/drift.rs | `DriftCorrection` ±1000ppm limiter: ZERO consumers — drift-compensation loop does not exist (no measurement, no re-anchor, no resample) | 3.B1 (carded; wiring unowned) |
| M13 | D-6 | whole workspace | No benchmark exists anywhere; every performance claim (incl. the violated ones) is syntactic-green only | ED-FIX-1/2 add measurement |
| M14 | D-5 | tests/fixtures/** + fuzz/ | All 24 fixtures generator-authored (writer=parser author); class-D external corpus 0% built; all 3 cargo-fuzz targets are dead files (no fuzz/Cargo.toml, never compiled); **phantom artifact**: 2 files cite `crates/codecs/tests/pcm_fuzz_survival.rs` which does not exist | ED-FIX-5 |
| M15 | D-5 | codecs/tests + baselines | Every "pinned hash" is self-referential (writer-truth or decoder-output-recorded); the ffmpeg cross-check was a one-time dev event, unreproducible; docs/baselines VLC data never consumed by any test | ED-FIX-5 |
| M16 | D-5 | demux tests | Adversarial gaps: no out-of-order idx1 / non-monotone ladders; no true short-read (dribble) source; no B5 wrong-container probe race | ED-FIX-5 |
| M17 | D-4 | apps/cli/src/run.rs:221 | REPL quit/EOF during Playing exits with no stop/close/teardown | ED-FIX-4 |
| M18 | D-2 | media-types/time.rs:120-188 | i128 software division per timestamp conversion (per packet/frame) — reciprocal fast path exists | ED-FIX-2 |
| M19 | D-3 | probe/demux vocabularies | mjpa/mjpb: probe hints mjpeg, demuxer rejects (stages disagree); cut-in-RIFF-head reports Unsupported not Truncated; wasm32 usize overflow in probe walk; channels=65535 accepted | ED-FIX-3 |

MINOR items (recorded, batched into owning cards): CLI unbounded
events/state_history Vecs (ED-FIX-4); OutputHub drain alloc per call
(3.B4); `as_wire` per-event Strings (3.B4); event.rs BTreeMap per fatal
(3.B4); ring per-sample AtomicU32 loads (D-2, later); NullDevice capture
unbounded (AC tooling, cap later); screenshot cold-path allocs (acceptable);
scheduler `insert_sorted` O(N) with N≤16 (fine); commit
`input read-into-borrow API` to the 3.B3 card (kills M4's root cause).

## Registered fix cards

- **ED-FIX-1** (crates/codecs mjpeg + fixtures regen): factored i32 IDCT +
  zero-AC fast path; LUT Huffman + word-refill bitreader; row-hoisted
  division-free color conversion; decode-context reuse (planes/LUTs/scratch);
  crop-in-place; throughput measurement (no new deps); reference regen with
  equivalence evidence.
- **ED-FIX-2** (audio-engine, audio-output, renderer-wgpu, video-engine,
  media-types): plane/output-buffer true reuse; refs Vec kill; null-sink
  stack chunks; staging/compositor buffer pool + row-slice copies; scheduler
  fixed-capacity drop arrays; time-conversion reciprocal fast path; counting
  allocator instrument (std-only GlobalAlloc wrapper) + zero-alloc tests on
  quantum/present paths; desktop transport direct handoff + no per-frame
  decoder alloc.
- **ED-FIX-3** (crates/demux AVI + probe alignment): negative biHeight;
  per-stream containment (bad strl skips a stream, never the file); chunk
  clamp + explicit damage (no silent EOS, no fatal on recoverable); idx1
  absolute-base autodetect + clamp-before-cap + coalesced damage; rec clamp
  symmetry + scan-path bound; truncated-head flavor; mjpa/mjpb alignment;
  wasm32 saturating; channels ceiling; adversarial tests + 2 new class-B
  fixtures.
- **ED-FIX-4** (apps/cli + playback-engine, AFTER 3.B4 merges): host seek
  migration to `SessionHandle::seek` (host generation duties: ring flush
  seam, queue publish, hold clear; rewrite pinned ACs); MJPEG per-frame
  isolation (`Recovered` not teardown); pre-roll presentation hold (3.C1
  carve-out); OutputHub epoch-aware staleness; bounded inbox + receipt cap;
  graceful REPL exit.
- **ED-FIX-5** (testing, AFTER ED-FIX-1/3): pcm_fuzz_survival.rs (fix the
  phantom); cargo-fuzz scaffolding + nightly wiring (Lead dep decision);
  class-D external corpus row (ffmpeg-encoded AVI+MJPEG pinned);
  DribbleStream short-read tests; out-of-order idx1 + non-monotone ladder
  tests; B5 wrong-container probe race; `scripts/oracle_check.sh` (external
  oracle executable, opt-in); consume baselines JSON in one comparison test.

Sequencing: 3.B4-R + ED-FIX-1 + ED-FIX-2 + ED-FIX-3 parallel now (disjoint
crates); ED-FIX-4 after 3.B4 merges (same files); ED-FIX-5 last. Evidence
note: ED-FIX-1 may shift decoded chroma by ±1 → fixtures + first-vertical
evidence regen REQUIRED on main after its merge (one Lead-run regen covers
both).
