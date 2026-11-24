# AUDIT: crates/plugin-api/src/semver.rs
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2025-06-27 |
| Lines / functions | 322 / 7 non-test + 5 tests |
| File verdict | REAL (correct parser; precedence has one spec hole) |
| Reachable from shipped entry point? | NO — compiled into the shipped binary via plugin-api (apps/cli/Cargo.toml:59), but no shipped code parses a `PluginManifest` (rg over apps/cli/src and non-plugin crates: zero `PluginManifest` users); the only runtime consumers are plugin-host-wasm's store and the `lumenext` bin, both outside the release lane |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "precedence" crates apps` -> sole semantic consumer install.rs:345; `rg "PluginManifest" apps/cli/src crates(–plugin)` -> none; `rg semver tests` in baseline -> 5/5 ok (:4842-4846); sed quote verification |

## 1. One-paragraph verdict
A real, hand-rolled SemVer 2.0.0 parser with correct grammar enforcement (build split before pre, exactly-3 numeric core, leading-zero rules correctly asymmetric between core and pre-release vs build metadata per §9/§10, typed errors, u64 overflow typed) and a nearly correct §11 precedence. One genuine spec defect: numeric pre-release identifiers that exceed u64 are silently clamped to 0 by `unwrap_or(0)` in `compare_pre`, so `1.0.0-18446744073709551616` compares equal to `1.0.0-0` and LESS than `1.0.0-1` — spec says greater — and this precedence feeds the extension store's load-picks-highest rule (install.rs:345). Tests are honest and include the semver.org §11 worked example, but do not cover the >u64 identifier case or the overflow boundary.

## 2. What actually works (verified)
— Baseline audit/runs/baseline/cargo-test-workspace.txt:4842-4846: `semver::tests::{doc_examples_parse, full_grammar, precedence_rules, rejects_bad_versions, zero_is_legal_but_only_unprefixed}` — 5/5 ok. `rejects_bad_versions` covers 18 malformed shapes incl. whitespace, `v`-prefix, leading zeros, empty pre/build, bad identifier chars; `precedence_rules` walks the full §11 worked example both directions.
— Grammar reasoning verified by read: multi-`+`/`-` handling (`1.2.3+a+b` → IdentChars; `1-2.3.4` → CoreShape; `1.2.3-a-b` → pre `a-b` valid), build-metadata leading zeros correctly ALLOWED (`1.2.3+001` valid, is_pre=false gates the check at L163).

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/semver.rs:182-186
Code:
```
let ord = match (xn, yn) {
    (true, true) => x
        .parse::<u64>()
        .unwrap_or(0)
        .cmp(&y.parse::<u64>().unwrap_or(0)),
```
What is wrong: `valid_identifiers` (L148-168) accepts numeric pre-release identifiers of unbounded length (all-digits, no leading zero — spec-legal), but `compare_pre` compares them as u64 with a silent `unwrap_or(0)` fallback. A 20+-digit numeric identifier (≥ 2^64) parses Ok at L41-84, then compares as 0. Consequences: `1.0.0-18446744073709551616` == `1.0.0-0` (Equal) and < `1.0.0-1` (Less) — both wrong per semver.org §11.4 (numeric compare of the actual number). The error is also swallowed (checklist: `unwrap_or` hiding a parse failure). Why it matters: `Semver::precedence` is the extension store's load-picks-highest rule — crates/plugin-host-wasm/src/install.rs:345 `parsed.precedence(current) == std::cmp::Ordering::Greater` — so among two installed versions differing only in an oversized numeric pre-release identifier, the store loads the OLDER one; the version string is attacker-supplied manifest data. What correct looks like: compare digit-strings by (length, lexicographic) when both are all-digits and either fails u64 parse — no allocation, exact numeric order. Confidence: HIGH — Verification: read (reasoned; no execution — sandbox forbids new test files; UNVERIFIED by run).

F-002 [S4]
Where: crates/plugin-api/src/semver.rs:16-18
Code:
```
//! Precedence (semver.org §11) is exposed for the registry's
//! same-major comparisons [PROPOSAL — the v1 load rule only needs the
//! major; precedence arrives with dependency resolution].
```
What is wrong: The doc claims the v1 load rule only needs the major, but the shipped load rule compares FULL precedence across minor/patch/pre (install.rs:345, exercised by plugin_install_lifecycle.rs:377-393 "load picks the highest" over 0.1.0 vs 0.2.0 and pre-release fallback). The "[PROPOSAL]" bracket describes a consumer that already exists and behaves differently. Why it matters: doc-vs-code drift on a security-adjacent selection rule. What correct looks like: update the bracket to describe the actual load-picks-highest usage. Confidence: HIGH — Verification: read + rg (install.rs:345).

F-003 [S4]
Where: crates/plugin-api/src/semver.rs:251-274 (test gap; file tests)
Code:
```
for bad in [
    "",
    "1",
```
What is wrong: The test corpus has no overflow-boundary cases: `18446744073709551615.0.0` (u64::MAX, must parse Ok), `18446744073709551616.0.0` (must be `Overflow`), and `1.2.3-18446744073709551616` (parses Ok; precedence hole of F-001). The `Overflow` variant (L215-216) is therefore never asserted by any test — mutation reasoning: replace the `parse::<u64>` error arm with a panic and no test fails; likewise F-001's clamping is invisible. What correct looks like: three extra corpus entries + two precedence assertions. Confidence: HIGH — Verification: read (corpus enumerated at L252-271).

## 4. Per-function table (mandatory, file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Semver::parse` | 41-84 | REAL | — | grammar correct incl. tricky multi-separator cases; typed errors; Overflow typed |
| `Semver::major/minor/patch` | 88-100 | REAL | — | const accessors |
| `Semver::precedence` | 107-129 | REAL | S3 | core tuple + pre-presence arms correct; delegates to compare_pre (F-001) |
| `Display for Semver` | 132-143 | REAL | — | round-trips (tested) |
| `valid_identifiers` | 148-168 | REAL | — | §9/§10 correct; leading-zero rule correctly pre-only |
| `compare_pre` | 171-197 | REAL | S3 | F-001 unwrap_or(0) clamp on >u64 numeric identifiers; set-size and numeric<alpha rules correct |
| `SemverError` | 201-223 | REAL | — | 6 typed variants, actionable messages |
| 5 tests | 229-321 | REAL | S4 | strong corpus; gaps per F-003 |

## 5. Test honesty (if tests exist)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| doc_examples_parse | YES | round-trip equality on 3 versions |
| full_grammar | YES | full pre+build parse + exact round-trip string |
| rejects_bad_versions | YES | 18 distinct malformed shapes each asserted err |
| precedence_rules | YES | §11 worked example both directions + build-ignored + equality-still-distinguishes-builds |
| zero_is_legal_but_only_unprefixed | YES | 0.0.0 / 0.0.0-0 ok, 0.0.0-00 err |
Gap: F-003 (overflow/`Overflow` variant never asserted; F-001 case absent).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Grammar implemented (semver.org, §1–§9)" | L7 | REAL | read-verified incl. §9/§10 asymmetries; tests |
| "numeric ⇒ no leading zeros" | L13 | REAL | L69-71 core, L163-164 pre-only — build correctly exempt (§10) |
| "the v1 load rule only needs the major" | L17-18 | FALSE | install.rs:345 uses full precedence; lifecycle test asserts 0.2.0 beats 0.1.0 (F-002) |
| "identifiers compare numeric < alphanumeric, field-by-field, shorter set loses" | L105-106 | REAL (except >u64 numerics) | precedence_rules test; F-001 for the hole |

## 7. Dead / duplicated / unreachable code
Not dead (used by manifest.rs:38,250-262 via `SemverField`), but runtime-unreachable from the shipped CLI (no shipped `PluginManifest` parse). No duplication.

## 8. Dependencies & flags
thiserror (declared external), std only otherwise. No feature gates. No unsafe. plugin-api is in the shipped dep graph, so this code compiles into `lumen`/desktop binaries as inert weight.

## 9. Open questions for the next agent
- If dependency resolution (the doc's "[PROPOSAL]") lands, F-001 becomes an S2: resolver ordering on attacker-supplied versions. Flag for the plugin-registry phase.
