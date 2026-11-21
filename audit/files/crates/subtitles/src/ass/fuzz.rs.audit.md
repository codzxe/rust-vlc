# AUDIT: crates/subtitles/src/ass/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-21 |
| Lines / functions | 67 / 3 (fuzz_sub_ass + 2 tests) |
| File verdict | UNREACHABLE (real smoke harness, tests green in baseline; no shipped entry point reaches the crate — and the harness itself is only ever executed by its 2 in-file tests) |
| Reachable from shipped entry point? | NO — crate-level proof (only `fuzz/Cargo.toml` depends on `subtitles`, nightly-only own-workspace scaffold). The registered cargo-fuzz target `fuzz/fuzz_targets/fuzz_sub_ass.rs` EXISTS (verified) and delegates here, but building it requires the unwired nightly toolchain — zero executions ever. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full read; `ls fuzz/fuzz_targets/` + read of fuzz_sub_ass.rs (delegation verified); comparative read of srt/fuzz.rs, dvb/fuzz.rs, pgs/fuzz.rs (render coverage per harness); read of ass/parser.rs:1385-1420 (the 2000-run sweep) and ass/render.rs:958-1012 (the 2048-run sweep — `runs: Vec::new()`, `pos: None`); baseline `orchrun-test-workspace-nofailfast.log.txt:5270-5272` (both harness tests ok). |

## 1. One-paragraph verdict
The ASS fuzz harness is the weakest of the crate's four: every sibling
drives its full decode→render lane (srt runs `place_plain_text` per
cue, dvb and pgs run their `CueBuilder::cues`), but `fuzz_sub_ass`
stops at the parser plus two getter probes — no layout, no render, no
override-tag engine. That matters because the matrix row's fuzz
evidence ("2000+2048 fuzz-lite runs clean") decomposes into a
parser-only sweep and a render sweep that constructs every cue with
`runs: Vec::new()` and `pos: None` — the G-2 override-tag RENDER path
(`\b \i \u \s \fs \fn \c \1c \alpha \1a \pos \move → styled runs +
position override`), the row's centerpiece claim, has never received
one fuzzed byte. The module doc is honest about its parser-only
scope; the evidence string that cites it is not precise about what
was actually proven. The harness itself never panics on its five
fixed smoke inputs, which is all the execution it has ever had.

## 2. What actually works (verified)
— The harness body is panic-free by construction for its surface:
`String::from_utf8_lossy` (bounded by input), `parser::parse`
(bounded per its own 2000-run sweep), `doc.title()` / `doc.style("Default")`
(bounded lookups). Trace read; no indexing, no unwrap.
— Baseline execution: `orchrun-test-workspace-nofailfast.log.txt:5270`
→ `test ass::fuzz::tests::empty_and_marker_inputs_survive ... ok`,
`:5272` → `test ass::fuzz::tests::hostile_wide_inputs_survive ... ok`
(inside the 267/0 subtitles unit run).
— The registered target file exists and correctly delegates:
`fuzz/fuzz_targets/fuzz_sub_ass.rs:15-17` → `subtitles::ass::fuzz::fuzz_sub_ass(data)`
(baseline #18 fuzz_registry_check PASS, 35 targets — this is one of
them).

## 3. Findings

### F-001 [S3] Harness covers parser only — the G-2 tag-engine render path receives zero fuzzed input anywhere in the tree
Where: crates/subtitles/src/ass/fuzz.rs:28-34 (harness), contrasted with the sibling harnesses and the matrix evidence
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
What is wrong: the harness's "one full untrusted-ASS pass" is the
parser plus two fixed-key lookups. The sibling harnesses all render:
`fuzz_sub_srt` (srt/fuzz.rs:45) calls `place_plain_text` per cue;
`fuzz_sub_dvb` (dvb/fuzz.rs:73-74) calls `DvbCueBuilder::cues`;
`fuzz_sub_pgs` (pgs/fuzz.rs, tail) calls `PgsCueBuilder::cues`. ASS
layout/render/tags — including the override-tag engine that the
matrix row calls its G-2 headline — are never driven with hostile
bytes. The row's evidence "2000+2048 fuzz-lite runs clean" decomposes
as: 2000 parser runs (ass/parser.rs:1392, junk incl. `{\b1}` at the
PARSE level) + 2048 render runs (ass/render.rs:977) whose cue
construction pins `runs: Vec::new()` and `pos: None`
(ass/render.rs:989, 991) — i.e., the plain-text render only. The
styled-run and `\pos`/`\move` placement paths inside render are
exercised exclusively by hand-written unit suites.
Why it matters: the fuzz-lite claims in docs/feature-matrix.toml row
subtitle-ass-ssa (905-916) are the row's robustness evidence; a
hostile `{\pos(...)}` or malformed `{\c&H` sequence flowing parse →
tags → styled runs → render → layout has never been fuzzed. The
wave's hostile question ("do the ASS subset claims match the code?")
gets: the subset is implemented and unit-tested, but its fuzz
evidence covers only half the lane it is cited for.
What correct looks like: extend `fuzz_sub_ass` to render every parsed
cue over a deterministic PlayRes (the srt harness pattern), and let
the 2048-run render sweep construct non-empty runs and Some(pos) from
the LCG.
Confidence: HIGH — Verification: read of this file + all four sibling
harnesses + both sweep sites; baseline log for execution.

### F-002 [S4] "Proven on stable by the deterministic sweeps below" — the sweeps below are five fixed inputs
Where: crates/subtitles/src/ass/fuzz.rs:16-18
Code:
```rust
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_sub_ass.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's ASS differential suites.
```
What is wrong: "the deterministic sweeps below" = 2 tests, 5 inputs
(`[]`, a Script-Info marker, an Events header, 4096×0xFF, 4096×0x00,
plus 64 well-formed dialogue lines). That is a smoke test, not a
sweep; the real sweeps live in other files (parser.rs, render.rs),
which the sentence's second half gestures at but does not name.
Why it matters: the doc's word "proven" attached to five inputs
inflates the evidence for exactly the audience (future maintainers)
who will trust it.
What correct looks like: "smoke inputs below; the parse and render
sweeps live in parser.rs/render.rs test modules" — name the actual
evidence.
Confidence: HIGH — Verification: read (both tests counted).

### F-003 [S4] The in-file tests are assertion-free survival probes — the harness's only stable execution
Where: crates/subtitles/src/ass/fuzz.rs:40-66
Code:
```rust
    #[test]
    fn empty_and_marker_inputs_survive() {
        fuzz_sub_ass(&[]);
        fuzz_sub_ass(b"[Script Info]\n");
```
What is wrong: both tests only fail on panic — no invariant is
asserted (no "damage ledger capped", no "cue count bounded", unlike
the dvb/pgs survival tests which assert ledger caps and stride
invariants around their harness calls). Combined with the unwired
nightly target, the harness's total execution history is these five
inputs.
Why it matters: "tests asserting nothing" is acceptable for a
panic-probe, but the crate's own precedent (tests/dvb_vobsub_fuzz_survival.rs,
tests/pgs_fuzz_survival.rs) shows the house style pins invariants
around harness calls; ASS is the one format without them.
What correct looks like: assert doc damage ≤ cap and cue count ≤ line
count around each call (the parser's own sweep asserts these —
parser.rs:1396-1404 — so the pattern exists).
Confidence: HIGH — Verification: read + comparison with the two
survival test files.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_sub_ass` | 28-34 | REAL (narrow) | F-001 | parser + 2 getters; lossy UTF-8 copy = one bounded alloc; no panic path in this body |
| `empty_and_marker_inputs_survive` | 40-45 | REAL | F-003 | survival-only, no asserts |
| `hostile_wide_inputs_survive` | 48-66 | REAL | F-003 | 0xFF/0x00 buffers + 64 synthetic dialogue lines; the `format!` field `{:02}` with `i % 60` produces valid centiseconds — inputs are well-formed-ish, not hostile text |

## 5. Test honesty (tests in this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_and_marker_inputs_survive | Only on panic/hang | no output assertions (by design; see F-003) |
| hostile_wide_inputs_survive | Only on panic/hang | same; also the "hostile" dialogue lines are grammatical |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "2000+2048 fuzz-lite runs clean" (subtitle-ass-ssa evidence) | docs/feature-matrix.toml:914 | TRUE-but-MISLEADING | 2000 = parser-only sweep (ass/parser.rs:1392); 2048 = render sweep with `runs: Vec::new()`, `pos: None` (ass/render.rs:977-1012) — the tag-engine render path is in neither (F-001) |
| "G-2: override-tag MINIMUM now native — \b \i \u \s \fs \fn \c \1c \alpha \1a \pos \move -> styled runs + position override + per-run render; … 267/0 lib tests incl. per-tag suites" | docs/feature-matrix.toml:915 | REAL (unit level) / UNREACHABLE (shipped) + fuzz-evidence gap | 267/0 verified in no-fail-fast baseline (includes ass per-tag suites); implementation audited by the ass/parser/tags/render agents; no fuzzed input ever flows through the full tag→render lane (F-001) |
| "One call = one full untrusted-ASS pass" | ass/fuzz.rs:4-6 (module doc) | SHALLOW (as worded) | "full pass" = parser surface only (the doc does enumerate its scope honestly one sentence later, but "full" oversells vs the sibling harnesses' full-lane passes) |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps below" | ass/fuzz.rs:16-18 | FALSE (as worded) | five fixed smoke inputs (F-002); the real sweeps are in other files |

## 7. Dead / duplicated / unreachable code
- Nothing dead in-file. Whole crate unreachable from shipped
  binaries. The cargo-fuzz target exists but has zero executions
  (nightly never wired — fuzz/README's own LEAD-DECISION marker).
- Cross-agent note: dvb/fuzz.rs and pgs/fuzz.rs module docs claim
  their cargo-fuzz target files "belong at fuzz/fuzz_targets/
  fuzz_sub_{dvb,pgs}.rs exactly as doc 04 §8.5 registers it" — those
  files DO NOT EXIST (ls of fuzz/fuzz_targets/: only sub_ass, sub_srt,
  sub_vtt). The dvb/pgs harness owners should flag that claim; the
  baseline registry check (35 targets) passes because the registry
  itself apparently lists only what exists.

## 8. Dependencies & flags
- None in-file (std only). The `fuzz/` scaffold depends on
  `subtitles` + `libfuzzer-sys` (nightly-only, not in any gate —
  baseline INDEX #18 runs the registry CHECK, never the campaign).

## 9. Open questions for the next agent
- ass/render.rs + ass/tags.rs owners: confirm whether ANY test drives
  render with non-empty `runs` and `Some(pos)` from FUZZED (not
  hand-written) input — expected answer: none (F-001). If confirmed,
  the subtitle-ass-ssa row's fuzz evidence needs a scope correction.
- Wave synthesizer: the four subtitle fuzz harnesses have four
  different coverage tiers (ass: parse-only; srt/dvb/pgs: full lane).
  A wave-level table of "what the fuzz evidence actually proves"
  would close the hostile question cleanly.
