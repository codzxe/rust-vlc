# WAVE 5 — SUMMARY (MP4 PRIORITY WAVE)

Date: 2026-11-21 · Agents W05-A01…A10 · Domain: demuxers II — MP4/MOV,
MKV/WebM, MPEG-TS, MPEG-PS.
Gate: **PASS — 37/37 assignments audited (19 file + 18 chunk), 0 missing, 0 thin.**

## Verdicts (file-level)
REAL 34 · UNREACHABLE 1 · SHALLOW 1

## Findings
**S0:0 S1:0 S2:28 S3:74 S4:93 — total 195**

## THE OWNER'S BUG — where the MP4 path actually stands
Convergent verdict of 5+ agents (A02, A05, A06, A07, A10), each tracing
independently:
- **The MP4 demux layer is REAL and is NOT the blocker.** 4:2:0 H.264+AAC
  opens: tracks ready → buffering → Playing, EXIT=0 (mp4-smoke-open-session-
  420.txt). Sample-table parsing, moov-at-end, faststart all test-covered and
  shipped-reachable.
- **The yuv444p refusal is decoder-side** (`decode.unsupported` h264.sw),
  consistent with the Wave-0 correction v2: h264/sw.rs exists but its
  shipped-build reachability/feature-gating is Wave 6's question.
- **What "cannot be opened properly" actually means for MP4 today:**
  1. **In-session MP4 seek is broken** — `seekability()` advertises
     `Fast{indexed:true}` but `Mp4Demuxer` implements **no `reanchor`**; every
     in-session seek fails after the flush while `lumen open` reports
     `seekable:true`. (W05-A10, S2)
  2. **Track language is garbage on every ffmpeg-produced MP4** — see
     unpack_language below; the user-visible `language: Some("voe")` in the
     engine's own MediaInfo event.
  3. `decode-bench`/`engine-info` hard-code their outputs (Wave-0 correction
     v2), so the user-facing tooling lies about decoder availability.

## Worst findings (citations verified at gate)
1. **[S2] mp4/mod.rs:535-539 `unpack_language` off-by-one** — uses base
   `b'a'+bits` (0x61) instead of ISO 639-2's 0x60: every `und` (0x55C4)
   decodes as garbage `Some("voe")`. Root cause of the standing baseline
   defect (3 smoke transcripts); zero test coverage. Found independently by
   A04, A06, A07, A08, A10. (One-line fix: `0x60 + bits`.)
2. **[S2] MP4 seek: `indexed:true` advertised, `reanchor` not implemented** —
   every in-session MP4 seek fails in the shipped engine after the flush.
   (W05-A10)
3. **[S2] MKV lacing count byte read as absolute; spec/ffmpeg say
   frames−1** — every real laced block mis-framed; ffprobe 7.1.5 proof:
   3 packets vs the code's 2. (W05-A02)
4. **[S2] MKV `ID_BIT_DEPTH=0x55EE` is MaxBlockAdditionID's ID (BitDepth is
   0x6264)** — `bit_depth` always None → PCM-in-MKV dead in every build;
   the CLI doc claiming "ffmpeg omits BitDepth" is FALSE. (W05-A03)
5. **[S2] MKV Cues over ceiling (>65,536 — realistic all-intra/long files)
   refuses the whole file** instead of the documented SlowLinear degrade
   (mod.rs:416-421 propagates with `?`). (W05-A01)
6. **[S2] ctts v1 reads an 8-byte i64 where the spec says i32** — zero
   coverage. (W05-A06)
7. **[S2] TS `probe()` panics on exactly-188-byte 0x47 input** (demux-side
   twin of the W04-A04 media-probe phase5 S1; baseline-proven shipped-binary
   panic EXIT=101) **+ unbounded DamageLog growth on the shipped TS open
   path.** (W05-A09)
8. **[S2] PS MPEG-1 PES `header_len` unchecked vs `declared_len`** →
   payload_len u64 underflow (debug panic / release wrap). (W05-A07)
9. **[S2] ts/pes.rs `finish()` aborts EOS flush on first truncated PID** —
   drops other PIDs' tails HashMap-randomly. (W05-A04)
10. **[S2] ffmpeg 0x55C4 unspecified-language surfaces as "voe" on every
    ffmpeg MP4** (same root as #1; user-visible). (W05-A02/A04)
11. **[S2] PES `observe_pcr`/`PcrRegressed` has zero production callers** —
    the module doc's "regressed PCR never silent" promise is false in every
    shipped binary. (W05-A05)
12. **[S3] MKV metadata surface (title/chapters/attachments/tags) reachable
    only by tests** — shipped binary opens an MKV and shows no title, no
    chapters, no cover art; the code's own `[PROPOSAL]` comment admits
    "nothing downstream reads it today". Matrix row container-matroska-webm
    advertises "tracks, chapters, attachments, cues-based seeking:
    implemented". (W05-A08)

## REQUEST-ORCH-RUN outcomes
- v1-ctts sample generation + chpl layout check (optional) — NOT RUN at this
  gate (spec text absent; sample generation needs the exact v1 encoder path;
  deferred to Wave 20 A05 trace if needed).
- Corpus-mounted differential rerun — **NOT RUN: corpus-v4 directory absent
  from the repository**; every "corpus differential" evidence lane in the
  baseline ran as skip-but-ok (proven by timings). Recorded as an
  evidence-class gap (see Worst #13-class findings; feeds Wave 18/19).

## Citation spot-check (§7.3 step 4)
10 audit files sampled: PASS 28 · FAIL 2 → both adjudicated **extractor
false-positives** (compound citations; quotes verified verbatim at
cluster.rs:283-291 and mkv/mod.rs:270-274, exactly as their labels stated).
**Confirmed fabrications: 0.**

## Blocked / UNVERIFIED carried forward
- ISO 6937 accent table, ISO 23003-5 fpcm default, EN 300 468 Annex A (no
  spec text in sandbox).
- mkvmerge-side LanguageIETF exact value (no mkvmerge in sandbox).
- h264/sw.rs reachability/feature-gating → Wave 6 (video codecs) owns it.
