# AUDIT: crates/product-config/tests/fixtures/pattern-bcp47.toml
| Field | Value |
|---|---|
| Auditor | W11-A07 | Date | 2025-06-14 |
| Lines | 226 (product.toml snapshot + 1 mutation) |
| File verdict | REAL (fixture doing its one job) |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/fixtures_matrix.rs:315-328 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | sed/grep quote verification; diff vs sibling fixtures; baseline citation |

## 1. Verdict
Pattern-violation fixture: `preferred_audio_languages = ["english"]` must be rejected at the array-element key `playback.preferred_audio_languages.0` with the pattern rule, found `"english"`, exact line. Test verified passing in baseline cargo-test-workspace.txt:5074 (`pattern_bcp47_language_tag_is_rejected ... ok`). The element-index key path (`…languages.0`) is the interesting part — the validator blames the array slot, not the array.

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt:5075-5107 (fixtures_matrix 27/27).

## 3. Findings
F-001 [S4]
Where: fixtures/pattern-bcp47.toml:175
Code:
```
preferred_audio_languages = ["english"]
```
plus the same whole-file staleness vs root product.toml as the sibling fixtures (32 diff lines: old versions, old min_width/min_height, streaming-http=false, no [web.size_budgets]). Field-local pattern check unaffected. Confidence: HIGH — Verification: read (diff run).

## 7. Dead/dup
None. ## 9. Open questions: none.
