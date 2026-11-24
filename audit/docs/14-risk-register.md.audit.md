# CLAIM AUDIT: docs/14-risk-register.md (338 lines + re-score log)
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 6 / 1 / 2 / 0 / 0 / 1 / 0 |

## 1. Blunt verdict
A risk register that does its job: risks are scored, owners named, triggers defined, and the
re-score log (§0.1) records a MATERIALIZED process risk (R-20 doc-governance debt) with the
exact defect pattern my own audits keep confirming (stale docs vs code). Two rows are stale
in the GOOD direction — R-06 "no usable pure-Rust Opus decoder" and R-01 "rust_h264
admission pending Lead" — both now landed in-tree. Its strongest claims (R-05 MIT closed,
R-28 AVI first vertical closed, ADR files exist) all verify.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | R-28 closed: "first vertical = AVI+MJPEG+PCM, all in-house, zero new deps" | §0.1 L31 | REAL | baseline rc-smoke.txt: headless-decode (AVI/MJPEG/PCM fixture) PASS; engine-info decoders: mjpeg, pcm_s16le |
| 2 | R-05 closed: "MIT via ADR-006, user decision" | L96 | REAL | LICENSE = "MIT License / Copyright (c) 2026 Lumen Media Player Contributors" (head -3); docs/adr/ADR-006-license-mit.md exists |
| 3 | R-01: "rust_h264 0.6.1 ... admission pending Lead" (2026-11-09 refresh) | L59, L92 | STALE (landed) | crates/codecs/Cargo.toml:69-73: "rust_h264 0.6.1, the Lead-admitted ... codec-h264-sw = [\"dep:rust_h264\"]"; apps/cli/src/pipeline_host.rs:65 imports H264SwDecoder — admission completed after the register's last re-score |
| 4 | R-06: "No usable pure-Rust Opus decoder is published ... open" | L97 | STALE (landed) | crates/codecs/src/audio/opus/mod.rs:1-2: "the `ruopus` engine behind our own OpusHead validation"; Cargo.toml:79 codec-opus = ["dep:ruopus"] |
| 5 | R-02: "VA-API hw lane drafted as ADR-013" | L60 | REAL | docs/adr/ADR-013-va-api-hw-decoder.md exists (ls docs/adr/) |
| 6 | R-20 materialized: "feature-matrix ... false header claim, ROADMAP two phases stale, ..." | L33, L111 | REAL | corroborated independently: my audits found stale docs (04 roadmap h264 row), parity 0/252, register's own remaining-gaps list |
| 7 | R-07 closed by "user attestation — open-source design, no terms restriction" | L98 | REAL-UNPROVEN | design/README.md attestation is documentation of a user decision — not independently verifiable from the repo; residual font check flagged honestly |
| 8 | R-17/R-18 clean-room + spec-conflict risks "mitigating" with triggers | L108-109 | REAL | the doc 03-vs-doc 04 first-vertical conflict it cites is real and was resolved by D-018 (R-28 row) — the register's own conflict-resolution trail works |
| 9 | R-23: "Pure-Rust software decoders will initially be slower than VLC's" | L114 | SHALLOW (unresolved) | docs/perf/raw/speed-x1a-20261024.json + speed-baseline exist (measured baselines present) — the honesty framework exists; verdict on the gap itself is doc 12's scope |
| 10 | "R-03 ... open" (patent risk) + "ADR-010 drafted, pending counsel sign-off" | L61, L94 | REAL | docs/adr/ADR-010-codec-patent-shipping.md exists; counsel sign-off cannot be repo-verified (external fact, honestly open) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| R-06 open: no usable pure-Rust Opus decoder | codecs: ruopus engine integrated behind codec-opus feature | code wins — risk resolved post-refresh, register stale |
| R-01 "admission pending Lead" | Cargo.toml:69-73 "Lead-admitted" rust_h264 | code wins — admission landed |

## 4. Fabrication / inflation indicators
None. This document is the repo's self-policing instrument; it records its own failures
(R-20) with specifics. The only defect is staleness lag on resolved risks (rows 3-4).

## 5. Matrix rows
N/A — the register feeds `blocked_reason` fields; hotkey-nav-disc's blocked_reason
(rows-235-247 audit) cites platform-disc-dvd, consistent with the register's DVD ADR
posture (R-03/R-14 style triggers).
Commands: ls docs/adr/; head LICENSE; rg codecs Cargo.toml (ruopus/rust_h264); sed opus
mod.rs:1-20; baseline rc-smoke.txt citation.
