# AUDIT: crates/codecs/src/video/av1/decoder.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 1043 / 18 prod + 5 tests |
| File verdict | REAL (reachable from CLI `lumen open` of an .ivf; pixel output proven against ffmpeg) |
| Reachable from shipped entry point? | YES — apps/cli/src/containers/mod.rs:55 `"ivf" => ivf::open` → crates/demux/src/ivf/mod.rs:211 `b"AV01" => Some("av1")` → apps/cli/src/pipeline_host.rs:303 `"av1" \| "av01"` arm → `Av1Decoder` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | rg/sed trace + quote verify (exit 0); baseline cites below |

## 1. One-paragraph verdict
A serious, working rav1d 1.1.0 wrapper. The hostile-focus answers: (a) it accepts EXACTLY 8-bit 4:2:0 (I420) — `av1.layout` refuses I400/I422/I444, `av1.bitdepth` refuses 10/12-bit, both typed `Unsupported` (lines 537-542); the 10-bit refusal is pinned by the passing `tenbit_fixture_is_a_typed_refusal_not_a_misdecode` differential. (b) Reachability from CLI .ivf open is YES with a fully verified chain, and the baseline proves real decoding: `containers::ivf::tests::av1_lanes_decode_the_full_stream ... ok` (cargo-test-workspace.txt:3114), `av1_differential` 6/6 incl. `differential_pixel_md5_against_ffmpeg_dump ... ok` (baseline :989-998), `av1_conformance ... ok` (71.72s, baseline :982-986), rav1d compiled into the release lane (cargo-build-release-lumen-cli.txt:98,162). The reference-state gate (workaround for a documented rav1d 1.1.0 process abort on broken reference state) is conservative-by-design: misclassification degrades to a typed skip, never a mis-decode. Remaining defects are edge-quality, not core.

## 2. What actually works (verified)
- rg trace of the full open chain (above) + sed quote verification.
- Baseline cargo-test-workspace.txt:977-998 (av1_bench ignored-by-design, av1_conformance 1/1, av1_differential 6/6: frame count/dims/pts vs ffprobe, pixel MD5 vs ffmpeg dump, fuzz-lite survival, midstream corruption resync, 10-bit typed refusal); :3011/:3114 CLI ivf AV1 lanes decode full stream.
- obu.rs cross-read: `has_independent_frame` = frame_type 0|2 && !show_existing (obu.rs:315-321) — the gate premise holds.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/video/av1/decoder.rs:666-670
Code:
```
    let matrix = match seq.mtrx {
        DAV1D_MC_BT601 => ColorMatrix::Bt601,
        DAV1D_MC_BT2020_NCL | DAV1D_MC_BT2020_CL => ColorMatrix::Bt2020Ncl,
        _ => ColorMatrix::Bt709,
    };
```
What is wrong: BT.2020 constant-luminance is silently relabeled as non-constant-luminance — CL uses a different derivation (YcCbcCrc), so the renderer will apply the wrong inverse matrix and mis-color the frame. This contradicts the repo's own stated philosophy (the DNxHD decoder at dnx/decoder.rs:392-396 refuses BT.2020 CL typed "refusing rather than mis-coloring"; this module's docs say "typed, not silent"). Unknown primaries/transfer likewise collapse to Bt709 silently (lines 652-664). Why it matters: HDR/CL AV1 streams mis-color with no error. What correct looks like: typed refusal (the dnx precedent) or a CL matrix in the vocabulary. Confidence: HIGH — Verification: read.

F-002 [S3]
Where: crates/codecs/src/video/av1/decoder.rs:455-501
Code:
```
            // Bitstream damage on one frame (EGeneric/EIO/ERANGE/…):
            // rav1d latches the error for exactly one get call; the
            // next TU resyncs. Count and keep draining.
            *damage += 1;
        }
```
What is wrong: `drain_pictures`' loop has no iteration bound. Every non-EAGAIN/EINVAL/ENOMEM return increments damage and loops again; termination rests entirely on the comment's claim that rav1d latches an error for exactly one get call. This very file documents that rav1d 1.1.0 contains port bugs (the on_error abort, module docs lines 44-61); a persistent-error engine bug here is an infinite loop (and unbounded `*damage += 1` wraps in release / panics in debug at u32 overflow). What correct looks like: a per-drain iteration cap (the file already uses SEND_RETRY_CAP=64 for the same class of concern). Confidence: MEDIUM — Verification: read; engine single-latch contract not independently verified (UNVERIFIED: would require a crafted persistent-error stream).

F-003 [S4]
Where: crates/codecs/src/video/av1/decoder.rs:954
Code:
```
        unsafe { dav1d_send_data(Some(ctx), Some(NonNull::from(&mut eos))) };
```
What is wrong: the EOS send's return value is discarded — a swallowed error. If the send fails (e.g. EINVAL from a prior drain-state), the reordering-delay frames are never yielded, `drain_pictures` returns 0, and `flush` reports `Drained` — a silent frame loss at end of stream instead of a typed error. Confidence: LOW — Verification: read.

F-004 [S4]
Where: crates/codecs/src/video/av1/decoder.rs:550-561
Code:
```
        // The default allocator only produces positive strides
        // (aligned width + optional 64-byte pad); a negative stride
        // would need bottom-up plane walks — refused typed, never
        // misread.
        if y_stride <= 0 || uv_stride <= 0 {
```
What is wrong: the check verifies strides are positive but never that `stride >= w`/`stride >= cw`; `copy_plane` then reads `w` bytes per row under the SAFETY comment's assertion "`row_bytes ≤ stride`" — asserted, not enforced. An engine allocator bug becomes an out-of-bounds read (UB) instead of a typed refusal. Confidence: LOW — Verification: read.

F-005 [S4]
Where: crates/codecs/src/video/av1/decoder.rs:565 + 754
Code:
```
        let len = w * h + 2 * cw * ch;
```
What is wrong: no frame-size cap before `vec![0; len]` (line 754). A crafted sequence header declaring 65536×65536 makes the engine allocate ~6.4 GB and the wrapper immediately doubles peak with its own 6.4 GB copy (plus up to 4 pooled retained slots). The engine's own ENOMEM is handled typed, but the wrapper adds an unguarded same-order allocation on top. Confidence: MEDIUM — Verification: read (allocation sizes arithmetic).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| errno_kind / is_eagain / is_einval | 145-159 | REAL | — | platform-correct via std mapping; total (unsigned_abs) |
| Rav1dHandle::open | 180-234 | REAL | — | zeroed-settings + default_settings; typed init failure; ctx_out None arm typed |
| Rav1dHandle::get / Drop | 240-256 | REAL | — | close-exactly-once Option dance |
| Av1Decoder::new / accepts | 287-295 | REAL | — | "av1"/"av01" |
| take_damage | 299-310 | REAL | — | saturating fold |
| push_pending / pop_pending | 314-333 | REAL | — | bounded PENDING_CAP=1024; O(n) search ok |
| feed_tu | 339-399 | REAL | — | data_create null-check typed; unref on untaken buffer prevents leak |
| feed_loop | 403-436 | REAL | — | SEND_RETRY_CAP bound; EAGAIN drain-then-retry |
| drain_pictures | 440-502 | REAL | S3 | F-002 unbounded loop |
| emit_picture | 506-611 | REAL | S4 | I420/8-bit gate; stride check F-004; len F-005 |
| copy_plane | 621-636 | REAL | — | documented unsafe contract |
| picture_color | 642-682 | REAL | S3 | F-001 CL→NCL mislabel |
| engine_state_error / classify_engine_error | 687-729 | REAL | — | ENOMEM→ResourceLimit; EINVAL→BadState; else Corrupt |
| pooled_frame_slot | 735-776 | REAL | — | bounded rotation; impossible-arm typed loudly |
| Decoder::configure | 779-805 | REAL | — | re-opens engine per generation (fresh refs after seek) |
| Decoder::decode | 814-920 | REAL | — | discontinuity → dav1d_flush + gate close; OBU framing before engine; reference-gate skip typed |
| Decoder::flush | 922-967 | REAL | S4 | F-003 discarded EOS-send result |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| descriptor_gate_accepts_the_delivery_vocabulary | YES | positive+negative |
| eagain_classification_is_platform_correct | YES | derives platform errno via std, tests negated convention |
| damage_folds_into_recovered | YES | exact enum equality incl. accumulate |
| pending_map_bounded_and_lifo_tolerant | YES | cap enforced, out-of-order pop, double-pop → Missing |
Note: the in-file tests cover classification/state only; the REAL decode proof lives in tests/av1_differential.rs + av1_conformance.rs + CLI ivf tests (baseline-cited, all ok). In-file tests do NOT exercise the rav1d seam itself (no unsafe-path test here).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "rav1d's only public Rust API is its dav1d_* C-ABI surface" | decoder.rs:6-9 | REAL (registry check) | imports from rav1d::src::lib lines 112-115 resolve; release build compiles |
| "emits 8-bit 4:2:0 as I420 and refuses anything else" | decoder.rs:75-80 | REAL | lines 537-542; baseline tenbit_fixture_is_a_typed_refusal ok |
| "confirmed process abort [in rav1d 1.1.0] when a dependent frame meets broken reference state" | decoder.rs:46-49 | REAL-UNPROVEN | the gate is implemented (872-878, 909-918) and midstream_corruption_resyncs_and_recovers passes; the upstream abort itself UNVERIFIED (would require reproducing the rav1d panic) |
| "one copy per packet is the ABI's cost" | decoder.rs:86-88 | REAL | dav1d_data_create + copy_nonoverlapping at 363-378 |

## 7. Dead / duplicated / unreachable code
None. All arms reachable; residue arms (engine absent/gone) are typed defensively.

## 8. Dependencies & flags
rav1d 1.1.0, asm-off pin (per Cargo.toml comments + module docs), `codec-av1` in codecs default features → compiled in the shipped CLI (baseline build log). `unsafe_code = "deny"` re-allowed file-locally at line 93; every unsafe block carries a SAFETY comment (verified by reading each).

## 9. Open questions for the next agent
- av1/obu.rs and error.rs are separate files (other agents): the gate's honesty depends on obu.rs's 3-bit peek; spot-checked, looks correct.
- F-002's engine single-latch contract: worth a crafted persistent-error fuzz case in av1 fuzz-lite.
