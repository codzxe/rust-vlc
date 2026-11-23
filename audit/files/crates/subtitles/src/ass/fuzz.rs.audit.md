# AUDIT: crates/subtitles/src/ass/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-23 |
| Lines / functions | 67 / 1 (+2 tests) |
| File verdict | REAL (harness works, wired, tests pass); coverage gap vs sibling harnesses; crate UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO (crate-level). Harness chain: `fuzz/fuzz_targets/fuzz_sub_ass.rs` (registered fuzz/Cargo.toml:251-252) -> `fuzz_sub_ass` here -> `ass::parser::parse`; stable evidence = in-crate tests + tests/ass_ffmpeg_ac.rs (baseline green). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p subtitles --lib ass::fuzz` -> 2 passed, 0 failed (exit 0); `ls fuzz/corpus/fuzz_sub_ass/` -> seed_ass_hostile.ass, seed_ass_plain.ass; `cat fuzz/fuzz_targets/fuzz_sub_ass.rs` (delegates here); `sed -n` quote verification. |

## 1. One-paragraph verdict
A thin, honest libFuzzer delegate: lossy UTF-8 -> full parser pass -> two bounded lookups, everything swallowed, contract explicitly "absence-of-panic, nothing else". The wiring it claims exists exactly as documented (target file + Cargo registration + two corpus seeds — verified). Two gaps: the harness stops BEFORE the render path while all five sibling harnesses (srt, dvb, cc, pgs, vobsub — Grep-verified) render over surviving cues, leaving ASS's most complex hostile-input surface (ass::render: font synthesis, shears, boxes, tag walk) without mutation coverage; and the "proven on stable by the deterministic sweeps" claim rests on 5 fixed inputs plus 64 near-identical template lines.

## 2. What actually works (verified)
— `cargo test -p subtitles --lib ass::fuzz` -> 2/2 pass (exit 0): empty input, bare section markers, all-0xFF (lossy path), all-0x00 (NULs), header + 64 hostile dialogue lines — no panics.
— Wiring claims verified real: `fuzz/fuzz_targets/fuzz_sub_ass.rs` exists and its entire body is `subtitles::ass::fuzz::fuzz_sub_ass(data);`; `fuzz/Cargo.toml:251-252` registers it; `fuzz/corpus/fuzz_sub_ass/` holds 2 seeds. (Building the campaign needs nightly — not run here; in-sandbox run impossible, consistent with the doc's honest "Lead-owned nightly wiring" note.)
— Harness body cannot itself panic: `from_utf8_lossy` (allocates ≤ 4·input), `parse` (returns a document, damage swallowed), `title()`/`style("Default")` bounded lookups. No `unwrap`/indexing in this file.

## 3. Findings
F-001 [S3]
Where: crates/subtitles/src/ass/fuzz.rs:28-34 (the harness body), contrast srt/fuzz.rs:35-42, dvb/fuzz.rs:71-74, cc/fuzz.rs, pgs/fuzz.rs:73, vobsub/fuzz.rs:41
Code:
```rust
pub fn fuzz_sub_ass(data: &[u8]) {
    let text = String::from_utf8_lossy(data);
    let doc = super::parser::parse(&text);
    // Observable-surface probes (bounded lookups, not walks).
    let _ = doc.title();
    let _ = doc.style("Default");
}
```
What is wrong: the harness exercises the parse surface only. Every sibling subtitle harness renders its survivors (`srt/fuzz.rs:7` "the render path (box + placement) over every surviving cue"; dvb/pgs/vobsub "the cue render path over whatever survived"). ASS is the one format whose render path (ass/render.rs — rasterization, font-pipeline synthesis, `\pos`/`\move` layout overrides, tag-driven styling) is the crate's most complex untrusted-input surface, and it gets zero fuzz coverage; render.rs's only hostile input is the fixed `junk_inputs_never_panic` vector (baseline).
Why it matters: a hostile ASS file reaching a future player hits parse AND render; "never panics" (line 26-27) is claimed for the harness's construction, but half the pipeline never runs under it. The doc is honest about the scope (it never claims render) — this is a coverage gap, not a false claim.
What correct looks like: mirror the srt/dvb pattern — build `CompositedCue`s over surviving events (bounded by cue count) and run the layout/render entry, plus corpus seeds exercising `\pos`/`\clip`/font tags.
Confidence: HIGH — Verification: read + Grep of all six harness bodies + baseline test list.
F-002 [S4]
Where: crates/subtitles/src/ass/fuzz.rs:15-18
Code:
```rust
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's ASS differential suites.
```
What is wrong: "proven ... by the deterministic sweeps below" — the sweeps are 5 fixed inputs and one 64-line template differing only in two digits. Notably the PARSER's own in-crate test `ass::parser::tests::fuzz_lite_survives_junk_and_mutations` (baseline) does mutation testing that this harness's stable evidence lacks; the claim's "proven" is stronger than the evidence.
What correct looks like: a seeded mutation loop over the corpus seeds (the srt harness precedent), or weaker wording.
Confidence: HIGH — Verification: read + ran (the two tests pass; they are 5 inputs + 1 template).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_sub_ass | 28-34 | REAL | F-001 | parse + 2 bounded probes; no panic paths in-file |
| tests::empty_and_marker_inputs_survive | 40-45 | REAL | - | panic-only contract — fails only on panic, which IS the contract |
| tests::hostile_wide_inputs_survive | 47-66 | REAL | F-002 | 0xFF/0x00 fills + 64-line template |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_marker_inputs_survive | YES iff panic | survival contract; no output asserts by design |
| hostile_wide_inputs_survive | YES iff panic | same; 0xFF exercises from_utf8_lossy, 0x00 exercises NUL-tolerant fields |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The cargo-fuzz target is fuzz/fuzz_targets/fuzz_sub_ass.rs" | ass/fuzz.rs:15-16 | REAL | file exists, body delegates here, Cargo.toml:251-252 registers it, 2 corpus seeds |
| "harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | ass/fuzz.rs:7-8 | REAL | body swallows all outputs |
| "harness logic is meanwhile proven on stable by the deterministic sweeps" | ass/fuzz.rs:17-18 | SHALLOW | F-002: 5 inputs + 1 template; parser's own fuzz_lite does more |
| "Parse: one pass, output ≤ O(input)" | ass/fuzz.rs:22-23 | REAL-UNPROVEN | parser property; not verifiable in this file (parser.rs owned by another agent) |

## 7. Dead / duplicated / unreachable code
None. (Whole-crate unreachability as per header of segment.rs audit.)

## 8. Dependencies & flags
None beyond crate-internal `ass::parser`. No unsafe. The stable test path needs no nightly features.

## 9. Open questions for the next agent
- ass/render.rs owner: does any doc/matrix row claim ASS fuzz coverage includes rendering ("fuzz_sub_ass" row in doc 04 §8.5)? If yes, F-001 upgrades to a claim-coverage contradiction.
- The corpus seeds (fuzz/corpus/fuzz_sub_ass/seed_ass_hostile.ass) — worth a fixture audit to see whether they actually reach hostile layout tags or are plain-dialogue only.
