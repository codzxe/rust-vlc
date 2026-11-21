# AUDIT: locales/en-US.toml
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-21 |
| Lines / keys | 97 / 16 (data file: reference message catalog) |
| File verdict | SHALLOW (an honestly-labeled 16-key seed posing as "the reference catalog — … complete, and the source of truth for the key set" while the product's real strings — 121+ settings msgids — live in a different catalog the Rust system never sees) |
| Reachable from shipped entry point? | NO at runtime. Consumers: (1) CI — `cargo test -p localization` (ci.yml:437-438 "C3 — localization audit over the shipped tree") runs seed_integrity over this file; (2) `cargo xtask config-validator` reads it for msgid joins (config_validator.rs:482-495) — but that command runs in NO workflow and NO gate script (rg over .github/workflows + scripts/ → only prose references; full_gate.sh runs check-architecture/fixtures/lint-literals/check-identity/sbom/web-budget, not config-validator); (3) no shipped binary reads `locales/` — the tauri app reads `product.toml` only (app.rs:243), the CLI never links `localization`. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `rg -c 'description = "' schemas/settings.toml` (121 msgids) · `rg -n "settings\." locales/en-US.toml` (3 hits, ALL comments — 0 real keys) · `rg -c '^\s*"settings\.' ui/src/i18n/messages.ts` (535 entries — the real catalog) · `rg -n "STUB" .github/workflows/ci.yml` (only "is a real build, not a stub" comment :55 — stub jobs gone) · `rg -n "config-validator" .github/ scripts/` (never invoked) · `rg -n "title" apps/desktop-tauri/tauri.conf.json` (:14 hard-coded "Lumen") · `rg -n "spec\.title\|WindowTitle" apps/desktop-tauri/src/` (apply_window_spec never reads the title) · `sed -n` on every quoted range |

## 1. One-paragraph verdict
The file is internally honest about being a Phase-1 seed (16 keys, no UI keys, xtask wiring explicitly `[PROPOSAL]`), and everything it declares passes its own audit suite in CI. But its header claims to be "the reference catalog — shipped first, complete, and the source of truth for the key set", and that claim is false against the product it ships in: the declarative settings schema references 121 `settings.*` msgids that are absent here (0/121), doc 07 §6.3 says a referenced-but-missing key is "a CI error", and the only rule-4 join that runs covers exactly product.toml's 2 refs (seed_integrity.rs:34-36 asserts `refs.len() == 2`) — so the "CI error" the doc promises cannot fire for the 121. The real settings strings resolve through `ui/src/i18n/messages.ts`, a parallel TypeScript catalog the Rust audit system never sees. On top of that, one key (`ci.job.stub-suffix`) mirrors CI stub jobs that no longer exist in ci.yml, and a test PINS the stale key so deleting it fails CI; the 12 `xtask.gate.*` keys are a declared-but-unconsumed mirror of strings gate.rs hard-codes; and neither of the 2 product-facing keys resolves at runtime — the shipped window title is hard-coded "Lumen" (tauri.conf.json:14).

## 2. What actually works (verified)
- File is valid TOML, parses through the crate's loader, all 16 keys conform to the key grammar (2-5 kebab segments, reserved namespaces per the header) and all placeholder contracts are legal v1 forms ({product_name}, {path}, {reason}, {failed}/{total}/{skipped}/{not_run}, {executed}, {name}/{card} — snake_case names): `shipped_locales_tree_passes_every_catalog_audit_rule ... ok` + `phase1_seed_keys_are_present ... ok` + `product_toml_msgids_resolve_in_the_reference_catalog ... ok` (seed_integrity, 3/3, orchrun-test-workspace-nofailfast.log.txt:2760-2767).
- The product.toml msgid join that DOES exist is real and green: `identity.description_key = "product.description"` (product.toml:32) and `window.title_idle_key = "window.title.idle"` (product.toml:84) both resolve here.
- The `[PROPOSAL]` deferral note (:61-65) is accurate: rg confirms xtask never reads this catalog.

## 3. Findings

### F-001 [S2] "Complete … source of truth for the key set" is false: 121 settings msgids referenced by schemas/settings.toml are absent (0/121), and the doc-promised CI error for that state cannot fire
Where: locales/en-US.toml:4-7
Code:
```toml
# Role (doc 07 §6.3): en-US is the reference catalog — shipped first,
# complete, and the source of truth for the key set. Every other catalog
# (de.toml, pt-BR.toml, …; file name = BCP-47 tag) is audited against THIS
# file by `crates/localization` (doc 01 §2.3):
```
What is wrong: `schemas/settings.toml` carries 121 `description = "settings.*"` msgid references (rg -c → 121). This file contains zero `settings.*` keys (its only 3 "settings." occurrences are comments at :14, :32, :54). Doc 07 §6.3 states: "It is a CI error for the reference catalog itself to miss a key referenced anywhere (schema descriptions D4, layout `label_key`s, menu entries, Rust/TS lookups)". The CI join that exists covers exactly product.toml's 2 refs — seed_integrity.rs:35 hard-asserts `refs.len() == 2` — and refs.rs knows the `description` field (MSGID_FIELDS, refs.rs:31-36) but nothing calls it on schemas/settings.toml. The real resolution path for all 121 keys is `ui/src/i18n/messages.ts` (535 `settings.*` entries), gated only by the UI's own l10n:check — invisible to the Rust audit rules this header invokes as the enforcement.
Why it matters: two parallel catalogs with disjoint key universes and disjoint checkers; the declared "source of truth" covers 2 of 123 referenced keys. A settings msgid missing from the TS catalog fails no Rust-side gate (the doc-promised error), and a future translator building `de.toml` from this "reference" gets a 16-key file that cannot localize the actual product.
What correct looks like: either the reference catalog absorbs the settings.* msgids (generated from schemas/settings.toml, as doc 07 §6.3 implies) with a rule-4 join over schemas/settings.toml, or doc 07 §6.3 and this header are rewritten to name messages.ts as the en-US source of truth and this file as the product.toml-join seed.
Confidence: HIGH — Verification: read + rg counts (121 / 0 / 535) + seed_integrity.rs:34-36 + ci.yml:437-438.

### F-002 [S3] `ci.job.stub-suffix` mirrors CI stub jobs that no longer exist — and a test pins the stale key, so deleting the corpse fails CI
Where: locales/en-US.toml:95-97
Code:
```toml
# --- ci.* — CI stub job-name suffix pattern (harvested from
# .github/workflows/ci.yml stub jobs; display name only, never parsed).
"ci.job.stub-suffix" = "{name} [STUB until {card} — always green, gates nothing]"
```
What is wrong: ci.yml contains no stub jobs anymore — `rg "STUB" .github/` matches only the comment "is a real build, not a stub" (ci.yml:55); every job in ci.yml is a named, implemented lane (lint, gate, schema-validation, config-propagation, …). The key's claimed source ("harvested from .github/workflows/ci.yml stub jobs") has vanished, no check compares catalog values against ci.yml, AND seed_integrity.rs:44-52 asserts `"ci.job.stub-suffix"` is present — so removing the stale data breaks a green test.
Why it matters: the "reference catalog" carries an entry whose real-world referent is gone, and the test suite actively defends it — dead data enforced as if it were load-bearing. It also documents "[STUB until {card} — always green, gates nothing]" as a live pattern when the repo's CI no longer has such jobs (misleading for auditors reading the catalog as CI documentation).
What correct looks like: delete the key and its seed_integrity pin together, or restore a real referent; add a reverse-check (catalog ci.* keys ↔ ci.yml) if the mirror is kept.
Confidence: HIGH — Verification: read + rg (no stub pattern anywhere in .github/).

### F-003 [S3] 14 of 16 keys have zero consumers; the 2 product-facing keys resolve only in CI joins — the shipped window title is hard-coded "Lumen", never `window.title.idle`'s "{product_name}"
Where: locales/en-US.toml:75 (and the 13 xtask.*/ci.* keys at :78-97)
Code:
```toml
# --- window.* — window titles. Referenced by product.toml
# window.title_idle_key (doc 07 §2.2, §3.2); {product_name} is the only
# allowed variable for the idle title.
"window.title.idle" = "{product_name}"
```
What is wrong: consumer trace for every key — `product.description`: referenced by product.toml:32; resolved by NOTHING at runtime (control-server's `engine_info_from_product` uses `identity.display_version`, never resolves description_key — config.rs:165-182). `window.title.idle`: flows to `WindowTitle::Idle { key }` (window_config.rs:142-146), but the tauri app's `apply_window_spec` (app.rs:234-268) applies size/min/maximize and NEVER reads `spec.title`; the shipped title is the hard-coded `"Lumen"` of tauri.conf.json:14; the webview layer the comment says "resolves the key" (window_config.rs:150-151) has no `window.title.idle` key (ui/src/i18n/messages.ts key universe is `settings.*`/`screen.*`/`player.*`/`tokens.*`). The 12 `xtask.gate.*` keys mirror strings gate.rs composes with `format!` (gate.rs:577-599) — consumed by nothing (documented :61-65). `ci.job.stub-suffix`: F-002.
Why it matters: this is the wave's declared-vs-consumed question answered at data level — a reference catalog whose product-facing keys are join-checked in CI but rendered by no shipped code path, sitting next to the real UI catalog. The `{product_name}` placeholder of `window.title.idle` has no renderer anywhere in a shipped binary.
What correct looks like: the rebrand-propagation card wires product.description/window.title.idle into a shipped surface, or the seed shrinks to what CI actually joins.
Confidence: HIGH — Verification: read + rg chains cited inline (all verified this session; see message.rs#part1 F-002 for the crate-level zero-dependents proof).

### F-004 [S4] The xtask.gate.* values deliberately diverge from gate.rs's composed output — documented, but the mirror is already not-verbatim
Where: locales/en-US.toml:61-65 vs tools/xtask/src/cmd/gate.rs:577-599
Code:
```toml
# [PROPOSAL — for the Lead] xtask does not consume this catalog yet: the
# gate report block is a machine-readable contract (doc 10 §2.4) asserted
# by exact-string tests, so wiring these keys into xtask output is a
# deliberate later decision (uniform-parameter templates below slightly
# normalize the conditional count suffixes that gate.rs composes today).
```
What is wrong: the divergence is honest but larger than "conditional count suffixes": gate.rs's SUCCESS line hard-codes `0` (`format!("result: SUCCESS (0 of {total} failed"` — gate.rs:581) where the template says `{failed}`, and omits zero-count segments entirely while the template always emits all three. If the catalog is ever wired in, every gate report with zero skips/not-runs changes bytes — against doc 10 §2.4's "machine-readable contract asserted by exact-string tests".
Why it matters: a future wiring ticket will either break the exact-string tests or require re-blessing them; the comment under-states the change surface.
What correct looks like: either templates that reproduce gate.rs byte-for-byte (conditional segments included), or the comment names the hard-coded-0 divergence too.
Confidence: HIGH — Verification: read of both sites.

## 4. Per-key table (this file's "functions" are its 16 keys)
| Key | Line | Verdict | Consumer | Notes |
|---|---|---|---|---|
| product.description | 70 | REAL-UNPROVEN (CI-joined only) | product.toml:32 → seed_integrity rule 4; NO runtime renderer | value matches doc 07 §6.2 example verbatim |
| window.title.idle | 75 | REAL-UNPROVEN (CI-joined only) | product.toml:84 → WindowTitle::Idle → **never applied** (app.rs ignores spec.title; tauri.conf.json:14 "Lumen" wins) | `{product_name}` has no renderer in any shipped binary (F-003) |
| xtask.gate.report.header | 78 | STUB-as-data (declared, unconsumed) | none | mirrors gate.rs:599 literal |
| xtask.gate.report.full-log | 79 | STUB-as-data | none | gate.rs:645 composes "full log: {REPORT_REL}" |
| xtask.gate.report.failure-details | 80 | STUB-as-data | none | — |
| xtask.gate.report.exit-code | 81 | STUB-as-data | none | gate.rs:664 `format!("exit {code}")` |
| xtask.gate.report.spawn-failure | 82 | STUB-as-data | none | gate.rs:665 |
| xtask.gate.status.pass/fail/skip/not-run/dry | 84-88 | STUB-as-data (5 keys) | none | skip/not-run take {reason} ✓ |
| xtask.gate.result.success/failure/dry-run | 91-93 | STUB-as-data (3 keys) | none | diverge from gate.rs composition (F-004) |
| ci.job.stub-suffix | 97 | STALE DATA | seed_integrity pins its presence | referent (ci.yml stub jobs) gone (F-002) |

## 5. Test honesty (tests over this file live in crates/localization/tests/seed_integrity.rs)
| Test | Would it fail if the data were broken? | Reason |
|---|---|---|
| shipped_locales_tree_passes_every_catalog_audit_rule | YES | audit_catalogs over locales/ — grammar, placeholder-set self-consistency, rule 5 (no plurals present, vacuous) |
| product_toml_msgids_resolve_in_the_reference_catalog | YES | asserts refs.len()==2 and both resolve — catches a deleted product/window key |
| phase1_seed_keys_are_present | PARTIAL | pins 4 representative keys (product.description, window.title.idle, xtask.gate.status.pass, ci.job.stub-suffix) — but it PINS THE STALE ci key (F-002): correct data hygiene now FAILS this test |
All 3 green: orchrun-test-workspace-nofailfast.log.txt:2760-2767.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "en-US is the reference catalog — shipped first, complete, and the source of truth for the key set" | en-US.toml:4-5 / doc 07 §6.3 | FALSE (as "complete/source of truth") | 0/121 settings msgids present; real catalog = ui/src/i18n/messages.ts (F-001) |
| "It is a CI error for the reference catalog itself to miss a key referenced anywhere (schema descriptions D4, layout label_keys, menu entries, Rust/TS lookups)" | doc 07 §6.3 | FALSE | the only rule-4 CI join is product.toml's 2 refs (seed_integrity.rs:34-36); schemas/settings.toml's 121 refs unchecked by any Rust gate |
| "Every other catalog … is audited against THIS file by `crates/localization`" | en-US.toml:6-7 | REAL (vacuously) | audit runs in CI (ci.yml:437-438) but no other catalog exists (locales/ holds only en-US.toml) |
| "[PROPOSAL — for the Lead] xtask does not consume this catalog yet" | en-US.toml:61-62 | REAL | rg: no xtask code reads locales/ |
| "Run by `cargo xtask l10n-audit` (backed by `crates/localization::audit_catalogs`)" | doc 07 §6.4 | FALSE (wiring) | no l10n-audit subcommand exists; ci.yml:468 admits it: "the l10n-audit xtask subcommand itself does not exist" |
| "ONLY keys for strings that actually exist today — … and the CI stub-name pattern (.github/workflows/ci.yml)" | en-US.toml:50-53 | FALSE today | the ci.yml stub pattern no longer exists (F-002) |

## 7. Dead / duplicated / unreachable code
- Dead data: 12 xtask.gate.* keys + ci.job.stub-suffix (13/16 keys unconsumed — F-002/F-003).
- Duplication: this file vs ui/src/i18n/messages.ts (two "en-US catalogs", disjoint key sets, disjoint checkers — F-001); xtask.gate.* values vs gate.rs format! literals.
- Unreachable at runtime: the entire file (no shipped binary reads locales/).

## 8. Dependencies & flags
- Pure data (TOML); no feature gates. Loader deps (toml/thiserror) pre-audited per crate header.
- Baseline: `cargo test -p localization` green (cited); `cargo xtask config-validator` (manual, baseline #10) exits 1 — but only on brand icons; its msgid join over THIS file passes. Note: config-validator itself runs in NO CI workflow or gate script (rg over .github/ + scripts/) — product.toml's header names it as the validator, yet only a human ever invokes it.

## 9. Open questions for the next agent
- Owner of schemas/settings.toml (W11): the 121 description msgids resolve only in ui/src/i18n/messages.ts — is there ANY gate (TS or Rust) asserting schemas/settings.toml msgids ⊆ messages.ts? l10n-check rule 1 checks union↔catalog only; the generated schema.ts types msgids as plain `string` (gen-settings-screens.mjs:430-440), so tsc does not enforce the join either.
- Owner of ci.yml: confirm the stub-job era ended and ci.job.stub-suffix should die (with its seed_integrity pin).
- Wave gate: doc 07 §6.3/§6.4 make two enforcement claims (reference-completeness CI error; l10n-audit driver) that do not exist — one consolidated FALSE-claim row for the docs owner.
