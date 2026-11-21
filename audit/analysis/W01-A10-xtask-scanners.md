# W01-A10 HOSTILE-QUESTION ANALYSIS — tools/xtask scanners
Task: W01-A10 (domain: tools/xtask source + remaining workspace manifests).
Files audited (per assignment): 7 manifests — see audit/files/*.audit.md.
This document answers the two hostile questions with verified citations.
All quotes verified with sed -n. No cargo runs (rule 8); executed evidence
cited from audit/runs/baseline/INDEX.md (#10, #13, #14, #3).

---

## HQ1 — config-validator: full validation or narrow slice?

**Blunt answer: it is a narrow slice — one file (product.toml) + a
hand-rolled schema subset + 7 join checks — it is wired into NO CI workflow
and NOT into full_gate.sh, and two of its three documented delegations are
only partially real. The "4 predicted violations" is the visible tip, not
evidence of coverage.**

### What config-validator actually validates (traced, config_validator.rs)

Structural walk `walk()` (config_validator.rs:242-425) over product.toml vs
product.schema.json, supporting exactly: `type`, `required`,
`additionalProperties` (false or per-key schema), `enum`, `not.enum`,
`minLength`/`maxLength`, `minimum`/`maximum`, `minItems`/`maxItems`+`items`,
and internal-only `$ref` (`#/$defs/...`, resolve_ref at 229-240).

Cross-file `cross_file()` (config_validator.rs:431-587):
1. brand asset existence (4 keys, lines 440-453) — this is the check that
   produced the 4 baseline violations;
2. `defaults.locale` → `locales/<tag>.toml` existence (456-461);
3. `defaults.theme`/`profile` vs `themes/`/`profiles/` listings (462-480);
4. msgid existence for exactly 2 keys (`identity.description_key`,
   `window.title_idle_key`) in `locales/en-US.toml` (482-497);
5. `title_template` variables ⊆ {product_name, media_title} (500-514);
6. `telemetry.enabled=true` ⇒ `urls.privacy` (516-522);
7. `schema_version` == max(migrations `to`) + ascending/no-gap rules (524-565);
8. app ids == reverse_dns + "." + PascalCase(name) (567-586).

### What it does NOT validate — and who (if anyone) does

| Constraint class | config-validator | product-config crate (tests) | Enforced in CI? |
|---|---|---|---|
| Structure (types/ranges/required/unknown keys) | YES | YES (load_root_product, tests/common/mod.rs:35-38) | YES — via `cargo test --workspace` (baseline #3), NOT via the gate |
| All 10 schema `pattern` constraints | NO — documented, config_validator.rs:16-18/117 | YES — hand-written matchers (patterns.rs:12-37), `Schema::load` fails loudly on schema drift (patterns.rs:5-6, schema.rs:79) | YES — via cargo test (real file loaded in tests/semantic.rs:279-293) |
| In-document semantics (app-id derivation, telemetry⇒privacy, migrations) | YES | YES (`SemanticInputs::none()` test, tests/semantic.rs:289) | YES — via cargo test |
| Brand asset EXISTENCE | YES (its only real consumer) | code exists but no caller passes `base_dir` for the real file | **NO — config-validator is in no workflow (rg over .github: zero matches) and not in full_gate.sh (11 steps, lines 10-77, no config-validator)** |
| Brand asset MAGIC BYTES | NO (documented, line 21) | code exists (semantic.rs:460-467 `kind.magic_ok`) but the real-file test passes `SemanticInputs::none()` → `base_dir: None` skips the filesystem (semantic.rs:21-23) | **NO — nobody, anywhere, on the real assets** |
| F1/F2 feature ids vs engine registry | NO — "needs the registry surface; v0 checks that every features.* value is boolean" (line 19-20) | code exists (semantic.rs:404-424) but gated on `if let Some(registry)` (405) and the ONLY callers constructing a registry use synthetic fixtures (tests/semantic.rs:201,218); the real-file test passes `none()` | **NO — the real product.toml's feature table is never checked against the real engine registry by anyone** |
| defaults.theme/profile joins | listing check; themes/ has ZERO *.toml (only generated/ + tokens.json) and profiles/ does not exist → printed as NOTE, not violation (469-472) | n/a (parked plane, doc 07 §9 C4) | NO (notes are not enforcement) |
| Duplicate keys/table headers in product.toml | **silently merged** — toml_lite `attach` pushes without dedup (toml_lite.rs:268-281), `insert_table` merges duplicate table headers (config_validator.rs:188-194), `lookup` takes the first match (225-227) | the real `toml` crate (used by product-config) rejects duplicates → CI test would fail | PARTIAL — caught only by the crate tests, while the gate itself would PASS the same file (gate↔parser verdict divergence) |
| Unresolvable `$ref` | value silently UNCHECKED — ref resolution failure leaves the walk with a schema object holding only `$ref`, so no type/constraint runs and no note is emitted (config_validator.rs:253-259) | n/a | latent (current schema has only internal refs) |
| Everything that is not product.toml | out of scope: schemas/settings.toml (settings-schema owns), keymaps/, boundaries.toml, Cargo manifests, ui config | — | — |

### The gate is not wired anywhere automated

- `.github/workflows/*.yml`: rg `config-validator` → zero matches.
- `scripts/full_gate.sh`: 11 steps enumerated at lines 10-77 (fmt, clippy,
  test, check-architecture, fixtures, fuzz-registry, lint-literals,
  check-identity, sbom, web-budget, packaging) — no config-validator.
- The repo knows: docs/release-checklist.md:55 (row 1.6) says "**RED today —
  4 violations, all the missing `assets/brand/*` files** … Note:
  `scripts/full_gate.sh` does not run this validator (its 9 steps are …) — do
  not rely on the gate to catch it." (The checklist's "9 steps" is itself
  stale — the script now has 11.)

### Findings (ecosystem-level; owners: xtask/product-config/CI auditors)

- **S2 — F1/F2 unenforced on the real product.toml.** product.toml:137-139
  claims "A feature id that is not in the engine's static feature registry is
  a VALIDATION ERROR here (rule F1)". The enforcement code exists
  (semantic.rs:404-424) but no caller ever supplies the real registry; both
  the xtask gate (config_validator.rs:19-20) and the real-file crate test
  (tests/semantic.rs:289, `SemanticInputs::none()`) skip it. Flipping
  `features.streaming-hls = true` (Phase 6, unshipped) today fails NOTHING,
  contradicting the config-can't-lie promise. Confidence HIGH (all call
  sites enumerated).
- **S3 — config-validator in no CI lane and not in full_gate.** A red gate
  (baseline #10) whose redness is only tracked in a manually-maintained
  checklist row. Confidence HIGH.
- **S3 — brand-asset magic bytes checked by nobody on the real file.** The
  schema promises "existence and magic bytes are checked in the validator's
  semantic pass" (product.schema.json:297); the crate implements it
  (semantic.rs:460-467) but only under `base_dir: Some`, which no real-file
  caller passes. (Moot until assets exist — they don't, which is baseline
  violation #10 — but the format promise is unenforced.) Confidence HIGH.
- **S3 — two parallel implementations of doc 07 §3.3/§3.4 cross-file rules**
  (xtask config_validator.rs `cross_file` vs product-config semantic.rs),
  already diverging in coverage (existence vs existence+magic+listing).
  Confidence HIGH.
- **S3 — toml_lite silently merges duplicate keys/table headers**
  (toml_lite.rs:268-281, config_validator.rs:188-194) while the crate's real
  parser rejects them: the gate can PASS a file CI tests reject. Confidence
  HIGH (code traced; no live instance).
- **S4 — unresolvable `$ref` silently skips validation**
  (config_validator.rs:253-259). Latent. Confidence HIGH.
- **S4 — themes/profiles joins are notes, not violations** (parked planes);
  `defaults.theme = "dark"` and `defaults.profile = "default"` reference
  planes with zero/no TOML files. Documented, but the values are unchecked.
  Confidence HIGH.

### Verified-working part (for fairness)
The 4 violations ARE real and correctly found: `assets/brand/{icon.ico,
icon.icns,icon-512.png,logo.svg}` are referenced at product.toml:97-101 and
none exists (baseline #10, xtask-config-validator.log). The existence loop
(config_validator.rs:440-453) is doing exactly its job. The prediction in
docs/release-checklist.md:55 matches the observed output 1:1.

---

## HQ2 — check-identity/lint-literals on apps/cli/src/host.rs:1799: real drift or false positive?

**Blunt answer: REAL DRIFT — a true violation of the repo's own C1/C2
policy, not a scanner bug. The literal landed in a 137-branch consolidation
merge (2026-11-16) that bypassed the gate CI runs; the site simply lacks the
mandated inline allow-marker.**

### The scanners (traced)

- `lint-literals` (C1) and `check-identity` (C2) share ONE engine:
  check_identity.rs:108 calls `lint_literals::scan_workspace(&root,
  &identity, Scope::C2)`. C2 = C1's brand trio (name, short_name, publisher
  — case-insensitive whole-word) + technical ids (exact substring) + accent
  hex (lint_literals.rs:170-182; check_identity.rs:15-25). Both scopes
  include the brand trio — that is why one site fails both gates.
- Scan domain: `crates/*/src/**`, `apps/*/src/**` string literals ONLY
  (comments never fire; the scanner is a lexer-shaped state machine,
  lint_literals.rs:69-77).
- **#[cfg(test)] modules inside src/ are deliberately scanned** — the module
  docs state the policy: "their legitimate literals carry inline markers
  like everything else. This is the difference between an exemption and a
  blind spot" (lint_literals.rs:48-52).
- Suppression: an inline `// lumen-allow-literal: <reason>` on or directly
  above the line, printed in the report (lint_literals.rs:36-40).
- The word matcher is bounded and unit-tested, including the exact shape at
  issue: `assert!(contains_word("LUMEN-CLI", "lumen"))` (lint_literals.rs:1359)
  — hyphen is a word boundary.

### The failing site

apps/cli/src/host.rs:1760-1761 opens the test module:
```
#[cfg(test)]
mod session_backpressure_tests {
```
and lines 1798-1799 hold the literal:
```
        let path = std::env::temp_dir().join(format!(
            "lumen-backpressure-{}-{}s.wav",
```
`rg "lumen-allow-literal" apps/cli/src/host.rs` → zero matches: no marker.
The literal contains the whole word "lumen" (identity.name = "Lumen",
product.toml:19), so C1 fires (baseline #13: "1 violation
apps/cli/src/host.rs:1799 identity") and C2 fires identically (baseline #14:
same site) — one engine, two scopes, one brand trio.

### Verdict: REAL DRIFT. Chain of evidence:

1. The scanners predate the literal: gates landed 2026-11-10
   (git log --diff-filter=A tools/xtask/src/cmd/lint_literals.rs → ba11828c).
2. The literal landed 2026-11-16 in "w32.consolidation.1 … the 137-branch
   merge" (git log -S "lumen-backpressure" → 59cfd32b) — six days after the
   gate existed.
3. CI runs both gates on every push (ci.yml:461-464, config-propagation
   job: `cargo run -p xtask --locked -- lint-literals` / `check-identity`)
   and full_gate.sh steps 7-8 (scripts/full_gate.sh:49-60) run them locally.
   A committed main that fails them means the merge landed without a green
   gate run (or overrode it).
4. The marker mechanism is real and actively used elsewhere — e.g.
   apps/cli/src/help.rs:101:
   `format!("lumen {}\n", env!("CARGO_PKG_VERSION")) // lumen-allow-literal:
   CLI argv0/usage prose — identity wiring pending (see help.rs [PROPOSAL])`
   — so the fix path is established practice, not theory.

### Is there ANY false-positive angle?
Only a policy argument, not a scanner defect: one could argue a test-helper
temp-file prefix ("lumen-backpressure-…wav") is not an identity surface. But
the tool's documented policy already answers this — test-module literals are
scanned ON PURPOSE and the marker-with-reason is the sanctioned exemption
(lint_literals.rs:48-52, 36-40). The site has no marker. Under the repo's
own rules this is a violation. Fix: add
`// lumen-allow-literal: test temp-file prefix, not a brand surface` at
host.rs:1799 (or derive the prefix from product-config if the policy is
taken literally).

Findings logged for the host.rs/release-checklist owners:
- **S3 — committed main fails its own C1/C2 identity gates at one site**
  (baseline #13/#14); the w32 consolidation merge (59cfd32b, 2026-11-16)
  landed an unmarked identity literal in a cfg(test) module. Confidence
  HIGH.
- **S3 — docs/release-checklist.md:53 (row 1.4) is now FALSE evidence.** It
  records lint-literals / check-identity as "**[verified locally]** both
  PASS" with "(0 violations, 452 allowlisted)" / "(0 violations, 453
  allowlisted)" — written 2026-11-10 (git log -S "452 allowlisted" →
  be4fedb4), six days BEFORE the violating literal landed (59cfd32b,
  2026-11-16). The executed baseline (2026-11-21) shows 1 violation in each
  gate. The release checklist was never refreshed after the w32 merge — the
  same committed-evidence drift class as baseline #4's transcripts. The
  checklist tracks config-validator's redness (row 1.6) but row 1.4's PASS
  claim silently rotted. Confidence HIGH.
