# AUDIT: crates/localization/src/plural.rs
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-21 |
| Lines / functions | 181 / 6 (+3 tests) |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches it — same crate as message.rs#part1, zero dependents) |
| Reachable from shipped entry point? | NO. Internal call chains: `select_plural_category` ← message.rs:500,505 (`render_placeholder`); `PluralCategory::parse` ← message.rs:812 (parser, part2) and message.rs:838; `categories_for_language` ← audit.rs:235 (rule 5). All crate-internal, exercised only by the crate's own tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "categories_for_language\|select_plural_category\|PluralCategory::" crates/localization/src/audit.rs crates/localization/src/lib.rs` (callers: audit.rs:22,235 — crate-internal only) · `sed -n '220,300p' crates/localization/src/audit.rs` (rule 5 strict/non-strict direction) · `rg -n "plural::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (3/3 ok at :2713-2716) · `sed -n` on every quoted range |

## 1. One-paragraph verdict
A small, mostly honest data-table module: the six-category enum, the roundtrip parse, and the ru/pl/ar/ja/zh/ko/th/vi seed rows are correct CLDR facts, the v1 selection simplification is candidly documented, and all 3 tests pass in the no-fail-fast baseline (:2713-2716). Two real defects: the doc's exactness claim for the seeded one/other languages is false for French and Portuguese (CLDR maps 0 → `one` in both; the v1 rule maps 0 → `other`), and the fr/it seed sets predate CLDR's `many` category, so audit rule 5 in strict mode would flag a CLDR-conformant French catalog as NON-conformant — the audit enforces the stale table against correct data. Additionally, the render side selects `one` for count 1 in languages the audit side only requires `other` for — an asymmetry whose swallowed-fallback effect is filed as F-005 in message.rs#part1. The whole module ships to nothing (crate has zero dependents).

## 2. What actually works (verified)
- Enum/parse/Display/ALL roundtrip: `plural::tests::category_roundtrip ... ok` (:2713), including rejection of `=1` (v1 subset) and `"manys"`.
- Seed rows en/de/es/it/nl (one/other), ru/pl (one/few/many/other), ar (all six), ja/zh/ko/th/vi (other-only) match public CLDR cardinal sets; `seeded_sets ... ok` (:2715) pins en, ru, ar, ja, xx.
- `select_plural_category` implements exactly what its doc says (1 → `one` else `other`; one-less seeded languages never select `one`): `v1_selection_rule ... ok` (:2716) pins en 1/0/2, ja 1, ru 2.
- Panic-free by construction: pure comparisons, no arithmetic, no indexing (read of :110-116).
- Unknown-language tolerance on the audit side is real: audit.rs:222-224 "extra arms are accepted, not flagged" — verified by reading `plural_conformance_detail` (strict flag only when the language is in the table).

## 3. Findings

### F-001 [S3] The "exact for the one/other languages" claim is false for French and Portuguese, and the fr/it rows predate CLDR's `many` — rule 5 then rejects CLDR-correct catalogs
Where: crates/localization/src/plural.rs:102-105 (claim) and :76-79 (seed row)
Code:
```rust
/// v1 rule (documented simplification): integer `1` selects `one`,
/// everything else selects `other`. This is exact for the `one/other` and
/// `other`-only languages seeded above; it is *not* the full CLDR
```
```rust
    match language {
        "en" | "de" | "es" | "fr" | "it" | "nl" | "pt" => {
            Some(&[PluralCategory::One, PluralCategory::Other])
```
What is wrong: two independent inaccuracies against public CLDR cardinal rules. (1) CLDR French maps 0 to `one` (rule "i = 0,1" — "0 pomme" is singular), and CLDR Portuguese (pt) likewise maps 0 to `one` ("i = 0..1"); the v1 rule maps 0 → `other`, so for fr and pt the claimed exactness ("This is exact for the `one/other` … languages seeded above") is false — the error is invisible to the seed-set test because the SET is right while the SELECTION is wrong. (2) Since CLDR 42 (2022) French and Italian carry a `many` category (millions-scale: "1 million de …"); the seeded fr/it sets `[one, other]` are stale, and because rule 5 runs strict for known languages (audit.rs:233-234 "Known languages: the arm set must equal the CLDR set exactly"), a CLDR-conformant fr catalog with `{one, many, other}` arms is rejected with "categories many not used by language 'fr'" — the audit enforces the stale table AGAINST correct data. The table's provenance note (:74-75 "unicode.org cldr plural rules") cites no CLDR version.
Why it matters: this is the only data this crate ships that pretends to external authority; any real translator producing a correct French catalog fails CI, while a wrong-for-0 French catalog passes. The file's own honesty framing (:4-9 "only category SETS that are stable, public CLDR facts are seeded") makes the staleness a claim violation, not just a data bug.
What correct looks like: pin the CLDR version in the comment; fr → `[one, many, other]`, it → `[one, many, other]`, pt → verify against the pinned version; scope the :102-104 exactness claim to the languages it actually holds for (en, de, es, it-integer-1, nl) and extend the `[VERIFY]` hedge to fr/pt 0-selection.
Confidence: HIGH on fr/pt 0 → `one` (longstanding public CLDR rule, version-independent); MEDIUM on the exact CLDR release that added fr/it `many` (offline sandbox — no CLDR copy to cite; the fr `many` category exists in current CLDR). Verification: read; external CLDR facts not runnable in sandbox — flagging for the wave gate to spot-check against unicode.org.
 
### F-002 [S3] Render/audit asymmetry: the renderer selects `one` for count 1 in languages the auditor only requires `other` for — a contract the catalog cannot satisfy
Where: crates/localization/src/plural.rs:110-116
Code:
```rust
pub fn select_plural_category(language: &str, value: i64) -> PluralCategory {
    match categories_for_language(language) {
        Some(set) if !set.contains(&PluralCategory::One) => PluralCategory::Other,
        _ if value == 1 => PluralCategory::One,
        _ => PluralCategory::Other,
    }
}
```
What is wrong: for a language NOT in the seed table (`None` arm falls through to `_ if value == 1`), the renderer selects `One` for count 1. But audit rule 5 for unknown languages requires only the `other` arm (audit.rs:231-234). So a fully rule-conformant `tr.toml` (only an `other` arm — exactly what CI demands) forces the renderer into the silent `other`-arm fallback at message.rs:517-520, with no diagnostic (the swallowing defect is F-005 in message.rs#part1, this file holds the root: the two halves of the same crate disagree about what an unknown language requires).
Why it matters: the crate's own audit green-lights catalogs its own renderer degrades on; the bug class is invisible until a non-seeded locale ships.
What correct looks like: unknown languages select `Other` for all values (matching what the audit requires), or the audit requires `one`+`other` for unknown languages (matching what the renderer selects) — one contract, stated in one place.
Confidence: HIGH — Verification: read of plural.rs:110-116 + audit.rs:226-258 + message.rs:517-531.

### F-003 [S4] `seeded_sets` pins 4 of 15 language rows — 11 rows can be silently corrupted
Where: crates/localization/src/plural.rs:133-167
Code:
```rust
    fn seeded_sets() {
        assert_eq!(
            categories_for_language("en"),
            Some(&[PluralCategory::One, PluralCategory::Other][..])
        );
```
What is wrong: the test asserts only `en`, `ru`, `ar`, `ja` (and `xx` → None). The other 11 seeded keys (de, es, fr, it, nl, pt, pl, zh, ko, th, vi) have no pin — mutating the `"de"` row to the ru set, or dropping `"fr"` from the one/other row entirely, leaves the suite green.
Why it matters: this file is a DATA table wearing a test; an unpinned data table is unaudited data. F-001 shows the stakes (stale rows enforce wrong rules).
What correct looks like: iterate ALL seeded languages (the match arms) and assert each row — or generate the test from a checked-in CLDR snapshot.
Confidence: HIGH — Verification: read; mutation reasoning only.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PluralCategory::as_str` | 26-35 | REAL | — | six CLDR names, matches `parse` (roundtrip test) |
| `PluralCategory::parse` | 39-49 | REAL | — | rejects `=1` (v1 subset, documented) and unknown names; total function, no panics |
| `PluralCategory::ALL` | 52-59 | REAL | — | canonical order; derived Ord consistent (declaration order) |
| `Display for PluralCategory` | 62-66 | REAL | — | delegates to `as_str` |
| `categories_for_language` | 73-97 | REAL | S3 (F-001) | en/de/es/it/nl/pt/ru/pl/ar/ja/zh/ko/th/vi rows; fr/it stale vs current CLDR; no CLDR version pinned |
| `select_plural_category` | 110-116 | REAL | S3 (F-001 fr/pt 0; F-002 asymmetry) | implements its doc faithfully; panic-free; ja/th/etc. correctly never `one` |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `category_roundtrip` :123 | YES | all six names roundtrip; `=1`, `"manys"`, `""` rejected |
| `seeded_sets` :133 | PARTIAL | pins en/ru/ar/ja/xx only — 11 rows unpinned (F-003) |
| `v1_selection_rule` :170 | YES for what it pins | en 1/0/2, ja 1, ru 2 exact — catches a `value == 1` mutation and a one-less-language leak; does NOT cover fr/pt 0 (F-001 gap) nor unknown-language count-1 (F-002 gap) |
All 3 ran green: orchrun-test-workspace-nofailfast.log.txt:2713-2716.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "only category SETS that are stable, public CLDR facts are seeded" | plural.rs:4-5 | SHALLOW | fr/it rows are stale vs current public CLDR (F-001); no CLDR version cited (:74-75) |
| "This is exact for the `one/other` and `other`-only languages seeded above" | plural.rs:102-104 | FALSE for fr, pt | CLDR fr/pt map 0 → `one`; v1 maps 0 → `other` (F-001) |
| "their audit-side category sets are already enforced" (ru/pl/ar) | plural.rs:106-108 | REAL | audit.rs:226-258 strict equality; `rule5_missing_plural_categories_are_flagged` + `rule5_unused_plural_categories_are_flagged` ok in audit_proof_matrix (no-failfast log :2754-2768) |
| "For languages outside the Phase-1 category table only the universal `other` category is required — extra arms are accepted, not flagged" | audit.rs:222-224 (rule 5 doc) | REAL | read of `plural_conformance_detail` (`strict` only when known) |

## 7. Dead / duplicated / unreachable code
- No dead functions inside the file; `ALL` is used by tests and is a reasonable public const (lib.rs:114 re-exports everything).
- Unreachable from shipped entry points (crate-level: zero dependents — see message.rs#part1 audit F-002 for the grep evidence).

## 8. Dependencies & flags
- None: pure std + `fmt` (read of :11). No feature gates. `[lints] workspace = true`.
- Baseline: `plural::tests` 3/3 ok (cited); clippy workspace clean.

## 9. Open questions for the next agent
- Wave gate: spot-check `fr`/`it`/`pt` against a pinned CLDR release (unicode.org cldr plural rules) — this sandbox has no CLDR copy; F-001's `many` claim is MEDIUM-confidence offline.
- Owner of audit.rs: rule 5's strict equality means seed-table staleness converts directly into false CI failures for correct catalogs — consider keying strictness to a CLDR version stamp.
- W11-A07: `PluralCategory::parse` is called twice per plural arm in parse_arms (message.rs:812 then :838 re-parses the same name) — cosmetic duplication in your part.
