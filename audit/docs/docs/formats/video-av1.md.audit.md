# CLAIM AUDIT: docs/formats/video-av1.md
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 8 / 3 / 0 / 0 / 0 / 1 / 0 |
| Internal contradictions found | 1 (fixture path) |

## 1. Blunt verdict
A dense, honest codec record whose code anchors all exist and match the
pinned dependency spec character-for-character: rav1d `=1.1.0` with
`default-features = false, features = ["bitdepth_8", "bitdepth_16"]`
(crates/codecs/Cargo.toml:125 — exact), the self-built OBU validator
(av1/obu.rs), IVF demux (demux/src/ivf/), fuzz-lite (:500), the 15/18
corruption-survival claim pinned as a code comment (av1_differential.rs:419),
the three pixel-MD5s pinned at :67/:73/:79, and the matrix row
codec-video-av1 now `implemented` ×3 citing this record — the flip this
record was written to justify. The record's speed numbers cross-cite
consistently (0.29× → docs/perf/breadth.md:102 "0.292×"). One real defect:
the claimed fixture location `tests/fixtures/av1/` does not exist — the
fixtures live at `crates/codecs/tests/fixtures/av1/`.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "status = implemented (8-bit 4:2:0 software decode via rav1d 1.1.0 asm-off)" | front-matter | REAL | crates/codecs/Cargo.toml:125 `rav1d = { version = "=1.1.0", optional = true, default-features = false, features = ["bitdepth_8", "bitdepth_16"] }` — spec matches verbatim; crates/codecs/src/video/av1/{decoder.rs,obu.rs,mod.rs,fuzz.rs} present. |
| 2 | "crates = [rav1d 1.1.0 … asm OFF]" + "the repo's second sanctioned unsafe exception (rav1d's … C ABI)" | :40-47 | REAL | rav1d pin verified above; `codec-av1 = ["dep:rav1d"]` (Cargo.toml:66); asm feature not enabled. |
| 3 | "IVF (`demux::ivf`, 5.I3b) is landed — AV01 fourcc … byte-for-byte passthrough" | :48-51 | REAL | `ls crates/demux/src/` -> `ivf/` present. |
| 4 | "fuzz-lite … lives in `crates/codecs/tests/av1_differential.rs::fuzz_lite_*`" | :13-15 | REAL | `rg -n 'fn fuzz_lite' av1_differential.rs` -> :500 `fn fuzz_lite_framing_and_decode_survival()`. |
| 5 | "5.E1 differential proves 15/18 frames survive a mid-stream corruption" | :57-58 | REAL | av1_differential.rs:419 `// Net: 15 of 18 frames survive (0-8 and 12-17), exactly one` — the claim is pinned as executable-test commentary. |
| 6 | "pixel MD5 bit-exact … for the three 8-bit fixtures (`275082…` / `fbedc17b…` / `f153f1d…`)" | :231-234 | REAL | The same three MD5 strings are pinned in av1_differential.rs:67,:73,:79. |
| 7 | "The always-on committed surface (5.E1, `tests/fixtures/av1/`, ≈39 KB): 5 fixtures with pinned pixel-MD5s" | :116-118 | FALSE (path) | `ls tests/fixtures/av1` -> ABSENT; the 5 named .ivf fixtures (testsrc2_320x240_60f, smpte_160x128_18f, rgb_192x108_6f, tenbit_128x96_5f, testsrc2_g6_192x108_18f) live in `crates/codecs/tests/fixtures/av1/`. Fixture set itself REAL; location claim wrong. |
| 8 | Matrix row is the flip target: "whose row reads `status = not_started` ×4 — this record is the evidence for the Lead's matrix pass" | :7-11 | REAL (flip landed) | docs/feature-matrix.toml:459+ `codec-video-av1` now `implemented` ×3 with evidence citing this record and counts consistent ("8-bit bit-exact x3, 10-bit typed x5" matches the record's 3 8-bit fixtures + tenbit fixture's 5 typed refusals). |
| 9 | Speed: "32.9 fps ≈ 68.1 Mpix/s … ffmpeg 112.7 fps … ratio ≈ 0.29×" | :160-166 | REAL-UNPROVEN | Cross-cited: docs/perf/breadth.md:102 `honest-gap ratio **0.292×** (5.E2 recorded 0.309× …)` — consistent story; numbers not re-measured in this audit (no benchmark run permitted). |
| 10 | "Ogg-AV1: no mapping exists (no `Av1` in the Ogg codec table — verified 5.E1)" | :52-53 | REAL-UNPROVEN | External-spec claim; no contradicting code (no ogg-av1 mapping in crates/demux/src/ogg*). |
| 11 | "Differential vs VLC 3.0.23: PENDING INFRA … no vlc/cvlc binary in the sandbox" | :151-152 | REAL | Honest-pending marker; matches the environment facts (VLC oracle runs exist for PCM lanes only; AV1 oracle differential absent). |
| 12 | "rav1d 1.1.0 has an upstream missing-reference abort … our reference-state gate contains it" | :57-60, :198-204 | REAL-UNPROVEN | The gate code exists (`Recovered{Discontinuity}` at av1_differential.rs:449); the upstream-abort characterization of rav1d internals not independently verified (would need rav1d source — not in tree, by design). |

## 3. Contradictions
| Doc says | Reality | Truth |
|---|---|---|
| "`tests/fixtures/av1/`, ≈39 KB" :116 | tests/fixtures/av1 absent; fixtures at crates/codecs/tests/fixtures/av1/ | Reality — path drift (S4) |
| Front-matter "whose row reads `status = not_started` ×4" :7-8 | matrix row now implemented ×3 | Not a lie — the record documents the pre-flip state and its own purpose; noted as self-referential staleness. |

## 4. Fabrication / inflation indicators
None. The record voluntarily lists 8 "Known gaps (honest)" (:247-258)
including unexercised `-cpu-used 0..3` and wasm32-untested. Speed honesty
is exemplary (asm-off cost labeled, ffmpeg process-spawn asymmetry labeled).

## 5. Matrix rows
Touched row: codec-video-av1 (matrix :459) — implemented ×3, web
not_started; evidence string reconciles with this record's 5.E1 counts.
Verdict for the row (from this record's evidence): REAL-UNPROVEN at
execution level (conformance suite exists and is wired; not re-run in
this audit — cargo-test budget), code-level REAL.
