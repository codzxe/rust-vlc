# AUDIT: crates/theme-tokens/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A07 |
| Date | 2026-11-21 |
| Lines / functions | 34 / 9 config units (package ×5 fields, dependencies, lints, bin ×2 fields) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES, indirectly (its committed outputs ship): `themes/generated/tokens.css` + the verbatim UI bridge copy `ui/src/tokens/generated/tokens.css` are diff-gated byte-equal by tests/diff.rs:90-102; the crate itself is built in CI (ci.yml:577 `cargo build -p theme-tokens --target wasm32-unknown-unknown --locked`) and its `tokens-gen` bin is the subprocess the xtask `tokens` subcommand drives (tools/xtask/src/cmd/tokens.rs:54-62). NO crate depends on it at runtime (zero dependents — a codegen pipeline crate by design; the two Cargo.toml hits in commands/settings-schema are comments only). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read crates/theme-tokens/Cargo.toml, tests/diff.rs, tools/xtask/src/cmd/tokens.rs:40-80; ls crates/theme-tokens/{src,tests,src/bin} + themes/generated (all claimed files exist); rg "theme.tokens" in crates/commands + crates/settings-schema (comments only — no dependents); ls docs/dependencies/records/ (no theme-tokens.md); rg "serde_json" docs/dependencies/preaudit-foundation.md:25 (1.0.154 PASS) + Cargo.lock:894-895 (pinned 1.0.154); rg "tokens" .github/workflows/ci.yml + scripts/full_gate.sh (canary build yes; no regen step); rg "\[\[bin\]\]" tools/{sweep,mem-audit}/Cargo.toml (both have bins). |

## 1. One-paragraph verdict
Every factual claim in this manifest checks out against the tree, which is rarer than it should be in this repo. The two committed outputs exist, the drift test that the header names (tests/diff.rs) is real, byte-equality-based, covers a third file the header does not mention (the UI bridge copy), and carries its own sensitivity proof; the `tokens-gen` bin exists at the declared path and is exactly what the xtask `tokens` subcommand spawns (`cargo run --quiet -p theme-tokens --bin tokens-gen`); serde_json is pinned to the exact pre-audited version (1.0.154, preaudit-foundation.md:25, Cargo.lock:894-895). The only defects are cosmetic: a dangling pointer to a dependency-record file that does not exist, and a stale "only crate with a [[bin]]" canary rationale in ci.yml that this manifest's existence no longer makes unique.

## 2. What actually works (verified)
— tests/diff.rs:80-104 `committed_outputs_match_regeneration` re-parses `themes/tokens.json`, re-emits CSS+Rust via the crate's own `parse`/`emit_css`/`emit_rust`, and asserts byte equality against all three committed artifacts; diff.rs:106-126 `the_comparison_detects_drift` mutates one token value and proves the gate is not vacuous. This ran in baseline INDEX #3 (`cargo test --workspace`, 1681 passed / 1 unrelated failure), so the committed outputs provably match regeneration.
— tools/xtask/src/cmd/tokens.rs:54-62 spawns exactly `run --quiet -p theme-tokens --bin tokens-gen` — the manifest's "subprocess bridge" claim, verbatim.
— serde_json = "1.0.154" → Cargo.lock:894-895 `name = "serde_json" / version = "1.0.154"` and preaudit-foundation.md:25 lists serde_json 1.0.154 as pre-audit **pass** (MIT OR Apache-2.0, pure Rust) — pin precise and policy-compliant.
— ci.yml:577 builds this crate for wasm32 as the G12 canary — the manifest's crate is CI-reachable.

## 3. Findings

### F-001 [S4]
Where: crates/theme-tokens/Cargo.toml:15-17
Code:
```
# External dependency policy (doc 11): serde_json is pre-audited PASS
# (docs/dependencies/preaudit-foundation.md §1); record:
# docs/dependencies/records/theme-tokens.md.
```
What is wrong: The cited record file does not exist. `docs/dependencies/records/` contains 9 files (audio-output-cpal.md, claxon-0.4.3.md, cpal-0.18.2.md, filmcraft-dnx-0.9.0.md, font-stack-0.2.10.md, h264-software-decoder-audits.md, oxideav-ac3-0.0.11.md, oxideav-mpeg4video-0.1.8.md, rav1d-1.1.0.md, ruopus-0.1.2.md) — no theme-tokens.md, and no serde_json record either.
Why it matters: The comment names a compliance artifact that isn't there; a reader auditing dependency policy per doc 11 follows the pointer into a 404. (Substantively covered: preaudit-foundation.md:25 does list serde_json 1.0.154 as PASS, and allowlist.md:46 names theme-tokens as an allowed consumer — only the pointer is wrong.)
What correct looks like: point at `docs/dependencies/preaudit-foundation.md` §1 and `docs/dependencies/allowlist.md` (both real), or write the per-crate record the comment promises.
Confidence: HIGH — Verification: read (directory listing).

### F-002 [S4]
Where: crates/theme-tokens/Cargo.toml:32-34 (context: .github/workflows/ci.yml:534-536)
Code:
```
[[bin]]
name = "tokens-gen"
path = "src/bin/tokens_gen.rs"
```
What is wrong: Not a defect of this manifest, but the CI canary rationale attached to it is stale: ci.yml:534-536 justifies the wasm canary by "theme-tokens is the one workspace crate with a [[bin]]" — tools/sweep/Cargo.toml:39-41 and tools/mem-audit/Cargo.toml:42-44 also declare `[[bin]]` targets, so the claim is false today (it was true at Phase 1). Also, `cargo xtask tokens` (the regen entry this manifest names) appears in no CI gate or full_gate.sh — drift duty is carried entirely by tests/diff.rs in the test lanes, which is fine, but the "regeneration is idempotent (twice → empty git diff)" claim (line 7-8) has no executed evidence in the baseline.
Why it matters: Stale uniqueness claims and unexercised regen commands rot into false documentation.
What correct looks like: ci.yml canary comment updated to "the Phase-1 canary (historically the only [[bin]] crate)"; the idempotence claim is adequately covered by the passing byte-equality drift test, so no code change needed here.
Confidence: HIGH — Verification: read (both sibling manifests and ci.yml text).

## 4. Per-key table (config units)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header comments (card/doc refs, pipeline description) | 1-17 | REAL | S4 | Output paths, diff test, subprocess bridge, preaudit claim all verified; F-001 dangling record pointer |
| [package] name = "theme-tokens" | 19-20 | REAL | — | Matches `cargo run -p theme-tokens` in tokens.rs:58-59 |
| version = "0.1.0" | 21 | REAL | — | Crate-local version; product version lives in product.toml (no false conflation) |
| edition = "2024" | 22 | REAL | — | Supported by pinned toolchain 1.99.1 |
| publish = false | 23 | REAL | — | Correct for a workspace-internal pipeline crate |
| description | 24 | REAL | — | Accurate one-liner |
| [dependencies] serde_json = "1.0.154" | 26-27 | REAL | — | Exact pre-audited pin; lockfile agrees; sole external dep — consistent with "pre-audited foundation only" policy |
| [lints] workspace = true | 29-30 | REAL | — | Resolvable (crates/* member); baseline hygiene shows workspace lints inherited (INDEX hygiene note: 39/40 crates inherit) |
| [[bin]] name/path | 32-34 | REAL | — | File exists (311 lines); --check mode real (tokens_gen.rs:46); entry for xtask subprocess + CI wasm canary |

## 5. Test honesty (tests for this crate)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| crates/theme-tokens/tests/diff.rs committed_outputs_match_regeneration | YES | Byte equality of 3 committed artifacts vs fresh emission; any emit/parse change or stale artifact fails it. Ran green in baseline #3 |
| diff.rs the_comparison_detects_drift | YES (vacuity guard) | Proves a mutated source yields different output — the equality test can't be silently vacuous |
| crates/theme-tokens/tests/{pipeline,semantic_lint}.rs | not read by me (out of scope for the manifest audit) | UNVERIFIED — assigned to whoever audits those files |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Both outputs are COMMITTED and diff-tested (tests/diff.rs)" | Cargo.toml:7-8 | REAL | diff.rs:80-104; artifacts present; green in baseline #3 |
| "regeneration is idempotent (cargo xtask tokens, twice → empty git diff)" | Cargo.toml:7-8 | REAL-UNPROVEN | Not executed in baseline; implied by committed == regenerated (diff test green) |
| "tokens-gen binary is the entry point the xtask tokens subcommand shells out to" | Cargo.toml:10-13 | REAL | tokens.rs:54-62 spawns exactly that |
| "xtask cannot take a path dependency without an edit to its Lead-owned manifest" | Cargo.toml:11-13 | REAL | tools/xtask/Cargo.toml (Lead-owned, per its own header) declares no path deps on workspace crates — verified while auditing that file |
| "serde_json is pre-audited PASS" | Cargo.toml:15-16 | REAL | preaudit-foundation.md:25 |
| "record: docs/dependencies/records/theme-tokens.md" | Cargo.toml:16-17 | FALSE | File does not exist (F-001) |

## 7. Dead / duplicated / unreachable code
- None in this manifest. (The crate as a whole has zero runtime dependents — by design for a pipeline crate; its outputs reach the shipped UI via the bridge copy, so it is not dead.)

## 8. Dependencies & flags
- Single external dep serde_json 1.0.154 (pre-audited, exact-pinned, allowlisted for this crate). No features declared. No dev-dependencies (tests are std + in-crate). `[lints] workspace = true` inherits the root deny set.

## 9. Open questions for the next agent
- tools/xtask/src/cmd/tokens.rs:52 `run(_args: &[String])` discards ALL arguments — `cargo xtask tokens --check` silently ignores `--check` and always regenerates (writes). The check path exists only in the bin itself (`tokens-gen --check`, tokens_gen.rs:46) and in the test lane. Worth a finding by the tokens.rs auditor.
- tokens.rs:54-62 inner `cargo run` is not `--locked` — a regen inside a gate could in principle touch Cargo.lock; benign in CI (clean checkout) but inconsistent with the repo's --locked posture.
- ci.yml:406 admits the config-validator deliberately SKIPS themes/ ("outside card 1.E2's scope") — corroborates the product.schema.json F-002 finding from my first audit.
