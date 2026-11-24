# AUDIT: crates/product-config/src/load.rs
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines / functions | 202 / 8 (line_of_offset, parse_diagnostic, toml_to_json, load_str, load_partial_str, read_file, load_file, load_partial_file) + 2 tests |
| File verdict | REAL (library code, honest errors) but UNREACHABLE from the shipped release lane |
| Reachable from shipped entry point? | NO. Release lane = lumen-cli + desktop (baseline). lumen-cli/desktop mention product-config only in comments (the owed "rebrand-propagation" join, apps/cli/src/host.rs:970). Actual callers: apps/control-server (main.rs:154, config.rs:240 — an app whose own feature gate is `control-server = false`, Phase 5+), product-config's own tests, and tests/integration. xtask does NOT use this crate (its config_validator walks product.schema.json itself). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg caller census; sed quote verification; `cargo test -p product-config` BLOCKED (disk full — "No space left on device"); baseline cited instead |

## 1. One-paragraph verdict
This is a clean, genuinely defensive load pipeline: TOML parse with span→line conversion, a JSON-schema structural pass that reports every violation before typed deserialization, and an honest "this is a crate bug" diagnostic for the theoretically-impossible stage-3 failure. No unwrap/expect/panic in production paths, no swallowed errors, every failure carries file/line/rule/expected/found/hint. The problem is who calls it: nobody shipped. The CLI runs on hardcoded product.toml mirrors, the desktop app never imports the crate, and the one real consumer (control-server) is itself feature-gated off. The file is well-written plumbing connected to nothing the release lane ships.

## 2. What actually works (verified)
— Structure verified by read: parse → toml_to_json → schema.validate(RequiredMode::Full) → serde typed model, early-return on each stage; partial variant uses RequiredMode::Partial for sparse user layers (L80-96).
— The in-file tests are honest: `line_of_offset_counts_newlines` pins boundary offsets (0/5/6/11/12/999); `parse_error_carries_the_parser_line` feeds real broken TOML ("a = \n[unclosed") and demands a locatable line.
— Indirect execution: crates/product-config/src/runtime.rs:143-148 `the_shipped_product_toml_resolves_lumen_identity` calls `load_str` on the REAL product.toml via include_str! (passes in baseline; asserts "Lumen 0.1.3" identity strings). Direct `cargo test -p product-config` rerun: UNVERIFIED — disk full (log W11-A02).
— The five product-config fixtures I audited (files 3-7) are consumed through this pipeline's `load_fixture` harness (tests/common), i.e. load_str drives their rejection paths.

## 3. Findings
F-001 [S3]
Where: crates/product-config/src/load.rs:19-21
Code (verbatim):
```
/// Line (1-based) of a byte offset in `src`, if the offset is in range.
fn line_of_offset(src: &str, offset: usize) -> Option<u32> {
    if offset > src.len() {
        return None;
    }
    Some(1 + src[..offset].bytes().filter(|b| *b == b'\n').count() as u32)
}
```
What is wrong: `src[..offset]` panics ("byte index not a char boundary") if `offset` lands inside a multi-byte UTF-8 sequence. The offset comes from `toml::de::Error::span()` (L30-32) on attacker-influenceable config text (user preference layers, §4.2 layer 4, are parsed by `load_partial_str`). Whether the toml crate ever emits a span starting mid-char is UNVERIFIED (could not compile a probe: disk full); TOML tokens begin at ASCII boundaries so the practical risk looks low, but the function takes a raw byte offset and slices without `is_char_boundary` — the guard tests only the length bound. What correct looks like: `if !src.is_char_boundary(offset) { return None; }` (or floor to the previous boundary). Confidence: MEDIUM — Verification: read; trigger UNVERIFIED (cargo-build-budget/disk-full).
F-002 [S4]
Where: crates/product-config/src/load.rs:98-109 + repo-wide caller census
Code (verbatim, L98-101 — the partial-layer doc header):
```
        }]),
    }
}

/// Stages 1–2 for a sparse layer document (user preferences, §4.2 layer 4):
```
What is wrong (reachability, not correctness): the pipeline's only non-test consumers are control-server (feature-gated false) — the shipped CLI substitutes hardcoded mirrors (`PRODUCT_QUEUE_PACKETS_AUDIO: u64 = 1024`, apps/cli/src/demux_host.rs:71) and placeholder get-setting strings ("-> product.toml {path}", apps/cli/src/host.rs:1569). Every diagnostic field this file carefully fills is therefore unreachable in the release lane. This is the load-side counterpart of the settings.toml F-002 echo-chamber finding. Confidence: HIGH — Verification: rg census (lumen-cli + desktop: zero product_config imports).
F-003 [S4]
Where: crates/product-config/src/load.rs:46-64 vs tools/xtask/src/cmd/config_validator.rs:11
Code (verbatim, L46-50):
```
/// Convert a parsed TOML document to the JSON value the schema pass speaks
/// (doc 07 §3.4 stage 1: `product.toml` → TOML → `serde_json::Value`).
/// The error is boxed to keep the `Result` small (`clippy::result_large_err`);
/// callers unbox it into the aggregate `Vec<Diagnostic>`.
```
What is wrong: two independent product.toml validators exist — this crate's `schema.rs` pass (driven through load_str) and xtask's own JSON-schema walker ("JSON-Schema subset walk of product.toml against product.schema.json", config_validator.rs:11), which does not depend on product-config at all. Rule semantics (minimum/maximum, patterns) are implemented twice; they can drift (the fixtures I audited are exercised only through THIS crate's tests; xtask's lane is the one the baseline reports as config-validator FAILURE — 4 missing brand assets). Confidence: HIGH — Verification: read both; xtask Cargo.toml has no product-config dependency.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| line_of_offset | 16-21 | REAL | S3 | char-boundary slice risk (F-001); boundary-tested otherwise |
| parse_diagnostic | 24-44 | REAL | — | span→line with `.or(Some(1))` fallback; typed rule/expected/found/hint |
| toml_to_json | 46-64 | REAL | — | boxed diagnostic; documents the non-string-datetime limitation |
| load_str | 66-96 | REAL | — | Full mode; early returns; stage-3 failure reported as crate bug (honest) |
| load_partial_str | 98-131 | REAL | — | Partial mode for sparse user layers |
| read_file | 133-149 | REAL | — | `found` uses Debug path format (`{path:?}`) where the rest uses Display — cosmetic inconsistency |
| load_file | 151-158 | REAL | — | thin path wrapper |
| load_partial_file | 161-168 | REAL | — | thin path wrapper |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| line_of_offset_counts_newlines | YES | pins 6 offsets incl. out-of-range 999 |
| parse_error_carries_the_parser_line | YES | real malformed TOML; demands line.is_some |
| (external) fixtures_matrix over this pipeline | YES | exact-key/rule/line assertions per fixture (files 3-7 audits) |
| (external) runtime.rs the_shipped_product_toml_resolves_lumen_identity | YES | real product.toml through load_str; identity strings asserted — BUT uses `Schema::empty_for_tests()`, so it does not exercise the schema pass (noted; the schema pass is exercised by fixtures_matrix) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Stage 2 runs the data-driven schema pass and reports *every* violation" | L4-5 | REAL | fixtures_matrix multi-violation tests (range-limits-multi asserts all four) — baseline passing |
| "after a clean stage 2 this cannot fail for schema-shaped input; if it does, the residual error is reported honestly" | L5-7 | REAL | L86-92 typed-fallback diagnostic read; no swallow paths |
| "Semantic-class rules (fs, cross-file) are a separate pass" | L7-9 | REAL | crate::semantic exists; F1/F2 tests seen in semantic.rs (files 3-7 audits) |

## 7. Dead / duplicated / unreachable code
- load_file/load_partial_file: reachable only from control-server + tests (F-002).
- Duplicate validator: xtask config_validator.rs vs this crate's schema.rs (F-003).

## 8. Dependencies & flags
toml (parse), serde_json (value bridge + typed model), crate-internal diagnostic/lines/model/partial/schema. No feature gates, no unsafe.

## 9. Open questions for the next agent
- Whoever owns tools/xtask/config_validator.rs: confirm the two validators' rule sets are semantically identical (minimum/maximum/pattern/enums) or record the drift — the fixtures in this crate only pin the Rust-side pass.
- The rebrand-propagation card's [PROPOSAL] (host.rs:970) is the load-bearing plan that would make this file shipped — until then every product.toml diagnostic is dev-lane only.
- UNVERIFIED: char-boundary span behavior of the toml crate (F-001) — needs a compiled probe when disk allows.
