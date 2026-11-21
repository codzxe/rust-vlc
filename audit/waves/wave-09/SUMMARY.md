# WAVE 9 — SUMMARY (subtitles)

Date: 2026-11-21 (session 2) · Agents W09-A01…A10 · Domain: subtitles —
SRT/VTT/ASS/SSA/PGS/DVB/CC, text rendering, overlay compositing.
Gate: **PASS — 60/60 assignments audited, 0 missing, 0 thin, citation
spot-checks clean.** (Verdict strings: several agents wrote composite verdicts;
canonicalized as UNREACHABLE-from-shipped in the counts below.)

## Verdicts (file level)
UNREACHABLE-from-shipped ~49 · REAL-library 2 · REAL-UNPROVEN 9
(Real, tested code — but nothing shipped links it.)

## Findings
**S0:0 S1:4 S2:28 S3:74 S4:120 — total 226**

## The wave's hostile questions
- **Is any subtitle path reachable in the shipped build? NO.** The entire
  `crates/subtitles` crate is a dependency of nothing shipped: the CLI refuses
  `subtitle.*` at dispatch (apps/cli/src/dispatch.rs:198); the pipeline host
  discards subtitle frames (pipeline_host.rs:1238); `sei.rs` discards GA94 CC
  bytes; the demuxer mints `pgssub` track kinds no consumer exists for. Sole
  reverse-dependency: the nightly, unwired `fuzz/` scaffold.
- **Do the ASS subset claims match the code? YES — the parser's** 13-tag G-2
  minimum with longest-match is verified arm-by-arm (A02, A09). But the
  **parser→render seam does not exist**: render entry points have zero
  non-test callers and are type-incompatible with parser output — no parsed
  ASS cue can ever be rendered. The record's "per-run rendering" composed
  claim is FALSE (A09).
- **Baseline honesty:** both primary baseline test runs aborted BEFORE the
  subtitles package ever ran; its 267/0 green exists only in the
  orchestrator's no-fail-fast rerun — and audit/runs/baseline/INDEX.md omits
  that log (INDEX drift, A09 self-correction).

## Worst findings (citations in per-file audits; sampled at gate)
1. [S1] dvb region-depth change with same dims keeps a stale 8-bit pixel
   buffer → compose indexes a 4/16-entry CLUT bank unchecked → guaranteed
   panic; violates the module's "never panics" contract; untested by all 267
   tests (A06, A07 — found independently; reproducer bytes filed).
2. [S1] dvb/segment.rs:381 unchecked `wx - x + 1` u16 under/overflow in
   `parse_display_def` — panics on spec-legal max/min values (A04).
3. [S1] Crafted <1 MB VobSub idx/sub allocates multi-GiB (fill-run 8192×
   amplification, uncapped cue list) — violates "never unbounded allocation"
   (A08).
4. [S1] vobsub/idx.rs:206/221 unchecked i64 timestamp parse — overflow panic
   in debug/fuzz, silent wrap in release (A01).
5. [S2] cc_data.rs flags byte implements the wrong bit layout vs ATSC A/53
   CEA-708 (no em_data skip; hard BadZeroBit on a legal bit) — legal streams
   misparse or are refused (A10).
6. [S2] wire.rs TextStyle round-trip silently drops strikeout / font_size /
   font_name — "cue→serialize→parse→equal" acceptance claim is data loss
   masked by a matrix that never constructs those fields (A03).

## Blocked / UNVERIFIED
- All spec/ffmpeg/libass reference comparisons (A/53, CTA-708, SSA v4, ETSI
  DVB, pgssubdec) — no sources in sandbox; claims are record-cited, MEDIUM
  confidence.
- REQUEST-ORCH-RUN filed: subtitles test rerun, depth-change panic repro,
  SRT quadratic perf proof, ffmpeg dvbsubdec differential.
