# AUDIT: crates/localization/src/key.rs
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines / functions | 250 / 8 prod fns (is_valid_segment, new, as_str, namespace, segment_count, in_reserved_namespace, Display::fmt, AsRef::as_ref) + 5 tests |
| File verdict | UNREACHABLE (real, correct, tested code — but no shipped entry point reaches it; zero workspace dependents) |
| Reachable from shipped entry point? | NO. No Cargo.toml in the repo declares a dependency on `localization` (grep `^localization` over every manifest: 0 matches). The crate is a workspace member (`members = ["crates/*"]`, root Cargo.toml:41) so it builds/tests in CI gates, but lumen-cli and desktop never link it. The UI's i18n is a separate TS implementation (ui/src/i18n/). Only in-crate callers: lib.rs, locale.rs, tests (verified by repo-wide grep). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `./target/debug/deps/localization-59558d0212b9967a key::` → 5 passed / 0 failed, exit 0 (executed, not just baseline); python3 grammar scan of 17 catalog TOMLs (0 violations, max 4 segments); rg greps for dependents and API signatures |

## 1. One-paragraph verdict
The grammar code is correct, hostile-input-safe, and honestly documented — I executed its 5 unit tests from the prebuilt binary (5 passed, exit 0) and independently re-derived its invariants: the byte-wise `[a-z0-9-]` check (L48-53) is UTF-8-safe because every non-ASCII codepoint contains bytes ≥0x80 that fail all three predicates; the `namespace()` `unwrap_or_default` (L112) is unreachable-default by construction (private `raw` field, `new()` rejects empty/1-segment keys); and the "deny by construction" header claim (L4-7) holds at the API level — `Catalog::get(&self, key: &MessageKey)` (catalog.rs:151) accepts nothing but the newtype. All 17 catalog files in the tree (locales/en-US.toml + 16 fixtures) pass the grammar with max 4 segments, so nothing real is rejected. The two real problems: (1) the crate is dead weight from a shipping standpoint — no dependent exists, so none of this enforcement protects any shipped binary today; (2) the 5-segment cap diverges from the unbounded `^[a-z0-9-]+(\.[a-z0-9-]+)+$` msgid pattern actually shipped in product.schema.json:279 — a msgid that passes product-config validation can fail `MessageKey::new` (latent, disclosed in-code at L15-18 as a "Lead change-control item").

## 2. What actually works (verified)
- `./target/debug/deps/localization-59558d0212b9967a key::` → `test result: ok. 5 passed; 0 failed` exit 0. All five tests exercise both acceptance and typed rejection (empty, 1 seg, 6 segs, uppercase, underscore, empty segments at head/tail/middle, space, non-ASCII "bé").
- Mutation reasoning: delete the segment-count check → `rejects_invalid_keys_with_typed_errors` fails on `("a", SegmentCount{1})`; weaken charset to allow uppercase → same test fails on "Player.play". The tests are honest.
- Grammar scan (python3, 17 files incl. locales/en-US.toml and all locales-* fixtures): 0 violations, max 4 segments — the grammar accepts every real key in the tree.
- Deny-by-construction: catalog.rs:151 `pub fn get(&self, key: &MessageKey) -> Option<&Message>` — no raw-string lookup overload exists (verified by reading the pub fn list of catalog.rs/message.rs).
- Baseline agreement: audit/runs/baseline/cargo-test-workspace.txt:2879-2976 — localization package exit_code=0 (lib 45 passed, audit_proof_matrix 14, seed_integrity 3, plus 2).

## 3. Findings
F-001 [S3]
Where: crates/localization/src/key.rs:15-19 (MAX_SEGMENTS) vs product.schema.json:277-281
Code: (verbatim, key.rs L15-19)
```
/// Maximum number of dotted segments in a key (doc 07 §6.1: "two to five
/// segments"). NOTE: the `msgid` pattern in doc 07 §3.2
/// (`^[a-z0-9-]+(\.[a-z0-9-]+)+$`) has no upper bound — the prose grammar
/// here is stricter; reconciling the two is a Lead change-control item.
pub const MAX_SEGMENTS: usize = 5;
```
What is wrong: two shipped validators disagree on the same concept. product.schema.json `$defs.msgid` (L279) enforces `"pattern": "^[a-z0-9-]+(\\.[a-z0-9-]+)+$"` — unbounded segments — while MessageKey rejects >5. A product.toml `description_key = "a.b.c.d.e.f"` passes product-config validation and then fails `MessageKey::new` the moment the localization crate renders it.
Why it matters: the reference-complete audit (locales/en-US.toml header item 4: "every msgid referenced from product.toml, schemas/settings.toml … resolves here") is keyed by MessageKey; a schema-valid msgid would be rejected by the resolver, so the two grammars cannot both be authoritative. Today it is latent (crude scan of product.toml/settings.toml: no msgid-like token exceeds 4 segments; and no runtime consumer of localization exists), and the divergence IS disclosed in the comment — but a comment is not a fix.
What correct looks like: one grammar: either add the bound to product.schema.json's msgid pattern or drop/raise MAX_SEGMENTS to match.
Confidence: HIGH — Verification: read + executed pattern comparison (grep product.schema.json:279; python3 scan)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| is_valid_segment (priv) | 48-53 | REAL | — | byte-wise UTF-8-safe charset check; verified by tests (segment_charset) and reasoning |
| MessageKey::new | 80-100 | REAL | — | empty→Empty, split+count 2-5, per-segment check; allocates one String; executed via tests |
| as_str | 103-105 | REAL | — | trivial accessor |
| namespace | 109-113 | REAL | — | unwrap_or_default is dead-default (invariant documented L110-111; private field) |
| segment_count | 116-118 | REAL | — | re-splits; consistent with new |
| in_reserved_namespace | 123-125 | REAL | — | informational only, honestly documented (L120-122: seed uses xtask/ci) |
| Display::fmt | 128-132 | REAL | — | writes raw |
| AsRef::as_ref | 134-138 | REAL | — | trivial |
| tests (5) | 144-249 | REAL | — | executed: 5/5 pass; rejection table is exhaustive incl. non-ASCII |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| accepts_valid_keys | YES | 9 keys incl. 5-segment and digits; as_str round-trip |
| rejects_invalid_keys_with_typed_errors | YES | 10 cases asserting the EXACT KeyError variant per case (not just is_err) |
| namespace_and_segments | YES | namespace/count/reserved on two keys |
| reserved_namespace_table_matches_doc07 | YES | pins the 8-entry table verbatim |
| segment_charset | YES | positive+negative charset cases at the helper level |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A key is 2–5 dotted segments, each `[a-z0-9-]+`" | key.rs:4-5, locales/en-US.toml header | REAL | tests executed; 17-catalog scan conforms |
| "an invalid or raw string can never reach a catalog lookup (doc 07 §6.5 deny by construction)" | key.rs:5-7 | REAL | catalog.rs:151 `get(&MessageKey)`; no raw-string overload |
| "the msgid pattern in doc 07 §3.2 has no upper bound — the prose grammar here is stricter" | key.rs:16-18 | REAL (and shipped that way) | product.schema.json:279 pattern is unbounded — F-001 |
| doc 07 §6.1 reserved namespaces table | key.rs:22-31 | REAL | test pins the table; matches the table reproduced in locales/en-US.toml header |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable from shipped entry points (see reachability). This is infrastructure-ahead-of-consumer: locales/en-US.toml documents the reference-catalog contract and the crate enforces it in CI, but no product binary renders strings through it. Not duplicated logic in Rust; the UI has its own TS i18n (ui/src/i18n/messages) — the two key sets must be kept in step by the seed_integrity/refs audits rather than by a shared type, a structural risk outside this file's scope.

## 8. Dependencies & flags
Only std + thiserror (KeyError derive) + fmt. No feature gates. Crate is std-only in this module.

## 9. Open questions for the next agent
- Who is the intended first runtime consumer of `localization` (lumen-cli --help localization? product strings?) — if none lands, the crate's 45 lib tests + 19 integration tests enforce a contract nothing ships.
- Does ui/src/i18n key set equal the en-US.toml key set (the TS side is not governed by MessageKey)? Suggest a cross-check in the UI domain wave.
