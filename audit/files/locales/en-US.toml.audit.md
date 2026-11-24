# AUDIT: locales/en-US.toml
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 97 / 16 message keys in 4 namespaces (product 1, window 1, xtask 13, ci 1) |
| File verdict | REAL as a self-consistent Phase-1 seed that passes every audit rule — but it is NOT the de-facto product catalog: 0 of its 16 keys are rendered by any shipped binary, and the real UI copy lives in a disjoint hand-written TS catalog |
| Reachable from shipped entry point? | NO as runtime data. Read only by: crates/localization/tests/seed_integrity.rs (audit_catalogs over `locales/` + product.toml rule-4 join, baseline pass) and the crate's unit tests. No shipped binary reads this file (no crate depends on `localization`; rg proof in audit.rs.audit.md). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | python3 catalog stats + placeholder scan (16 keys, 9 with placeholders, namespaces product/window/xtask/ci); rg cross-refs: messages.ts for the 16 keys (exit 1 — zero overlap), apps/desktop title handling, tools/xtask gate.rs result-line composition, `rg 'STUB' .github/` (exit 1 — pattern gone). Baseline: localization seed_integrity 3/3 pass. |

## 1. One-paragraph verdict
The file is honest about what it is — a Phase-1 seed: 16 keys covering exactly the strings that existed at task 1.B3 (2 product.toml msgids, 13 xtask gate-report labels, 1 CI stub-name pattern), with an unusually candid header that admits xtask "does not consume this catalog yet" and that no player/settings/menu keys are seeded because "those screens do not exist yet (Phase 2+)". That last clause is now FALSE: the screens exist (ui/src/screens/{player,settings,...}) and they did NOT grow into this catalog — they grew a parallel, hand-written TypeScript catalog (ui/src/i18n/messages.ts, 1000+ keys, zero overlap with these 16 — rg proof). So the doc 07 §6.3 role ("reference catalog — source of truth for the key set") is unrealized: the actual source of truth for user-visible copy is messages.ts, gated by `npm run l10n:check`, not this file, and the two systems have no mechanical link. Within its own scope the file is flawless: all 16 keys pass the grammar, the 9 placeholder-bearing messages use well-formed doc 07 §6.2 placeholders with snake_case names, `window.title.idle` uses only `{product_name}` as its own contract demands, and the seed-integrity suite proves it satisfies all five audit rules.

## 2. What actually works (verified)
- Baseline: seed_integrity 3/3 pass — `shipped_locales_tree_passes_every_catalog_audit_rule` (all 5 rules over this file), `product_toml_msgids_resolve_in_the_reference_catalog` (both product.toml msgids exist here), `phase1_seed_keys_are_present`.
- python3 stats: 16 keys; 9 with placeholders ({product_name}; {path}; {code}; {reason}x2; {failed}/{total}/{skipped}/{not_run}x2; {executed}; {name}/{card}); 7 plain. Placeholder names all match `[a-zA-Z_][A-Za-z0-9_]*` (`not_run` is legal for placeholder names, distinct from key-segment grammar).
- Consumers verified: `product.description` <- product.toml:32 `description_key`; `window.title.idle` <- product.toml:84 `title_idle_key` (both joins rule-4-tested). The other 14 keys: no consumer anywhere (rg).

## 3. Findings
F-001 [S3]
Where: locales/en-US.toml:51-55 (PHASE-1 SEED SCOPE header) vs the current tree
Code:
```
# that actually exist today — xtask gate-report labels (harvested verbatim
# from tools/xtask/src/cmd/gate.rs), the CI stub-name pattern
# (.github/workflows/ci.yml), and the product.toml msgids specified in
# doc 07 §2–3 (description_key / title_idle_key). NO player.*/settings.*/
# menu.* UI keys are seeded: those screens do not exist yet (Phase 2+).
```
(byte-verified lines 51-55)
What is wrong: the "screens do not exist yet" premise expired. ui/src/screens/ ships player, settings, playlists, onboarding, etc., whose copy (player.*, settings.*, screen.*, playlist.* — 1000+ keys) lives in ui/src/i18n/messages.ts with its own grammar/coverage/dead-key gate (ui/scripts/l10n-check.mjs) — a completely parallel i18n system. rg proof: none of this file's 16 keys occurs in messages.ts (exit 1). Consequence: the "reference catalog" governs 2 runtime-rendered msgids' worth of product (and even those are only validated, never rendered from here — apps/desktop/src/window_config.rs:104-107 stores `title_idle_key` as a string and validates the TITLE TEMPLATE's variables itself; nothing translates `{product_name}` via this file). The two-catalog split also means the localization crate's rules 1–5 can never apply to the catalog users actually see. What correct looks like: either the UI catalog is generated from/checked against this file, or this file's "source of truth" role is formally reduced to the Rust-side config plane.
Confidence: HIGH — Verification: read + rg (messages.ts, window_config.rs, ui/scripts).

F-002 [S4]
Where: locales/en-US.toml:95-97
Code:
```
# --- ci.* — CI stub job-name suffix pattern (harvested from
# .github/workflows/ci.yml stub jobs; display name only, never parsed).
"ci.job.stub-suffix" = "{name} [STUB until {card} — always green, gates nothing]"
```
What is wrong: the documented source no longer exists — `rg 'STUB' .github/` returns no matches (exit 1); ci.yml contains no stub jobs anymore (only a comment at ci.yml:55 stating the wasm job "is a real build, not a stub"). The key is a fossil whose provenance comment is now false, and it is consumed by nothing.
Confidence: HIGH — Verification: rg.

F-003 [S4]
Where: locales/en-US.toml:91-93 vs tools/xtask/src/cmd/gate.rs:575-596
Code:
```
"xtask.gate.result.success" = "result: SUCCESS ({failed} of {total} failed, {skipped} skipped, {not_run} not run)"
"xtask.gate.result.failure" = "result: FAILURE ({failed} of {total} failed, {skipped} skipped, {not_run} not run)"
"xtask.gate.result.dry-run" = "result: DRY RUN ({executed} of {total} executed, {skipped} skipped)"
```
What is wrong: these are NOT verbatim harvests: gate.rs composes `result: SUCCESS (0 of {total} failed` and appends `, {skipped} skipped` / `, {not_run} not run` only when the counts are non-zero (gate.rs:583-593). The catalog templates always include all segments. The header pre-declares this ("uniform-parameter templates below slightly normalize the conditional count suffixes that gate.rs composes today", lines 78-80), so it is honest — but "harvested verbatim" in the PHASE-1 SEED paragraph (line 51) overstates it, and any future wiring of these keys would change gate.rs output and break its exact-string tests.
Confidence: HIGH — Verification: read + gate.rs:575-596.

F-004 [S4]
Where: locales/en-US.toml:77-93 (13 xtask.gate.* keys)
What is wrong: 13 of 16 keys (81%) are consumed by nothing and blocked on a Lead `[PROPOSAL]` decision (lines 61-65) — dead inventory by design. No severity beyond hygiene because the header says so explicitly and rule 4/5 audits still exercise them.
Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
n.a. (data file). Key inventory: product.description; window.title.idle; xtask.gate.report.{header,full-log,failure-details,exit-code,spawn-failure}; xtask.gate.status.{pass,fail,skip,not-run,dry}; xtask.gate.result.{success,failure,dry-run}; ci.job.stub-suffix.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seed_integrity::shipped_locales_tree_passes_every_catalog_audit_rule | YES | audit_catalogs over this exact file — any grammar/placeholder/plural regression fails (baseline pass). |
| seed_integrity::product_toml_msgids_resolve_in_the_reference_catalog | YES | Both product.toml msgids must exist here. |
| seed_integrity::phase1_seed_keys_are_present | YES (change-detector) | Four representative keys pinned. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "en-US is the reference catalog — … the source of truth for the key set. Every other catalog … is audited against THIS file" | en-US.toml:7-10; doc 07 §6.3 | SHALLOW | True within the (unwired) Rust plane; the shipped UI's key set lives in messages.ts with zero mechanical link (F-001). |
| "harvested verbatim from tools/xtask/src/cmd/gate.rs" | en-US.toml:51 | SHALLOW | Result-line templates are normalized, not verbatim (F-003). |
| "harvested from .github/workflows/ci.yml stub jobs" | en-US.toml:95 | FALSE | No STUB jobs exist in .github/ anymore (F-002). |
| "NO player.*/settings.*/menu.* UI keys are seeded: those screens do not exist yet (Phase 2+)" | en-US.toml:55 | FALSE (stale) | Screens exist; their keys live in ui/src/i18n/messages.ts (F-001). |

## 7. Dead / duplicated / unreachable code
14/16 keys unconsumed (13 xtask.* parked on a Lead decision + 1 ci.* fossil). File itself unreachable from shipped binaries (data for an unwired crate).

## 8. Dependencies & flags
None (data). Encoding UTF-8 with em-dashes inside double-quoted TOML strings — valid TOML; the localization catalog parser accepts it (baseline proves).

## 9. Open questions for the next agent
- Does any doc/matrix row claim "full i18n" or "localized UI" as shipped? The UI offers exactly one locale (ui/src/screens/settings/areas/language/locales.ts:15 AVAILABLE_LOCALES = ["en-US"]) — check feature-matrix wording against that.
- If the Lead ever ratifies xtask consuming these keys, gate.rs exact-string tests and the normalized templates must be reconciled (F-003).
