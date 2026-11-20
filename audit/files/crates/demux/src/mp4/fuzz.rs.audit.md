# Audit — crates/demux/src/mp4/fuzz.rs

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W05-A02 |
| Date | 2026-11-16 |
| Lines | 84 (verified `wc -l`) |
| Functions | 1 public harness `fuzz_demux_mp4` + helper `span` + 3 public budget consts (`FUZZ_EVENT_BUDGET`, `FUZZ_POST_SEEK_DRAIN`, `FUZZ_SEEK_PROBES`) |
| File verdict | **REAL-UNPROVEN** — the harness code is real, registered in `fuzz/`, and exercised deterministically by in-crate tests; the libFuzzer CAMPAIGN it exists for has never been executed anywhere in the recorded history |
| Reachable | YES — `pub mod fuzz;` (mod.rs:100); called by tests.rs:1083/1089/1095/1103/1108 (fuzz-lite tests, part of the 1001-passed orchestrator run) and by `fuzz/fuzz_targets/fuzz_demux_mp4.rs:19` (the cargo-fuzz target, `#![no_main]` libFuzzer shim) |
| Findings | S0:0 S1:0 S2:0 **S3:1** **S4:1** (2 total) |
| Commands run | `wc -l`; full `Read`; `ls fuzz/fuzz_targets/` + `Read fuzz_demux_mp4.rs`; `ls fuzz/corpus/fuzz_demux_mp4/` (4 seeds); `od -A d -t x1z` seed headers (real ftyp/moov magic); `rg -n "fuzz_demux_mp4|fuzz run|nightly fuzz|cargo fuzz"` over *.md (registry + run evidence); `sed -n '4,10p'`, `'26,34p'`, `'66,71p'` (quote verification) |

## 1 One-paragraph verdict

The harness body is a genuine full-pass untrusted-input driver: it wraps the bytes in the real `SourceStream` seam (`test_support::mem_stream`, fuzz.rs:46-48), constructs the real demuxer, runs `start()`, drains every event under a hard 500 000-event budget, then fires three data-derived seek probes and drains 64 events after each — with every error path swallowed by contract ("absence-of-panic/hang/unbounded-allocation, nothing else", fuzz.rs:7-9, an honestly-declared non-oracle). It is registered as a real cargo-fuzz target (`fuzz/fuzz_targets/fuzz_demux_mp4.rs` — a `#![no_main]` libFuzzer shim calling exactly this function) with a 4-seed corpus of real MP4 bytes (verified magic: `ftyp isom … moov`). What is NOT proven is the campaign: the worklog records verbatim that cargo-fuzz was never installed and no libFuzzer/sanitizer/coverage run ever executed ("cargo-fuzz is NOT installed in this sandbox … NO libFuzzer runs, NO sanitizer/coverage campaigns executed here", docs/worklog/archive-2026-10.md:9792); the target file itself carries an honest "Wiring status" note deferring the build to a future nightly wiring. So: real code, real registration, real deterministic in-crate execution (the two fuzz-lite tests + the mp4_fixtures.rs malformed derivatives) — and zero coverage-guided fuzzing ever, which is the actual purpose of a fuzz target.

## 2 What actually works (verified)

- The harness exercises the full demuxer lifecycle on untrusted bytes: construct (49-51) → `start()` (53) → bounded drain (56-65) → seek probes with post-seek drains (67-83). A panic, hang, or runaway allocation anywhere in that path fails the calling test or crashes the campaign.
- Budgets are real constants, not vibes: `FUZZ_EVENT_BUDGET = 500_000` (27) checked on every loop iteration (58-60, 74-76), matching the integration suite's identical 500 000 budget (mp4_fixtures.rs:115).
- Seek probes are data-derived (last three bytes × 10 ms each → up to 2.55 s targets, 68-70), so mutated inputs steer seek targets without hardcoding.
- The target wiring is correct: `fuzz/fuzz_targets/fuzz_demux_mp4.rs` is a thin `fuzz_target!(|data: &[u8]| { demux::mp4::fuzz::fuzz_demux_mp4(data); })` shim; `fuzz/README.md:61` registers the row `fuzz_demux_mp4 → demux::mp4::fuzz::fuzz_demux_mp4 (MP4/MOV/3GP (5.C))`.
- The seed corpus holds genuine container bytes: `seed_mp4_h264_aac.mp4` header verified `00 00 00 20 ftyp isom … iso2 avc1 mp41 … moov … mvhd` (od dump), `seed_mp4_trunc0.mp4` is a 28-byte ftyp-only prefix (a truncation seed by construction), plus `seed_mp4_chapters.mp4` (24 839 B) and `seed_mp4_frag.mp4` (14 045 B) matching the fixture family names.
- Deterministic in-crate execution is real: tests.rs:1077-1111 drives the harness over truncation prefixes, stride-37/53 single-bit flips, and 512-byte garbage tails on both progressive and fragmented fixtures — these passed in the orchestrator's 1001-passed run (cited, not re-run).

## 3 Findings

### Z-001 [S3] Registered fuzz target, committed seeds — and a campaign that has never run anywhere
Where: `crates/demux/src/mp4/fuzz.rs:45-84` + `fuzz/fuzz_targets/fuzz_demux_mp4.rs:1-19` + `docs/worklog/archive-2026-10.md:9792`
Code (fuzz.rs:43-51):
```rust
/// Run one fuzz pass over `data` (see the module docs). Never panics
/// by construction of the demuxer and the budgets above.
pub fn fuzz_demux_mp4(data: &[u8]) {
    let Ok(stream) = super::test_support::mem_stream(data.to_vec()) else {
        return;
    };
    let Ok(mut mp4) = Mp4Demuxer::new(stream) else {
        return;
    };
```
Code (fuzz/fuzz_targets/fuzz_demux_mp4.rs:5-11, the honest wiring note):
```
// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
// compiles on stable (`cargo check` in fuzz/) and the harness logic
// is proven by the in-crate deterministic hostile-input sweeps.
```
Evidence (docs/worklog/archive-2026-10.md:9792): "cargo-fuzz is NOT installed in this sandbox ("no such command: `fuzz`") — so NO libFuzzer runs, NO sanitizer/coverage campaigns executed here."
What's wrong: nothing in the harness code — the gap is execution history. The entire coverage-guided fuzzing story for MP4 consists of (a) 2 deterministic in-crate sweep tests over ONE tiny synthetic progressive fixture + ONE tiny fragmented fixture, and (b) the mp4_fixtures.rs derivatives (9+ malformed files from the 24 committed). No coverage feedback, no ASan/MSan, no corpus growth, no crash artifacts (no `crashes/` dir exists in fuzz/). The "proven by the in-crate deterministic hostile-input sweeps" phrasing is honest about this, but the matrix/doc layer ("fuzz_target = `fuzz_demux_mp4`") reads as if a fuzz program exists; what exists is a compiled-never-run target plus a fixed-input smoke loop.
Why it matters: absence-of-panic over an enumerable ~25 fixed inputs is categorically weaker than coverage-guided fuzzing; the demuxer's real hostile-input robustness is UNPROVEN at the campaign level. The fuzz-lite stride mutations touch 1-in-37/53 bytes of one 5-sample fixture — total unique mutated inputs ≈ (7 + 31 + 1 + 5 + 1) ≈ 45 passes.
What correct looks like: run the campaign (the README already documents the exact commands, e.g. `cargo +nightly fuzz run fuzz_demux_mp4 -s address -O -- -max_total_time=600`, fuzz/README.md:155) on a machine with nightly + cargo-fuzz, commit the coverage/crash evidence, and only then let doc claims carry the target name as executed evidence.
Confidence: HIGH (worklog quote; no crashes/ dir; no run logs beyond the deterministic sweeps; `rg` over docs/ found zero libFuzzer run evidence).

### Z-002 [S4] Seek probes derive targets from the last 3 bytes only — weak steering, bounded range
Where: `crates/demux/src/mp4/fuzz.rs:66-71`
Code (verified `sed -n '66,71p'`):
```rust
    // Seek probes at data-derived targets.
    for probe in 0..FUZZ_SEEK_PROBES {
        let at = data
            .get(data.len().saturating_sub(probe + 1))
            .map_or(0, |b| u64::from(*b) * 10_000_000);
```
What's wrong: targets are `byte × 10 ms` for the last three bytes — the reachable target space is 0..2.55 s in 10 ms steps, so a mutated size field or a tfdt ladder in a long stream never steers a seek past the extent (the interesting out-of-range / mid-fragment-seek behaviors are only sampled at the coarse 10 ms grid). `.map_or(0, ...)` also collapses empty input to target 0 (harmless — `start()` already refused).
What correct looks like: derive targets from a wider slice (e.g. two u32s read from head and tail) so moof/tfdt values can steer the probe into fragment interiors and past the end.
Confidence: HIGH (arithmetic verified; impact bounded — the unit seek tests cover the semantics anyway).

## 4 Per-function/per-test table

No `#[test]` in this file. Its callers:

| Caller | Where | Contract | Verdict |
|---|---|---|---|
| `fuzz_lite_mutations_never_panic` | tests.rs:1077-1096 | absence-of-panic only | weak-by-design (see tests.rs audit #37) |
| `fuzz_lite_fragment_mutations_never_panic` | tests.rs:1098-1111 | absence-of-panic only | weak-by-design (tests.rs #38) |
| `malformed_mp4_fixtures_only_ever_refuse_or_demux_typed` | crates/demux/tests/mp4_fixtures.rs:255+ | drain + typed-refusal-or-demux, never a panic/fake packet | real hostile-input pin (integration level) |
| cargo-fuzz target | fuzz/fuzz_targets/fuzz_demux_mp4.rs | libFuzzer entry | REGISTERED, never run (Z-001) |

| Function/const | Lines | Classification |
|---|---|---|
| `span` | 35-41 | real helper (ns span) |
| `FUZZ_EVENT_BUDGET` / `FUZZ_POST_SEEK_DRAIN` / `FUZZ_SEEK_PROBES` | 27-33 | real bounds (used at 58/72/67) |
| `fuzz_demux_mp4` | 45-84 | real harness; REAL-UNPROVEN as a fuzz target (Z-001) |

## 5 Doc claims touching this file

- `fuzz/README.md:61` — registry row `fuzz_demux_mp4 → demux::mp4::fuzz::fuzz_demux_mp4 (MP4/MOV/3GP (5.C))`. VERDICT: TRUE (target file + function exist and match; `ls fuzz/fuzz_targets/` verified).
- `docs/formats/container-mp4.md:14` — "fuzz_target = `fuzz_demux_mp4` (in-crate fuzz + 14 committed malformed derivatives; doc 04 §8.5 registry name)". VERDICT: PARTLY TRUE / IMPRECISE — "in-crate fuzz" is real and ran (deterministic sweeps); "14 committed malformed derivatives" matches no count in the tree (24 mp4-family malformed files committed in tests/fixtures/generated/malformed; the fuzz corpus dir itself holds 4 seeds). The doc conflates the fixtures corpus with the fuzz corpus.
- `docs/formats/container-mp4.md:62` — "Fixtures: 7 valid + 14 malformed derivatives through fuzz + typed passes". VERDICT: counts stale (8 valid parents / 24 malformed derivatives counted on disk); the "through fuzz + typed passes" mechanism is real (mp4_fixtures.rs runs derivatives through the fuzz harness AND a typed pass, header lines 5-6).
- `docs/04-codec-format-roadmap.md:392` — `fuzz_target = "fuzz_demux_mp4" # required when status != not_started`. VERDICT: satisfied at registry level; execution-level evidence absent (Z-001).
- `docs/worklog/archive-2026-10.md:9792` — the honest no-cargo-fuzz statement. VERDICT: TRUE and controlling — it caps every fuzz_target claim in the matrix at "scaffolding + deterministic sweeps".
- `docs/feasibility/sections/subsystems.md:124` — "`fuzz_demux_mp4` seed corpus = these fixtures". VERDICT: PARTIAL — 4 seeds vs 8 committed valid parents (see test_support.rs audit §5).
- `docs/formats/container-mp4.md:68` — "no panics (fuzz-lite + in-crate fuzz_mutations green)". VERDICT: SUPPORTED at the deterministic-sweep level only (the 2 fuzz-lite tests passed in the orchestrator's run); it is not campaign-grade no-panic evidence.

## 6 Dead/duplicated code

- None in this file. All three consts are consumed; `span` is used by the probe loop.
- Reachability note (verified against mod.rs:610-631): `Mp4Demuxer::new` fails only when `SourceReader::new` rejects the stream (unseekable / unknown length / empty). The harness's first early-return arm (fuzz.rs:46-48) is unreachable in practice — `test_support::mem_stream` over an in-memory `Arc<[u8]>` never fails (avi/test_support.rs:121-126 wraps a `BytesStream` with known length) — and the second arm (49-51) is likewise dead here since `new` then always succeeds. Two dead defensive arms; harmless, but they read as if construction could fail, which it cannot for this seam.
- Repo-level: the entire fuzz target crate is "dead" in the sense that it has never executed outside `cargo check` — recorded in Z-001, not counted again here.

## 7 Open questions

1. Who owns the "Lead's nightly wiring" for the fuzz campaign, and is there a tracked deadline? The target's honest note and README's LEAD-DECISION block (archive-2026-10.md:2066) defer it indefinitely; until run, every matrix row that cites `fuzz_demux_mp4` as evidence for hostile-input robustness is scaffolding-evidence.
2. Seed provenance: were the 4 committed seeds generated from tests/fixtures/generated/valid parents (copies) or freshly muxed? Sizes differ from the fixtures (13 703 B vs the fixture lane) — UNVERIFIED; if freshly muxed, the fuzz corpus and the fixture corpus cover two disjoint byte populations, which is fine, but then the "seed corpus = these fixtures" feasibility claim (subsystems.md:124) is false in the strict reading.
