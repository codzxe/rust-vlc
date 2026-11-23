# AUDIT: crates/subtitles/src/pgs/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-23 |
| Lines / functions | 77 / 1 (fuzz_sub_pgs) |
| File verdict | REAL (harness) — genuinely exercised by pgs_fuzz_survival.rs (2/2 pass); wiring-status doc is half-stale and half-false |
| Reachable from shipped entry point? | N/A (fuzz harness) — called by crates/subtitles/tests/pgs_fuzz_survival.rs:228,250 (stable gate, executed); NO cargo-fuzz target exists for it |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | prebuilt `./target/debug/deps/pgs_fuzz_survival-40c8df5a95c36609` -> 2 passed / 0 failed (exit 0); rg fuzz_sub_pgs callers (exit 0); `ls fuzz/fuzz_targets` -> no fuzz_sub_pgs.rs (exit 0); sed quote verification |

## 1. One-paragraph verdict
The PGS harness is real and non-trivial: it splits mutated payloads into pseudo display-sets at END-segment signatures (0x80 00 00) to drive the epoch state machine across sets, feeds monotonic 90 kHz PTS values, runs a single-chunk truncation sweep (1/3, 1/2, len-1), resets and re-feeds to exercise re-lock, then recomposes and pushes the last set through the cue render path (`PgsCueBuilder::cues`). It is exercised on stable by tests/pgs_fuzz_survival.rs — seeded LCG mutations + exhaustive truncation sweeps over hand-crafted seeds, 2/2 passing. Two documentation defects: the wiring-status note claims the cargo-fuzz target "belongs at fuzz/fuzz_targets/fuzz_sub_pgs.rs" — that file does NOT exist (only sub_ass/sub_srt/sub_vtt were ever wired), so the registry's fuzz lane cannot run PGS at all; and the same note claims the stable proof includes a "committed mutated/truncated corpus" — no committed PGS corpus exists (the survival test synthesizes hand-crafted seeds, per its own honest label).

## 2. What actually works (verified)
— verified by `./target/debug/deps/pgs_fuzz_survival-40c8df5a95c36609` -> `test result: ok. 2 passed; 0 failed` (exit 0): `seeded_mutation_survival_stays_typed_and_bounded` and `truncation_sweep_never_panics_and_stays_typed` both drive `fuzz_sub_pgs` (pgs_fuzz_survival.rs:228, 250) across both framings (Sup + Stripped). Harness internals read sound: empty-input edge (chunks empty -> single-chunk path with three empty truncation decodes), `start.min(data.len())` defensive slice, saturating_sub guard, all bounds safe.

## 3. Findings
F-001 [S2]
Where: context — the pgs module lives in the crate wired to nothing (apps/cli/Cargo.toml has no `subtitles` dep; only fuzz/Cargo.toml:50)
Code: `subtitles = { path = "../crates/subtitles" }` (fuzz/Cargo.toml:50)
What is wrong: the fuzz-hardened PGS decoder protects a path no shipped binary can invoke (tags.rs F-001). Confidence: HIGH — Verification: ran (rg).
F-002 [S3]
Where: crates/subtitles/src/pgs/fuzz.rs:18-23 vs fuzz/fuzz_targets/ directory
Code:
```
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_pgs.rs` exactly as doc 04 §8.5
//! registers it, delegating here. Building it needs the
//! `libfuzzer-sys` nightly scaffolding — a Lead-owned dependency
//! decision (AGENTS.md §3.2, C5); the `fuzz/` tree is outside this
//! card's file scope. Until the Lead wires it, the harness logic is
```
What is wrong: the Lead wired sub_ass/sub_srt/sub_vtt targets but `fuzz/fuzz_targets/fuzz_sub_pgs.rs` does not exist (ls verified). ADR-012 (docs/adr/ADR-012-bitmap-subtitles.md:176) also registers fuzz_sub_dvb/vobsub/cc — none exist either. The registry-promised coverage-directed fuzzing lane for bitmap subtitles is absent; only the stable fuzz-lite runs exist. What correct looks like: add the 3-line delegating targets (same shape as fuzz_sub_srt.rs) or amend the registry/ADR. Confidence: HIGH — Verification: ran (ls + rg).
F-003 [S4]
Where: crates/subtitles/src/pgs/fuzz.rs:23-26 vs crates/subtitles/tests/pgs_fuzz_survival.rs:1-3
Code:
```
//! proven on stable by the deterministic survival runs in
//! `crates/subtitles/tests/` (committed mutated/truncated corpus +
//! a seeded LCG mutation loop).
```
vs the survival test's own header: "deterministic seeded mutations + systematic truncation sweeps over hand-crafted display sets."
What is wrong: there is no committed PGS mutated/truncated corpus (tests/fixtures/fuzz/ holds only SRT mutants; the PGS test synthesizes seeds in memory). The sentence is copy-pasted from the SRT harness doc and overstates the evidence base. The actual proof (hand-crafted seeds, honestly labeled "NO reference-encoder differential oracle") is real but different. Confidence: HIGH — Verification: read + ran (ls fixtures).
F-004 [S4]
Where: crates/subtitles/src/pgs/fuzz.rs:47-52
Code:
```
    while i + 3 <= data.len() {
        if data[i] == 0x80 && data[i + 1] == 0x00 && data[i + 2] == 0x00 && i > start {
            chunks.push(&data[start..i + 3]);
            start = i + 3;
        }
```
What is wrong: the END-signature scan can split inside ODS RLE pixel data (0x80 00 00 is a legal in-data byte triple), producing arbitrary "display sets" — acceptable for a panic-hunting harness, but the doc sells it as "the END segment the mutator left us", implying structural framing awareness it does not have. Cosmetic overstatement of a heuristic. Confidence: HIGH — Verification: read.

## 4. Per-function table (file < 100 LOC — table still given for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_sub_pgs | 39-77 | REAL | S4 | chunk split (heuristic, F-004), truncation sweep, reset+re-feed, empty-decode recompose, render path; all bounds safe (traced empty/short/full inputs) |

## 5. Test honesty
| Test (in pgs_fuzz_survival.rs, drives this harness) | Would it fail if code were broken? | Reason |
|---|---|---|
| seeded_mutation_survival_stays_typed_and_bounded | YES (panic/ledger/rgba-shape bugs) | 1-8 real mutations/iter, ledger cap + w*h*4 assertions, 200 KB budget clamp |
| truncation_sweep_never_panics_and_stays_typed | YES | every prefix of both framings' seeds + harness call per prefix |
The harness itself asserts nothing (by contract, "nothing else") — the pinned invariants live in the survival test. Coherent split.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "fuzz_target = fuzz_sub_pgs (in-crate survival harness" | docs/formats/sub-pgs.md:16 | REAL (in-crate) / cargo-fuzz target MISSING | F-002: no fuzz_sub_pgs.rs in fuzz_targets |
| "the harness logic is proven on stable by ... committed mutated/truncated corpus + a seeded LCG mutation loop" | fuzz.rs:23-26 | HALF-FALSE | F-003: no committed PGS corpus; LCG loop real (2/2 pass) |
| "One call = one full untrusted-payload pass through every decode surface" | fuzz.rs:4-10 | REAL | decode_display_set covers framing/walk/PCS/WDS/PDS/ODS/RLE + cues() render (mod.rs:302-380 read) |
| "never allocates without a ceiling" | fuzz.rs:37-38 | REAL-UNPROVEN | canvas cap MAX_OBJECT_PIXELS + damage cap referenced; not re-measured here (alloc test is another file's compositing_alloc.rs) |

## 7. Dead / duplicated / unreachable code
No dead code in-file. Harness reachable from the stable test gate only (plus the absent cargo-fuzz lane, F-002).

## 8. Dependencies & flags
std-only within crate `subtitles`; no features; no unsafe. Depends on pgs::{PgsDecoder, PgsCueBuilder} (other agents' files; signatures verified at mod.rs:302, 601).

## 9. Open questions for the next agent
- Who owns fuzz/Cargo.toml target registration — the missing sub_pgs/sub_dvb/sub_vobsub/sub_cc targets vs ADR-012's registry promise is a cross-cutting wiring debt worth one card.
- The final `decode_display_set(&[], None)` recompose path relies on compose()-from-banks semantics after mem::take of composed_objects (mod.rs:373-379) — pgs/mod.rs auditor should confirm the empty-decode set can actually carry objects (my read: yes, banks persist).
