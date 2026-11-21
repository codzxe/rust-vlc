# AUDIT: crates/demux/src/aiff/fuzz.rs (lines 1-189, whole file)
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-11-21 |
| Lines / functions | 189 / 10 items (harness, 3 constants, Scrambler + 2 methods, 4 tests) |
| File verdict | REAL (with one false-coverage test) |
| Reachable from shipped entry point? | Harness: reachable as a fuzz target (`fuzz/fuzz_targets/fuzz_demux_aiff.rs:18-20` calls it via libfuzzer_sys) and from the integration corpus (`crates/demux/tests/wav_aiff_fixtures.rs:715`); the demuxer under test is reachable via `lumen open x.aiff|aif|aifc` (probe AiffSignature → containers aiff arm). Baseline: all 4 in-crate tests ok (`orchrun-test-workspace-nofailfast.log.txt:1531-1538`); registry gate PASS, 35 targets (baseline INDEX #18) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | full Read of aiff/fuzz.rs + aiff/test_support.rs (layout ground truth); byte-offset arithmetic for the adversarial patches hand-verified against `minimal_aiff`; Read of `fuzz/fuzz_targets/fuzz_demux_aiff.rs`; `grep -rn "fuzz_demux_aiff"` → shim + integration caller; `grep -n` of the wav/avi fuzz twins' `adversarial_size_fields_survive` (cross-ref); `sed -n` verification of every quote; baseline greps |

## 1. One-paragraph verdict
The AIFF hostile-input harness is real, bounded and honest about its contract (absence-of-panic, errors swallowed by design): the event budget, post-seek drains and rate guard are all correctly placed, the deterministic scrambler is a sound xorshift64*, and all four in-crate tests executed green in the baseline. Two defects: the `adversarial_size_fields_survive` test claims to patch the SSND size field but its offset arithmetic forgot the 8-byte `FORM`+`AIFF` prefix — both `(34, …)` patches land inside the extended-80 sample-rate field, so the named adversarial site (extreme SSND declared sizes) is never exercised and a panic there would pass this test; and the module's "wiring status (honest)" section is stale — it says the cargo-fuzz target "until it lands", but `fuzz/fuzz_targets/fuzz_demux_aiff.rs` exists, is a real libFuzzer shim calling this exact harness, and is registry-gated green in the baseline.

## 2. What actually works (verified)
- Harness bounds (47-104): event budget checked before every event (62-64); `EndOfStream`/`Err` break; seek probes gated on `seekability() != None` (78-80), a `comm()` rate guard with `TimeBase::new` fall-through (81-86); post-seek drains capped (96-101). Seek-probe arithmetic (87-89) traced: `span/4 × quarter` over `0..=4` — five targets 0..span, no division by zero (constant 4).
- Deterministic scrambler (113-133): xorshift64* with the standard trimatrix constants; `flip_byte` bounds-checked; seeded per severity (151) — reproducible, no rand dependency (claim at 111-112 verified).
- `truncations_at_every_header_site_survive` (181-188): the cut list [0,1,8,11,12,16,20,26,30,34,38,42,100] verified against `minimal_aiff`'s real layout (FORM 0-7, AIFF 8-11, COMM 12-19, channels/frames/bits 20-27, ext80 28-37, SSND 38-45, samples 54+) — every boundary and mid-field site is genuinely hit.
- Integration corpus claim true: `wav_aiff_fixtures.rs:715` runs `fuzz_demux_aiff` over the mutated fixture family (grep-verified).
- Baseline: `aiff::fuzz::tests::*` 4/4 ok (nofailfast:1531-1538); demux crate 462/0/0.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/aiff/fuzz.rs:164-169
Code:
```rust
        for patch in [
            (4u32, u32::MAX), // FORM size
            (16, u32::MAX),   // COMM size (at 12 + 4)
            (34, 0),          // SSND size (COMM is 8 + 18 = 26 → SSND header at 30, size at 34)
            (34, u32::MAX),
        ] {
```
What is wrong: the offset arithmetic forgot the 8-byte `FORM`+size+`AIFF` prefix. `minimal_aiff` (test_support.rs:24-47) places `COMM` at 12 (chunk = 8 + 18 = 26 bytes, ending at 38), so `SSND`'s header is at **38** and its size field at **42** — not 30/34 as the comment computes (30 would only be right if COMM started at 4). Both `(34, …)` patches therefore overwrite bytes 34-37 — the low mantissa bytes of the 10-byte extended-80 sample rate (28-37) — not the SSND size. The test's stated intent ("the FORM size, the COMM size, the SSND size") is one-third undelivered: no pass ever feeds the demuxer a `u32::MAX` or zero SSND declared size.
Why it matters: the test exists to prove the demuxer survives extreme declared sizes; for the SSND site it proves nothing while claiming to. A panic on an oversized SSND declaration would pass this suite. The `(16, COMM size)` patch is correct only because its comment's "12 + 4" happens to match the real layout — the same broken arithmetic produced a right answer there and a wrong one here. The disease is propagated: the WAV twin (`crates/demux/src/wav/fuzz.rs:228`, `(28, …) // data size (at 24 + 4)`) patches the avg-byte-rate field of the fmt payload — the real data size sits at offset 40.
What correct looks like: `(42, 0)` and `(42, u32::MAX)` here, `(40, …)` in the wav twin — with offsets derived from the builder, not hand-remembered.
Confidence: HIGH — Verification: read (`minimal_aiff` layout traced byte-by-byte; both twins' comments and real layouts compared; sed-verified quotes).

### F-002 [S4]
Where: crates/demux/src/aiff/fuzz.rs:12-18
Code:
```rust
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file (`fuzz/fuzz_targets/fuzz_demux_aiff.rs`)
//! is a Lead-owned dependency decision (`libfuzzer-sys`), exactly as
//! the AVI/WAV cards recorded; until it lands the harness logic is
//! proven by the deterministic in-crate sweeps here and by the
//! integration corpus run over the mutated fixture family.
```
What is wrong: the target file has landed. `fuzz/fuzz_targets/fuzz_demux_aiff.rs` exists, is a real `libfuzzer_sys::fuzz_target!` shim whose body calls `demux::aiff::fuzz::fuzz_demux_aiff(data)` (shim lines 18-20), and the registry gate that pins target-file ↔ registry consistency passed in the baseline (INDEX #18: "35 targets, diff empty"). What has *not* happened is a libFuzzer campaign run (the shim's own comment: "building the campaign needs the Lead's nightly wiring") — the module doc conflates the file's existence with the campaign's execution.
Why it matters: a section headed "wiring status (honest)" that misstates the wiring status is worse than no status section; a reader concludes the shim is absent when it is present-but-unexercised.
What correct looks like: "the shim exists and compiles on stable; no libFuzzer campaign has been run (nightly wiring pending); meanwhile the harness logic is proven by …".
Confidence: HIGH — Verification: read (shim file read; baseline INDEX #18 cited; no baseline run builds the fuzz package — campaign status UNVERIFIED-BY-RUN, consistent with my reading).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FUZZ_EVENT_BUDGET` / `FUZZ_POST_SEEK_DRAIN` / `FUZZ_SEEK_PROBES` | 34-42 | REAL | — | note: `FUZZ_SEEK_PROBES = 4` documented as "number of seek probes" but the loop `0..=FUZZ_SEEK_PROBES` runs 5 (0%,25%,50%,75%,100%) — off-by-one doc, harmless |
| `fuzz_demux_aiff` | 47-104 | REAL | — | bounds all verified (§2); errors swallowed by documented contract |
| `Scrambler::next_u64` | 116-123 | REAL | — | correct xorshift64* |
| `Scrambler::flip_byte` | 125-132 | REAL | — | empty-slice guard; modulo indexing safe |
| test `empty_and_tiny_inputs_survive` | 135-141 | REAL | — | no-panic contract; empty/2-byte/empty-FORM/4 KiB 0xff |
| test `scrambled_structured_files_survive` | 143-157 | REAL | — | 64 severities, deterministic; assertion is absence-of-panic (the honest contract) |
| test `adversarial_size_fields_survive` | 159-178 | REAL | S3 | F-001: SSND-size patches miss the field entirely |
| test `truncations_at_every_header_site_survive` | 181-188 | REAL | — | cut list verified against the real layout — every site genuine |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_and_tiny_inputs_survive | YES (panic/hang only) | the contract is absence-of-panic; a typed error is a pass — by design, documented |
| scrambled_structured_files_survive | YES (panic/hang only) | same contract; 64 deterministic severities |
| adversarial_size_fields_survive | YES for FORM/COMM sites; **NO for the SSND site** | F-001: the SSND patches never reach the SSND size field |
| truncations_at_every_header_site_survive | YES | real cut sites verified |
| Baseline execution | — | 4/4 ok (nofailfast:1531-1538) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:6-10 | REAL | the harness body swallows errors (73, 95-102); the in-crate tests assert only survival |
| "until it lands the harness logic is proven by the deterministic in-crate sweeps here and by the integration corpus run" | fuzz.rs:14-18 | FALSE (first half) / REAL (second half) | the target file exists (shim read; registry PASS baseline #18); the integration caller verified (wav_aiff_fixtures.rs:715) |
| "Payload memory: the demuxer's own ceilings apply — this harness adds none" | fuzz.rs:24-27 | REAL | harness allocates only per-event packets; no unbounded buffers in 47-104 |

## 7. Dead / duplicated / unreachable code
- None in this file; the `(34, 0)` patch pair is worse than dead — it silently tests the wrong field (F-001).

## 8. Dependencies & flags
- `#[cfg(test)]` test module; harness itself compiled into the lib (called by the fuzz package and the integration corpus); no feature gates, no unsafe. The fuzz package (libfuzzer-sys) is workspace-external and was NOT built by any baseline run — its compile status is claimed ("compiles on stable", shim comment) but UNVERIFIED by executed evidence.

## 9. Open questions for the next agent
- A07 (wav/fuzz.rs): same offset disease — `(28, …) // data size` patches the fmt byte-rate field; real data size at 40 (verified against minimal_pcm_wav). File it there.
- A01 (avi/fuzz.rs): the third twin's `adversarial_size_fields_survive` (194+) builds its core inline — verify its patch offsets against that core before trusting it.
- Wave gate: none needed; all findings are static-arithmetic conclusive.
