# AUDIT: crates/subtitles/src/dvb/clut.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-21 |
| Lines / functions | 386 (229 production + 157 tests) / 9 production units + 6 tests |
| File verdict | UNREACHABLE (correct, self-tested color math; internal-only consumers; zero shipped-entry-point callers) |
| Reachable from shipped entry point? | **NO.** `pub(crate)` type; the only consumers are `crates/subtitles/src/dvb/mod.rs:274,532,812,824` and `crates/subtitles/src/dvb/segment.rs:249,267,285` (entry construction) — all inside the crate, itself unreachable (see dvb/mod.rs#part1 audit: no crate outside `crates/subtitles` depends on it; `fuzz/` scaffold unwired; `lumen engine-info` has no subtitle lane). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cat crates/subtitles/src/dvb/clut.rs` (full read); `sed -n '24,34p;170,177p;211,228p;369,385p;5,8p'` (quote verification); `rg -n "clut::|Clut::|ycbcr_to_rgba" crates/subtitles/src --glob '!**/clut.rs'` (consumer map — dvb only, plus pgs/palette.rs's parallel implementation); `sed -n '1,110p' crates/subtitles/src/pgs/palette.rs` (duplication comparison); `python3` hex-dump of pgs/palette.rs:38 (the apparent `#ust_use` is a display artifact of my tooling eating `[m` — the real bytes are `#[must_use = "..."]`, valid Rust; NOT a finding); `rg -n "Doc-tests subtitles|test result" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (lib 267/0 green). Arithmetic re-derivation of all five FIX constants and three color cases by hand (see F-002 verification). |

## 1. One-paragraph verdict
The color core is right: I re-derived all five FIX constants by hand (1634/401/832/2066/1192 = round(k·255/224·2¹⁰), 512 = ONE_HALF — all exact), re-computed white/black/red/blue cases through the code's expressions and they match the tests, and the Y==0-forces-transparency plus inverted-T alpha rules match EN 300 743 §7.2.4.2. Bank routing by entry depth flags matches the spec's flag semantics. But: the module's headline claim — "integer-identical to the reference decoders … so the differential oracle is bit-exact" — is unprovable in this tree (the record itself says the ffmpeg differential is NOT generatable), the `bank()` accessor hands out depth-dependent-length slices that the dvb compose path indexes unchecked (the S1 panic enabler I filed in dvb/mod.rs#part1), the crate carries a second full copy of this exact integer math in `pgs/palette.rs`, and one "test" re-implements the segment parser's packing arithmetic instead of executing it. Nothing shipped calls any of it.

## 2. What actually works (verified)
— `ycbcr_to_rgba` correctness: verified by hand-computation. White (235,128,128,T=0): luma=219×1192=261048, r=(261048+512)>>10=255 ✓. Black (16,128,128): 0 ✓. Red (81,90,240): r=(77480+183520)>>10=255, g=(77480−77434)>>10=0, b=clamp((77480−77996)>>10)=0 ✓. Blue (81,240,90): b=255, r=15 ✓. i32 headroom ≈500k, no overflow. Matches tests `black_is_opaque_white_is_white`, `red_and_blue_primaries` — green in baseline no-fail-fast run (`subtitles` lib 267/0).
— Version semantics (§7.2.4.2) and partial-redefinition-from-defaults: `from_entries` starts at `with_default_banks` then routes flagged entries — consistent with the decoder's version-skip (`dvb/mod.rs:523-528`).

## 3. Findings

### F-001 [S3] The studio-range conversion core exists twice in the crate (this file and `pgs/palette.rs`)
Where: crates/subtitles/src/dvb/clut.rs:197-228 vs crates/subtitles/src/pgs/palette.rs:51-92
Code (clut.rs:197-203):
```rust
/// FIX() constants, SCALEBITS = 10 (see module docs).
const FIX_R_CR: i32 = 1634; // round(1.40200 * 255/224 * 1024)
const FIX_G_CB: i32 = 401; // round(0.34414 * 255/224 * 1024)
const FIX_G_CR: i32 = 832; // round(0.71414 * 255/224 * 1024)
const FIX_B_CB: i32 = 2066; // round(1.77200 * 255/224 * 1024)
const FIX_Y: i32 = 1192; // round(255/219 * 1024)
```
Code (pgs/palette.rs:51-56, verbatim constants):
```rust
const FIX_R_CR_601: i32 = 1634; // FIX(1.40200 * 255/224)
const FIX_G_CB_601: i32 = 401; // FIX(0.34414 * 255/224)
const FIX_G_CR_601: i32 = 832; // FIX(0.71414 * 255/224)
const FIX_B_CB_601: i32 = 2066; // FIX(1.77200 * 255/224)
```
What is wrong: the BT.601 constants, `clamp255`, `ONE_HALF`, and the add/shift expression shape are duplicated verbatim across the two modules (pgs adds a `ColorMatrix` parameter around the same core; its comment admits "BT.601 column identical to `dvb/clut.rs`"). A future constant correction (e.g., a rounding fix found against a real corpus) must be applied in two places with no compile-time link; the DVB and PGS paths can silently diverge.
Why it matters: this is the crate's *only* color-accuracy guarantee; divergence between the two copies would produce differently-colored subtitles per format with no test tying them together (no test asserts both implementations agree).
What correct looks like: one shared `color` module with the studio-range core parameterized by matrix (exactly pgs's `ColorMatrix` shape), both formats calling it; a cross-implementation equality test.
Confidence: HIGH — Verification: read both files; constants compared.
### F-002 [S4] `bank()` returns depth-dependent-length slices — the API that enables the unchecked-index panic downstream
Where: crates/subtitles/src/dvb/clut.rs:170-177
Code:
```rust
    /// The bank for a region depth (2 => 4 entries, 4 => 16, 8 => 256).
    pub fn bank(&self, depth: u8) -> &[Rgba] {
        match depth {
            2 => &self.bank4,
            4 => &self.bank16,
            _ => &self.bank256,
        }
    }
```
What is wrong: the accessor is correct for depth ∈ {2,4,8} (the only values `RegionSegment::depth()` produces), but it invites the exact failure mode that materializes in `dvb/mod.rs:826-829`: a caller holding indices painted at one depth and a bank for another depth gets a 4- or 16-entry slice indexed unchecked with values up to 255 → panic (my W09-A07 part1 finding F-001, S1). Also: any depth not 2/4 silently maps to bank256 — a future depth value would be quietly wrong rather than rejected.
Why it matters: the panic is enabled by an API whose length is data-dependent; a checked accessor (`bank_get(depth, index) -> Rgba`) or 256-wide banks would make the whole class impossible.
What correct looks like: `pub fn color(&self, depth: u8, index: u8) -> Rgba` masking/clamping per depth, or `bank` returning `Option<&[Rgba]>` for unknown depths.
Confidence: HIGH — Verification: read; downstream panic traced in dvb/mod.rs#part1 F-001.
### F-003 [S4] Out-of-range entry ids for the 2/4-bit banks are dropped silently
Where: crates/subtitles/src/dvb/clut.rs:47-55
Code:
```rust
            if e.depth_2 && e.entry_id < 4 {
                clut.bank4[e.entry_id as usize] = rgba;
            }
            if e.depth_4 && e.entry_id < 16 {
                clut.bank16[e.entry_id as usize] = rgba;
            }
```
What is wrong: a CLUT entry flagged 2-bit with `entry_id` ≥ 4 (or 4-bit with id ≥ 16) is silently discarded — no damage plumbing exists at this layer (the ledger lives in the decoder, and `from_entries` has no sink parameter). A hostile or buggy stream loses entries with zero diagnostics, against the module family's "every malformation is typed data" contract.
Why it matters: limited blast radius (such entries are meaningless per spec), but the crate's diagnostic discipline stops at this function boundary with no documented note.
What correct looks like: pass a `&mut DamageSink`-style sink (the DVB decoder already owns one) or return a count of dropped entries for the caller to type.
Confidence: HIGH — Verification: read.
### F-004 [S4] `limited_range_entries_unpack` re-implements the segment parser's packing math instead of executing it
Where: crates/subtitles/src/dvb/clut.rs:369-385
Code:
```rust
        let b0 = 0b1010_0110u8;
        let b1 = 0b1101_0011u8;
        let b2 = 0b0110_1001u8;
        let y = b0 & 0xFC;
        let cr = (((b0 & 0x03) << 4) | (b1 >> 4)) << 2;
        let cb = (((b1 & 0x0F) << 2) | (b2 >> 6)) << 2;
        let alpha = ((b2 >> 4) & 0x03) << 6;
```
What is wrong: these lines are a verbatim copy of the unpacking expressions in `segment.rs:282-295` (`ClutSegment::entries`'s limited-range branch). The test asserts the copy equals hand-computed constants — it never calls the real unpacker, so a regression in `segment.rs`'s actual code cannot be caught here (if both drifted identically the test still passes; if only segment.rs drifted this test is blind). The genuine coverage lives in `segment.rs`'s own test `clut_entries_full_and_limited` (which asserts the same constants through `entries()`).
Why it matters: a test that duplicates the algorithm instead of exercising it inflates confidence without adding protection — exactly the "tests asserting nothing" class the checklist targets.
What correct looks like: delete the copy and assert the constants through `ClutSegment::entries()` (already done in segment.rs), or convert this into a cross-check that the clut-side `ClutEntry` field semantics (Y==0 transparency, T inversion) hold for limited-range values end-to-end.
Confidence: HIGH — Verification: read both sites; expressions compared token by token.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct Clut` | 26-34 | REAL | — | version + three banks; `pub(crate)`. `version` pub field read by the decoder's skip logic. |
| `Clut::from_entries` | 39-58 | REAL | S4 (F-003) | Defaults-first partial redefinition; flagged-bank routing per §7.2.4.2 (0x80/0x40/0x20) — matches the reference's flag semantics; bank256 indexing always in range (u8 id). |
| `Clut::default_banks` | 62-64 | REAL | — | Constructor alias for the `BadClutId` fallback — wired at `dvb/mod.rs:812`. |
| `Clut::with_default_banks` | 66-168 | REAL | — | ffmpeg `default_clut` transcription: bank4 (transparent/black/white/gray), bank16 (RGB combos 255/127), bank256 (bit-composed 85/170/127/43 levels + 0x88 alpha/level cases). Shape verified against the in-file tests (bank16[7]=white, bank16[8]=black, bank256[1]=red-a63); ffmpeg-identity itself UNPROVEN (see §6). |
| `Clut::bank` | 171-177 | REAL | S4 (F-002) | Depth→slice; enables the downstream unchecked-index panic; `_` arm swallows unknown depths into bank256. |
| `struct ClutEntry` | 181-195 | REAL | — | Pre-extraction data; field order (y, cb, cr) differs from segment.rs's byte order (y, cr, cb) — safe because segment.rs constructs with named fields (verified: `segment.rs:267-296`). |
| `FIX_*` consts + `ONE_HALF` | 197-203 | REAL | S3 (F-001) | Hand-verified exact; duplicated in pgs/palette.rs. |
| `clamp255` | 205-208 | REAL | — | i32 clamp→u8. Duplicated in pgs/palette.rs (F-001). |
| `ycbcr_to_rgba` | 211-228 | REAL | — | Y==0→transparent, T inverted (a=255−t), BT.601 headroom; arithmetic re-derived by hand (see §2); no overflow (max ≈5×10⁵ « i32::MAX). |

## 5. Test honesty (6 tests, lines 234-385)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `black_is_opaque_white_is_white` | Yes | Exact RGBA asserts at studio-swing endpoints. |
| `y_zero_is_transparent_even_when_t_says_opaque` | Yes | Pins the §7.2.4.2 Y==0 rule. |
| `alpha_is_inverted_and_clamped` | Yes | T=255→a=0, T=128→a=127. |
| `red_and_blue_primaries` | Yes | Channel isolation with Cb/Cr swapped — would catch a Cb/Cr transposition (the classic bug this format invites). |
| `entry_depth_flags_route_to_their_banks` | Yes | Three flag combinations; asserts banks hit AND untouched (negative asserts). Strong. |
| `default_banks_match_reference_shape` | Partially | Asserts four entries of ffmpeg's table; the other ~260 entries of bank256/bank16 are unasserted — a transcription typo in an untested slot (e.g. bank256[0x44]) would pass everything. |
| `limited_range_entries_unpack` | **No** | F-004: re-implements segment.rs's math; cannot fail when the real unpacker breaks. |
| Coverage gaps (mutation reasoning) | — | No test covers: entry_id ≥ 4 with depth_2 flag (F-003 — silent drop passes); bank() with depth ∉ {2,4,8} (F-002 `_` arm); cross-agreement with pgs/palette.rs's copy (F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The conversion is the ITU-R BT.601 headroom matrix with the SAME integer arithmetic the reference decoders use (ffmpeg YUV_TO_RGB*_CCIR, SCALEBITS=10), so the differential oracle is bit-exact" | clut.rs:5-8 | REAL-UNPROVEN | The math is verified correct by hand (§2) and the constants are the canonical BT.601 headroom set — but "same as ffmpeg" is asserted, not proven: no ffmpeg source in sandbox, and the record itself states the differential is NOT generatable (sub-dvb.md:57-61 "cross-checked line-by-line" is a human claim I cannot re-run). |
| "the default banks (the `BadClutId` fallback)" = "ffmpeg's `default_clut`" | clut.rs:60-62 | REAL-UNPROVEN | Same basis: shape plausible, four slots test-asserted, full-table identity unprovable here. |
| "CLUTs: versioned banks, BT.601-headroom YCbCr→RGBA conversion (integer-identical to the reference decoders), 2/4/8-bit default banks as the BadClutId fallback" | sub-dvb.md:32-34; matrix row `subtitle-dvb-subs` evidence | REAL-UNPROVEN (same caveat) | See above; the fallback wiring itself is REAL (`dvb/mod.rs:805-815`, tested `bad_clut_id_uses_default_banks`). |
| "integer-identical to the reference decoders" as *evidence* for matrix status "implemented" | feature-matrix.toml:1007 | UNREACHABLE + REAL-UNPROVEN | Crate-level implementation real; no shipped consumer (Reachability row); the identity claim is the unproven part. |

## 7. Dead / duplicated / unreachable code
- Duplicated: the entire conversion core vs `pgs/palette.rs` (F-001).
- Unreachable: whole module from shipped entry points.

## 8. Dependencies & flags
- Depends only on `crate::compositing::target::Rgba` (in-crate). No features, no unsafe, no `#[allow]`. Clippy-clean per baseline #5 (the `#[must_use = …]` attribute at pgs/palette.rs:38 displays as `#ust_use` through my tooling — hex-verified as valid `#[must_use`; noted so no later agent "finds" the same artifact and files it).

## 9. Open questions for the next agent
- To the `pgs/palette.rs` owner: mirror F-001 (your file is the second copy); also your `Palette::from_entries` resets unmentioned entries to *transparent black* while DVB's `Clut::from_entries` resets to *default banks* — both documented as reference behavior; if a corpus lane ever lands, both claims can be checked in one pass.
- If the orchestrator ever gets a real `.ts`+DVB corpus lane (sub-dvb.md's next step 1), the first thing to diff is `with_default_banks`'s unasserted bank256 slots and the ffmpeg-identity of the FIX constants — the two REAL-UNPROVEN claims above.
- Cross-ref: the S1 panic in dvb/mod.rs#part1 F-001 is *enabled* by this file's F-002 API — fixing either end kills the panic; fixing both is correct.
