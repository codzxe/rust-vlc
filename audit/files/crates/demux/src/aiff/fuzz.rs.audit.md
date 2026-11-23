# AUDIT: crates/demux/src/aiff/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Lines / functions | 190 / 6 (harness + Scrambler + 4 tests) |
| File verdict | REAL (harness logic real and executed; one test's byte-targeting is wrong) |
| Reachable from shipped entry point? | Harness body: NO (test/fuzz infra), but it is the body the cargo-fuzz shim fuzz/fuzz_targets/fuzz_demux_aiff.rs calls; the demuxer under test (AiffDemuxer) is shipped-reachable via CLI "aiff"/"aifc" lanes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p demux --lib aiff::fuzz` → 4 passed (exit 0); layout verification by byte-counting minimal_aiff (test_support.rs:24-52); read fuzz/fuzz_targets/fuzz_demux_aiff.rs + fuzz/Cargo.toml:97-98 |

## 1. One-paragraph verdict
The harness itself is a real, bounded absence-of-panic pass (construct → start →
budgeted drain → 5 seek probes with budgeted post-seek drains), honestly documented
as proving nothing but no-panic/no-hang/no-unbounded-alloc, and its four
deterministic tests execute and pass. But the most specific test lies about what it
feeds the demuxer: adversarial_size_fields_survive believes the SSND size field sits
at byte 34 ("COMM is 8 + 18 = 26 → SSND header at 30, size at 34") — the real
minimal_aiff layout puts the SSND magic at 38 and its size at 42, so both (34, 0)
and (34, u32::MAX) patches corrupt the last four bytes of the extended-80 sample
rate, and a wild SSND size is never actually exercised by this test. The module doc
also describes the cargo-fuzz target as not-yet-landed when the shim file exists
(what is missing is the nightly wiring, per fuzz/README.md).

## 2. What actually works (verified)
- `cargo test -p demux --lib aiff::fuzz` → 4/4 pass: empty/tiny/0xFF-4096 inputs,
  64 scrambles (0..64 flips, xorshift64*), 4 size patches, 13 truncation cuts.
- Loop bounds audited: drain capped by FUZZ_EVENT_BUDGET (200k) with the counter
  incremented before every next_event (a Recovered-storm terminates); post-seek
  drain capped at 64; seek probes are 5 fixed iterations. No unbounded loop.
- Seek-probe safety: TimeBase::new(1, rate) errors on rate 0 → early return (no
  divide-by-zero path); span uses saturating_mul.
- The cargo-fuzz shim fuzz/fuzz_targets/fuzz_demux_aiff.rs exists, calls exactly
  this harness, and is registered in fuzz/Cargo.toml:97-98 (campaign itself
  scaffolded-not-wired pending a Lead nightly decision, per fuzz/README.md).
- Truncation cuts at 38/42 do exercise the SSND header/size boundary (byte 38 =
  end of COMM = start of "SSND"; 42 = start of the size field) — that part of the
  coverage claim holds.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/aiff/fuzz.rs:160-169
Code:
```
        // Extreme declared sizes at the interesting sites: the FORM
        // size, the COMM size, the SSND size.
        let base = minimal_aiff(1, 44_100, 16, &[0x11, 0x22]);
        for patch in [
            (4u32, u32::MAX), // FORM size
            (16, u32::MAX),   // COMM size (at 12 + 4)
            (34, 0),          // SSND size (COMM is 8 + 18 = 26 → SSND header at 30, size at 34)
            (34, u32::MAX),
```
What is wrong: the offset arithmetic forgot the 4-byte "AIFF" form type. Actual
minimal_aiff layout (byte-counted from test_support.rs:24-52): FORM 0-3, size 4-7,
"AIFF" 8-11, COMM 12-15, size 16-19, channels 20-21, frames 22-25, bits 26-27,
ext-80 rate 28-37, "SSND" 38-41, SSND size 42-45, offset/blockSize 46-53, data 54+
(the Vec is allocated with_capacity(54 + len) — data start confirmed). So SSND size
lives at 42, not 34; both (34, …) patches corrupt the ext-80 rate mantissa. The
test's name and comment promise wild SSND sizes (0 and u32::MAX with a full header);
those inputs are never constructed. Why it matters: this is the only in-crate
wild-SSND-size fuzz case; the conformance suite (aiff/tests.rs) covers
declared>actual and absent SSND but not a maximal declared size with an intact
header, so the claimed adversarial coverage does not exist. What correct looks
like: patch at 42 (and 38 for the magic), or better derive offsets from the parsed
chunks. Confidence: HIGH — Verification: read + byte-level layout reconstruction;
executed the (passing, hence unproving) test.
F-002 [S4]
Where: crates/demux/src/aiff/fuzz.rs:12-18
Code:
```
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file (`fuzz/fuzz_targets/fuzz_demux_aiff.rs`)
//! is a Lead-owned dependency decision (`libfuzzer-sys`), exactly as
//! the AVI/WAV cards recorded; until it lands the harness logic is
```
What is wrong: the target file DOES exist (a real libfuzzer-sys shim calling this
harness, registered in fuzz/Cargo.toml:97-98); what has not landed is the nightly
campaign wiring (fuzz/README.md: "SCAFFOLDED, NOT WIRED"). The doc's phrasing reads
as if the file were absent. Confidence: HIGH — Verification: read both files.
F-003 [S4]
Where: crates/demux/src/aiff/fuzz.rs:41-42 vs 88
Code:
```
/// Number of data-derived seek probes per pass.
pub const FUZZ_SEEK_PROBES: usize = 4;
```
vs
```
    for quarter in 0..=FUZZ_SEEK_PROBES {
```
What is wrong: the constant documents 4 probes; the inclusive range runs 5 (quarters
0..=4 → 0/25/50/75/100%). Off-by-one between the documented count and the loop.
Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/demux/src/aiff/fuzz.rs:34-39
Code:
```
/// Maximum events drained per pass `[PROPOSAL: 200_000 — the WAV
/// twin's budget]`.
pub const FUZZ_EVENT_BUDGET: usize = 200_000;
```
What is wrong: unresolved `[PROPOSAL: …]` review markers baked into production
constants (also FUZZ_POST_SEEK_DRAIN `[PROPOSAL: 64]`) — the proposal was never
adjudicated in-tree. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_demux_aiff | 47-104 | REAL | — | bounded drain + seek probes; errors swallowed by documented contract |
| FUZZ_* consts | 34-42 | REAL | S4 | [PROPOSAL] markers; probe-count off-by-one (F-003/F-004) |
| Scrambler::next_u64 | 116-123 | REAL | — | xorshift64*, correct shape |
| Scrambler::flip_byte | 125-132 | REAL | — | random index+bit, empty-guard |
| empty_and_tiny_inputs_survive | 136-141 | REAL | — | executed |
| scrambled_structured_files_survive | 144-157 | REAL | — | 64 severities; executed |
| adversarial_size_fields_survive | 160-178 | REAL | S3 | SSND patches hit the wrong bytes (F-001); executed (vacuously for its claim) |
| truncations_at_every_header_site_survive | 181-188 | REAL | — | 38/42 boundaries covered; "every header site" slightly overstated (no 46/54 cuts) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_tiny_inputs_survive | YES (panic-only) | contract is absence-of-panic; a panic fails the test |
| scrambled_structured_files_survive | YES (panic-only) | same; mutations genuinely random-indexed |
| adversarial_size_fields_survive | panic-only; its named coverage (wild SSND size) is NOT delivered | F-001 |
| truncations_at_every_header_site_survive | YES (panic-only) | cuts do hit header boundaries |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:6-8 | REAL | code asserts nothing, swallows errors; honest |
| "until it lands the harness logic is proven by the deterministic in-crate sweeps" | fuzz.rs:16-18 | SHALLOW | target file exists; wiring pending (F-002) |
| "SSND size (… at 34)" | fuzz.rs:167 | FALSE | real offset 42 (F-001) |
| "the sound walk is monotone (positions only advance)" | fuzz.rs:22-23 | UNVERIFIED | property of AiffDemuxer (other files), not proven here |

## 7. Dead / duplicated / unreachable code
None. (Scrambler used by one test; harness used by 4 tests + the external shim.)

## 8. Dependencies & flags
No external deps; no feature gates; harness reachable from fuzz/ workspace via the
shim (fuzz/ is a separate workspace root, not part of release builds).

## 9. Open questions for the next agent
- Whoever audits aiff/tests.rs + chunks.rs: confirm a maximal SSND declared size
  with intact header is covered in conformance (declared>actual with small values
  is covered at tests.rs:286-291; u32::MAX may be uncovered everywhere — the exact
  case F-001 accidentally skips).
- fuzz/README claims a registry gate script (scripts/fuzz_registry_check.sh) pins
  registry-vs-implementation empty — scripts auditor should verify it runs in CI.
