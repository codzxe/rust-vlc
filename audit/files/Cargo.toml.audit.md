# AUDIT: Cargo.toml (workspace root)
| Field | Value |
|---|---|
| Auditor | W01-A10 |
| Date | 2026-11-21 |
| Lines / config keys | 67 / 13 (workspace: resolver, members, exclude; workspace.lints.rust: 4; workspace.lints.clippy: 7) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (manifest) — it governs every shipped crate: lumen-cli and desktop are members (`apps/*`), lumen-desktop-tauri is excluded with a dedicated CI build lane (.github/workflows/tauri-launch.yml:156) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | ls/tally of `crates/*/Cargo.toml` (31/31 dirs have manifests), `apps/*/Cargo.toml` (5/5); existence checks for explicit members + apps/desktop-tauri/Cargo.lock; rg `\[workspace\]` in tools/vta/*/Cargo.toml + tools/bench-vs-vlc (standalone ✓); rg lints in apps/desktop-tauri/Cargo.toml (inline mirror ✓); sed docs/10-testing-ci-quality.md §3.1-3.2 (lint block matches verbatim); version tally across all member manifests (drift evidence); rg Cargo.lock serde_json/toml (single locked versions); sed tools/xtask/src/cmd/check_architecture.rs:625-700 + toml_lite.rs:240-262 (multi-line array support) |

## 1. One-paragraph verdict
The root manifest does what it says: the membership policy is verifiable and complete (every glob target exists, every explicit member exists, the one exclusion carries its own lockfile, inline lint mirror, and a dedicated CI build lane), and the lint block matches doc 10 §3.1 verbatim and is actually enforced (baseline #5 clippy `-D warnings` CLEAN; baseline #11 check-architecture 39/39 lint inheritance). Two defects, both hygiene: the "reader does not support multi-line arrays" comment is factually stale (both xtask parsers handle multi-line arrays since W25), and the manifest has no `[workspace.dependencies]`, with the predictable result that member manifests drift between requirement strings (`serde_json = "1"` vs `"1.0.154"`, `toml = "1"` vs `"1.1"`) — currently harmless only because Cargo.lock pins single versions.

## 2. What actually works (verified)
- Membership completeness: 31/31 `crates/*` dirs have manifests (all members, including `_template`/package `template`, which is `publish = false` at crates/_template/Cargo.toml:19 and has zero dependents — rg over all member manifests: only self-references); 5/5 `apps/*`; `tools/xtask`, `tools/mem-audit`, `tools/sweep`, `tests/integration/first-vertical` all exist with manifests. Non-member tool trees are standalone by design: tools/vta/{ac3,dnx,mpeg4,ruopus,vp9} and tools/bench-vs-vlc each carry their own `[workspace]` table (rg: 1 match each).
- Exclusion honesty (ADR-004, lines 42-51): `apps/desktop-tauri/Cargo.lock` exists; its manifest mirrors the full 11-lint policy inline (apps/desktop-tauri/Cargo.toml:34-47 — identical to the root block); .github/workflows/tauri-launch.yml:156 builds it with `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml`. Baseline #2 (tauri build) failed only on missing sandbox GUI libs — an infrastructure deviation, not a manifest defect.
- Lint block vs doc 10 §3.1: byte-identical (4 rust + 7 clippy lints, same order) — root Cargo.toml:53-66 vs docs/10-testing-ci-quality.md:190-204.
- Enforcement: baseline #5 `cargo clippy --workspace --all-targets --locked -- -D warnings` exit 0; baseline #11 check-architecture reports "unsafe-lints 39/39 inherit" and 0 cycles; baseline #1 `cargo build --release --locked --workspace` exit 0 proves members ↔ Cargo.lock consistency.
- Version discipline: all non-app members at 0.1.0, shipped binaries (lumen-cli, desktop, lumen-desktop-tauri) at 0.1.3 — 41-member tally, no stray versions.

## 3. Findings

### F-001 [S4]
Where: Cargo.toml:37-40
Code:
```
# Single-line members array: the xtask TOML reader (check-architecture)
# does not support multi-line arrays — the salvage's multi-line form
# broke the arch gate; restored to the reader's shape with the same
# membership (the 2.E1 pre-announced edit, unchanged in content).
```
What is wrong: The stated constraint is FALSE against current code. check-architecture's own parser handles newlines inside arrays (`tools/xtask/src/cmd/check_architecture.rs:625-646`, `parse_array` — `skip_blank` "spans newlines + comments inside the array"; `parse_opaque_array` at 658-670 whose doc comment says "(multi-line arrays, W25)"), and the shared reader does too (`toml_lite.rs:246-257` "Multi-line continuation for arrays", unit test `parses_multiline_array_and_nested_tables` at 564-575).
Why it matters: The most-edited manifest in the repo asserts a parser limitation that was fixed in W25; editors keep flattening arrays for a reason that no longer exists, and the comment misattributes the W25 fix (the parser was fixed, not just the manifest).
What correct looks like: Past-tense the comment ("until W25 …; the reader now accepts multi-line arrays — single-line kept by convention") or delete it.
Confidence: HIGH — Verification: read (sed of both parsers + the passing unit test).

### F-002 [S4]
Where: Cargo.toml:35-51 (workspace tables — no [workspace.dependencies] section exists)
Code:
```
[workspace]
resolver = "2"
```
What is wrong: No `[workspace.dependencies]`/`[workspace.package]` centralization, and member manifests already drift in requirement strings for the same crates: `serde_json = "1.0.154"` (crates/commands, crates/control-protocol, crates/theme-tokens) vs `serde_json = "1"` (apps/web) vs `"1.0"` (one more member); `toml = "1.1"` (crates/plugin-api, crates/localization) vs `toml = "1"` (apps/cli, apps/control-server, crates/settings-schema, crates/plugin-host-wasm, crates/product-config); `thiserror = "2"` vs `"2.0"`.
Why it matters: Nothing structural prevents two members from requiring incompatible ranges, silently forking a crate into two graph versions (bloat + type mismatch errors at link sites). Today it is masked because Cargo.lock resolves each to a single version (verified: `serde_json` → one entry, 1.0.154), but the lockfile is regenerated on any `cargo update` and nothing in the gates compares requirement strings.
What correct looks like: Hoist shared externals into `[workspace.dependencies]` with `dep = { workspace = true }` in members, or add an xtask check that fails on divergent requirement strings per crate.
Confidence: HIGH — Verification: read (rg/tally across all 38 member manifests + Cargo.lock single-version check). No graph duplication today (lockfile cite).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| workspace.resolver = "2" | 36 | REAL | — | correct for edition-2024 members |
| workspace.members | 41 | REAL | — | globs + 4 explicit paths; every target exists (31 crates + 3 apps + 3 tools + 1 suite = 38 members); lock-consistent per baseline #1 |
| workspace.exclude (apps/desktop-tauri) | 51 | REAL | — | own lockfile ✓, inline lint mirror ✓ (apps/desktop-tauri/Cargo.toml:34-47), dedicated CI build lane ✓ (tauri-launch.yml:156); baseline #2 tauri build UNVERIFIED (sandbox libs) |
| workspace.lints.rust ×4 | 53-57 | REAL | — | matches doc 10 §3.1 verbatim; enforced (baseline #5) |
| workspace.lints.clippy ×7 | 59-66 | REAL | — | same; hygiene baseline: 15 `.unwrap()` literals, all in non-member tools/vta or xtask tests/lint_canary (hygiene-counts.txt) |

## 5. Test honesty
No tests in a manifest. Canary claim (line 32-33: "Inheritance is proven by the canary in crates/_template/src/lint_canary.rs") — the canary exists (hygiene-counts.txt lists 1 `.unwrap()` there); its actual assertion content is another agent's file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`members = ["crates/*"]` — later agents add crates by creating crates/<name>/ WITHOUT editing this file" | Cargo.toml:4-5 | REAL | 31 crate manifests, zero individual entries needed |
| "an excluded crate cannot resolve [lints] workspace = true" | Cargo.toml:9-10 | REAL | desktop-tauri therefore mirrors inline (34-47) — consistent resolution; exact cargo error text UNVERIFIED (no cargo runs allowed) |
| "Its own lockfile (apps/desktop-tauri/Cargo.lock) carries the tauri closure" | Cargo.toml:46-47 | REAL | file exists; baseline #2 confirms it resolves separately |
| "dedicated CI lanes compile it on webview-equipped hosts" | Cargo.toml:47-48 | REAL | tauri-launch.yml:156 `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml` |
| "the xtask TOML reader (check-architecture) does not support multi-line arrays" | Cargo.toml:37-38 | FALSE | F-001: check_architecture.rs:625-670 + toml_lite.rs:246-257 both support multi-line since W25 |
| Lint block "verbatim" per doc 10 §3.1 | Cargo.toml:31-32 | REAL | byte-identical comparison done (see §2) |

## 7. Dead / duplicated / unreachable code
None in this file. Downstream consequence tracked in file-1 audit: member apps/web is UNREACHABLE from shipped entry points (audit/files/apps/web/Cargo.toml.audit.md F-001).

## 8. Dependencies & flags
No deps at root (virtual manifest apart from lints). Member external-requirement drift: see F-002. Standalone trees (tools/vta ×5, tools/bench-vs-vlc) are intentionally outside — documented and verified.

## 9. Open questions for the next agent
- Who audits `tools/vta/**` and `tools/bench-vs-vlc` (standalone, never built by workspace gates — the 5 `.unwrap()` in tools/vta/ac3/src/main.rs are lint-exempt by construction)? Not in my assignment list.
- The lockfile agent should confirm zero dual-version crates (my spot check: serde_json, toml single entries).
