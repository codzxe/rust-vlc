# AUDIT: crates/demux/src/es/annexb.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-23 |
| Lines / functions | 539 / 25 |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/containers/mod.rs:63-64 (`"hevc" => es::open_annexb(...)`) -> apps/cli/src/containers/es.rs:186 (`AnnexbVideoDemuxer::new(codec, stream)`) -> this file. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | rg (callers, tests, reasons), sed -n quote verification, python3 od/hex byte checks, ls of corpus dir (absent). No cargo runs (baseline cited). |

## 1. One-paragraph verdict
A genuinely implemented, windowed, NAL-type-aware Annex-B demuxer with correct boundary semantics (verified against H.264 `first_mb_in_slice` / HEVC `first_slice_segment_in_pic_flag` bit positions), bounded memory (16 MiB AU ceiling + lazily-grown 2-slot pool), typed sticky errors, and honest missing-stamp timing. All indexing is guarded (`while i + 3 + cl <= win.len()`), the zero-walk-back is clamped by `floor`, and `to - from` cannot underflow (boundary `at` is provably `>` prior `au_start`). Two real holes: (1) the multi-window scan path — WINDOW=256 KiB carry/overlap logic at lines 300-306 — is never executed by any test in the repo (largest test input is 4096 B fuzz, <200 KB fuzz-lite, tiny unit streams); (2) the `MAX_AU_BYTES` bounded-memory refusal is likewise untested. Both guards exist in code but a mutation to either passes the whole suite.

## 2. What actually works (verified)
— verified by rg over baseline cargo-test-workspace.txt lines 1894-1898, 1903: `es::tests::h264_aus_follow_the_nal_type_rule ... ok`, `hevc_aus_group_on_first_slice_flag_and_vps ... ok`, `empty_source_refuses_at_construction ... ok`, `next_event_before_start_is_typed ... ok`, `fuzz_lite_mutations_never_panic_or_hang ... ok`; corpus differential `es_corpus` 4 passed (baseline line ~2156) — but corpus files are absent in this sandbox (ls exit 2), so corpus content claims are cited from the baseline, not re-run.
— Bit-position reasoning verified by read: H.264 `head[1] & 0x80` = first slice-header bit (`ue(0)` single 1 bit) at line 151; HEVC `first_slice_segment_in_pic_flag` = bit 15 = MSB of second header byte — same expression, correct for both.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/es/annexb.rs:300-307 (carry logic) + tests evidence
Code (verbatim, 300-306):
```
            // No boundary in this window: carry the overlap forward,
            // never behind the last classified NAL.
            self.pos = if win_end >= len {
                len
            } else {
                (win_end - OVERLAP).max(self.floor).min(win_end)
```
What is wrong: the multi-window path is dead code under test. `WINDOW = 256 * 1024` (line 77); the largest inputs anywhere in the test suite are the es fuzz harness's 4096-byte hostile buffers (es/fuzz.rs:107-109), fuzz-lite's `< 200_000` base (es/tests.rs:312), and the hand-built unit streams (tens of bytes). No test input reaches 256 KiB, so the overlap carry, the `.max(self.floor)` clamp, and the window-straddling start-code dedup via `last_code` NEVER execute in CI. Why it matters: a mutation `(win_end - OVERLAP)` -> `win_end` (dropping the carry) or removing `.max(self.floor)` passes every test, and in production it silently mis-bounds access units on any raw stream > 256 KiB — i.e., every real-world .264/.265 capture. What correct looks like: one unit test with a synthetic ~600 KiB stream (e.g., three 250 KiB NAL payloads) asserting exact AU geometry across at least two window edges. Confidence: HIGH — Verification: read + rg input sizes (counts above); corpus size UNVERIFIED (dir absent here; baseline says small 26/56-packet streams).
F-002 [S3]
Where: crates/demux/src/es/annexb.rs:449-452
Code (verbatim):
```
        let size = to - from;
        if size > MAX_AU_BYTES {
            return Err(self.fail(super::error::au_too_large(size)));
```
What is wrong: the bounded-memory refusal (`MAX_AU_BYTES`, 16 MiB, MP §12) has zero test coverage — no test constructs an access unit above the ceiling, so `demux.es.au-too-large` (es/error.rs:56-64) is never minted under test. The pool slot cap makes the failure mode safe today (guard exists), but the guard is the only thing between a hostile multi-GiB one-NAL file and an unbounded `pool.fill` allocation, and nothing pins it. Same class as the AVI `chunk-payload-over-ceiling` gap (see avi/tests.rs audit F-001). Confidence: HIGH — Verification: rg `au_too_large|MAX_AU` over es/tests.rs, es_corpus.rs, es/fuzz.rs -> 0 hits.
F-003 [S4]
Where: crates/demux/src/es/annexb.rs:528-538
Code (verbatim):
```
        let _ = target;
        Err(EngineError::new(
            media_types::PipelineStage::Demux,
            media_types::ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.es.seek.unsupported"),
```
What is wrong: the seek refusal is honest (no timing exists to map a target onto — module docs) but untested: no test asserts `demux.es.seek.unsupported`. If seek were accidentally wired to return Ok with a fabricated span, no test would fail. Confidence: HIGH — Verification: rg `seek` in es/tests.rs + es_corpus.rs -> no annexb seek test.
F-004 [S4]
Where: crates/demux/src/es/annexb.rs:390-395 + 353
Code (verbatim, 390-391):
```
    /// Total bytes logged as damage (always 0 today — Annex-B damage
    /// is not byte-attributable; kept for parity with the siblings).
```
What is wrong: `damage: DamageLog` is a never-written field; `damage_total()` is a constant-0 accessor kept "for parity". Documented, but it is dead state — a caller using it to detect corruption gets a permanent 0. Confidence: HIGH — Verification: rg `self.damage` -> only the declaration and the read.
F-005 [S4]
Where: crates/demux/src/es/annexb.rs:481-493
Code (verbatim, 490-492):
```
        self.probe()?;
        self.started = true;
        Ok(vec![self.mint_track()?])
```
What is wrong: `started` is set to true BEFORE `mint_track()` runs; if mint_track ever failed, a subsequent `start()` would take the `started` branch, find `track == None`, and return the misleading `not-started` error instead of replaying the real failure. Unreachable today (descriptor() over static ASCII "h264"/"hevc" cannot fail — line 170-172 documents this), so it is an ordering smell, not a live bug. Also `classify_len` (157-162) is a two-arm match returning 2 in both arms — copy-paste shape. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AnnexbCodec::header_len | 98-103 | REAL | — | 1/2 bytes per spec |
| AnnexbCodec::nal_type | 107-112 | REAL | — | H.264 & 0x1F; HEVC >>1 & 0x3F — correct bit fields |
| AnnexbCodec::is_vcl | 116-121 | REAL | — | H.264 1..=5; HEVC <=31 — correct |
| AnnexbCodec::is_irap | 125-130 | REAL | — | H.264 IDR(5); HEVC IRAP 16..=23 — correct |
| AnnexbCodec::is_prefix_class | 134-139 | REAL | — | narrow sets, excludes filler/suffix-SEI per docs |
| AnnexbCodec::starts_new_picture | 150-152 | REAL | — | head[1]&0x80 for both codecs — bit math verified |
| AnnexbCodec::classify_len | 157-162 | REAL | — | both arms 2 (F-005 note) |
| AnnexbCodec::descriptor | 165-173 | REAL | — | cannot-fail residue typed via bad_header(0) |
| AnnexbCodec::format | 177-182 | REAL | — | error vocabulary |
| probe (free fn) | 189-208 | REAL | — | bounds-guarded scan; conservative plausibility |
| AuScan::new | 230-240 | REAL | — | zero-init state |
| AuScan::next_boundary | 248-309 | REAL | S3 (coverage) | logic sound; window carry never tested (F-001); walk-back clamp correct; no panic path |
| StickyShape::of / mint | 321-331 | REAL | — | replayable sticky errors |
| AnnexbVideoDemuxer::new | 364-382 | REAL | — | source capability checks via SourceReader; pool lazy |
| aus_demuxed | 386-388 | REAL | — | count truth for differential |
| damage_total | 393-395 | STUB | S4 | constant 0 (F-004) |
| fail | 397-400 | REAL | — | sets sticky |
| probe (method) | 404-414 | REAL | — | 64 KiB bounded open scan |
| mint_track | 417-441 | REAL | — | 1/25 base, Missing span — honest timing |
| emit_au | 444-470 | REAL | S3 (coverage) | size ceiling untested (F-002); no underflow (at > au_start proven) |
| take_pending | 476-478 | REAL | — | one-packet queue |
| Demuxer::start | 482-493 | REAL | S4 | ordering smell (F-005); idempotent on success |
| Demuxer::next_event | 495-526 | REAL | — | sticky replay, EOS idempotent, final partial AU emitted (documented ffmpeg parity) |
| Demuxer::seek | 528-538 | REAL | S4 (coverage) | honest typed refusal, untested (F-003) |

## 5. Test honesty
Tests live in es/tests.rs + es/fuzz.rs + tests/es_corpus.rs (not this file):
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| es::tests::h264_aus_follow_the_nal_type_rule | YES | exact AU count (4), exact byte sizes, keyframe flags, contiguous-coverage sum, Missing stamps |
| es::tests::hevc_aus_group_on_first_slice_flag_and_vps | YES | 3 AUs, IRAP flags, coverage sum |
| es::tests::h264_no_sync_is_typed_unsupported_after_scan | YES | typed refusal after scan |
| es::tests::empty_source_refuses_at_construction | YES | construction-stage refusal |
| es::tests::next_event_before_start_is_typed | YES | contract-order |
| es::tests::fuzz_lite_mutations_never_panic_or_hang | PARTIAL | absence-of-panic only (by design); corpus < 200 KB so no window edge (F-001) |
| es::fuzz::tests::empty_and_marker / hostile_wide_inputs_survive | PARTIAL | panic/hang absence only, 4096 B max |
| tests/es_corpus.rs (4 tests, baseline ok) | YES for geometry | exact ffprobe packet sizes/counts on real files; corpus ABSENT in this sandbox (UNVERIFIED locally, baseline cited) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "each next_event reads only what it needs... never the whole file" | annexb.rs:44-47 | REAL-UNPROVEN | windowed reads verified by read; but the scan walks every window to EOF across calls, and emit re-reads each AU (2x file I/O total) — claim is about per-call reads, holds |
| "a larger access unit is a typed ResourceLimit refusal" | annexb.rs:49-51 | REAL (code) / UNTESTED | guard at 450-452; no test (F-002) |
| "Packets therefore carry Stamp::Missing pts/dts" | annexb.rs:33-36 | REAL | es/tests.rs asserts Missing; corpus asserts Missing |
| "Seeks are the honest typed refusal" | annexb.rs:340-341 | REAL (code) / UNTESTED | seek 528-538; no test (F-003) |
| "the final partial access unit is emitted (ffmpeg's behavior)" | annexb.rs:57-60 | REAL | next_event None-branch emits [from, len) |

## 7. Dead / duplicated / unreachable code
- Window carry path (300-307): live in production, dead under test (F-001).
- `DamageLog damage` field + `damage_total()`: never written (F-004).
- `classify_len` two identical arms (F-005).
- `take_pending().unwrap_or(DemuxEvent::EndOfStream)` (512, 519): the unwrap_or arm is unreachable after a successful emit_au — defensive dead branch.

## 8. Dependencies & flags
No feature gates; std-only + media_types/plugin_api + reuse of `crate::avi::riff::{PayloadSlots, SourceReader}` and `crate::avi::header::DamageLog` (documented cross-module reuse; layering within the same crate, acceptable). No unsafe. House rules (no unwrap/panic in non-test code) hold — verified by read.

## 9. Open questions for the next agent
- Measure the real corpus .h264/.265 sizes: if any exceeds 256 KiB, es_corpus partially covers F-001; if not, the gap is total (corpus dir absent here — UNVERIFIED).
- The engine layer: what happens on `demux.es.seek.unsupported` at playback time — is seek UI disabled for raw ES files, or does the user see an error? (apps/cli + playback-engine auditors.)
