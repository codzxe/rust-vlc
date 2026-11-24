# AUDIT: fuzz/fuzz_targets/fuzz_demux_aiff.rs
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / functions | 20 / 1 fuzz_target shim |
| File verdict | REAL — thin shim over a real harness; campaign lane UNWIRED (honest) |
| Reachable from shipped entry point? | NO — fuzz/ workspace, nightly-only campaign. Harness exercised on stable by crates/demux/tests/wav_aiff_fixtures.rs:715 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; rg harness fn + stable consumer; bin check |

## 1. One-paragraph verdict
Same honest-shim shape: dispatches to `demux::aiff::fuzz::fuzz_demux_aiff`
(fn verified at crates/demux/src/aiff/fuzz.rs:47). Notably, the harness is
NOT merely registered — it is executed on stable in the demux test suite
over every committed AIFF fixture and derivative
(wav_aiff_fixtures.rs:715 calls `demux::aiff::fuzz::fuzz_demux_aiff(&bytes)`),
including my audited aiff_pcm_s16be_44k1_st__flip16 bytes.

## 2. What actually works (verified)
- Shim body (18-20, verbatim):
```
fuzz_target!(|data: &[u8]| {
    demux::aiff::fuzz::fuzz_demux_aiff(data);
});
```
- Harness exists: `crates/demux/src/aiff/fuzz.rs:47: pub fn fuzz_demux_aiff(data: &[u8])`.
- Stable-lane execution: `crates/demux/tests/wav_aiff_fixtures.rs:715: demux::aiff::fuzz::fuzz_demux_aiff(&bytes);`.
- Registry gate PASS (baseline full_gate.txt:57-58); bin `fuzz_demux_aiff` at fuzz/Cargo.toml:97; corpus fuzz/corpus/fuzz_demux_aiff/ present (3 seeds, os.listdir earlier).

## 3. Findings
None (fn + bin + gate + corpus + stable consumer all verified).

## 4. Per-function table (20-line file)
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| header | 1-12 | REAL (accurate) | |
| shim | 18-20 | REAL | dispatches to verified harness |

## 5. Test honesty
N/A (fuzz target); stable-lane proof in the demux suite (baseline green).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "harness body is the in-crate demux::aiff::fuzz module" | :3-4 | REAL | crates/demux/src/aiff/fuzz.rs:47 |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
libfuzzer-sys, demux path dep. Nightly campaign only.

## 9. Open questions
None.
