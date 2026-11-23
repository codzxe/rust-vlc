# AUDIT: crates/commands/src/catalog/codegen.rs
| Field | Value |
|---|---|
| Auditor | W02-A10 |
| Date | 2026-11-24 |
| Lines / functions | 552 / 14 prod items + 7 tests + 1 test helper |
| File verdict | REAL |
| Reachable from shipped entry point? | NO for the generator itself (rg over apps/: zero callers of `generate_outputs`/`codegen`; it runs via the `catalog-gen` bin, `cargo xtask codegen` and CI drift gate `.github/workflows/ci.yml:280` `cargo run -p xtask --locked -- codegen --check`). Its OUTPUT ships: `ui/src/generated/commands.ts` is imported by the shipped UI (`ui/src/screens/convert/convertQueue.ts:27` et al.) and the commands crate (envelope/catalog types) is a lumen-cli dependency. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `cargo test -p commands --lib catalog::codegen` -> 6 passed, exit 0; `cargo run -q -p commands --bin catalog-gen -- --check` -> "no drift — outputs match the catalog (65 commands / 26 events)", exit 0; rg caller checks; sed quote verification. |

## 1. One-paragraph verdict
The codegen is a genuine Rust-types->schemars->JSON-Schema->TypeScript pipeline and the drift gate is honest as far as it goes: I regenerated and the committed artifacts match byte-for-byte, and the doctored-artifact negative control proves the *detector* works. But the answer to "is it circular?" is: half yes. The JSON Schema cannot drift from the Rust types because both are produced by the same `#[derive(JsonSchema)]` — that half is sound. The TS half is a hand-rolled emitter (`emit_typescript`/`ts_type`/`ts_object`) whose only anchors are substring greps, self-regeneration, and the UI's `tsc` pass — none of which check semantics. I found a live semantic defect that passes every gate: `PlaylistItemId(pub u64)` (types.rs:66) is emitted as `export type PlaylistItemId = number;` (ui/src/generated/commands.ts:62), and a u64 above 2^53 silently corrupts in JS. That is the concrete proof of the circularity hole. External anchors that DO exist and work: catalog_shape.rs pins doc-05 counts (65/26, 19A/46S) and the id grammar against the committed schema, and wire_forms.rs pins serde wire forms.

## 2. What actually works (verified)
- 6/6 module tests pass — `cargo test -p commands --lib catalog::codegen` -> `test result: ok. 6 passed; 0 failed` (determinism/idempotency, envelope presence, TS surface substrings, no-`unknown`-type leak, checksum counts, doctored-artifact drift detection).
- Cross-process determinism + no drift — `cargo run -q -p commands --bin catalog-gen -- --check` -> `catalog-gen: no drift — outputs match the catalog (65 commands / 26 events)`, exit 0: files committed by an earlier process match this process's regeneration.
- CI enforces the gate: `.github/workflows/ci.yml:280` runs `cargo run -p xtask --locked -- codegen --check`.
- `check_reports_doctored_artifacts` (codegen.rs:483-544) is a real negative control: seeds one doctored schema byte, asserts drift is reported; deletes a file, asserts the "missing" message — the drift detector is tested, not just exercised.

## 3. Findings
F-001 [S3]
Where: crates/commands/src/catalog/codegen.rs:288-289 (with crates/commands/src/catalog/types.rs:66)
Code:
```
        Some("string") => "string".to_owned(),
        Some("integer") | Some("number") => "number".to_owned(),
```
and types.rs:66:
```
pub struct PlaylistItemId(pub u64);
```
What is wrong: every JSON-Schema `integer` — including the u64 `PlaylistItemId` — is emitted as TS `number`. JS `number` is an f64: integers above 2^53 lose precision. The committed artifact proves it: `ui/src/generated/commands.ts:62` is `export type PlaylistItemId = number;`, and `playlist.play-item/remove/move` plus `PlaylistChange.ids` all carry it. Why it matters: the UI is a shipped consumer of this wire; a playlist id minted as a large/random u64 round-trips corrupted, and the TS type system blesses it. Mitigation today: rg shows `PlaylistItemId(` is constructed only in tests (no production minter yet), so nothing currently emits a big id — the hole is latent but contractual. What correct looks like: emit u64/i64 fields as `string` (or `bigint`) with a documented rule, as JSON-safe APIs do. Confidence: HIGH — Verification: read + ran (all gates pass with the defect present, which IS the finding). Severity note: bumped to S3 not S2 because no production code path currently mints a value above 2^53 (Timestamp is µs i64 — 2^53 µs ≈ 285 years, safe; Bookmark id is u32, safe).
F-002 [S3]
Where: crates/commands/src/catalog/codegen.rs:197-224 and 227-312 (the whole hand-rolled TS emitter)
Code:
```
fn emit_typescript(schema: &serde_json::Value) -> String {
    let mut out = String::new();
    out.push_str(
        "// @generated — Lumen command-catalog TypeScript types (card 1.D4, doc 05 §2-§3, §5.1).\n", // lumen-allow-literal: generated-artifact header emitted by codegen — folding the generator into the rebrand surface set is the recorded follow-up
    );
```
What is wrong: the golden/drift tests (catalog_drift.rs, catalog_shape.ts-side, and the substrings here) regenerate with this same emitter, so any semantic regression in `ts_type` — wrong optionality, wrong nullable form, wrong integer mapping (F-001 is the live instance) — regenerates the committed artifact and goes green. The module doc admits the shared-code design at codegen.rs:12-16 ("the same code the golden tests call, so the committed artifacts and the tests can never disagree about what 'regenerated' means") — which guarantees consistency, never correctness. Why it matters: this is the exact "tests generated by the code they test" shape the audit was asked to hunt; it is confined to the TS surface (the schema half is anchored by construction to the Rust derives), but that surface is what the shipped UI compiles against. What correct looks like: an independent anchor for the TS — e.g. a committed hand-checked excerpt asserted verbatim (not regenerated), or property tests mapping known Rust values to expected TS snippets, or emitting JSON-Schema-compatible TS via a battle-tested generator. Confidence: HIGH — Verification: read + ran (F-001 passed all gates).
F-003 [S4]
Where: crates/commands/src/catalog/codegen.rs:102-110
Code:
```
        OutputPaths {
            schema: Box::leak(schema.into_boxed_path()),
            typescript: Box::leak(typescript.into_boxed_path()),
            checksum: Box::leak(checksum.into_boxed_path()),
        }
```
What is wrong: `Box::leak` in a `pub fn` (`default_paths`/`under`) — every call leaks three paths. Current callers resolve once per process (tests; the bin uses its own copy), so nothing leaks in practice; the doc-comment says so. A long-lived caller looping `default_paths` would leak unboundedly. What correct looks like: return owned `PathBuf`s and drop the `Copy` derive, or take a &'static root once. Confidence: HIGH — Verification: read (callers enumerated by rg).
F-004 [S4]
Where: crates/commands/src/catalog/codegen.rs:240-245
Code:
```
        if parts.len() == 1 {
            return parts
                .into_iter()
                .next()
                .unwrap_or_else(|| "never".to_owned());
        }
```
What is wrong: `parts.len() == 1` guarantees `.next()` is `Some`; the `unwrap_or_else(|| "never")` branch is unreachable dead code. Harmless but it is the kind of never-executed defensive line that hides review attention. What correct looks like: `.expect("len checked")` or restructure with match. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/commands/src/catalog/codegen.rs:82-88
Code:
```
/// The three relative locations live here so the bin, the golden
/// tests and this module cannot disagree about where the committed
/// artifacts belong.
```
What is wrong: the doc claims a single source of truth for the artifact locations, but the `catalog-gen` bin does NOT call `default_paths` — it hard-codes its own three paths in `crates/commands/src/bin/catalog_gen.rs:217-227` (`root.join("schemas").join("commands.schema.json")`, etc.). Two independent definitions exist; they currently agree. The safety net is real (if they diverged, the drift tests reading `default_paths` would fail on the stale file), so this is a doc-accuracy defect, not a functional one. What correct looks like: the bin calls `default_paths(root)` (its `--out-dir`/`--*-out` overrides layered on top), making the doc true. Confidence: HIGH — Verification: read (both files).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WireRequest/WireResponse/WireEvent (type aliases) | 39-43 | REAL | — | alias wiring for the schema root. |
| CatalogDocument | 49-57 | REAL | — | schema root; verified in committed schema (title/sections). |
| Outputs / OutputPaths (structs) | 60-80 | REAL | — | plain data. |
| default_paths | 89-96 | REAL | S4 | F-003 leak; F-005 doc claim. |
| OutputPaths::under | 102-112 | REAL | S4 | F-003. |
| generate_outputs | 119-132 | REAL | — | schemars -> pretty JSON + Value + TS + checksum; ran clean twice (idempotency test) and cross-process (--check). |
| checksum_text | 137-153 | REAL | — | fnv1a64 digests incl. catalog checksum; counts pinned by test. |
| check_against | 160-187 | REAL | — | byte-exact drift compare; NotFound => drift; negative control test covers doctor+missing. |
| emit_typescript | 197-224 | REAL | S3 | F-002: hand-rolled, self-anchored. |
| ts_type | 227-312 | REAL | S3 | F-001: integer->number for u64; flatten-before-union ordering is correct for envelopes. |
| ts_object | 317-364 | REAL | — | Record<> for maps, required->optional mapping, `& (union)` for flattened enums — matches committed output. |
| ts_literal | 368-374 | REAL | — | strings quoted via {:?}, numbers via to_string. |
| is_ts_identifier | 377-384 | REAL | — | conservative; tested. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| generation_is_deterministic_and_idempotent | YES (for nondeterminism) | two full runs compared byte-exact; catches map-ordering regressions only. |
| schema_document_references_all_three_envelopes | YES (for absence) | substring needles incl. real command/event tags; a schema missing a variant fails. |
| typescript_exports_the_wire_surface | PARTIALLY | substrings catch missing exports, and `ts_leaks_unknown_type` is a real tripwire for unmapped shapes; but semantic errors (F-001) pass. |
| checksum_text_carries_the_counts_and_digests | YES (for counts) | hard-pins "65 commands / 26 events" — must be (and is meant to be) bumped with catalog changes. |
| check_reports_doctored_artifacts | YES | real negative control: doctored byte and missing file both asserted reported. |
| ts_identifier_check_is_conservative | YES | direct unit of the helper. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No TS type is ever hand-written" (doc 05 §5.1, quoted at codegen.rs:10-11) | codegen.rs:8-11 | REAL (for authorship) | commands.ts header says @generated; --check passes; but the EMITTER is hand-written and semantically unanchored (F-002). |
| "the same code the golden tests call, so the committed artifacts and the tests can never disagree about what 'regenerated' means" | codegen.rs:12-16 | REAL | accurate and honest about the consistency-only guarantee — the doc does not oversell correctness. |
| "Determinism: ... the same types always produce byte-identical output ... Regeneration is idempotent; the outputs are committed and `--check` fails on drift." | codegen.rs:18-23 | REAL | idempotency test + my cross-process `--check` run, exit 0. |
| "The three relative locations live here so the bin ... cannot disagree" | codegen.rs:84-86 | FALSE (doc) | bin hard-codes its own paths (catalog_gen.rs:217-227); F-005. |
| schemars emits maps sorted (BTreeMap) | codegen.rs:18-19 | REAL | schemars 1.2.3 locked (Cargo.lock:822), no preserve_order; empirically byte-identical output. |

## 7. Dead / duplicated / unreachable code
- Dead branch: codegen.rs:244 (`unwrap_or_else(|| "never")`, F-004).
- Duplication: artifact-path constants exist in both codegen.rs:91-93 and catalog_gen.rs:208-228 (F-005).
- Not dead: every pub fn has a caller (bin/tests). `default_paths` has no bin caller today (tests only).

## 8. Dependencies & flags
serde 1.0.229 (derive), serde_json 1.0.154 (no preserve_order), schemars 1.2.3 — locked, preaudit-approved per Cargo.toml comments. No feature gates. No unsafe. Bin target `catalog-gen` in the same crate.

## 9. Open questions for the next agent
- Does anything ever mint a real `PlaylistItemId` in production (playlist engine)? If yes, F-001 escalates from latent to live (S2).
- catalog_shape.rs (not in my list) is the strongest anti-circularity anchor — whoever owns it should confirm the 65/26 and 19A/46S counts against doc 05 itself, not just against `catalog_data()`.
