# AUDIT: crates/product-config/tests/fixtures/range-probe-max-bytes.toml
| Field | Value |
|---|---|
| Auditor | W11-A07 | Date | 2025-06-14 |
| Lines | 226 (product.toml snapshot + 1 mutation) |
| File verdict | REAL (fixture doing its one job — and the doc-07 §3.5 golden) |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/golden_s35.rs:13-65 (loaded under the file label `product.toml`) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | sed quote verification; python3 read of product.schema.json probe_max_bytes; baseline citation |

## 1. Verdict
The §3.5 golden fixture: `probe_max_bytes = 1048576` (1 MiB) must be rejected against the schema range 16 MiB..=64 MiB, rendering the doc 07 §3.5 example diagnostic byte-for-byte (location `product.toml:<line>:limits.probe_max_bytes`, `value: 1048576 (1.0 MiB)`, `rule: must be >= 16777216 (16 MiB) and <= 67108864 (64 MiB)`, the full hint prose). Verified passing in baseline cargo-test-workspace.txt:5108-5112 (`probe_max_bytes_renders_the_doc_07_s35_example ... ok`). This fixture also exposes that the product.toml inline comment lies about the range (F-002).

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt:5108-5112; python3: product.schema.json `probe_max_bytes` = `{minimum: 16777216, maximum: 67108864}` — the fixture value 1048576 is genuinely below the enforced floor.

## 3. Findings
F-001 [S4]
Where: fixtures/range-probe-max-bytes.toml:199
Code:
```
probe_max_bytes = 1048576
```
plus the shared whole-file staleness vs root product.toml (32 diff lines) — same as sibling fixtures. Confidence: HIGH — Verification: read (diff run).

F-002 [S4] (cross-file, proven by this fixture)
Where: product.toml:205 comment vs product.schema.json
Code (product.toml:205):
```
probe_max_bytes = 16777216      # probing bound (MP §12). schema 1 MiB..=64 MiB
```
vs schema (python3, ran): `'minimum': 16777216, '$comment': "16 MiB floor per the doc 07 §3.5 error contract and the x-lumen-hint rationale; the §2.9 inline comment ('1 MiB..=64 MiB') is the outlier."`
What is wrong: the inline comment claims a 1 MiB floor; the enforced floor is 16 MiB. The schema itself documents the discrepancy — known and acknowledged, but every fixture copy (and product.toml) still carries the false comment. A reader setting 8 MiB per the comment gets a validation error. Confidence: HIGH — Verification: ran python3 schema read + golden test baseline.

## 7. Dead/dup
None. ## 9. Open questions: none.
