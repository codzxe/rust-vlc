# AUDIT: crates/localization/src/plural.rs
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines / functions | 181 / 5 fns (as_str, parse, ALL, categories_for_language, select_plural_category) + 3 tests |
| File verdict | REAL (with a false exactness claim and stale CLDR sets) |
| Reachable from shipped entry point? | NO — same as the whole localization crate (zero app reverse-deps). Reached via crate tests + audit tooling. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `sed -n` quote checks (exit 0); `rg` plural usage in message.rs/audit.rs (exit 0); baseline localization suite (45 pass) cited |

## 1. One-paragraph verdict
A deliberately minimal plural module: six CLDR category names, a 4-group seed table (one/other; ru/pl one/few/many/other; ar all six; ja/zh/ko/th/vi other-only), and a v1 selection rule (1→one, else other, never one for one-less languages). The honesty framing is good — the module loudly documents what it does NOT implement. But two claims don't survive scrutiny: (a) the doc says the v1 rule "is exact for the one/other and other-only languages seeded above" — CLDR French and Portuguese put 0 in `one` ("i = 0..1"), so fr/pt zero is rendered via `other` where native CLDR uses `one`: not exact; (b) the seed sets for fr/es/it/pt predate CLDR 42's `many` category (millions agreement), and audit rule 5 runs STRICT for known languages (arm set must equal the seeded set exactly), so a CLDR-current fr catalog carrying a mandatory `many` arm would be REJECTED. Rendering is safe from the fallback (`or_else(other)` in message.rs).

## 2. What actually works (verified)
— verified by baseline `cargo-test-workspace.txt:2879-2977` -> localization 45 passed incl. `plural::tests::category_roundtrip`, `seeded_sets`, `v1_selection_rule`; and `rg` confirming message.rs:496-514 falls back to the `other` arm when the selected category is missing (parse enforces an `other` arm exists).

## 3. Findings
F-001 [S3]
Where: crates/localization/src/plural.rs:103-106
Code:
```
/// v1 rule (documented simplification): integer `1` selects `one`,
/// everything else selects `other`. This is exact for the `one/other` and
/// `other`-only languages seeded above; it is *not* the full CLDR
```
What is wrong: the exactness claim is false for two of the seeded one/other languages. CLDR cardinal rules for French and Portuguese include 0 in `one` (fr: "i = 0,1"; pt: "i = 0..1"), so `select_plural_category("fr", 0)` returning `Other` diverges from CLDR for zero — e.g. "0 fichier" renders the `other` arm where native French grammar takes singular. en/de/es/it/nl are genuinely 1-only, so the claim holds for them. Why it matters: the module's whole value proposition is being precise about what is and isn't exact; this is the one border it overstates. What correct looks like: "exact except fr/pt zero (CLDR `one` includes 0)" or special-case 0.
Confidence: MEDIUM — Verification: read; CLDR rule text recalled from the published tables, not re-fetched (no network in sandbox — CLDR data-currency UNVERIFIED, flagged accordingly).

F-002 [S3]
Where: crates/localization/src/plural.rs:77-79 + crates/localization/src/audit.rs:238-239
Code (plural.rs):
```
        "en" | "de" | "es" | "fr" | "it" | "nl" | "pt" => {
            Some(&[PluralCategory::One, PluralCategory::Other])
        }
```
Code (audit.rs):
```
    // Unknown languages: require `other`, tolerate extra arms. Known
    // languages: the arm set must equal the CLDR set exactly.
    let strict = known.is_some();
```
What is wrong: the seed table predates CLDR 42 (2021), which added a `many` cardinal category to the Romance languages (fr, es, it, pt — millions-agreement forms like "1,5 million de…"). Because rule 5 is strict for known languages, a fr/es/it/pt catalog that is correct per CURRENT CLDR (carrying `many`) is rejected with "category many not used by language". The module doc's "stable, public CLDR facts" framing hides that the facts are a stale snapshot. Blast radius: audit tooling only (no shipped app consumes this crate), but this is precisely the "honest data scope" the module claims to uphold.
Confidence: MEDIUM — Verification: read; CLDR 42 change recalled, not re-verified against a live cldrplural repository (no network — UNVERIFIED-data).

F-003 [S4]
Where: crates/localization/src/plural.rs:110-115
Code:
```
pub fn select_plural_category(language: &str, value: i64) -> PluralCategory {
    match categories_for_language(language) {
        Some(set) if !set.contains(&PluralCategory::One) => PluralCategory::Other,
        _ if value == 1 => PluralCategory::One,
        _ => PluralCategory::Other,
    }
```
What is wrong: for an UNKNOWN language (table miss → None), value 1 selects `one` — while the audit side, per the same module's doc and audit.rs, requires only `other` for unknown languages. A catalog for an unseeded language that passes audit (only `other` arm) will select `one` at render time and silently fall back to `other` (message.rs:512-514). Output stays grammatical, but the selection contract contradicts the audit contract; if the unseeded language actually lacks `one` (e.g. id/ms/yn), selecting `one` is conceptually wrong even though the fallback rescues it.
Confidence: HIGH — Verification: read + trace of message.rs fallback.

## 4. Per-function table (mandatory, file <200 LOC but included)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PluralCategory::as_str | 26-35 | REAL | — | canonical CLDR names |
| PluralCategory::parse | 39-49 | REAL | — | rejects `=1` exact forms (v1 subset), tested |
| PluralCategory::ALL | 52-59 | REAL | — | canonical order |
| categories_for_language | 73-97 | REAL | S3 | F-002 stale Romance sets; ru/pl/ar/ja-sets correct |
| select_plural_category | 110-116 | REAL | S3/S4 | F-001 exactness claim, F-003 unknown-language arm |
| tests (3) | 118-181 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| category_roundtrip | YES | all 6 names parse back; "manys"/"=1"/"" rejected |
| seeded_sets | YES | pins all four groups + unknown → None |
| v1_selection_rule | YES | pins en 1/0/2, ja 1 (Other), ru 2 (Other, the documented deviation) |
Gap: nothing tests fr/pt 0 (the F-001 divergence) or an unknown-language 1 (F-003) — the two edges where the code disagrees with its own doc/CLDR.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "only category SETS that are stable, public CLDR facts are seeded" | plural.rs:4-5 | SHALLOW | sets are stale for fr/es/it/pt (F-002) |
| "This is exact for the one/other and other-only languages" | plural.rs:103-104 | FALSE for fr/pt zero | F-001 |
| "those languages' runtime plural rendering is [VERIFY] while their audit-side category sets are already enforced" | plural.rs:106-108 | REAL | audit.rs rule 5 strict path + locales-plural-ru fixture (other agent's row) |

## 7. Dead / duplicated / unreachable code
None in-file. `select_plural_category`'s only caller is message.rs render (crate-internal).

## 8. Dependencies & flags
std only. No feature gates.

## 9. Open questions for the next agent
- Verify against live CLDR (networked lane): do current fr/es/it/pt cardinal rules carry `many`, and do fr/pt include 0 in `one`? If yes, F-001/F-002 upgrade from MEDIUM-confidence data issues to confirmed S3s.
- audit.rs (rule 5 strictness) and message.rs (render fallback) are not my files; my F-002/F-003 claims cite them read-only.
