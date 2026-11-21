# AUDIT: crates/subtitles/src/vobsub/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-21 |
| Lines / functions | 54 / 1 production fn |
| File verdict | REAL-UNPROVEN as a fuzz harness (logic real + survival-test-executed; the registered cargo-fuzz target does not exist — PAPER-ONLY at the registry level) |
| Reachable from shipped entry point? | NO — `fuzz_sub_vobsub` callers: only crates/subtitles/tests/dvb_vobsub_fuzz_survival.rs:35,305 (workspace test, green in baseline). No cargo-fuzz target `fuzz_sub_vobsub` exists in fuzz/fuzz_targets/ (35 targets; subtitle targets are srt/ass/vtt only) and none in fuzz/README.md or fuzz/Cargo.toml. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | rg "fuzz_sub_vobsub" repo-wide (docs/04:356, ADR-012:176, sub-vobsub.md:16 — planning docs only; NOT in fuzz/); rg -c "^\| \`fuzz_" fuzz/README.md (=35); Read of vobsub/mod.rs vobsub_cues (382-425) + MAX_DAMAGE_EVENTS (57); baseline: `vobsub_fuzz_survival_mutations_and_truncations ... ok` (orchrun-test-workspace-nofailfast.log.txt, dvb_vobsub_fuzz_survival suite) + fuzz_registry_check.sh PASS 35 targets (INDEX.md row 18). |

## 1. One-paragraph verdict
This is an honest, small, correctly-shaped fuzz harness: one call runs the full idx+sub decode (`parse_vobsub`) plus the render contract (`vobsub_cues`), then five deterministic truncation re-parses, discarding all outputs — the documented contract is "absence of panic/hang/unbounded allocation, nothing else", and the code does exactly that. It is executed today by exactly one caller, the deterministic survival test, which passes in the baseline run — so the harness logic is REAL and test-proven. What does not exist is the thing the module is named after: the `fuzz_sub_vobsub` cargo-fuzz target. The module doc admits the wiring is Lead-owned and pending, but doc 04, ADR-012 and the sub-vobsub record all register the target name, and the repo's own registry gate (35 targets, diff empty) is structurally blind to those doc-level registrations — four registered bitmap-subtitle fuzz targets (pgs/dvb/vobsub/cc) have no campaign and never have. Separately, the module's bounds claim ("output ≤ O(input) + the 128 KiB SPU accumulation cap") understates the real per-cue allocation ceiling by two orders of magnitude (up to ~64 MiB RGBA per cue via the 12-bit-coordinate path audited in spu.rs F-010).

## 2. What actually works (verified)
— The full-surface claim's call chain: `parse_vobsub` (idx text parse + PES walk + fragment accumulation + SPU command walk + RLE decode — verified in vobsub/mod.rs:130-181 during the spu.rs audit) then `vobsub_cues` (render to `CompositedCue` — vobsub/mod.rs:382-425) — every surface the doc lists has a real function behind it.
— Deterministic truncation sweeps: `sub_data.len()/4`, `/2`, `len-3`, `idx/3`, `/2` — all `saturating_sub`-safe on empty inputs (cut=0 → empty-slice parse, no panic).
— The damage-ledger cap reference: `crate::vobsub::MAX_DAMAGE_EVENTS` exists (vobsub/mod.rs:57, `= 64`) and is enforced at mod.rs:135.
— Execution proof: `vobsub_fuzz_survival_mutations_and_truncations` (which calls this harness at line 305) passes in the baseline workspace test run (orchrun-test-workspace-nofailfast.log.txt dvb_vobsub_fuzz_survival suite: "2 passed").
— Honest contract statement: outputs are `let _ =`-discarded and the doc says so (11-14) — no fake assertions inside the harness.

## 3. Findings

### F-001 [S3] The registered fuzz target does not exist — two disjoint "registries" and the gate only checks one
Where: crates/subtitles/src/vobsub/fuzz.rs:1-2 and 16-26
Code:
```
//! The VobSub fuzz harness (card 5.N2 `+FB`; doc 04 §5 — registry
//! name `fuzz_sub_vobsub`).
```
```
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_vobsub.rs` exactly as doc 04 §8.5
//! registers it, delegating here.
```
What is wrong: `fuzz/fuzz_targets/` contains 35 targets; the subtitle ones are exactly `fuzz_sub_srt`, `fuzz_sub_ass`, `fuzz_sub_vtt` — no `fuzz_sub_vobsub`, no `fuzz_sub_pgs`, no `fuzz_sub_dvb`, no `fuzz_sub_cc`. fuzz/README.md and fuzz/Cargo.toml match (35). Meanwhile docs/04-codec-format-roadmap.md:356, docs/adr/ADR-012-bitmap-subtitles.md:176 and docs/formats/sub-vobsub.md:16 all speak of `fuzz_sub_vobsub` as a registry entry. The repo's own gate (`scripts/fuzz_registry_check.sh`, baseline PASS "35 targets, diff empty") compares only the fuzz/ tree against itself — it cannot detect a doc-registered-but-unwired target. The module doc IS honest about the Lead-owned pending wiring, but the aggregate effect is that four bitmap-subtitle "fuzz targets" exist as names in three documents and as zero buildable targets; the only thing ever executed is the deterministic (non-fuzzing) survival loop.
Why it matters: a reader of ADR-012 or the formats records believes a fuzzing campaign exists for hostile bitmap-subtitle input; what exists is a fixed-seed mutation loop with a committed corpus — materially weaker coverage (no coverage feedback, no corpus growth).
What correct looks like: either wire the four targets (Lead decision, nightly toolchain per fuzz/README) or change the docs' "fuzz_target = …" front-matter to "harness = … (target pending)" and make fuzz_registry_check.sh cross-check the doc-04 table.
Confidence: HIGH — Verification: read + rg counts (35 vs absent), baseline INDEX.md row 18.

### F-002 [S3] Bounds claim understates the real allocation ceiling by ~2 orders of magnitude
Where: crates/subtitles/src/vobsub/fuzz.rs:28-32 (and 36-38)
Code:
```
//! # Bounds (MP §12 / D-2)
//!
//! One pass, output ≤ O(input) + the 128 KiB SPU accumulation cap;
//! the damage ledger is capped at
//! [`crate::vobsub::MAX_DAMAGE_EVENTS`].
```
What is wrong: the emphasized cap is the 128 KiB SPU accumulation buffer, but the decode path's dominant allocation is the bitmap: `decode_spu` allocates `w*h` indices where w/h come from 12-bit coords (up to 4096×4096 = 16.7 MiB from a ≤128 KiB packet — spu.rs F-010), and the render path turns that into `w*h*4` RGBA (up to ~64 MiB per cue, vobsub_cues → CueBitmap::from_rgba). "Never allocates without a ceiling" survives (the 12-bit fields ARE a ceiling), but "output ≤ O(input) + the 128 KiB cap" points the reader at the wrong ceiling by a factor of 512; an adversarial idx with many entries multiplies the transient 64 MiB allocations per entry.
Why it matters: a fuzzing campaign (or a hostile file through any future wiring) hits memory-amplification behavior the harness's own documentation says cannot happen; the survival corpus (≤200 KB inputs, small geometries) never approaches it.
What correct looks like: a bitmap-pixel cap in spu.rs (see spu.rs F-010's fix) and a bounds doc that names the real ceilings.
Confidence: HIGH — Verification: read of both allocation sites (spu.rs:264, vobsub/mod.rs:389 via CueBitmap::from_rgba expected-size check in bitmap.rs:116-128); field widths from the 0x05 command parse.

### F-003 [S4] "every decode surface" is verified only up to the cue builder — palette/grayscale/crop coverage is asserted, not traced here
Where: crates/subtitles/src/vobsub/fuzz.rs:4-10
Code:
```
//! One call = one full untrusted idx+sub pass through **every**
//! decode surface: the idx text parse (palette/size/timestamp/
//! delay tolerances), the PS PES walk (start codes, lengths, PTS,
//! substream routing), per-entry fragment accumulation, the SPU
//! command sequences (dates, colormap, alpha, coords, offsets, HD
//! commands), the nibble-RLE interlaced bitmap, the palette/grayscale
//! application, the transparent-border crop, and the cue render
//! path.
```
What is wrong: nothing is wrong with the harness code — but the checklist item "palette/grayscale application" and "transparent-border crop" execute inside `parse_vobsub`'s cue building (vobsub/mod.rs render-side helpers, A08's assignment); I verified the call chain reaches `vobsub_cues` and the SPU decode surfaces, and did NOT independently trace the crop helper's invocation. The claim is plausible (the survival test asserts `rgba.len() == width*height*4` per cue, which requires the render path to have run) but the crop/palette legs are UNVERIFIED by me — recorded for A08 to confirm, not asserted as false.
Why it matters: "every decode surface" is a coverage claim; an unaudited leg of it could be dead code that the harness silently never reaches.
Confidence: MEDIUM — Verification: partial trace; blocked at vobsub/mod.rs internals outside my assignment (A08's file) — flagged as UNVERIFIED-leg, not a defect claim.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_sub_vobsub` | 39-54 | REAL | — | full pass + 5 deterministic truncation re-parses; all outputs discarded per the documented absence-of-panic contract; no state between calls (stateless harness — multi-call SPU accumulation is exercised only within one blob); panics impossible by construction of the callees except the latent escape-hatch class (see F-002, spu.rs F-010 is a bound not a panic) |
| (module doc) | 1-33 | REAL (honest) with two claims discounted | S3 | F-001 (registry), F-002 (bounds); the wiring-status section is unusually honest — names the Lead-owned decision and the stable-test substitute |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| tests/dvb_vobsub_fuzz_survival.rs::vobsub_fuzz_survival_mutations_and_truncations (the harness's only executor) | YES for panics/hangs/ledger-cap violations | seeded LCG mutations + committed corpus + truncation sweep through this harness; asserts cue invariants and `rgba.len() == w*h*4`; passes in baseline |
| Inherent harness limit (by design, documented) | NO for wrong values | the harness itself asserts nothing about decoded content (e.g., it would not catch spu.rs F-002's garbage PTS or F-003's silent zero-fill) — that is the documented contract ("nothing else"), so this is honest, not fake; the AC suites are supposed to pin values, and the PTS leg is untested there too (spu.rs F-009) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "registry name `fuzz_sub_vobsub`" / doc 04 §8.5 registers the target | fuzz.rs:1-2, 18-20; docs/04:356; ADR-012:176 | PAPER-ONLY | no such target in fuzz/fuzz_targets (35 files), fuzz/README.md (35 rows), or fuzz/Cargo.toml; registry gate checks only the fuzz/ tree |
| "the harness logic is proven on stable by the deterministic survival runs … (committed mutated/truncated corpus + a seeded LCG mutation loop)" | fuzz.rs:23-26 | REAL | dvb_vobsub_fuzz_survival.rs:305 calls it; baseline suite green |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:11-14 | REAL | code discards all outputs; matches behavior |
| "output ≤ O(input) + the 128 KiB SPU accumulation cap" | fuzz.rs:30 | MISLEADING | F-002: real ceiling is 12-bit-coord-derived (≤16.7 MiB indices, ≤64 MiB RGBA per cue), 128–512× the named cap |
| "fuzz_target = `fuzz_sub_vobsub` (in-crate harness `src/vobsub/fuzz.rs`; cargo-fuzz absent, Lead-owned wiring)" | docs/formats/sub-vobsub.md:16-17 | PAPER-ONLY (target) / REAL (harness) | same as F-001; the honesty marker is present but the front-matter key still names a nonexistent target |
| "fuzz-lite survival … zero panics/hangs/unbounded allocations; the 128 KiB SPU cap never exceeded; every cue's RGBA buffer exactly `width × height × 4`" | docs/formats/sub-vobsub.md:50-54 | REAL-UNPROVEN | survival suite passes in baseline (proves panics/cap/RGBA-size invariants on ITS corpus); "unbounded allocations" is unproven in general — the 12-bit-coord amplification (F-002) is a bounded-but-huge allocation the corpus never triggers |

## 7. Dead / duplicated / unreachable code
- No dead code in the file; the single fn is called (survival test).
- The harness is unreachable from shipped entry points (crate-level verdict).

## 8. Dependencies & flags
- `use crate::vobsub::{parse_vobsub, vobsub_cues}` — internal only; no unsafe; no feature flags; no dependencies beyond the crate itself.

## 9. Open questions for the next agent
- A08 (vobsub/mod.rs): confirm the "palette/grayscale application" and "transparent-border crop" legs actually run inside `parse_vobsub`'s cue building (my F-003 unverified leg); also note vobsub/mod.rs:394/399 replicate the pgs S2/S3 patterns (Anchor::BOTTOM_CENTER silent fallback + TitleSafe clamp on exact positions) — same findings apply to VobSub.
- A08: vobsub/mod.rs:435-436 asserts "0x01/0x02 take no arguments (the reference grammar)" — this repeats spu.rs F-004's unverified arity claim for command 0x00; one oracle check settles both.
- Lead / W17-W18: the four doc-registered-but-unwired fuzz targets (pgs/dvb/vobsub/cc) vs the gate's 35 — a registry cross-check belongs in fuzz_registry_check.sh or doc 04 needs "pending" markers.
- W19 (docs): sub-vobsub.md's bounds language should match the real allocation ceilings once spu.rs F-010 is fixed.
