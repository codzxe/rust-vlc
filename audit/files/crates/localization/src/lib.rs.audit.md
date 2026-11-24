# AUDIT: crates/localization/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines / functions | 155 / 1 (+1 test) (doc-root module; 8 submodules re-exported) |
| File verdict | REAL (mechanism crate; UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO. Zero dependents: no Cargo.toml in the tree depends on `localization` (the crates/playlists/Cargo.toml:5 mention is a comment); apps/cli and apps/desktop do not reference it. Enforcement is the crate's own test suite — exactly what the doc discloses (lib.rs:77-81). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg localization over all Cargo.tomls (no dependents); rg unwrap/expect/panic/unsafe over src/*.rs (only lib.rs's own doc comment — lint-policy claim holds); baseline cargo-test-workspace.txt:2879-2976 (localization exit 0: 45 unit + 14 audit_proof_matrix + 3 seed_integrity + 2, all passed); rg l10n-audit in tools/xtask (absent — matches the honest deferral note); sed fallback.rs:146-149 |

## 1. One-paragraph verdict
A documentation-honest crate root. Every deferral it admits is true and every
claim I could execute or trace reconciles: the five catalog-audit rules have
real module backends and pass in baseline via seed_integrity.rs (the shipped
tree) and audit_proof_matrix.rs (violation fixtures); the `cargo xtask
l10n-audit` wiring is indeed absent from the xtask dispatch table (cmd/ has
no l10n-audit) and the doc says so; the lint-policy claim (no
unwrap/expect/panic/unsafe in src/) is verified by scan; the debug-gated
`missing:<key>` fallback claim is implemented verbatim in fallback.rs:148-149;
and `line_col` is a correct scalar-value column counter used by catalog.rs
and refs.rs. The structural problem is the same as the sibling mechanism
crates: nothing shipped consumes it — the tree's only locale is en-US.toml,
so no runtime surface renders a message through this crate.

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt:2879-2976: localization exit 0 —
45 unit tests (incl. `key::tests::rejects_invalid_keys_with_typed_errors`,
`catalog::tests::duplicate_keys_are_parse_errors_with_position`),
14 audit_proof_matrix, 3 seed_integrity — all passed.
— Lint policy scan: `rg "unwrap\(\)|expect\(|panic!|todo!|unimplemented!|unsafe"`
over crates/localization/src/*.rs → only lib.rs's own doc comment (lines
93-94). Claim holds.
— `cargo xtask l10n-audit` absent from tools/xtask/src/cmd/ (dir listing) —
the doc's "belongs to card 1.E2 ... until then this crate's own test suite
enforces the rules" (lib.rs:76-81) is an accurate description of reality.

## 3. Findings
F-001 [S4]
Where: crates/localization/src/lib.rs:1-6 (whole crate posture)
What is wrong: the crate has zero dependents — no binary or library in the
tree calls it. `locales/` ships exactly one catalog (en-US.toml), so the
fallback chain's parent-locale and non-reference-locale links never fire on
real data (they are exercised only by tests/fixtures/locales-* fixtures).
The mechanism is real and self-tested, but "catalogs are data in locales/"
describes a one-file dataset. Not dishonest — the doc never claims runtime
wiring — but the audit surface should know the entire localization plane is
currently test-enforced only.
Confidence: HIGH — Verification: rg over Cargo.tomls; ls locales/.
## 3b. line_col scrutiny (no finding)
`line_col` (lib.rs:122-137): byte offset → 1-based (line, scalar-column).
Correct: char_indices walks scalar starts; `i >= pos` break means a mid-char
pos reports the containing char's start (never panics, never off-by-a-byte);
past-end clamps (test :153 asserts (3,2)). Used by catalog.rs:207,241,243 and
refs.rs:75. Honest doc, honest test.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| line_col | 122-137 | REAL | — | correct scalar columns; 4 real call sites |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| line_col_counts_lines_and_scalar_columns | YES | 6 exact asserts incl. multi-byte column and past-end clamp |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Key grammar 2–5 segments, `[a-z0-9-]+`, reserved namespaces | lib.rs:8-28 | REAL | key.rs (RESERVED_NAMESPACES :22, checks :124); baseline key tests pass |
| Message format v1 subset (placeholder/select/plural) | lib.rs:30-44 | REAL | message.rs/plural.rs; 45 unit tests + plural fixtures pass in baseline |
| Fallback chain, debug-only `missing:<key>`, never panics | lib.rs:46-52 | REAL | fallback.rs:148-149 `if cfg!(debug_assertions) { format!("missing:{key}") }`; fallback tests pass |
| Five catalog-audit rules + audit entry points | lib.rs:54-75 | REAL | audit.rs backends; audit_proof_matrix 14 tests pass |
| "cargo xtask l10n-audit ... belongs to card 1.E2 ... until then this crate's own test suite enforces the rules" | lib.rs:76-81 | REAL | xtask cmd/ has no l10n-audit; seed_integrity passes |
| "translation API accepts only MessageKey values" | lib.rs:83-89 | REAL | MessageKey newtype (key.rs); catalog API takes MessageKey |
| Lint policy: no unsafe/unwrap/expect/panic in crate | lib.rs:91-97 | REAL | rg scan: clean |

## 7. Dead / duplicated / unreachable code
No dead code in this file. Crate-level: production-unreachable (F-001).

## 8. Dependencies & flags
No external deps (Cargo.toml: registry layer L1, no internal deps). No
feature gates.

## 9. Open questions for the next agent
- Rule 4 (reference-completeness) joins product.toml/settings.toml msgids to
  the reference catalog — the same join product-config's semantic.rs
  performs via `reference_catalog_keys`. Two implementations of one rule
  (crate-level here, SemanticInputs there) — worth checking they agree when
  auditing refs.rs/audit.rs.
