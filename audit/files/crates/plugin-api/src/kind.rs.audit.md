# AUDIT: crates/plugin-api/src/kind.rs
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2025-06-27 |
| Lines / functions | 315 / 7 (PluginKind::{ALL, as_str, instance_scope, extension_allowed_in_v1}, Display, InstanceScope, LUMEN_PREFIX) + 4 tests |
| File verdict | UNREACHABLE (correct, exhaustively tested data model; no shipped path consumes it) |
| Reachable from shipped entry point? | NO — `PluginKind` users: `crates/plugin-api/src/manifest.rs` (extension manifests) and `crates/plugin-host-wasm/src/{state,limits,api,lib}.rs` (crate outside the release lane). The shipped CLI consumes plugin-api for `Demuxer`/`SourceStream`/`SeekSupport` traits only (apps/cli/src/containers/{ogg,mp4,ts}.rs); `rg PluginKind apps/cli` → no hits |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg (usage tracing, exit 0); sed -n / od -c (quote byte-verification, exit 0); baseline test citation |

## 1. One-paragraph verdict
A clean sixteen-variant data definition: manifest spellings, instance scopes, and the v1 extension-allow policy, each backed by an exhaustive table test that would fail on any edit. The honest parts are explicitly honest — the fourteen→sixteen split and the unlisted subtitle kinds are flagged `[PROPOSAL]` for Lead reconciliation rather than silently decided. The defect list is short: the whole model is dead code in the shipped artifact (only the unshipped extension host reads it), one vestigial clippy allow, and one logically dead test assertion. No fakeness: every function is a total mapping over the enum with zero input-dependent behavior beyond the enum value.

## 2. What actually works (verified)
— All 4 tests pass in the orchestrator baseline: `audit/runs/baseline/cargo-test-workspace.txt` lines 4824-4827 (`kind::tests::* ... ok` ×4, PACKAGE plugin-api section).
— Verified every function is a pure const mapping; `Display` delegates to `as_str` (read, lines 197-201).

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/kind.rs:20-71 (the enum), vs usage tracing
Code:
```
/// What a plugin *is* — the sixteen kinds of doc 06 §1.2.
///
/// Doc 01 §2 defines fourteen variants; doc 06 §1.2 splits
```
What is wrong: Nothing in the code — the reachability. No shipped entry point (lumen-cli, desktop) touches `PluginKind`; its only consumers are plugin-api's own manifest validation and plugin-host-wasm, which the release lane never builds (see instance.rs audit; the repo's own gap test records "plugins.crashed not wired — handoff H-10 ... Phase 7"). 300 lines of compiled-but-dead taxonomy riding in the shipped plugin-api crate. Why it matters: dead surface in a shipped dependency; the extension policy encoded here (`extension_allowed_in_v1`) is unenforced anywhere a user can reach. What correct looks like: fine to keep as the Phase-7 foundation — but the reachability fact belongs in the wave report, not just this file. Confidence: HIGH — Verification: read (rg usage tracing, exit 0).
F-002 [S4]
Where: crates/plugin-api/src/kind.rs:124
Code:
```
    #[allow(clippy::match_same_arms)]
    pub const fn instance_scope(self) -> InstanceScope {
```
What is wrong: The match below groups all same-bodied variants into single arms (`PluginKind::Access | PluginKind::NetworkProtocol => InstanceScope::PerOpenSource`, etc.), so `match_same_arms` cannot fire; the allow is vestigial lint suppression (the tree carries 121 `#[allow]`s per the wave-0 baseline — this is one). Why it matters: suppressions that no longer bind train readers to ignore them all. What correct looks like: delete the attribute; clippy -D warnings is CLEAN in baseline, proving it is unnecessary. Confidence: HIGH — Verification: read + baseline clippy clean (audit/runs/baseline/cargo-clippy.txt).
F-003 [S4]
Where: crates/plugin-api/src/kind.rs:230-234 (test spellings_are_unique_kebab_case)
Code:
```
            assert!(
                s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{s} is kebab-case"
            );
            assert!(!s.contains("__"), "{s} no double underscores");
```
What is wrong: The `__` assertion is logically dead: the preceding `all()` already restricts every char to `[a-z0-9-]`, which excludes `_` entirely, so `contains("__")` is provably false whenever reached. A tautological assertion posing as coverage. Why it matters: test noise; counts as an assertion that can never fail. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| LUMEN_PREFIX | 8-10 | REAL (unreachable) | — | consumed by manifest.rs prefix/origin rules |
| PluginKind (16 variants) | 27-71 | REAL (unreachable) | — | each variant documented with scope/policy cross-refs |
| PluginKind::ALL | 74-93 | REAL | — | doc-table order; test-pinned |
| PluginKind::as_str | 103-121 | REAL | — | total kebab-case mapping; Display delegates |
| PluginKind::instance_scope | 123-149 | REAL | S4 | correct table; vestigial allow F-002 |
| PluginKind::extension_allowed_in_v1 | 160-175 | REAL | — | 7 allowed / 9 refused incl. [PROPOSAL] subtitle classification, honestly flagged |
| Display for PluginKind | 197-201 | REAL | — | = as_str |
| InstanceScope | 206-219 | REAL | — | 4 scopes |
| 4 tests | 222-315 | REAL | S4 | exhaustive tables; one dead assertion F-003 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| sixteen_kinds_in_doc_order | YES | exact 16-element spelling vector |
| spellings_are_unique_kebab_case | YES (except the `__` line) | HashSet uniqueness + charset + Display==as_str are real; the `__` assert is dead (F-003) |
| v1_extension_policy_matches_doc_lists | YES | exact allowed/refused vectors — any policy edit fails it |
| instance_scopes_match_doc_table | YES | all 16 kinds asserted |
All pass in baseline (4824-4827). No #[ignore].

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "sixteen kinds (doc 06 §1.2)" with doc 01's fourteen split | kind.rs:1, 17-22 | REAL | the enum has exactly 16 variants, ALL pinned by test; the split is flagged [PROPOSAL], not hidden |
| subtitle kinds "builtin-only ... [PROPOSAL] for the Lead reconciliation" | kind.rs:158-160 | REAL (policy data) | extension_allowed_in_v1 excludes them; test pins it |
| "the host refuses the rest with a typed error" | kind.rs:157-158 | REAL-UNPROVEN | enforcement lives in plugin-host-wasm (check_import_policy path) — unshipped; not executed by me |
| LUMEN_PREFIX "the registry rejects it for extensions" | kind.rs:7-9 | REAL-UNPROVEN | enforced in manifest.rs:346 (read); registry itself unshipped |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable from shipped entry points (header). Within tests: the dead `__` assertion. No duplication.

## 8. Dependencies & flags
No external deps (std fmt only). `#[non_exhaustive]` on the pub enum — forward-compat choice, fine. No feature gates. File is <100 effective LOC of logic; table provided anyway.

## 9. Open questions for the next agent
- If doc 06 §10 Q1 reconciles to fourteen kinds (doc 01 §2's number), this enum, ALL, and both policy tests change shape — the [PROPOSAL] debt is real; whoever audits doc 06 should register it in the doc ledger.
