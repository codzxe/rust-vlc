# AUDIT: tools/mem-audit/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / keys | 56 / 1 package, 1 bin, 7 path deps, workspace lints |
| File verdict | REAL — the manifest matches the tool it names; unsafe posture disclosed in-file |
| Reachable from shipped entry point? | NO — dev tool; workspace member (root Cargo.toml members includes "tools/mem-audit") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg workspace members (root Cargo.toml:41); baseline package run cite; dependency set vs walk.rs imports cross-check |

## 1. One-paragraph verdict
An honest manifest: std-only claim holds (zero external dependencies — the
seven `[dependencies]` are all in-repo path deps, exactly the engine set
walk.rs imports: codec_api, codecs, demux, input, media-probe, media-types,
plugin-api — cross-checked against walk.rs's `use` list, matches 7/7). The
 UNSAFE POSTURE header discloses the only sanctioned unsafe in the crate
(`unsafe impl GlobalAlloc` + `#[global_allocator]` in allocator.rs, raw
alloc micro-bench in selfcheck.rs) with the Lead-ratification trail and an
open [PROPOSAL: ADR]. The baseline compiled and ran the package's 9 tests
(cargo-test-workspace.txt:3278-3294).

## 2. What actually works (verified)
— Root workspace membership: root Cargo.toml:41 `members = ["crates/*",
"apps/*", "tools/xtask", "tests/integration/first-vertical",
"tools/mem-audit", "tools/sweep"]` — present.
— Baseline: cargo-test-workspace.txt:3273-3296 — PACKAGE lumen-mem-audit
compiled, 9 tests passed, exit_code=0.
— Dependency/import parity: walk.rs `use` statements (codec_api, demux,
media_types, plugin_api, input, media_probe, codecs) all appear in
[dependencies] — no undeclared dep, no unused dep at the manifest level.

## 3. Findings
F-001 [S4]
Where: tools/mem-audit/Cargo.toml:1-32 (header block)
Code:
# UNSAFE POSTURE (Lead flag, recorded for ratification): a counting
# GLOBAL allocator is impossible without `unsafe impl GlobalAlloc` +
# `#[global_allocator]` — there is no safe Rust surface for either.
What is wrong: the ADR is still a [PROPOSAL] (L29-32: "[PROPOSAL: ADR
'measurement instruments may carry unsafe impl GlobalAlloc' — Lead
decision.]") — the unsafe is sanctioned by a comment trail, not a ratified
ADR. Same deferral class as faults.rs F-002. The unsafe itself is documented
and scoped to allocator.rs/selfcheck.rs (consistent with walk.rs's
"no unsafe" module claim).
Confidence: HIGH — Verification: read.

## 4. Per-function table (keys)
| Key | Lines | Verdict | Notes |
|---|---|---|---|
| [package] | 35-40 | REAL | name/bin/description consistent with main.rs |
| [[bin]] | 42-44 | REAL | src/main.rs exists |
| [dependencies] | 46-54 | REAL | 7/7 parity with walk.rs imports |
| [lints] workspace = true | 56 | REAL | inherits workspace lint tables |

## 5. Test honesty
No tests in this file; the package's 9 tests ran green in baseline (§2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only: zero external dependencies (C5)" | Cargo.toml:15 | REAL | [dependencies] is 100% path deps |
| "no commands/playback-engine edge" | Cargo.toml:16-18 | REAL | dep list lacks both crates |
| "boundaries.toml [crates.lumen-mem-audit] entry: Lead union handoff" | Cargo.toml:32-34 | REAL-UNPROVEN | boundaries.toml not read in this audit (UNVERIFIED: file not in my assignment) |

## 7. Dead / duplicated / unreachable code
None (manifest).

## 8. Dependencies & flags
7 in-repo path deps; no features; edition 2024; publish=false.

## 9. Open questions for the next agent
- Does boundaries.toml actually carry the [crates.lumen-mem-audit] entry
  (claimed as a Lead handoff)?
