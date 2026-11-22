# AUDIT: apps/cli/src/containers/aiff.rs

| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-22 |
| Lines / functions | 452 / 10 (2 production + 8 test fns/helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `main.rs` → `run::run` → `lumen_dispatch` → `CliHost::open_session` → `open_source` → `containers::open` (mod.rs:53: `"aiff" => aiff::open(stream, opts)`). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:3 |

Commands run:
- `sed -n '103,109p' apps/cli/src/containers/aiff.rs` (quote verification, exit 0)
- `ls tests/fixtures/generated/valid | rg -i "aiff|aifc"` → all 4 valid fixtures exist; `ls tests/fixtures/generated/malformed | rg -i aiff` → 6 derivatives exist (flip1/4/16, garbage0, trunc0-4)
- `rg -n '"aiff" =>' apps/cli/src/containers/mod.rs` → line 53

## 1. One-paragraph verdict

The best-behaved container host in the directory: PCM-only vocabulary (every AIFC compression form outside it refuses typed at the demuxer before the harvest exists), a truthful `pcm` pair (the one family — with WAV — where that field means what its doc says), a clamped `--start-time` against the declared extent, and a test suite with ZERO corpus gating — all five tests run unconditionally over committed fixtures or synthesized bytes, including a hand-built `ima4` refusal, an 8-bit-signed refusal, and a committed truncated-COMM fixture. The findings are trivia: an invariant-value recomputed per loop iteration, the discarded pure-seek landing shared with every sibling, and the fifth copy of the `CountingSink`/`decode_frames` test harness in this directory.

## 2. What actually works (verified)

- Wiring (mod.rs:53); all cited fixtures and malformed derivatives exist on disk.
- The decode differential is fixture-based, not corpus-based (module docs 234-236 explicitly record "No corpus lanes exist for this family … these fixtures ARE the differential") — so unlike es/adts/wav, this file's ffmpeg-truth claims (8,820/9,600/8,820/8,820 frames) are backed by committed bytes present in this workspace.
- Both typed-refusal tests assert stage AND error code (`demux.aiff.unsupported-compression`, `demux.aiff.truncated`).
- Not executed (no cargo): assertions read and found specific; W00 baseline corroborates the suite runs.

## 3. Findings

### F-001 [S4]
Where: apps/cli/src/containers/aiff.rs:105-109
Code:
```rust
    for track in tracks {
        // The pair the provider's `with_format` seam consumes; the
        // codec descriptor is the demuxer's own track vocabulary
        // (`pcm_s16be`, `pcm_s16le`, `pcm_u8`, `pcm_f32be`, …).
        let pcm = comm.map(|comm| (u32::from(comm.channels), comm.sample_rate));
```
What is wrong: the `pcm` pair is loop-invariant but recomputed inside the per-track loop (an AIFF has exactly one track; if that ever changed, the recompute would still be identical). Also: if `comm()` answered `None` after a successful `start()`, the fact would carry codec `pcm_s16be` with NO pcm pair — the decoder's `with_format` would then refuse typed (`pcm facts absent`), an honest late failure rather than an open-stage one. Defensive-None class again; believed unreachable.
Confidence: HIGH — Verification: read.

### F-002 [S4]
Where: apps/cli/src/containers/aiff.rs:74
Code:
```rust
        demuxer.seek(target).map_err(|err| wire_error(&err))?;
```
What is wrong: the pure-seek landing is discarded (validation only) — the identical harmless-but-misleading pattern as ps.rs:164, adts.rs:87, wav.rs:68. Fourth instance; the clamp at line 61 is the real bound and is asserted.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: apps/cli/src/containers/aiff.rs:153-204 (vs ps.rs, es.rs, adts.rs, wav.rs)
Code:
```rust
    /// The audio sink: counts decoded **sample frames** (the ffmpeg
    /// differential's unit — one `Frame::Audio` carries a whole
    /// buffer's worth; `frames_in` is the honest count).
    struct CountingSink(u64);
```
What is wrong: the FIFTH near-identical copy of `CountingSink` + `decode_frames`/`open` test harness in `apps/cli/src/containers/` (ps, es, adts, wav, aiff). Test-only code, so S4 per file — but the aggregate is a real duplication debt: any change to the provider's test seam touches five files.
Confidence: HIGH — Verification: read + grep across the directory.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 51-94 | REAL | S4 | clamp-then-validate; watermark positioning; discarded seek landing (F-002) |
| `harvest_facts` | 102-128 | REAL | S4 | PCM pair truthful (codec vocabulary guarantees it); loop-invariant recompute + defensive-None (F-001) |
| test `fixture` | 144-148 | REAL | — | |
| test `CountingSink::accept` | 154-163 | REAL | S4 | fifth copy (F-003) |
| test `open` | 167-172 | REAL | — | real open path |
| test `decode_frames` | 177-204 | REAL | S4 | fifth copy (F-003) |
| test `pcm_be_fixture_opens_with_expected_facts` | 212-227 | REAL | — | facts + exact 8,820 extent |
| test `all_aiff_fixture_flavors_open_and_decode` | 238-310 | REAL | — | 4 committed fixtures, loud presence asserts, ffmpeg-truth decode counts — the full differential, no gating |
| test `unsupported_aifc_compression_refuses_typed` | 320-377 | REAL | — | hand-built ima4 AIFC + classic 8-bit; stage+code pinned both |
| test `truncated_aiff_refuses_at_the_demux_stage` | 383-396 | REAL | — | committed malformed fixture; code pinned |
| test `start_time_clamps_and_skips` | 403-450 | REAL | — | clamped past-end (0 packets) + in-media watermark |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| pcm_be_fixture… | YES | exact codec/pair/extent |
| all_aiff_fixture_flavors… | YES | 4-lane decode counts vs ffmpeg truth, committed bytes |
| unsupported_aifc… | YES | synthesized; stage + code asserted twice |
| truncated_aiff… | YES | committed derivative; code asserted |
| start_time_clamps_and_skips | YES | both watermark ends asserted |
No corpus gating in this file — every test runs everywhere the fixtures exist (they are in-tree).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every form the demuxer exposes is a PCM descriptor … the facts pair is the whole decoder-config truth" | module docs 13-19 | REAL | harvest 102-128; codec vocabulary verified by the refusal tests |
| "Everything else refuses before the harvest exists" | module docs 23-29 | REAL | unsupported-compression tests (start() refuses) |
| "No corpus lanes exist for this family … these fixtures ARE the differential" | test docs 234-236 | REAL | no corpus_dir in this file; all fixtures in-tree (verified by ls) |

## 7. Dead / duplicated / unreachable code

- The `comm() == None` branch of the harvest's `map` — believed unreachable after successful `start()` (defensive).
- Test-harness duplication: fifth copy (F-003).
- No dead production functions.

## 8. Dependencies & flags

No feature gates. Depends on `demux::aiff` (comm accessor, test_support for synthesized bytes), shared `SelectDemuxer`, `pipeline_host::HostDecoderProvider` (tests).

## 9. Open questions for the next agent

- The `extended80` sample-rate conversion (80-bit IEEE float → u32) lives in the demuxer (Wave 4) — the host trusts `comm.sample_rate` blindly; a conversion bug there flows straight into the limb geometry here.
- Why do the 6 committed malformed derivatives (flip1/4/16, garbage0, trunc0-4) have no container-host test? Only `__trunc1` is used here; the others are presumably demux-crate lanes — fine, but the host-level damage surface is tested at 1 of 6 available shapes.
