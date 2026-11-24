# AUDIT: fuzz/fuzz_targets/fuzz_codec_alac.rs
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / functions | 20 / 1 fuzz_target shim |
| File verdict | REAL — thin shim over a real harness; campaign lane UNWIRED (honest) |
| Reachable from shipped entry point? | NO — fuzz/ workspace, nightly-only campaign. Harness proven on stable by crates/codecs/tests/alac_fuzz_survival.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; rg harness fn; fuzz/Cargo.toml bin check |

## 1. One-paragraph verdict
Identical honest-shim shape to fuzz_codec_pcm: dispatches to
`codecs::audio::alac::fuzz::fuzz_codec_alac` (fn verified at
crates/codecs/src/audio/alac/fuzz.rs:55), bin declared at
fuzz/Cargo.toml:209, corpus present
(fuzz/corpus/fuzz_codec_alac/ — 5 seeds incl. cookie_badch/cookie_badver
hostile cookies, audited sibling files). Header states the wiring status
without inflation.

## 2. What actually works (verified)
- Shim body (18-20, verbatim):
```
fuzz_target!(|data: &[u8]| {
    codecs::audio::alac::fuzz::fuzz_codec_alac(data);
});
```
- Harness exists: `crates/codecs/src/audio/alac/fuzz.rs:55: pub fn fuzz_codec_alac(data: &[u8])`.
- Registry gate PASS (baseline full_gate.txt:57-58); bin `fuzz_codec_alac` at fuzz/Cargo.toml:209 (rg verified).

## 3. Findings
None (same verification set as the pcm target: fn + bin + gate + corpus all present).

## 4. Per-function table (file is 20 lines — table waived by size, included for completeness)
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| header | 1-12 | REAL (accurate) | wiring status matches fuzz/README.md |
| shim | 18-20 | REAL | dispatches to verified harness |

## 5. Test honesty
N/A (fuzz target); stable-lane proof lives in the codecs crate's alac fuzz-survival suite.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "harness body is the in-crate codecs::audio::alac::fuzz module (panic-free by construction)" | :3-5 | REAL-UNPROVEN | fn exists; panic-freedom exercised by in-crate sweeps (baseline green) |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
libfuzzer-sys, codecs path dep. Nightly campaign only.

## 9. Open questions
None.
