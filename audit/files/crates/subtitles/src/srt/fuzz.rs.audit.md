# AUDIT: crates/subtitles/src/srt/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A01 |
| Date | 2026-11-21 |
| Lines / functions | 47 / 1 function (`fuzz_sub_srt`) + module-doc claims |
| File verdict | REAL (harness executed in baseline via the committed survival tests; the cargo-fuzz target exists and delegates here; the crate itself is unreachable from shipped entry points) |
| Reachable from shipped entry point? | NO for shipped binaries (crate unlinked). The harness's own consumers: `fuzz/fuzz_targets/fuzz_sub_srt.rs` (`subtitles::srt::fuzz::fuzz_sub_srt(data)` — verified present in the 35-target registry) and `tests/srt_fuzz_survival.rs:51` (4/4 ok in baseline). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read tool (whole file); `cat fuzz/fuzz_targets/fuzz_sub_srt.rs` → target EXISTS and delegates to this function; `rg -n "MAX_PROBE_PIXELS" srt/render.rs` → `1 << 22` px with "= 16 MiB RGBA8" doc (:36, :50); `rg -n "MAX_DAMAGE_EVENTS" srt/damage.rs` → `256` (:17) with capped-record logic (:148); `rg -n "fuzz_sub_srt" tests/srt_fuzz_survival.rs` → :51 call site; baseline log → `srt_fuzz_survival` 4/4 ok (:5638-5643) with test names matching the doc's claimed sweep kinds; aggregate-render-work arithmetic hand-checked |

## 1. One-paragraph verdict
A 47-line harness facade whose every checkable claim checks out: the cargo-fuzz target it describes exists at the exact path and delegates to this function; the damage-ledger cap it quotes is really 256 (`damage.rs:17`); the probe-bitmap cap is really `1 << 22` pixels = 16 MiB RGBA8 (`render.rs:36,50`); the "proven on stable by deterministic survival runs" claim maps one-to-one onto the four baseline-green `srt_fuzz_survival` tests (seeded mutation loop, committed corpus, truncations, canonical parents); and the aggregate render cost is genuinely bounded — sum over cues of (text_len × 20 × 40) ≤ 800 × input bytes of transient pixel work, with the per-cue 16 MiB cap clamping pathological single cues, so "never hangs / allocates without a ceiling" is defensible arithmetic, not marketing. The only defect is staleness: the wiring-status paragraph still describes the target file as a future obligation ("belongs at … Until the Lead wires it") when card 5.Q1 already created it — the target's own header says so.

## 2. What actually works (verified)
— Baseline execution: `tests/srt_fuzz_survival.rs` — `canonical_parents_still_parse_exactly`, `committed_corpus_survives`, `seeded_mutation_loop_survives`, `truncations_never_beat_the_parser` all `ok` (4/4, finished in 0.01s), each calling `subtitles::srt::fuzz::fuzz_sub_srt(bytes)` (:51).
— Target wiring: `fuzz/fuzz_targets/fuzz_sub_srt.rs` exists (one of the 35 targets the registry check PASSed in baseline run #18) and its body is exactly `fuzz_target!(|data: &[u8]| { subtitles::srt::fuzz::fuzz_sub_srt(data); });` — the delegation claim is literal truth.
— Bounds claims vs code: `MAX_DAMAGE_EVENTS = 256` (damage.rs:17, capped push at :148) ✓; `MAX_PROBE_PIXELS = 1 << 22` px, documented "= 16 MiB RGBA8" (render.rs:50, :36) ✓ — both constants match the numbers in this file's doc.
— "Every parse surface" claim traced: `parse(data)` → `srt/parser.rs:78-79` `encoding::decode` (all three BOM paths + CP1252 lossy fallback — my encoding.rs audit) + `normalize_line_endings`, then block walk/timing/markup inside the parser; the render loop (`place_plain_text` per cue at 1920×1080 with `TextMetrics::new(40, 20)`) covers the render path.
— Aggregate-bounds arithmetic: cues ≈ input/7 bytes minimum (an arrow line + separators), each probe ≤ min(text extent × 800 px, 16 MiB) and freed per cue — total transient work ≤ ~800 × input_len pixels + cue_count × overhead. Bounded; no unbounded-growth path found in the harness itself.
— Contract honesty: results of `place_plain_text` discarded with `let _ =` — explicitly documented ("both swallowed here", :10-11); the harness asserts nothing, by design, and says so in bold.

## 3. Findings

### F-001 [S4]
Where: crates/subtitles/src/srt/fuzz.rs:13-23
Code:
```
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_srt.rs` exactly as doc 04 §8.5
//! registers it, delegating here. Building it needs the
//! `libfuzzer-sys` nightly scaffolding — a Lead-owned dependency
```
What is wrong: the wiring-status paragraph is stale. It frames the target file as a not-yet-existing obligation ("belongs at … Until the Lead wires it, the harness logic is proven on stable …"), but `fuzz/fuzz_targets/fuzz_sub_srt.rs` exists, delegates to this exact function, self-documents as card 5.Q1, and is part of the 35-target registry that `scripts/fuzz_registry_check.sh` PASSed in the baseline (run #18). What remains true is only the narrower claim that the libFuzzer campaign has not been run (no campaign output exists in the baseline).
Why it matters: a reader concludes the fuzz/ tree lacks the SRT target and that wiring is blocked on a Lead decision, when the file is present and the blocker is only campaign execution. Doc drift, no runtime impact — S4.
What correct looks like: update the paragraph to "the target exists (card 5.Q1) and delegates here; the libFuzzer campaign is not yet wired into CI — until then the harness logic is proven by the deterministic survival runs."
Confidence: HIGH — Verification: read + `cat fuzz/fuzz_targets/fuzz_sub_srt.rs` (file exists, delegates) + baseline run #18 (registry PASS, 35 targets).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_sub_srt` | 40-47 | REAL | — | parse + per-cue render; `let _ =` discard documented; fixed metrics/extent are adequate probe parameters; no assertions (by documented contract) |
| module docs (claims) | 1-32 | REAL (one stale paragraph) | S4 | F-001; all numeric claims (256 cap, 16 MiB probe, ≤ input/2 cue bound) verified against damage.rs/render.rs; "≤ input bytes / 2" is loose (real minimum ≈ input/7) but valid as a bound |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (this file has no tests; it IS the harness) | — | its correctness signal is the survival suite: `seeded_mutation_loop_survives` / `committed_corpus_survives` / `truncations_never_beat_the_parser` would panic if the parse/render path panicked on hostile bytes — they ran green in baseline (release profile; overflow-checked builds would additionally surface arithmetic panics, none exist in the SRT path per my encoding.rs trace and the parser agent's scope) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "fuzz_sub_srt" registry target exists | doc 04 §8.5 / module doc :2 | REAL | fuzz/fuzz_targets/fuzz_sub_srt.rs present; registry check PASS 35 targets (baseline #18) |
| "the harness logic is proven on stable by the deterministic survival runs" | module doc :20-23 | REAL | tests/srt_fuzz_survival.rs:51 calls it; 4/4 ok in baseline |
| "Until the Lead wires it" (target as future work) | module doc :20 | FALSE (stale) | F-001: the target exists (card 5.Q1); only the campaign is unwired |
| "damage ledger is capped (256 events)" | module doc :27-28 | REAL | damage.rs:17 `MAX_DAMAGE_EVENTS = 256` |
| "probe bitmap of at most MAX_PROBE_PIXELS pixels (16 MiB)" | module doc :30-31 | REAL | render.rs:50 `1 << 22` px; :36 "= 16 MiB RGBA8" |
| matrix `subtitle-srt` evidence: "230/0 current tree" | feature-matrix.toml | REAL (stale count) | baseline: 267/0 lib (:5269) + srt_fuzz_survival 4/4 + srt_canonical_differential 6/6 |
| status implemented x3 | same row | UNREACHABLE (per-platform) | no shipped binary links the subtitles crate |

## 7. Dead / duplicated / unreachable code
None in-file. Contrast worth recording for the orchestrator: the SRT/ASS/VTT cargo-fuzz targets exist; the CC/DVB/PGS/VobSub harnesses (same doc pattern, e.g. vobsub/fuzz.rs:15-23) have NO targets in `fuzz/fuzz_targets/` — the registry asymmetry my vobsub/idx.rs audit flagged (S2 overflow there would be found immediately by a wired target).

## 8. Dependencies & flags
std-only harness; depends on `crate::srt::parse` + `crate::srt::render` (other agents' files). No unsafe, no features. The external target needs `libfuzzer-sys` (fuzz/ workspace, out of tree scope).

## 9. Open questions for the next agent
- srt/render.rs owner: confirm `TextMetrics::new(line_height, char_width)` argument order matches the fuzz call's intent (40 = line height, 20 = char width — the probe is square-ish either way, so no behavioral stake).
- Orchestrator: consider a REQUEST-ORCH-RUN for `cargo +nightly fuzz run fuzz_sub_srt -max_total_time=60` once nightly is available — the SRT path is the only subtitle lane whose target is fully wired; vobsub's missing target is the one that matters (my idx.rs F-001).
