# AUDIT: tools/xtask/data/rebrand-allowlist.toml
| Field | Value |
|---|---|
| Auditor | W01-A04 |
| Date | 2025-11-23 |
| Lines / functions | 101 / 15 [[exempt]] entries (config data; no functions) |
| File verdict | REAL (data plane live, validated, exercised) — header carries 4 FALSE claims |
| Reachable from shipped entry point? | NO (dev-tool data); consumed by tools/xtask/src/cmd/rebrand.rs:762; exercised end-to-end by tests/rebrand/rebrand_skeleton.rs (via tools/xtask/tests/cli.rs), passing in baseline |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | for-loop existence check of all 15 paths (all OK); `rg -n 'lumen' -i design/` (2 hits); `rg -n 'name = "lumen' Cargo.lock` (4 hits); `rg -n 'allowlist' check_identity.rs` (doc-only); `rg -l 'rebrand-allowlist'` repo-wide (1 code consumer) |

## 1. One-paragraph verdict
The register's data plane is real: all 15 `[[exempt]]` paths exist in the tree, every entry carries `path`/`reason`/`owner`, the file is loaded and integrity-validated by `rebrand.rs` (`parse_allowlist` returns "every [[exempt]] block needs path + reason + owner" on any incomplete block, rebrand.rs:634-638), and the passing rebrand acceptance test asserts the live file's count line ("0 in the v0 surface set", rebrand_skeleton.rs:483-487). But the header — which stakes the file's identity on being "the honest register" — makes four claims that are false against the shipped tree and tool: the promised per-entry token scan with `exempt:` output lines does not exist in code; `check-identity` was implemented and deliberately does NOT consume this file; `design/` DOES carry identity literals today; and `Cargo.lock` DOES carry four `lumen-*` package names. The register under-reports and over-promises.

## 2. What actually works (verified)
- All 15 exempt paths exist (shell existence check: 15/15 OK).
- Integrity validation is real code: rebrand.rs:634-638 refuses incomplete blocks; the acceptance test drives the real file through it (baseline tests/cli.rs `nova_fixture_rebrand_passes_skeleton_assertions ... ok`).
- Trailing-`/` recursive-prefix semantics match the header (rebrand.rs `is_exempt`, L650-659).

## 3. Findings
F-001 [S3]
Where: tools/xtask/data/rebrand-allowlist.toml:12-16
Code:
```
# `reason` and `owner` — `cargo xtask rebrand` refuses to run when an entry
# is incomplete. Paths are repo-relative; a trailing `/` marks a recursive
# directory prefix. The tool scans each entry for old identity tokens and
# reports the counts as `exempt:` lines.
```
What is wrong: the last sentence describes behavior that does not exist. `rg 'exempt:' tools/xtask/src/cmd/rebrand.rs` -> no matches; the orchestration (rebrand.rs:762-770) reads the allowlist, validates it, and prints exactly one line — `allowlist: {} exempt path(s), {} in the v0 surface set` — no per-entry token scan, no `exempt:` lines. Why it matters: the header is the register's operational contract; an auditor or Lead extending rebrand would believe exemption counts are reported and reviewable per entry. They are not. What correct looks like: delete the sentence or implement the scan.
Confidence: HIGH — Verification: read (rebrand.rs orchestration) + rg (no `exempt:` output anywhere).

F-002 [S3]
Where: tools/xtask/data/rebrand-allowlist.toml:22
Code:
```
#   * design/ — carries no identity literal today;
```
What is wrong: FALSE. `rg -n 'lumen' -i design/` -> design/solstice-v2-brief.md:72 `.lumen-button--primary` and :103 `lumen:open-media` — the working name as a CSS class prefix and a command namespace. The "verified by card 1.E4-R" register is stale: design/ now carries identity literals and is NOT exempt, so under the header's own rule (L7-8) those sites are unregistered "rebrand bugs".
Confidence: HIGH — Verification: ran (rg).

F-003 [S3]
Where: tools/xtask/data/rebrand-allowlist.toml:23
Code:
```
#   * Cargo.lock — crate package names carry no lumen-* literal;
```
What is wrong: FALSE. `rg -n 'name = "lumen' Cargo.lock` -> :383 `lumen-cli`, :408 `lumen-mem-audit`, :421 `lumen-sweep`, :434 `lumen-web` — four workspace crate package names carrying the `lumen-*` literal. (These derive from apps/* and tools/* manifests, which are themselves outside the register's coverage — apps/ has no exempt entry at all.) The claim is false as shipped.
Confidence: HIGH — Verification: ran (rg, 4 hits).

F-004 [S4]
Where: tools/xtask/data/rebrand-allowlist.toml:9-10
Code:
```
# and is neither a rewritten surface nor listed here is a rebrand bug
# (doc 07 §9 rule C2; the future `check-identity` subcommand consumes this
# same file).
```
What is wrong: stale forward reference now false: `check-identity` is implemented (cmd/mod.rs:165-169) and does NOT consume this file — deliberately, with the rationale recorded in lint_literals.rs:54-65 ("consuming it here would vacate the C1/C2 scan of exactly the trees it must cover") and check_identity.rs:28-33. `rg 'allowlist' check_identity.rs` shows doc-comment mentions only, no code path reads the file. A reader of this header today believes C2 is backed by this register; it is backed by a different, finer-grained mechanism.
Confidence: HIGH — Verification: read + rg.

## 4. Per-entry table (all 15 entries)
| Entry | Path exists | Consumed | Notes |
|---|---|---|---|
| worklog.md | YES | count only | append-only history rationale |
| docs/adr/ | YES | count only | recursive prefix |
| LICENSE | YES | count only | legal attribution |
| rust-toolchain.toml | YES | count only | comment-only mention |
| Cargo.toml | YES | count only | root manifest only (member manifests NOT covered — apps/ has no entry) |
| keymaps/default.toml | YES | count only | comment-only |
| themes/tokens.json | YES | count only | lumen.tokens DTCG namespace |
| product.schema.json | YES | count only | x-lumen-* keywords |
| schemas/settings.toml | YES | count only | comment-only |
| locales/ | YES | count only | recursive prefix |
| crates/ | YES | count only | exempts the ENTIRE crates tree (v0 posture, tension documented in lint_literals.rs:57-64) |
| tools/ | YES | count only | recursive prefix |
| ui/ | YES | count only | ESLint rule namespace |
| .github/ | YES | count only | recursive prefix |
| tests/rebrand/ | YES | count only | fixture-by-necessity |
"Consumed" = the file is read + integrity-validated by rebrand.rs:762-765 and used only for `is_exempt` counting over the 5 v0 surfaces (currently 0 overlap).

## 5. Test honesty
No tests in this file (data). The data is exercised by tests/rebrand/rebrand_skeleton.rs (asserts the allowlist count line and 0 v0-surface overlap) — passing in baseline (tests/cli.rs section, 12 passed / 3 ignored).

## 6. Doc claims touching this file (claim ledger)
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "refuses to run when an entry is incomplete" (L13-14) | header | REAL | rebrand.rs:634-638 error path |
| "a trailing `/` marks a recursive directory prefix" (L14-15) | header | REAL | rebrand.rs is_exempt L650-659 |
| "The tool scans each entry … reports the counts as `exempt:` lines" (L15-16) | header | FALSE | no such code (rg + read) |
| "the future `check-identity` subcommand consumes this same file" (L9-10) | header | FALSE (as shipped) | check_identity.rs consumes nothing; refusal documented in lint_literals.rs:54-65 |
| "design/ — carries no identity literal today" (L22) | header | FALSE | design/solstice-v2-brief.md:72,103 |
| "Cargo.lock — crate package names carry no lumen-* literal" (L23) | header | FALSE | Cargo.lock:383,408,421,434 |
| "Anything that carries an identity literal and is neither a rewritten surface nor listed here is a rebrand bug" (L7-8) | header | TRUE-as-rule, violated by F-002/F-003 | design/ + Cargo.lock cases |

## 7. Dead / duplicated / unreachable code
Not applicable (data file). No orphan entries: all 15 paths exist. The register itself is near-dead weight at runtime: its only live effect on `cargo xtask rebrand` is one printed count (0 of 5 surfaces).

## 8. Dependencies & flags
TOML data parsed by rebrand.rs's own `parse_allowlist` (in-crate; no toml_lite — different parser than product.toml uses).

## 9. Open questions for the next agent
- rebrand.rs auditor (already filed: audit/files/tools/xtask/src/cmd/rebrand.rs.audit.md): does the rebrand card's roadmap actually intend the per-entry `exempt:` scan (F-001)? If yes it is unimplemented; if no the header lies.
- Whoever owns apps/* manifests: `apps/` has NO register entry while `crates/` does — apps/cli, apps/web package names are `lumen-cli`/`lumen-web` (Cargo.lock:383,434). If doc 07 §8.1's crate-name rewrite plane ever activates, apps is the uncovered hole.
