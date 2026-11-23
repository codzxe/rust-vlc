# AUDIT: crates/demux/src/mp4/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W05-A10 |
| Date | 2026-11-23 |
| Lines / functions | 84 / 1 fn + 1 helper + 3 consts |
| File verdict | REAL (harness real + exercised; coverage has a named hole) |
| Reachable from shipped entry point? | The harness itself: NO production caller (by design — a fuzz entry); reached by `cargo test` (mp4/tests.rs:1077-1111 sweeps, tests/mp4_fixtures.rs:242-249) and by the cargo-fuzz shim `fuzz/fuzz_targets/fuzz_demux_mp4.rs` (campaign NOT wired — see F-003). The demuxer it drives IS shipped: apps/cli/src/containers/mod.rs:65 `"mp4" => mp4::open` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/demux-036dd705535b2fb3 mp4::tests::fuzz` -> 2 passed, exit 0; `./target/debug/deps/mp4_fixtures-40ff1b8beab65767` -> 5 passed, exit 0; python3 byte-scan of 39 mp4/mov/3gp fixtures + 4 campaign seeds for `co64`; rg for ceiling constants in mp4/mod.rs |

## 1. One-paragraph verdict
The harness does what its doc says and its bounds claims are all real (verified against mp4/mod.rs: `MAX_MOOV_BYTES` 16 MiB enforced at :767, `MAX_MOOF_BYTES` 1 MiB at :117, `MAX_SAMPLE_BYTES` 16 MiB with the 2-slot pool at :109/:614, `MAX_FRAGMENT_SCAN_BOXES` 65,536 at :129/:717/:849; event budget shared across the main drain and post-seek drains). It is genuinely executed on stable: two deterministic mutation sweeps and 14 malformed ffmpeg-derived fixtures run through it (both test binaries run, all green). The hostile-focus verdict on "fuzz coverage vs the real box grammar": one real box the demuxer implements — `co64`, the 64-bit chunk-offset table mandatory for >4 GB files — receives zero input through this harness and zero valid input anywhere in the repository (39 fixtures byte-scanned, 0 contain co64; the only co64 test restamps an stco with invalid 4-byte entries and bypasses the harness). Second, the seek probes can only ever target 0–2.55 s (last three bytes × 10 ms), so late-seek arithmetic in longer files is unfuzzed. Third, the coverage-guided campaign has never executed — honestly documented as "SCAFFOLDED, NOT WIRED".

## 2. What actually works (verified)
— verified by `./target/debug/deps/demux-036dd705535b3 mp4::tests::fuzz` -> `2 passed; 0 failed` (truncation prefixes, stride bit-flips 37/53, 512-byte garbage tail, through `fuzz_demux_mp4`).
— verified by `./target/debug/deps/mp4_fixtures-40ff1b8beab65767` -> `5 passed; 0 failed` (incl. `malformed_mp4_fixtures_survive_the_fuzz_harness`: 14 derivatives — h264_aac flips/garbage/truncs, qt, 3gp h263, chapters, edts, frag).
— verified by rg: every bounds constant cited in the module doc exists and is enforced in mp4/mod.rs (line numbers above).
— verified by reading fuzz/fuzz_targets/fuzz_demux_mp4.rs: the libFuzzer shim calls `demux::mp4::fuzz::fuzz_demux_mp4(data)` — one line of logic, no divergence between target and harness.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/mp4/fuzz.rs:45-84 (harness input surface) vs crates/demux/src/mp4/mod.rs:1278-1287 (the unexercised path)
Code (fuzz.rs:66-71):
```
    // Seek probes at data-derived targets.
    for probe in 0..FUZZ_SEEK_PROBES {
        let at = data
            .get(data.len().saturating_sub(probe + 1))
            .map_or(0, |b| u64::from(*b) * 10_000_000);
        if mp4.seek(span(at)).is_ok() {
```
What is wrong (the co64 half of the coverage claim): the demuxer implements `co64` (64-bit chunk offsets — the box every >4 GB movie carries), but nothing ever feeds that path through fuzzing or with valid bytes. Evidence, all executed: (a) python3 byte-scan of all 39 mp4/mov/3gp fixtures under tests/fixtures/generated/ — 0 contain `co64`, all carry `stco` or nothing; (b) the deterministic sweeps build video-only stco files (test_support has no co64 builder; rg: no `co64` in test_support.rs); (c) the campaign seed corpus fuzz/corpus/fuzz_demux_mp4/ has 4 seeds, 0 contain co64; (d) the single co64 test in the repo (tests.rs:988-1012) restamps an stco header as "co64" leaving 4-byte entries — deliberately invalid, ceiling-only, and not through this harness. The implementation (mod.rs:1278-1287: count at +4, 8-byte BE entries at +8+i*8, MAX_SAMPLES clamp + damage note) is correct by inspection — and a regression in it would be caught by nothing. What correct looks like: one valid co64 fixture in the sweep set (or a builder arm) plus a seed.
Confidence: HIGH — Verification: ran (byte-scans, sweeps, fixtures) + read
F-002 [S4]
Where: crates/demux/src/mp4/fuzz.rs:67-71 (quoted above)
What is wrong: The seek targets derive from the last three bytes × 10_000_000 ns — a 0..2.55 s window. For any file longer than 2.55 s the probes never exercise late landings (deep binary-search over the sample tables, late tfdt conversion, end-of-media clamping); mp4's seek refuses targets past the extent (mod.rs:2071-2076), so past-extent refusal is fuzzed only for files shorter than 2.55 s. The doc's "probe seeks at data-derived targets" is delivered, but the derivation cannot reach the interesting half of the timeline. What correct looks like: derive targets from 8 bytes (e.g. last 8 as u64 ns, or byte × duration/255).
Confidence: HIGH — Verification: read (arithmetic) + ran (sweeps confirm probes execute)
F-003 [S4]
Where: crates/demux/src/mp4/fuzz.rs:1-19 (module doc) + fuzz/README.md ("Status: SCAFFOLDED, NOT WIRED — one Lead decision outstanding")
Code:
```
//! - Event budget: hard cap on every loop.
//! - Structural reads are ceiling-checked inside the demuxer
//!   (`moov`/`moof` ≤ 16 MiB/1 MiB; samples ≤ 16 MiB each through
//!   the 2-slot pool).
```
What is wrong: Not a code defect — the coverage reality. The coverage-guided libFuzzer campaign has never executed (nightly-toolchain Lead decision pending per fuzz/README.md); what "fuzz coverage" actually means today is 2 deterministic sweeps + 14 fixed malformed fixtures + 4 seeds, all run on stable. The docs are honest about this (the target file: "until then this package compiles on stable ... the harness logic is proven by the in-crate deterministic hostile-input sweeps"), so this is recorded as a bounded-coverage fact, not fraud. Consequence: no coverage guidance has ever steered input toward uncovered boxes (F-001's co64 being the concrete instance).
Confidence: HIGH — Verification: read (README + target file; no corpus artifacts beyond the 4 hand-made seeds)

## 4. Per-function table (file < 100 LOC; table given for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FUZZ_EVENT_BUDGET / FUZZ_POST_SEEK_DRAIN / FUZZ_SEEK_PROBES | 26-33 | REAL | — | budget shared across phases (correct accounting at :57/:73) |
| span | 35-41 | REAL | — | ns base; `unwrap_or` fallback is dead-defensive (TimeBase::new(1,1e9) is Ok) |
| fuzz_demux_mp4 | 45-84 | REAL | S3/S4 | bounds real; seek-probe window + co64 coverage holes (F-001/F-002); Err-swallowing matches the documented contract |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mp4::tests::fuzz_lite_mutations_never_panic (tests.rs:1077) | YES for panic/abort/OOM only | absence-of-panic is the assertion; a wrong-value regression passes by contract ("nothing else" — the conformance suites own values) |
| mp4::tests::fuzz_lite_fragment_mutations_never_panic (tests.rs:1098) | YES for panic/abort/OOM only | same; fragmented shape (moof/traf/trun) |
| tests/mp4_fixtures.rs malformed_mp4_fixtures_survive_the_fuzz_harness (242) | YES for panic/abort/OOM only | 14 fixed derivatives; breadth real (h264_aac+esds, qt, 3gp h263, chapters, edts, frag), depth nil (no mutation engine) |
| (no test in fuzz.rs itself) | N/A | harness file carries no #[test]; its sweeps live in tests.rs — fine, noted for the inventory |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:6-8 | REAL | harness swallows Err by design (:62, :78); sweeps/fixtures ran green |
| "moov/moof ≤ 16 MiB/1 MiB; samples ≤ 16 MiB each through the 2-slot pool" | fuzz.rs:15-17 | REAL | mod.rs:113/:767, :117, :109/:614 (rg) |
| "fragment scan bounded by MAX_FRAGMENT_SCAN_BOXES" | fuzz.rs:18-19 | REAL | mod.rs:129, enforced :717, :849 |
| "probe seeks at data-derived targets" | fuzz.rs:5-6 | SHALLOW | real but 0–2.55 s window only (F-002) |
| "proven by the in-crate deterministic hostile-input sweeps" (campaign status) | fuzz target file + fuzz/README.md | REAL | 2 sweeps + 14 fixtures executed (commands above); campaign itself never run |

## 7. Dead / duplicated / unreachable code
- None in this file. (The `span` fallback base is unreachable defensive code — noted, not counted.)

## 8. Dependencies & flags
- media_types + plugin_api + super (Mp4Demuxer, test_support::mem_stream). `pub mod fuzz` (mod.rs:100) — compiled in every demux build including release; no feature gate. No unsafe, no unwrap/expect outside the dead-defensive `unwrap_or` fallback.

## 9. Open questions for the next agent
- mp4/mod.rs owner: the co64 path (:1278-1287) has zero valid-input coverage repo-wide (F-001) — add a fixture before touching that code.
- fuzz/ owner: when the nightly wiring lands, seed corpus should include a co64 file and a >2.55 s file so the seek probes can reach late landings.
- tests.rs:988's restamped co64 relies on `u32be` returning 0 for short entries — confirm the damage-note expectations still hold if co64 parsing ever adds strict truncation (currently `unwrap_or(0)` swallows short tables silently: mod.rs:1282).
