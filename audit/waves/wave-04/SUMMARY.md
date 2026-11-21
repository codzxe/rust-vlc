# WAVE 4 — SUMMARY

Date: 2026-11-21 · Agents W04-A01…A10 · Domain: demuxers I — adts, aiff, asf,
avi, es, ivf, ogg, wav + media-probe (signatures/phases) + demux fixtures.
Gate: **PASS — 107/107 assignments audited, 0 missing, 0 thin.**
(Infra note: 8/10 first spawns hit platform timeouts; all retries succeeded,
picking up persisted worktree work. One agent (A08) needed only verification
of its prior incarnation's completed audits.)

## Verdicts (file-level)
REAL 84 · UNREACHABLE 13 · SHALLOW 2

## Findings
**S0:1 S1:1 S2:18 S3:76 S4:179 — total 275**

## Worst findings (citations verified at gate)
1. **[S0] annexb.rs HEVC AU-boundary reads `nuh_layer_id` MSB instead of
   `first_slice_segment_in_pic_flag`** — an entire .265 stream demuxes as
   ONE packet; AU grouping never happens. (W04-A05)
2. **[S1, EXECUTED-CONFIRMED at gate] TS signature off-by-one panics the
   shipped binary** — `prefix[188]` at media-probe/src/signature/
   phase5.rs:161 panics on any exactly-188-byte TS file. Orchestrator repro
   with the shipped `lumen` binary: "index out of bounds: the len is 188
   but the index is 188", EXIT=101
   (orchrun-ts188-panic-repro.log.txt). (W04-A04; fix: `>` instead of `>=`
   at :160)
3. **[S2] The entire ASF demuxer is dead code in the shipped product** —
   36 tests green, but no probe signature and no registry arm: every
   .wmv/.wma/.asf is refused `probe.no-demuxer` in both shipped frontends
   while the matrix claims "implemented" on windows/macos; doc's "every
   landed container" claim FALSE. (W04-A02/A07/A08)
4. **[S2] ASF header children-walk unchecked `off as u64 + size` wraps near
   u64::MAX → slice start>end PANIC on crafted input** — no fuzz/test
   coverage on the path. (W04-A06)
5. **[S2] avi/index.rs `Ladders::new` allocates per unbounded strl ordinal**
   — ~3× file-size RAM amplification from a crafted hdrl (DoS vector on
   open). (W04-A01)
6. **[S2] ogg/mod.rs:272 reads Theora KFGSHIFT from bytes 40–41; spec puts
   it at 43–44** — real .ogv files get wrong durations, frame counts and
   seek landings. (W04-A10)
7. **[S2] ps/mod.rs reanchor conflates SCR with media time** — a 0.54s seek
   on the VOB fixture lands at 1.29s of content; unpinned by any test.
   (W04-A03)
8. **[S2] adts/mod.rs ADIF unconditional 4-bit num_pce read** (W04-A09);
   **[S2] corpus differential lanes skip-as-pass** — ogg corpus SKIPPED in
   every committed run while evidence strings cite it (W04-A02).
9. **[S3] libFuzzer targets: registered=35, executed=0** — fuzz coverage is
   paperwork. (W04-A01)
10. **[S3] fuzz-lite test name promises "never panic or hang" with no
    timeout** — a hang would surface as a CI stall, not a failure.
    (W04-A02, citation precision note in fabrications.md)

## REQUEST-ORCH-RUN outcomes
- TS 188-byte panic repro → **CONFIRMED, EXIT=101** (above).
- `cargo test -p media-probe --features contract --release --locked` →
  **PASS** (orchrun-test-mediaprobe-contract.log.txt).
- corpus-dependent dumps (0044-raw-video.265 NAL layout, theora header) →
  **NOT RUN: corpus directory absent from the repository** (recorded as
  evidence-gap; the corpus the docs cite is not in the tree).
- `cargo +nightly fuzz run` → NOT RUN (nightly toolchain not installed;
  low priority). walk_bench example run → NOT RUN (low priority, clippy
  proves compilation).

## Citation spot-check (§7.3 step 4)
12 audit files sampled: PASS 20 · FAIL 1 → adjudicated **citation
imprecision, not fabrication** (W04-A02 es/tests.rs: doc comment quoted
verbatim from :298-299 under a :302-303 citation that correctly anchors the
fn name; finding genuine). **Confirmed fabrications: 0.**

## Blocked / UNVERIFIED carried forward
- ADIF/LPCM-DVD spec conformance (no ISO text in sandbox).
- ffprobe-side counts for ps/adts lanes (documentary only).
- Fuzz executions (nightly absent).
