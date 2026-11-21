# AUDIT: crates/commands/src/catalog/codegen.rs
| Field | Value |
|---|---|
| Auditor | W02-A05 |
| Date | 2026-11-21 |
| Lines / functions | 552 / 16 (+7 tests) |
| File verdict | REAL (executed dev tooling; UNREACHABLE from the shipped `lumen`/`lumen-tauri` binaries) |
| Reachable from shipped entry point? | NO for the binaries (rg `codegen::` over apps/ → 0 hits). YES from dev/CI chain: `cargo run -p xtask --locked -- codegen --check` (`.github/workflows/ci.yml:280`) → `catalog-gen` bin (`crates/commands/src/bin/catalog_gen.rs:92-118` calls `generate_outputs`/`check_against`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg callers of `default_paths|generate_outputs|check_against` (→ bin + 2 test files only); sed quote verification; python scan of `schemas/commands.schema.json` (23 objects carry `properties`+`additionalProperties:false`, 0 carry a schema-valued map; 58 `$defs`; no `prefixItems`/`patternProperties`/`propertyNames` — the 3 rg hits were the word "contains" in doc strings); rg `preserve_order` Cargo.lock → 0 hits (BTreeMap order, determinism claim holds); rg `deny_unknown_fields` catalog/ → 23+ hits |

## 1. One-paragraph verdict
This is the one piece of the catalog that does exactly what it advertises: a deterministic, dependency-free JSON-Schema→TypeScript emitter plus drift checking, with byte-for-byte golden tests that ran and passed in the baseline (`audit/runs/baseline/test-workspace.log.txt:1306-1310,1362,1375-1376,1392`). It is dev tooling, not shipped-engine code — nothing in `apps/` calls it — and it is honestly wired into CI via `xtask codegen --check`. The defects found are drift between the generated TS surface and the Rust wire surface (`deny_unknown_fields` constraints silently dropped), a stale doc-05 quote in the header, and two pieces of dead/fragile test-support code. No fakeness, no panic paths on realistic input, no swallowed errors.

## 2. What actually works (verified)
— verified by `audit/runs/baseline/test-workspace.log.txt` lines 1306-1310, 1362, 1375-1376, 1392: all 5 unit tests of this module plus `catalog_drift::committed_artifacts_match_regeneration`, `regeneration_is_deterministic`, `catalog_shape::committed_checksum_recomputes` ran **ok** (baseline #3: 1681 passed / 1 unrelated failure).
— verified by `sed -n '1,12p' schemas/generated/commands.checksum.txt` → `counts: 65 commands / 26 events` matches the value `checksum_text` computes from `catalog_data().len()`/`event_catalog_data().len()`; digests present.
— verified by `sed -n '113,123p' ui/src/generated/commands.ts` → `export type CatalogDocument = { event: Event; request: CommandRequest; response: CommandResponse };` — the root emission works; committed file byte-matches regeneration (drift test ok above), and the committed file compiles under tsc (baseline #22, `ui-build.log.txt`).
— verified by rg `preserve_order` over `Cargo.lock` → 0 hits: serde_json uses BTreeMap, so the header's determinism argument (§lines 18-23) holds for this workspace.

## 3. Findings
### F-001 [S3]
Where: crates/commands/src/catalog/codegen.rs:8-16 (against doc 05 §5.1, `docs/05-command-layer.md:481`)
Code:
```rust
//! Doc 05 §5.1: "`crates/commands` derives `schemars::JsonSchema` for
//! every wire type; `cargo xtask schema-gen` emits (a)
//! `schemas/commands.schema.json` — the single machine description of
//! the catalog; (b) the TS types the UI imports … No TS type is ever
//! hand-written." Card 1.D4 owns this generation (the xtask subcommand
//! is `codegen`, shelling out to the `catalog-gen` bin, which calls the
```
What is wrong: The header quotes doc 05 §5.1 verbatim, which names the subcommand `cargo xtask schema-gen` and (doc line 481) the output path `ui/packages/commands/src/types.ts`. The implementation is the `codegen` subcommand writing `ui/src/generated/commands.ts` (lines 91-93). The parenthetical corrects only the subcommand name, not the artifact path; `ui/packages/` does not exist (checked with `ls ui/packages` → nothing).
Why it matters: A reader following the module's own citation chain lands on a doc that misnames both the command and the artifact location; the "No TS type is ever hand-written" audit trail points at a path that was never the real one.
What correct looks like: Either doc 05 §5.1 updated to `cargo xtask codegen` / `ui/src/generated/commands.ts`, or the header here stating explicitly that both the subcommand and the output path drifted from the quoted doc.
Confidence: HIGH — Verification: read (doc 05:481, `ui/src/generated/commands.ts`, `tools/xtask/src/cmd/codegen.rs:67`).

### F-002 [S3]
Where: crates/commands/src/catalog/codegen.rs:326-331 (and 317-364 generally)
Code:
```rust
    if properties.is_empty() {
        if let Some(additional) = schema.get("additionalProperties") {
            return format!("Record<string, {}>", ts_type(additional, defs));
        }
        return "Record<string, unknown>".to_owned();
    }
```
What is wrong: `ts_object` consults `additionalProperties` only when `properties` is empty. Python scan of the committed schema: 23 objects carry BOTH `properties` and `additionalProperties` — all 23 are `additionalProperties: false` (emitted because 23+ wire structs carry `#[serde(deny_unknown_fields)]`, e.g. `types.rs:113,129,145,160,241,266,343`, `command.rs:674`). In every one of those 23 cases the emitter drops the constraint, so the emitted TS type literals are open (extra fields allowed) while the Rust wire types reject unknown fields at deserialization.
Why it matters: The module's stated purpose (header lines 8-11) is that the TS surface is *the* machine description of the catalog — "no TS type is ever hand-written". A UI author who adds or misspells a field gets no compile error on the TS side but a hard deserialization failure on the Rust side. The two type surfaces disagree on strictness, which is exactly the drift class this codegen exists to prevent.
What correct looks like: Emit exact types for `additionalProperties: false` (idiomatic approximation: a comment, or `Exact<T>` helper, or at minimum a documented note in the generated header that TS types are non-exact by design).
Confidence: HIGH — Verification: read + python scan of `schemas/commands.schema.json` (23 both-keyed objects, 0 with schema-valued maps); rg `deny_unknown_fields` in `crates/commands/src/catalog` → 23+ hits.

### F-003 [S4]
Where: crates/commands/src/catalog/codegen.rs:240-244
Code:
```rust
        if parts.len() == 1 {
            return parts
                .into_iter()
                .next()
                .unwrap_or_else(|| "never".to_owned());
        }
```
What is wrong: Inside `if parts.len() == 1`, `next()` on a 1-element iterator is always `Some`; the `unwrap_or_else(|| "never".to_owned())` fallback is dead code that can never execute.
Why it matters: Dead defensive branches mislead readers into thinking the case is reachable, and add an untested (untestable) path.
What correct looks like: `.expect("len==1 guarantees an element")` or restructure as `parts.join(" | ")` without the special case (a 1-element parenthesized union is valid TS anyway).
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/commands/src/catalog/codegen.rs:453-464
Code:
```rust
    fn ts_leaks_unknown_type(typescript: &str) -> bool {
        let mut outside_quotes = String::new();
        let mut in_quotes = false;
        for ch in typescript.chars() {
            match ch {
                '"' => in_quotes = !in_quotes,
                _ if !in_quotes => outside_quotes.push(ch),
                _ => {}
            }
        }
        outside_quotes.contains("unknown")
    }
```
What is wrong: The quote-stripper ignores backslash escaping. `ts_literal` (line 370) emits Rust-`Debug`-escaped strings, which render an embedded double quote as `\"`; the helper toggles `in_quotes` on that inner quote and then misclassifies everything after it. No current catalog literal contains a quote (verified: the emitted needles and committed `commands.ts` have none), so the guard is latent, not active.
Why it matters: The helper backs the `typescript_exports_the_wire_surface` assertion "unmapped schema shape leaked into the TS output"; with quoted data containing `\"`, the guard could false-pass or false-fail exactly when a shape genuinely leaks.
What correct looks like: Track the previous character and skip a `"` preceded by `\`, or lex the string properly.
Confidence: HIGH (that the weakness exists) / latent (no current input triggers it) — Verification: read; rg over `ui/src/generated/commands.ts` for `\\\"` → none present today.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `WireRequest`/`WireResponse`/`WireEvent` (type aliases) | 39-43 | REAL | — | Trivial, correct; generic instantiations used by `CatalogDocument`. |
| `CatalogDocument` | 49-57 | REAL | — | Root schema object; shape verified via committed schema + `catalog_shape` test (baseline ok). |
| `Outputs` | 60-68 | REAL | — | Plain data carrier. |
| `OutputPaths` | 72-80 | REAL | — | `Copy` struct of borrowed paths. |
| `default_paths` | 89-96 | REAL | — | `root` param is honored via `.under(root)` (`root.join`); temporaries live to end of statement — sound. Leaks 3 PathBufs per call; documented, call sites short-lived. |
| `OutputPaths::under` | 102-111 | REAL | — | `Box::leak` documented rationale (bin/test one-shot processes); acceptable. |
| `generate_outputs` | 119-132 | REAL | — | Executed (baseline test log 1306-1309, 1362); errors mapped with context; no panics. |
| `checksum_text` | 137-153 | REAL | — | Counts verified against committed checksum file (`65 commands / 26 events`); calls `catalog_data()` twice — harmless one-shot cost. `lumen-allow-literal` markers present and justified. |
| `check_against` | 160-187 | REAL | — | Executed via `check_reports_doctored_artifacts` (baseline ok); NotFound→drift, other I/O→Err; messages actionable. |
| `emit_typescript` | 197-224 | REAL | — | Deterministic (sorted def names; BTreeMap serde_json confirmed by Cargo.lock scan); clones the whole `$defs` map — one-shot tool, acceptable. |
| `ts_type` | 227-312 | REAL | F-002 context | The object-with-properties branch BEFORE `oneOf` (253-260) is load-bearing for the flattened envelope — comment explains why; verified in emitted `CommandRequest` intersection. `None` arm re-reads `type` as array for `["string","null"]` — correct. Unsupported keywords (`prefixItems` etc.) absent from the schema today (python scan). |
| `ts_object` | 317-364 | REAL | F-002 | Drops `additionalProperties:false` when properties exist (see F-002); `oneOf`-beside-properties intersection emitted correctly. |
| `ts_literal` | 368-374 | REAL | — | Debug-format escaping is TS-compatible (`\"`, `\\`, `\u{...}` all valid in TS string literals). |
| `is_ts_identifier` | 377-384 | REAL | — | Allows TS keywords as unquoted keys — valid TS (`{ class: CompletionClass }` compiles, and does in `commands.ts:16`); conservative enough. |
| tests: `generation_is_deterministic_and_idempotent` | 390-401 | REAL | — | See §5. |
| tests: `schema_document_references_all_three_envelopes` | 403-421 | REAL | — | See §5. |
| tests: `typescript_exports_the_wire_surface` | 423-448 | REAL | F-004 | See §5. |
| helper `ts_leaks_unknown_type` | 453-464 | REAL | F-004 | Latent escaping bug, see F-004. |
| tests: `checksum_text_carries_the_counts_and_digests` | 466-477 | REAL | — | Pins `65 commands / 26 events` — matches committed file. |
| tests: `check_reports_doctored_artifacts` | 479-544 | REAL | — | Genuine negative control: clean→empty, doctored→reported, missing→reported; temp dir keyed by pid, cleaned up (`let _` on cleanup only — acceptable). |
| tests: `ts_identifier_check_is_conservative` | 546-551 | REAL | — | 3 asserts incl. empty-string negative. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `generation_is_deterministic_and_idempotent` | YES for in-process nondeterminism | HashMap-order or time-dependent emission would differ between the two calls. Does not catch cross-build/environment nondeterminism — covered instead by `catalog_drift::committed_artifacts_match_regeneration` (baseline ok). |
| `schema_document_references_all_three_envelopes` | YES | Removing `response` from `CatalogDocument` or a wire tag drops a needle; substring check is weak (would pass with `"CommandResponseX"`) but adequate as a tripwire. |
| `typescript_exports_the_wire_surface` | PARTLY | Catches gross emitter breakage (missing exports, bare `unknown` leak); does NOT run tsc — TS *validity* is only proven by baseline #22 compiling the byte-identical committed file. Escaping weakness in the helper = F-004. |
| `checksum_text_carries_the_counts_and_digests` | YES | Dropping a catalog row changes the count line; hard-coded `65/26` also pins accidental catalog growth. |
| `check_reports_doctored_artifacts` | YES | The strongest test here: three behavioral scenarios (clean / doctored / missing) against `check_against`; a `check_against` that always returns empty drift fails the doctored and missing legs. |
| `ts_identifier_check_is_conservative` | YES (for its scope) | `assert!(!is_ts_identifier(""))` fails if the predicate degenerates to `true`. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`cargo xtask schema-gen` emits … `ui/packages/commands/src/types.ts`" | doc 05 §5.1 (`docs/05-command-layer.md:481`), quoted in this file's header | **FALSE** (path) / renamed (subcommand) | Real subcommand is `codegen` (`tools/xtask/src/cmd/mod.rs:95`), real path `ui/src/generated/commands.ts` (`codegen.rs:92`); `ui/packages/` does not exist. See F-001. |
| "No TS type is ever hand-written" | doc 05 §5.1 | REAL (for `ui/src/generated/commands.ts`) | Drift test byte-compares committed TS to regeneration (baseline `test-workspace.log.txt:1375` ok); header of committed file says `@generated`. Not audited here: whether OTHER hand-written TS types exist in `ui/src` (UI wave's scope). |
| "the same code the golden tests call, so the committed artifacts and the tests can never disagree" | this file's header (14-16) | REAL | `catalog_drift.rs:16-17` imports the same `codegen::{default_paths, generate_outputs}`; baseline ok. |
| "Regeneration is idempotent; the outputs are committed and `--check` fails on drift" | header lines 22-23 | REAL | `--check` → exit 1 on drift (`catalog_gen.rs:103-108`); CI runs it (`ci.yml:280`). |
| "schemars 1.x emits maps as BTreeMaps … serde_json (without preserve_order, as configured here) keeps that order" | header lines 18-20 | REAL | rg `preserve_order` Cargo.lock → 0 hits; Cargo.toml pins `serde_json = "1.0.154"`, `schemars = "1.2.3"`. |

## 7. Dead / duplicated / unreachable code
- `.unwrap_or_else(|| "never".to_owned())` fallback is dead (F-003).
- No unreachable exports otherwise; `default_paths` has real callers (`catalog_shape.rs:32,230`, `catalog_drift.rs:62`).
- Not reachable from the shipped `lumen`/`lumen-tauri` binaries (rg `codegen::` in `apps/` → 0 hits) — by design; reachable from the `catalog-gen` bin and CI.

## 8. Dependencies & flags
- `schemars = "1.2.3"`, `serde_json = "1.0.154"` (crates/commands/Cargo.toml:40-41) — no feature flags gate this module; no cfg-gating to hide code; `preserve_order` not enabled workspace-wide (Cargo.lock scan).
- No unsafe blocks in this file. No panics on realistic input (no unwrap/expect/index outside test `unreachable!` messages, which carry the error text).

## 9. Open questions for the next agent
- Does the UI rely on exactness anywhere (e.g., a `satisfies`/exhaustiveness check) that F-002 would weaken? UI-wave agent to check `ui/src/client/transport.ts` consumers of `commands.ts`.
- Doc 05 §5.1 also claims "(c) validation JSON Schemas reused by the control server and the keymap validator" — whether that reuse exists is NOT verified here (no `schemas/commands.schema.json` consumer found yet in this audit; control-protocol/`protocol.rs` audit may answer).
