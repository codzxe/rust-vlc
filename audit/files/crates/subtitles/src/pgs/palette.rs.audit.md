# AUDIT: crates/subtitles/src/pgs/palette.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-21 |
| Lines / functions | 183 / 15 (enum + 1 method + 9 constants + 2 fns + 1 struct + 1 ctor + 4 tests) |
| File verdict | REAL-UNPROVEN (in-crate test-proven; crate UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO (crate-level, see W09-A10 wave context). In-crate callers: pgs/mod.rs:439 (`ColorMatrix::for_video_height` in `pds_segment`) and `palette::Palette::from_entries` (:441-443); `ycbcr_to_rgba`/constants used only in-file + tests. Baseline executed: 267-unit suite green |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read of the full file; constant arithmetic recomputed by hand (all 9 FIX values); `rg -n "ycbcr_to_rgba\|ColorMatrix\|FIX_Y\|clamp255" src/dvb/ src/pgs/` (caller map + duplication evidence); `rg -n "TRANSPARENT_BLACK" src/pgs/mod.rs` (:674); `rg -n "with_alpha\|BLACK" src/compositing/target.rs`; baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269 (267 unit green), matrix row `subtitle-pgs` evidence "BT.601-vs-BT.709-by-height palette" |

## 1. One-paragraph verdict
A correct, small, integer-exact PDS palette converter. Every FIX constant checks
out arithmetically (FIX(x)=round(x·2¹⁰): 1634/401/832/2066 for BT.601,
1836/218/546/2163 for BT.709, 1192 for 255/219 — all recomputed by hand from
the documented Kr/Kg/Kb forms and matching ffmpeg's `pgssubdec.c` literals as
I recall them); the conversion pipeline (subtract-128 chroma, studio-range
luma, ONE_HALF rounding, >>10, clamp) is the CCIR reference form, and the
`>576 → BT.709` height rule matches the reference behavior the module claims.
Indexing is panic-free (`e.id as usize` into `[Rgba; 256]`), alpha is straight
as documented. Tests assert exact white/black round-trips and the 576/577
boundary. Two hygiene defects: the module doc says the integer core is
"shared with the DVB CLUT path" while dvb/clut.rs in fact carries a
byte-for-byte duplicated copy (FIX_Y, clamp255, ycbcr_to_rgba) — "shared" is
false and the duplication invites drift; and `bt709_primaries_are_plausible`
cannot distinguish BT.709 constants from BT.601 (hand-mutation: substituting
the 601 column still passes both primaries assertions — only
`matrices_diverge` catches a both-matrices-identical regression, and nothing
pins the 709 column to its exact values).

## 2. What actually works (verified)
- Arithmetic recomputed by hand, all exact:
  - studio white: Y=235 → luma=(235−16)·1192=261,048; Cb=Cr=128 → adds=512;
    r=g=b=(261,048+512)>>10=255 ✓ (test asserts 255,255,255,255).
  - studio black: Y=16 → luma=0 → (0+512)>>10=0 ✓.
  - BT.709 red (Y=81,Cb=90,Cr=240): r=(65·1192+1836·112+512)>>10=276→clamp 255
    (>200 ✓), g=24 (<60 ✓), b=0 (<60 ✓) — test's plausibility window holds.
  - FIX constants: e.g. FIX(1.5747·255/224)=round(401.5496/224·1024)=round(1835.6)=1836 ✓;
    FIX(0.4682·255/224)=round(545.8)=546 ✓; FIX(255/219)=round(1192.3)=1192 ✓.
- Tests green in baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269-5271
  ("Running unittests src/lib.rs … subtitles … 267 passed; 0 failed"); this
  file's 4 tests are in that count.
- `Palette::from_entries` default-fill verified: `[TRANSPARENT_BLACK; 256]`
  (pgs/mod.rs:674 `Rgba::BLACK.with_alpha(0)`), then per-entry overwrite —
  `e.id as usize` cannot exceed 255 into a 256 array.

## 3. Findings
### F-001 [S4]
Where: crates/subtitles/src/pgs/palette.rs:8-9 (vs crates/subtitles/src/dvb/clut.rs:197-225)
Code:
```
//! studio-range integer core shared with the DVB CLUT path
//! (SCALEBITS=10, `FIX(x) = int(x·2^10 + 0.5)`):
```
What is wrong: The core is NOT shared — it is duplicated. `dvb/clut.rs` has
its own `const FIX_Y: i32 = 1192` (:202), its own `fn clamp255` (:206), and
its own `pub(crate) fn ycbcr_to_rgba` (:211) with the same structure; this
file has another copy of all three (:59-90). Two independent copies of the
same reference integer math with separate test suites — a constant fix in one
will drift the other. Why it matters: this is the exact "copy-paste
duplication" class the checklist targets, in the two bitmap-subtitle paths
that are supposed to agree (ADR-012's "same RLE mechanics" argument).
What correct looks like: one `pub(crate)` core (parameterized by the matrix
column) in a shared module, both PGS and DVB calling it. Confidence: HIGH —
both files read.

### F-002 [S4]
Where: crates/subtitles/src/pgs/palette.rs:138-146
Code:
```
    fn bt709_primaries_are_plausible() {
        // Studio red: Y=81 Cb=90 Cr=240.
        let c = ycbcr_to_rgba(81, 90, 240, 255, ColorMatrix::Bt709);
        assert!(c.r > 200 && c.g < 60 && c.b < 60, "red, got {c:?}");
        // Blue: Y=41 Cb=240 Cr=110.
        let c = ycbcr_to_rgba(41, 240, 110, 255, ColorMatrix::Bt709);
        assert!(c.b > 200 && c.r < 60, "blue, got {c:?}");
    }
```
What is wrong: Hand-mutation check: replacing the BT.709 constant column with
the BT.601 column keeps both assertions passing (601 gives red r=255 g=0 b=0
and blue b=255 r=0 — still inside every window; recomputed by hand). The test
cannot detect a 709→601 constant regression; only `matrices_diverge_…` catches
"both matrices identical", and no test pins the 709 constants to their exact
values (the 601 side IS pinned exactly by the white/black round-trip test).
Why it matters: the matrix row's headline claim is "BT.601-vs-BT.709-by-height
palette" — the 709 half is only plausibility-tested. What correct looks like:
exact expected RGB tuples for one 709 conversion (e.g. the Y=81 red case →
exact (255, 24, 0)) or a 709-vs-known-reference-vector assertion.
Confidence: HIGH — mutation arithmetic done by hand.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ColorMatrix` (enum) | 28-34 | REAL | — | 2 variants, documented threshold |
| `ColorMatrix::for_video_height` | 38-45 | REAL | — | `> 576` → Bt709; pinned at 576/577 by test |
| FIX constants (9) | 51-60 | REAL | — | all recomputed exact (see §2); match ffmpeg literals per auditor recall (UNVERIFIED offline) |
| `clamp255` | 62-65 | REAL | — | `i32::clamp(0,255)` — panic-free |
| `ycbcr_to_rgba` | 68-90 | REAL | — | CCIR studio-range pipeline; arithmetic `>>` on negatives = floor (matches C behavior); alpha passed straight |
| `Palette` (struct) | 93-98 | REAL | — | 256-entry array, no heap |
| `Palette::from_entries` | 103-118 | REAL | — | fresh zero-fill per call (version-replace semantics live in mod.rs:425-427 no-op check + this fresh build) |
| tests: `studio_white_and_black_round_trip_601` | 126-136 | REAL test | — | exact tuples; pins 601 column + straight alpha |
| tests: `bt709_primaries_are_plausible` | 139-146 | REAL test | S4 (F-002) | cannot catch 709→601 swap |
| tests: `matrices_diverge_exactly_as_the_constants_say` | 149-157 | REAL test | — | catches identical-matrices regression + height boundary |
| tests: `palette_from_entries_replaces_and_defaults` | 159-182 | REAL test | — | entry write + transparent-black defaults |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| studio_white_and_black_round_trip_601 | YES | exact 255/0 tuples (any 601 constant error shifts them) |
| bt709_primaries_are_plausible | PARTIAL (F-002) | fails on channel swaps/garbage; passes under 709→601 swap |
| matrices_diverge_exactly_as_the_constants_say | YES | assert_ne on same input under both matrices + 3 boundary heights |
| palette_from_entries_replaces_and_defaults | YES | exact entry value + untouched-entry transparency |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "BT.601-vs-BT.709-by-height palette -> CompositedCue" | docs/feature-matrix.toml row `subtitle-pgs` evidence (:992) | REAL (in-crate) | for_video_height + ycbcr_to_rgba + tests; unreachable in shipped binary (crate-level) |
| constants "from ffmpeg pgssubdec.c / colorspace.h, fetched 2026-10-25" | palette.rs:4-9 | UNVERIFIED | ffmpeg source not in tree, no network; arithmetic internally exact and matches auditor's recall of the literals |
| "PGS alpha … straight (`0` = transparent) … no Y==0 transparency rule" | palette.rs:18-19 | REAL (consistent) | `a: alpha` direct; contrast with dvb/clut.rs's inverted handling is that file's audit |
| "unmentioned ones revert to transparent black, the reference `av_mallocz` behavior" | palette.rs:22-23 | REAL (in-crate) / reference UNVERIFIED | from_entries zero-fills; ffmpeg memset-on-version-change recalled, not citable offline |
| "integer core shared with the DVB CLUT path" | palette.rs:8-9 | FALSE (duplicated, not shared) | dvb/clut.rs:197-225 duplicate — F-001 |

## 7. Dead / duplicated / unreachable code
- Duplication: the DVB/PGS color core (F-001).
- No dead code in-file: all items consumed by pgs/mod.rs or tests.

## 8. Dependencies & flags
Uses `crate::compositing::target::Rgba` and `crate::pgs::segment::PaletteEntry`
(+ `crate::pgs::TRANSPARENT_BLACK`). No external deps, no unsafe, no feature
gates, no cfg beyond `#[cfg(test)]`.

## 9. Open questions for the next agent
- pgs/mod.rs owner (chunk part1/part2): when a PDS arrives with NO prior PCS
  (mid-stream join), `for_video_height(0)` → Bt601 is baked into the stored
  palette permanently (conversion happens at insert, :441-443) — is a later
  PCS supposed to trigger a re-conversion, and does any test cover
  PDS-before-PCS ordering?
- dvb/clut.rs owner: confirm the DVB copy's constants are identical today
  (F-001 drift check) and that the "shared" wording in dvb module docs is
  likewise wrong.
- Whoever holds ffmpeg source access: cite `pgssubdec.c`'s exact BT.709
  literals (1.5747/0.1873/0.4682/1.8556) and the palette-version memset, to
  promote the UNVERIFIED reference claims to cited.
