# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__garbage0.avi
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 19487 (binary; 429,612 bytes) / n/a |
| File verdict | REAL (data fixture) |
| Reachable from shipped entry point? | NO (test-only fixture, consumers are cargo tests) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256/md5 batch -> 429612 B, sha256 d479e132fb85a1ac…, md5 9867aee7b328c1aa…; rg "garbage0" tests/ tools/ crates/; python3 byte-prefix compare |

## 1. One-paragraph verdict
Machine-generated class-B malformed fixture: byte-identical copy of the valid
AVI parent (425,516 B) plus exactly 4,096 bytes of seeded pseudo-random garbage
appended past the declared RIFF extent (RIFF size field 0x00067e24). Generated
by xtask, consumed by five test files that assert bounded recovery. The
fixture itself is exactly what its name and generator promise; the honest
caveat (which the repo itself documents) is that garbage past a declared
extent is a weak mutation — the demuxer never reads it, so these tests prove
almost nothing about parser hardening.

## 2. What actually works (verified)
— python3 prefix compare: first 425,516 bytes identical to
tests/fixtures/generated/valid/avi_mjpeg_pcm.avi; extra 4,096 bytes contain all
byte values 0x00-0xff (seeded garbage, not zero-fill). Matches generator
`garbage_derivative` (tools/xtask/src/cmd/fixtures.rs:2572-2586):
```
fn garbage_derivative(parent: &GeneratedFile) -> Result<Derivative, String> {
    let mut rng = parent_prng(&parent.bytes);
    let mut bytes = parent.bytes.clone();
    bytes.extend(garbage_block(&mut rng, GARBAGE_LEN));
```
— Consumers (rg, all cite the file by exact relative path):
crates/demux/tests/fixtures_class_b.rs:318
(`garbage0_appended_past_extent_is_invisible`, asserts
`expect.size_bytes == 425_516 + 4_096` — consistent with the bytes on disk),
crates/codecs/tests/fuzz_survival.rs:24, crates/media-probe/tests/fixtures_matrix.rs:376+549,
crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:119.
Expectation file tests/fixtures/expectations/malformed/avi_mjpeg_pcm__garbage0.avi.toml exists.

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/fixtures_class_b.rs:318-323 (consumer honesty)
Code: `fn garbage0_appended_past_extent_is_invisible() { ... assert_eq!(expect.size_bytes, 425_516 + 4_096);`
What is wrong: the mutation under test (append past declared RIFF extent) is
invisible by construction — a size-field-respecting demuxer never reaches the
garbage. The repo admits this (comment at fixtures_matrix.rs:28 "garbage0 is
an append, so the honest verdict is the parent's") yet ships a dedicated test
that can only fail on size accounting, not on parser behavior.
Why it matters: padded "malformed corpus" coverage numbers overstate the
robustness actually exercised. What correct looks like: fold garbage0 into the
parent expectation, or flip bytes *inside* the extent.
Confidence: HIGH — Verification: read + executed byte-compare.

## 4. Dead / duplicated / unreachable code
n/a (data file). Matrix evidence: cited in docs/corpus/vlc-diff-report-20261023.md
only via the parent; docs/soak/raw scenarios use the valid/trunc3/noidx0
siblings, not garbage0.
