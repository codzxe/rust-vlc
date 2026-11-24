# AUDIT: crates/testkit/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W16-A04 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / keys | 26 / 4 sections ([package] 5 keys, [lints], comments) |
| File verdict | REAL (zero-dep dev crate; built + tested green in baseline) |
| Reachable from shipped entry point? | NO by contract — dev-deps only; 14+ consumers as [dev-dependencies] (rg: demux, codecs, playback-engine, streaming, media-probe, media-clock, audio-output, renderer-wgpu, transcode, plugin-api, platform-windows, apps/{cli,desktop,web}, first-vertical) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg consumers; baseline PACKAGE testkit L6198-6248 |

## 1. Verdict
The minimal manifest of the std-only test kit: name/version/edition-2024/description,
publish=false, workspace lints inherited, and NO [dependencies] table at all — the "Std-only:
zero dependencies of any kind" header claim is literally true in the manifest. The package
compiled and ran green in the orchestrator baseline (### PACKAGE testkit … exit_code=0,
cargo-test-workspace.txt:6198/6248; unit + e2e + doctest results cited in the alloc_count
audit of this run).

## 2. What actually works (verified)
- Zero-dependency claim: no [dependencies]/[dev-dependencies] tables exist in the file
  (read in full) — nothing to resolve, nothing to audit (C5 rule honored).
- Direction contract: no production crate's [dependencies] names testkit (spot-check
  consumers list — all hits are dev-dep contexts or testkit's own file); testkit itself
  depends on nothing, so the contract's "testkit DEPENDS ON production crates" half is
  currently vacuous (stronger isolation than stated, not weaker).
- publish=false blocks accidental crates.io publication.

## 3. Findings
F-001 [S4]
Where: crates/testkit/Cargo.toml:3-8
Code:
# Direction contract (MP §3.7, doc 01 §5): testkit DEPENDS ON production
# crates; NO production crate ever depends on testkit — it may appear only
# as a [dev-dependencies] entry. The boundary check that enforces this is
# card 1.E1 (cargo xtask check-architecture); until then the proof is
# the manifest grep recorded in the 1.C3 worklog entry …
What is wrong: the enforcement story is "a grep recorded in a worklog entry" — a
worklog is not a gate. If a production crate added testkit as a regular dependency
tomorrow, nothing in CI fails on it (UNVERIFIED whether xtask check-architecture now
exists and covers this — it was 'until then' at authoring time).
What correct looks like: the check-architecture step owning this rule.
Confidence: MEDIUM — Verification: read (manifest has no such gate reference).

## 4. What was checked and how
Manifest read in full; consumer rg across all Cargo.tomls (14 consumer files listed);
baseline package section (testkit exit 0, tests green); lint inheritance ([lints]
workspace=true matches the workspace lint table pattern seen in sibling crates).
No code executes from this file itself.
