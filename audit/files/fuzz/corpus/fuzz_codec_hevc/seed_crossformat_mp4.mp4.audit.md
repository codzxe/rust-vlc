# AUDIT: fuzz/corpus/fuzz_codec_hevc/seed_crossformat_mp4.mp4
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 13703 B / 3acc7ebae6f41c3194bab8f23f935e57e27adb81d61d8f2b99b8cd98124c7d8e |
| File verdict | REAL (as data); executed nowhere on stable |
| Reachable from shipped entry point? | NO — fuzz corpus seed |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |

## 1. Verdict (corpus-seed audit)
A real MP4 container (`ftyp isom`, moov+free+mdat, h264 video + aac audio tracks) deliberately seeded into the HEVC target's corpus as cross-format confusion material — it is byte-identical (sha256 verified) to the committed class-A fixture `tests/fixtures/generated/valid/mp4_h264_aac.mp4`. Its bytes are exercised on stable only through that fixture twin (mp4 fixtures suite); the corpus file itself is never read by any stable gate — the cargo-fuzz campaign that would consume it is not wired (nightly Lead decision outstanding).

## 2. What was checked and how
- batch.py: 13703 B, sha256 3acc7eba…, box walk ftyp/moov(1792)/free/mdat(11871).
- `sha256sum` both files -> identical hash: seed == tests/fixtures/generated/valid/mp4_h264_aac.mp4 (the "crossformat" seed is literally the h264+aac MP4).
- Consumers of the corpus path: `rg -l "fuzz/corpus"` -> scripts/fuzz_registry_check.sh (existence check only), scripts/quality/25_fuzz_campaign.sh (nightly, unwired), crates/streaming/tests/netfuzz_sweep.rs (comment only). Registry check run by me: exit 0 ("CLEAN — 35 targets, diff empty, corpora present").
- Twin exercised: crates/demux/tests/mp4_fixtures.rs (mp4_h264_aac lane) + h264 parser corpus tests — baseline passed (audit/runs/baseline/cargo-test-workspace.txt, demux + codecs suites ok).

## 3. Findings
F-001 [S2]
Where: fuzz/corpus/fuzz_codec_hevc/seed_crossformat_mp4.mp4 (no consumer reads this path)
Code (context, fuzz/Cargo.toml:5-7):
```
# This manifest is a *scaffold*: it makes the three previously-dead
# target files under fuzz_targets/ buildable by cargo-fuzz. It is NOT
# wired into any gate: building it requires a nightly toolchain
```
What is wrong: this seed's bytes reach the HEVC parser harness on stable only via the fixture twin under a different name; the corpus lane (libFuzzer re-feeding corpus seeds + mutations to `fuzz_codec_hevc`) never runs in any gate, so the seed contributes exactly zero executed coverage. Why it matters: the registry gate counts it as present ("corpora present") — presence masquerading as coverage. What correct looks like: a stable sweep that reads fuzz/corpus/* seeds into the in-crate harnesses (the netfuzz_sweep.rs pattern), or a wired nightly campaign.
Confidence: HIGH — Verification: ran (rg consumer map; twin sha verified).
