# AUDIT: crates/product-config/src/load.rs
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-21 |
| Lines / functions | 202 / 8 fns + 2 tests |
| File verdict | REAL (base loader); the partial/user-layer half is REAL code with NO production caller |
| Reachable from shipped entry point? | `load_file`/`load_str`: NOT from `lumen` CLI or `lumen-tauri` (rg over apps/*/Cargo.toml: no product-config dep). YES from workspace release binary `lumen-control-server`: main.rs:174 `load_product` → main.rs:154 `product_config::load_file(path, &schema)` → load.rs:159 `load_str` (runs BEFORE the feature gate; product.toml:167 `control-server = false` gates the serving, not the load). `load_partial_str`/`load_partial_file`: NO production caller — product-config's own tests only (tests/web_budgets.rs:61, tests/merge.rs:23,77,96). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | Read file complete; Grep `load_file\|load_str\|load_partial_str\|load_partial_file` repo-wide (production callers: apps/control-server/src/main.rs:154 only; config.rs:240 and e2e.rs:91 are tests; everything else product-config's own tests + tests/rebrand); rg 'product-config' over apps/*/Cargo.toml (single dependent: control-server); Read apps/control-server/src/main.rs:95-174 (startup chain, pre-gate order); Read product.toml:145-171 (features.control-server=false); Grep `resolve_layers\|merge::layer` (test-only — confirms W11-A04's semantic.rs audit); sed -n verification of quoted ranges; baseline: load::tests green (orchrun-test-workspace-nofailfast.log.txt:4627,4629), rebrand suite through the real pipeline green (4724-4729), web_budgets layer test green (4757-4761) |

## 1. One-paragraph verdict
The base loader (parse → JSON → schema pass → typed model, with honest residual-error reporting) is real, well-built and executed: control-server runs it at startup before its feature gate, the rebrand acceptance suite drives the committed product.toml and a Nova fixture through this exact pipeline, and every failure mode produces a structured diagnostic rather than a silent fallback. The file's central lie is one of composition, not code: its second half — `load_partial_str`/`load_partial_file`, the "user preferences, §4.2 layer 4" loader — has zero production callers. lib.rs:23-26 advertises "v1 implements *product defaults + user*: load the base with `load_str`, the user layer with `load_partial_str`, then `merge::resolve_layers`", but no binary, app or UI in the tree performs that composition; user settings written via `config.set-setting` land in an in-memory `BTreeMap` (apps/cli/src/host.rs:181) and die with the process. One of the two tests is tautological for its named claim.

## 2. What actually works (verified)
- The three-stage base pipeline is wired and exercised: control-server main.rs:174→154→load.rs:159→68; stage-1 syntax failures produce a located `Stage::Parse` diagnostic (load.rs:71), stage-2 violations come back as a full `Vec<Diagnostic>` (load.rs:78-81), and a stage-2-clean document that still fails typed deserialization is reported as an explicit "the schema pass and the typed model disagree — this is a product-config crate bug" error (load.rs:84-94) instead of being swallowed.
- The committed product.toml loads through this pipeline in baseline evidence: control-server config.rs test `repo_product_today_refuses_the_feature_gate` (loads the real file via `load_file`, green — test-workspace.log.txt:1499), and `tests/rebrand/rebrand_acceptance.rs` — `surface_fixture_load_nova_through_the_real_pipeline ... ok` (nofailfast 4727) — loads a full rebrand fixture via `load_str` (rebrand_acceptance.rs:67).
- TOML→JSON conversion failure is a real, reachable guard: non-finite floats / non-representable values produce the stage-1 "must be representable as JSON" diagnostic (load.rs:50-65) rather than a panic or silent coercion.
- `line_of_offset` boundary check (load.rs:21-23) rejects out-of-range offsets (returns None) — pinned by an exact-value test (177-185, including the 999→None case; green at nofailfast 4627).

## 3. Findings

### F-001 [S2] The layer-4 "user preferences" loader is production-dead — lib.rs's "v1 implements product defaults + user" composition exists only in tests
Where: crates/product-config/src/load.rs:98-133 (with lib.rs:23-26, apps/cli/src/host.rs:181,1404)
Code:
```rust
/// Stages 1–2 for a sparse layer document (user preferences, §4.2 layer 4):
/// every present key is validated against the same schema entries; missing
/// keys fall through to the next-lower layer (§4.4 rule 4).
pub fn load_partial_str(
```
and the crate-level claim it serves (lib.rs:23-26):
```rust
//! v1 implements *product defaults + user*: load the base with [`load_str`],
//! the user layer with [`load_partial_str`], then [`merge::resolve_layers`]
//! (or the [`merge::layer`] convenience).
```
What is wrong: Repo-wide caller search for `load_partial_str`/`load_partial_file` finds only product-config's own test suite (tests/web_budgets.rs:61; tests/merge.rs:23,77,96). No app calls them. The workspace's sole product-config dependent, `lumen-control-server`, calls `load_file` only (main.rs:154). Meanwhile the settings write path in the shipped CLI stores values in an in-memory map — host.rs:181 `settings: BTreeMap<String, SettingValue>`, host.rs:1404 `self.settings.insert(full, value.clone())` — with no file write, and the UI persistence layer is a documented localStorage mock (docs/feature-matrix.toml:2251-2252 "persistence is mock (localStorage), not the config.set-setting command path"). `merge::resolve_layers` is likewise test-only (W11-A04's semantic.rs audit, §1: "resolve_layers/`layer` themselves have no caller outside tests").
Why it matters: This is the preferences-persistence question of this wave, answered: there is none. Doc 07 §4.2/§4.1 defines layer 4 as "persisted settings written by `config.set-setting`"; the loading half of that contract (this file), the merging half (merge.rs), and the writing half (anywhere) are all unwired. Every setting a user changes dies at process exit. The crate-level "v1 implements" claim describes a composition that no shipped binary performs — the same declared-vs-consumed disease as Wave 8's `audio.set-eq`, one layer up: not a missing backend for one command, but a missing backend for the entire persistence story.
What correct looks like: One of: (a) the CLI host gains a layer-4 file (path from the platform adapter, per read_file's own hint at load.rs:147) — on boot `load_partial_file` + `resolve_layers`, on set-setting a validated write; or (b) lib.rs and doc 07 stop claiming "v1 implements product defaults + user" until the composition has a caller. The current state — real, tested loader functions exported from the public API (lib.rs:54) with no consumer — is the textbook "config declared but never read".
Confidence: HIGH — Verification: read + repo-wide caller grep; cross-checked W11-A04 semantic.rs audit (same conclusion for resolve_layers); UI mock status quoted from docs/feature-matrix.toml:2251-2252.

### F-002 [S3] `parse_error_carries_the_parser_line` cannot fail for the property it names — the line assertion is tautological
Where: crates/product-config/src/load.rs:29-32, 188-201
Code:
```rust
    let line = err
        .span()
        .and_then(|span| line_of_offset(src, span.start))
        .or(Some(1));
```
```rust
        assert_eq!(diags[0].stage, Stage::Parse);
        assert_eq!(diags[0].file, "bad.toml");
        assert!(
            diags[0].line.is_some(),
            "parse errors are locatable: {diags:?}"
        );
```
What is wrong: `.or(Some(1))` guarantees `line` is always `Some` — even when the span is missing or `line_of_offset` returns None. The test then asserts only `line.is_some()`. Mutations that keep the test green: make `line_of_offset` always return `None` (falls back to `Some(1)`), make it return `Some(9999)` for every offset, or delete the `span()` call entirely. The test name promises "carries the parser line"; the body proves only "a parse diagnostic exists with a stage and file". The line NUMBER (the thing the module doc advertises: "syntax failures carry the `toml` crate's span, converted to a 1-based line", load.rs:3-4) is never checked — `"a = \n[unclosed"` has its error at a specific line the test could have asserted.
Why it matters: A regression in span→line conversion (e.g. after a `toml` crate upgrade changing span semantics) would ship silently while this test keeps certifying "parse errors are locatable". The fallback itself also mislabels unlocatable errors as line 1 with no distinguishing mark.
What correct looks like: Assert the value: for `"a = \n[unclosed"` assert `diags[0].line == Some(2)` (or whatever the parser reports — pin it); add a case exercising `line_of_offset`'s None path and decide whether `Some(1)` or `None` is the honest diagnostic for a spanless error.
Confidence: HIGH — Verification: read; mutation reasoned through both `.or(Some(1))` arms.

### F-003 [S4] `line_of_offset` can panic on a non-char-boundary offset where its settings-schema twin cannot
Where: crates/product-config/src/load.rs:20-25
Code:
```rust
fn line_of_offset(src: &str, offset: usize) -> Option<u32> {
    if offset > src.len() {
        return None;
    }
    Some(1 + src[..offset].bytes().filter(|b| *b == b'\n').count() as u32)
}
```
What is wrong: `src[..offset]` panics if `offset` falls inside a multi-byte UTF-8 character. The duplicated twin in settings-schema (src/parse.rs:720-726) does the same computation via `src.as_bytes()[..bound]` — panic-free by construction. The two copies have diverged in the unsafe direction here.
Why it matters: The only current caller feeds `toml::de::Error::span().start()` (load.rs:31), and TOML token spans start on char boundaries, so no live panic path exists today — but the function is a general utility whose contract ("line of a byte offset") invites future callers, and the fix is one word (`as_bytes()`).
What correct looks like: `Some(1 + src.as_bytes()[..offset].iter().filter(|b| **b == b'\n').count() as u32)` — byte iteration over bytes, exactly like the settings-schema twin.
Confidence: HIGH — Verification: read both copies (load.rs:20-25, settings-schema/src/parse.rs:720-726).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `line_of_offset` | 20-25 | REAL | S4 | Correct counting + range guard; char-boundary panic risk (F-003). Test-pinned with exact values (177-185). |
| `parse_diagnostic` | 28-44 | REAL | S3 | Span→line with unconditional `Some(1)` fallback (F-002); full structured diagnostic (stage/file/key_path/rule/expected/found/hint) — good contract. |
| `toml_to_json` | 50-65 | REAL | — | Boxes the error for `result_large_err`; non-representable values (non-finite floats, exotic datetimes) become an honest stage-1 diagnostic; no silent coercion. |
| `load_str` | 68-96 | REAL | — | Stages 1–3 exactly as documented; early-returns on each stage; residual stage-3 failure reported as crate bug, never swallowed. Production-called (control-server main.rs:154). |
| `load_partial_str` | 101-133 | REAL / UNREACHABLE-in-production | S2 | Same pipeline with `RequiredMode::Partial` into `PartialProduct`; zero production callers (F-001). Exercised only by crate tests (green: web_budgets 4757-4761, merge tests). |
| `read_file` | 135-150 | REAL | — | IO errors → structured diagnostic with per-layer-locations hint; no unwrap. |
| `load_file` | 153-160 | REAL | — | Thin path wrapper; the one production entry (control-server startup). |
| `load_partial_file` | 163-170 | REAL / UNREACHABLE-in-production | S2 | Thin path wrapper; test-only callers (F-001). |
| tests ×2 | 176-201 | REAL | S3 | `line_of_offset_counts_newlines` mutation-sensitive (exact values + None case); `parse_error_carries_the_parser_line` tautological for the line claim (F-002). Both green (nofailfast 4627,4629). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `line_of_offset_counts_newlines` (177-185) | YES | Six exact assertions incl. boundary offsets 5/6 and 11/12 and the out-of-range 999→None. |
| `parse_error_carries_the_parser_line` (188-201) | NO (for the line claim) | `.or(Some(1))` makes `line.is_some()` unfalsifiable; the parser's actual line is never asserted (F-002). Would catch only a missing/mis-staged diagnostic. |
| External suite (context) | YES | tests/rebrand (through-the-real-pipeline fixture loads), tests/web_budgets.rs (`layers_cannot_override_web_budgets` exercises load_partial_str + the merge lockout), tests/merge.rs — all green in baseline (4721-4761), mutation-sensitive per their asserted diagnostics. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Stage 2 runs the data-driven schema pass and reports *every* violation. Stage 3 deserializes … after a clean stage 2 this cannot fail for schema-shaped input; if it does, the residual error is reported honestly instead of being swallowed." | load.rs:4-7 | REAL | Code matches (78-81, 84-94); rebrand suite drives real files through it green (4724-4729). |
| "syntax failures carry the `toml` crate's span, converted to a 1-based line" | load.rs:3-4 | REAL-UNPROVEN | Mechanism present (29-32) but no test pins the converted value, and the `Some(1)` fallback masks failures (F-002); not executed with a known-line fixture in evidence. |
| "Stages 1–2 for a sparse layer document (user preferences, §4.2 layer 4)" | load.rs:98-100 | REAL code / UNREACHABLE | Zero production callers (F-001). |
| "v1 implements *product defaults + user*: load the base with load_str, the user layer with load_partial_str, then merge::resolve_layers" | lib.rs:23-26 | FALSE (as a composition claim) | No caller anywhere performs it; resolve_layers test-only (W11-A04 audit §1); control-server calls load_file only. |
| Doc 07 §4.2/§4.1: layer 4 = "persisted settings written by `config.set-setting`" | docs/07-config-driven-system.md:466,470 | FALSE (in practice) | set-setting writes an in-memory BTreeMap (host.rs:181,1404); no file writer, no partial loader wired (F-001). |
| "Invalid critical config is never silently replaced (MP §5)" | lib.rs:32-33 | REAL | Every failure path in this file returns structured diagnostics; no fallback-to-defaults branch exists. |

## 7. Dead / duplicated / unreachable code
- UNREACHABLE-in-production: `load_partial_str`, `load_partial_file` (this file), and downstream `merge::resolve_layers`/`merge::layer` (W11-A04's confirmation) — real, tested, exported in the public API (lib.rs:54-55), called by nothing shipped.
- DUPLICATED: `line_of_offset` vs settings-schema/src/parse.rs:720-726 (diverged: byte-safe there, boundary-panicky here — F-003); the whole scanner duplication is catalogued in the lines.rs audit (F-001 there).
- NOT dead: `load_str`/`load_file`/`read_file`/`parse_diagnostic`/`toml_to_json` — control-server startup chain.

## 8. Dependencies & flags
External deps exercised in this file: `toml` (parse + Value), `serde_json` (Value bridge + typed deserialization) — both allowlisted (product-config Cargo.toml, W01-A10-audited family). No unsafe, no unwrap/expect (the two `unreachable!`-style behaviors live in callers' tests, not here); workspace lints inherited; no feature gates; std `fs` for file reads only.

## 9. Open questions for the next agent
- Whoever audits `apps/cli/src/host.rs` or the tauri shell: confirm no layer-4 file write exists anywhere (I found none — host.rs:181 BTreeMap only); F-001's fix will land in your file, not here.
- `config.reset-setting` is in the commands catalog (command.rs:425-426) with no apps/ handler found (grep) — if a future reset lands, it should route through this file's partial loader + merge to honor §4.4 rule 4.
- control-server's `--product` default (main.rs DEFAULT_PRODUCT) — if the default path is the repo's product.toml, then the feature gate (`control-server = false`) makes the binary refuse to serve out of the box; worth a dedicated look in the control-server wave (I only verified the load runs pre-gate).
