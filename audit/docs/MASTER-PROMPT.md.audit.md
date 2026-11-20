# CLAIM AUDIT: docs/MASTER-PROMPT.md

| Field | Value |
|---|---|
| Auditor | W19-A01 |
| Date | 2026-11-16 |
| Claims extracted | 14 | REAL 8 | REAL-UNPROVEN 4 | SHALLOW 0 | STUB 0 | PAPER-ONLY 2 | FALSE 0 | UNVERIFIED 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict

MASTER-PROMPT is a prescriptive brief, not a status document — ~90% of it is rules, and the rules are the honesty vocabulary the rest of the doc layer borrows (verified-requires-automated-evidence, never-claim-without-evidence, mock/real labeling). Every empirically checkable anchor in it holds: the VLC 3.0.23 pin, the parity definition quoted verbatim by PARITY-REPORT, the monorepo layout matches the tree, the boundary-check tooling exists and even currently FAILS on a real boundary drift (which proves it works). The only unmet claims are the two program-level ones it defines as terminal conditions: "measure against VLC on the same files and hardware" (no such measurement exists anywhere in the repo) and the "project is done when" list. Those are PAPER-ONLY by nature — the brief states requirements the program has not yet satisfied. No false statements found. This is the standard document; everything else in the doc layer is audited against it.

## 2. Claim ledger

| # | Claim (verbatim ≤25 words) | Location (path:line) | Verdict | Evidence you gathered |
|---|---|---|---|---|
| 1 | "Parity is defined against a pinned baseline, VLC 3.0.23 (desktop, released 2026-01-08)" | docs/MASTER-PROMPT.md:15-16 | REAL-UNPROVEN | docs/baselines/vlc-3.0.23.md:43 + vlc-3.0.23-windows.md:20 pin 2026-01-08 with sourcing (news anchor); external verification impossible from sandbox. |
| 2 | "Parity is the share of inventory features in docs/feature-matrix.toml that are verified by automated evidence. Never claim 'exactly like VLC' …" | docs/MASTER-PROMPT.md:17-19 | REAL | Binding definition; quoted verbatim at PARITY-REPORT.md:15-16; the repo vocabulary (verified=0/252 reported) follows it. |
| 3 | Fixed decisions: engine 100% Rust; UI React+TS; shell Tauri; web wasm; typed command layer; config-driven | docs/MASTER-PROMPT.md:29-45 | REAL | Tree implements the shape: crates/ (Rust engine), ui/ (React+TS), apps/desktop-tauri, apps/web, crates/commands, product.toml+schemas. |
| 4 | "The project must never be described as all-Rust" | docs/MASTER-PROMPT.md:34-35 | REAL | README.md:6 complies verbatim ("never described as 'all-Rust'"). |
| 5 | "No cyclic crate dependencies … An automated boundary check runs in CI" | docs/MASTER-PROMPT.md:69-70 | REAL | check-architecture tooling exists and currently FAILS on real drift (lumen-cli's streaming dep missing from boundaries.toml allowlist — executed fact 2026-11-16): the check runs and catches. "Runs in CI" itself is GitHub-side, but the local gate records it (worklog 7.w21a check-architecture 5/5 PASS then). |
| 6 | Monorepo layout block (crates product-config … testkit; tools; plugins; apps; ui) | docs/MASTER-PROMPT.md:82-100 | REAL | crates/ listing matches: product-config, localization, theme-tokens, commands, settings-schema, media-types, media-probe, input, demux, packet-queue, codec-api, codecs, playback-engine, media-clock, audio-engine, audio-output, video-engine, renderer-wgpu, subtitles, playlists, streaming, transcode, device-discovery, platform-{windows,linux,macos}, plugin-api, plugin-host-wasm, control-protocol, testkit. |
| 7 | "Every crate needs a real responsibility, a public interface, a dependency direction and tests. No empty scaffolds." | docs/MASTER-PROMPT.md:102-103 | REAL-UNPROVEN | 31 crates present with tests (worklog records per-crate suite results, e.g. platform crates 60 passed); not exhaustively re-verified per crate in this audit. |
| 8 | "CI fails on hardcoded user-visible strings, duplicated identity values, missing translation keys, schema violations and broken crate boundaries" | docs/MASTER-PROMPT.md:139-140 | REAL | The tooling exists and passes/fails on content: lint-literals + check-identity (worklog 7.w21a: 0 violations, 340/341 allowlisted) + check-architecture (currently FAILING on a real boundary drift). |
| 9 | "A plugin conformance kit is mandatory. Every plugin passes it, and every parser plugin has a registered fuzz target" | docs/MASTER-PROMPT.md:163-164 | REAL-UNPROVEN | Conformance suites exist (crates/demux container_conformance.rs — the mp4_h264_family_lane test passed in the executed run); fuzz/ directory with registry; "every plugin passes" not re-provable here beyond the executed workspace test run (1001 passed, 2 failed on an unrelated catalog pin). |
| 10 | "Acceptance test: automated screenshot diffs against the Figma exports at defined viewport sizes and DPI" | docs/MASTER-PROMPT.md:174-176 | REAL-UNPROVEN | Visual/e2e lanes wired (ui.yml; vitest 1705 passed executed); Chromium e2e/visual NOT RUN in this sandbox per executed facts. |
| 11 | "Measure against VLC 3.0.23 on the same files and hardware: startup, probe latency, time to first frame and sound, seek latency, …" | docs/MASTER-PROMPT.md:241-243 | PAPER-ONLY | No wall-clock Lumen-vs-VLC measurement exists anywhere in the repo (PERF-VS-VLC.md:9/§1 admits; KNOWN-LIMITS.md:18 admits). The requirement is real; the measurement program behind it never materialized. |
| 12 | "Differential tests: compare outputs against VLC 3.0.23 run externally as an oracle" | docs/MASTER-PROMPT.md:280-281 | REAL | The differential ran for real: docs/corpus/vlc-oracle-run-20261109.json (29/35 bit-exact, 6 f32 FAIL). |
| 13 | Status vocabulary + verified definition ("tested on each claimed platform, documented with its limits, and non-regressing") | docs/MASTER-PROMPT.md:358-362 | REAL | The matrix uses exactly this vocabulary (not_started/in_progress/implemented/verified/blocked); verified=0 rows at HEAD is consistent with the strict definition. |
| 14 | "The project is done when: … CI is merge-blocking on all platforms, and the parity report states exactly what is verified…" | docs/MASTER-PROMPT.md:364-369 | PAPER-ONLY | Definition of done; several clauses unmet at HEAD (web build plays real media — web is 0/252; CI merge-blocking — unverifiable/contradicted by release.yml:26; Figma-tolerance matching — not re-run). |

## 3. Contradictions

| This doc says | Other source says (path:line) | Which is true (proof) |
|---|---|---|
| "Measure against VLC 3.0.23 on the same files and hardware … Report that honestly" (MASTER-PROMPT.md:241-247) | PERF-VS-VLC.md:9 "No wall-clock Lumen-vs-VLC timing number exists anywhere in this repo"; KNOWN-LIMITS.md:18 | Both — the brief mandates a measurement program the repo never executed; the perf doc honestly reports the omission. The requirement is unmet, not misrepresented. |
| "A phase is done only when its gate passes with recorded evidence on each claimed platform" (MASTER-PROMPT.md:348-349) | ROADMAP.md:24-26 Phase 5 "PASS WITH REGISTERED CAVEATS"; ROADMAP.md:41-50 Phase 7 "COMPLETE" via a gate that ran NOT PASSED and was closed by later merges | The brief's strictness exceeds the roadmap's practice — phases were closed with local-evidence caveats; the roadmap is the weaker standard, applied honestly (each gate doc says so). |

## 4. Fabrication/inflation indicators

None found. This document makes almost no empirical claims — it is the ruleset. Its one measurable empirical anchor (the VLC baseline pin) is properly sourced in-tree, and its "never claim without evidence" rule is the very standard that exposes the stale/false claims catalogued in the other seven audits. The gap between this brief and the doc layer is not fabrication in the brief — it is the brief's performance-vs-VLC mandate (§11) that remains a plan.
