# AUDIT: crates/subtitles/src/pgs/palette.rs
| Field | Value |
|---|---|
| Auditor | W09-A01 |
| Date | 2026-11-23 |
| Lines / functions | 183 / 4 (+4 tests) |
| File verdict | REAL-UNPROVEN against its reference claim (ffmpeg provenance of the height rule + BT.709 constants not verifiable in sandbox; all arithmetic independently verified and executed) |
| Reachable from shipped entry point? | NO (crate-level: no app depends on `subtitles`). In-crate: YES — `pgs/mod.rs:439-444` (`pds_segment` → `ColorMatrix::for_video_height(self.pcs…height)` → `Palette::from_entries`) → `PgsDecoder` → `pgs::fuzz` + tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 FIX() constant verification (exit 0, all 9 exact); EXECUTED probe (audit/runs/w09a01_probe_pgs.rs, fresh rlibs) exit 0; rg callers; baseline tests ok |

## 1. One-paragraph verdict
Small, tight, correct-by-construction color math. All nine FIX() constants reproduce exactly from the documented formulas (python `int(x*1024+0.5)`: 1634/401/832/2066, 1836/218/546/2163, 1192), the BT.601 column is byte-identical to `dvb/clut.rs:198-203` as the comment claims, and I executed the public API: studio white/black round-trip to exact (255,255,255)/(0,0,0) under BOTH matrices, the 709 red lands (255,24,0), and the height rule flips at 576/577. The unit tests pin the 601 path exactly but only *plausibility*-test the 709 constants (range asserts), and the ffmpeg-provenance claim (pgssubdec.c height>576 rule, 709 constants "fetched 2026-10-25") is unverifiable here — no ffmpeg source in the sandbox and the CLI cannot render PGS bitmaps for a differential; binary disassembly was inconclusive. The whole crate remains unreachable from shipped entry points.

## 2. What actually works (verified)
- python3 constant check (all exact):
```
601: 1634 401 832 2066
709: 1836 218 546 2163
Y : 1192
```
- EXECUTED probe output (exit 0):
```
Bt601 studio-white -> r=255 g=255 b=255 a=255
Bt709 studio-white -> r=255 g=255 b=255 a=255
Bt709 red(81,90,240) -> r=255 g=24 b=0 a=255
Bt601 studio-black -> r=0 g=0 b=0 a=255
height rule: 576=Bt601 577=Bt709 1080=Bt709
```
- Baseline: `pgs::palette` tests pass (cargo-test-workspace.txt PACKAGE subtitles, all ok).
- Hand-derivation cross-check: 709 red g = ((81−16)·1192 + (−218·(90−128) − 546·(240−128) + 512)) >> 10 = (77480 − 52356) >> 10 = 24 ✓ matches probe.

## 3. Findings
F-001 [S4]
Where: crates/subtitles/src/pgs/palette.rs:138-146
Code:
```
    fn bt709_primaries_are_plausible() {
        // Studio red: Y=81 Cb=90 Cr=240.
        let c = ycbcr_to_rgba(81, 90, 240, 255, ColorMatrix::Bt709);
        assert!(c.r > 200 && c.g < 60 && c.b < 60, "red, got {c:?}");
```
What is wrong: the BT.709 constants (1836/218/546/2163) are only range-tested; a transcription slip (e.g. 546→546±20) passes. The 601 column is exact-pinned (white/black round-trips) but no exact-vector test exists for 709 (my probe's (255,24,0) is exactly the kind of golden the suite lacks). Why it matters: 709 is the matrix actual Blu-ray content (1080p) uses — the common path is the weakly-pinned one. What correct looks like: exact asserts for 2-3 709 vectors. Confidence: HIGH — Verification: read + ran (probe values would catch a ±20 slip: g=24 vs threshold 60).

F-002 [S4]
Where: crates/subtitles/src/pgs/mod.rs:438-441 (the caller's rule for this file's matrix choice)
Code:
```
        let matrix = palette::ColorMatrix::for_video_height(
            self.pcs.as_ref().map_or(0, |p| u32::from(p.height)),
        );
```
What is wrong: a PDS arriving before any PCS converts with height 0 → BT.601 even for 1080p content. The reference keys off `avctx->height` (context, possibly already set) — divergence possible only for out-of-spec segment order (per HDMV the epoch's first segment is a PCS), so blast radius ~zero; noted for honesty since the module doc sells "the reference decoder's rule". Confidence: MEDIUM — Verification: read; ffmpeg side UNVERIFIED (no source).

Zero S0-S3: checked — no overflow (max |i32| term ≈ 5·10^5), no panics (`e.id as usize` ≤ 255; clamp before cast), palette replace semantics (full redefinition, unmentioned → transparent black) matches the documented `av_mallocz` behavior, duplicate entry ids → last wins (benign), `from_entries` infallible, no allocation beyond the fixed 256-entry array.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ColorMatrix::for_video_height` | 39-45 | REAL | — | executed: 576/577/1080 flip ✓ |
| `clamp255` | 62-65 | REAL | — | |
| `ycbcr_to_rgba` | 68-90 | REAL | F-001 (test) | executed exact white/black/red; arithmetic-shift on negatives = floor (matches C semantics) |
| `Palette::from_entries` | 103-118 | REAL | — | executed; zero-fill + overwrite ✓ |
| 4 tests | 125-182 | REAL | F-001 | 601 exact, 709 range-only |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| studio_white_and_black_round_trip_601 | YES | exact (255,255,255,255)/(0,0,0,255) |
| bt709_primaries_are_plausible | WEAKLY | ranges only — a wrong 709 constant within ±thresholds passes (F-001) |
| matrices_diverge_exactly_as_the_constants_say | YES (for the divergence fact) | asserts c601 != c709 + exact height-rule boundaries |
| palette_from_entries_replaces_and_defaults | YES | exact entry + untouched transparent-black slots |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| FIX() constants as listed; SCALEBITS=10 | L9, L51-59 | REAL | python reproduction, all 9 exact |
| "BT.601 column identical to dvb/clut.rs (same ffmpeg macros)" | L48-50 | REAL | dvb/clut.rs:198-203 byte-identical |
| ffmpeg rule: BT.709 when video taller than 576, else BT.601 | L5-8 | REAL-UNPROVEN | internal behavior executed ✓; ffmpeg's own rule UNVERIFIED (no source; CLI can't render PGS; objdump inconclusive) |
| 709 constants from colorspace.h / pgssubdec, "fetched 2026-10-25" | L4-9 | REAL-UNPROVEN | values are exact FIX() of the ITU-R BT.709 coefficients (python ✓); ffmpeg-source attribution unverifiable |
| PGS alpha straight, no Y==0 transparency rule | L18-19 | REAL | code: `a: alpha` verbatim (contrast dvb path) |
| PDS same id+version no-op; new version replaces all (unmentioned → transparent black) | L20-23 | REAL (module side) | pgs/mod.rs:428-430 no-op guard; from_entries zero-fill (executed) |

## 7. Dead / duplicated / unreachable code
No dead code. Shipped-entry unreachability is crate-level (see parser.rs audit). The 601 constants duplicate dvb/clut.rs deliberately (documented shared-macro origin).

## 8. Dependencies & flags
`crate::compositing::target::Rgba`, `crate::pgs::segment::PaletteEntry`, `crate::pgs::TRANSPARENT_BLACK` — all in-crate. No features, no unsafe, std-only.

## 9. Open questions for the next agent
- pgs/mod.rs owner: confirm the PDS-before-PCS → BT.601 edge (F-002) against the real reference (needs ffmpeg source or a real .sup with reordered segments).
- The `matrices_diverge` test name overpromises ("exactly as the constants say" — it only checks inequality).
