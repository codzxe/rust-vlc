# AUDIT: crates/theme-tokens/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines | 34 (manifest + header comment) |
| File verdict | REAL (accurate manifest; one broken doc pointer) |
| Reachable from shipped entry point? | The crate is dev-lane by design: no other Rust crate depends on it (rg over every Cargo.toml: zero `theme-tokens =` declarations; settings-schema/commands mention it in comments only). Its reach is real but indirect: `cargo xtask tokens` (subprocess bridge) and the UI pipeline consume its committed outputs (themes/generated/tokens.css → ui/scripts/gen-tokens.mjs → ui/src/tokens/generated/tokens.css, byte-identical copy verified by diff). The shipped lumen-cli/desktop binaries embed none of this crate's code. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg dependency census (all Cargo.tomls); Cargo.lock version check; ls docs/dependencies/records/ (22 files, no theme-tokens.md); diff themes/generated/tokens.css vs ui/src/tokens/generated/tokens.css (identical); `cargo test -p theme-tokens --offline` (38 ok, covers diff.rs/pipeline.rs/semantic_lint.rs); `./target/debug/tokens-gen --check` (no drift) |

## 1. One-paragraph verdict
The manifest is honest and minimal: one pre-audited dependency (serde_json 1.0.154 — exactly the Cargo.lock pin, actually used at parse.rs:21), workspace lints inherited, publish = false, and the tokens-gen bin declared with the right path. The header's pipeline claims are all true and were verified by execution in this audit (idempotent regeneration, committed diff-tested outputs). One claim is false: the pointer "record: docs/dependencies/records/theme-tokens.md" names a file that does not exist — the records directory holds 22 files, none for theme-tokens (the serde_json preaudit itself does exist in preaudit-foundation.md:25, so the substantive dependency-audit claim is true; the per-crate record pointer is broken).

## 2. What actually works (verified)
— `serde_json = "1.0.154"` matches Cargo.lock:894-895 (`name = "serde_json"`, `version = "1.0.154"`) and is used (crates/theme-tokens/src/parse.rs:21 `use serde_json::Value;`).
— "Both outputs are COMMITTED and diff-tested (`tests/diff.rs`)" — tests/diff.rs exists, its 2 tests pass (`cargo test -p theme-tokens --offline`, 38 total ok), and `./target/debug/tokens-gen --check` reports no drift (executed, file 2 audit).
— "regeneration is idempotent" — proven by direct execution: regeneration to a scratch dir is byte-identical to both committed artifacts (file 2 audit). The comment's "empty `git diff`" wording is their claim; my byte-level `diff -q` is the equivalent proof (git is forbidden to agents).
— The xtask subprocess bridge claim matches tools/xtask/src/cmd/tokens.rs:61 (spawns `run -p theme-tokens --bin tokens-gen`).

## 3. Findings
F-001 [S4]
Where: crates/theme-tokens/Cargo.toml:14-15 (header, verbatim):
```
# External dependency policy (doc 11): serde_json is pre-audited PASS
# (docs/dependencies/preaudit-foundation.md §1); record:
# docs/dependencies/records/theme-tokens.md.
```
What is wrong: `docs/dependencies/records/theme-tokens.md` does not exist — the directory contains 22 record files (audio-output-cpal.md, claxon-0.4.3.md, cpal-0.18.2.md, …, wgpu-30.0.1.md) and none for theme-tokens. The pointer names a missing compliance artifact. The substantive claim (serde_json preaudit PASS) is true and lives at docs/dependencies/preaudit-foundation.md:25 (`| serde_json | 1.0.154 | MIT OR Apache-2.0 | 2026-10-11 | Pure Rust | **pass** |`). Why it matters: doc 11's dependency-policy chain is auditable only if its record pointers resolve; a dangling one hides whether the per-crate record was never written or was deleted. What correct looks like: write the record or point at preaudit-foundation.md §1 directly. Confidence: HIGH — Verification: ls + rg.

## 4. Per-function table
Not applicable (manifest, 34 lines). Field inventory: package (name/version/edition/publish/description) accurate; `[dependencies]` 1 entry, verified used; `[lints] workspace = true` (baseline: clippy -D warnings CLEAN); `[[bin]] tokens-gen` path exists (audited as my file 2, REAL).

## 5. Test honesty
Tests belong to the crate, not this manifest; 38/38 pass (run). The manifest's claims about tests (diff.rs exists, pipeline tested) verified true.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Parses the canonical token source themes/tokens.json … and emits, deterministically" both outputs | L2-6 | REAL | tokens-gen --check + regen diff (file 2 audit, executed) |
| "Both outputs are COMMITTED and diff-tested (tests/diff.rs)" | L6-7 | REAL | tests pass; artifacts byte-identical |
| "regeneration is idempotent (`cargo xtask tokens`, twice → empty git diff)" | L7-8 | REAL | equivalent byte-diff proof (git not runnable by agents) |
| "serde_json is pre-audited PASS (docs/dependencies/preaudit-foundation.md §1)" | L13-14 | REAL | preaudit-foundation.md:25 |
| "record: docs/dependencies/records/theme-tokens.md" | L14-15 | FALSE | file absent (F-001) |

## 7. Dead / duplicated / unreachable code
The crate's public lib surface (model/parse/emit/lint/color) has zero external Rust consumers — it exists to serve its own bin and tests. Not dead (the bin is the xtask bridge), but it is a one-client library. The shipped player binaries contain no theme-tokens code; the UI consumes only the committed CSS copy (byte-identical, verified).

## 8. Dependencies & flags
serde_json 1.0.154 only. No feature flags, no dev-dependencies, no unsafe. Edition 2024.

## 9. Open questions for the next agent
- Whoever audits docs/dependencies/**: confirm whether doc 11 requires a per-crate record for theme-tokens (F-001) or whether the preaudit-foundation entry satisfies it and the pointer is simply stale.
