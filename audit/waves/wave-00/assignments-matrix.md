# Matrix row-range assignments (W18/W19)

docs/feature-matrix.toml = 252 rows, 3421 lines. Per spec §8, split by ROW RANGE:

## Wave 18 (rows 1-126, 10 agents)
- W18-A01: rows 1-13    - W18-A06: rows 66-78
- W18-A02: rows 14-26   - W18-A07: rows 79-91
- W18-A03: rows 27-39   - W18-A08: rows 92-104
- W18-A04: rows 40-52   - W18-A09: rows 105-113
- W18-A05: rows 53-65   - W18-A10: rows 114-126

## Wave 19 (rows 127-252 + doc corpus)
- W19-A01: rows 127-168  - W19-A02: rows 169-210  - W19-A03: rows 211-252
- W19-A04..A10: doc corpus per assignments.tsv (tier A docs first: full claim
  ledger; tier C docs: batch ledgers)

Each matrix agent writes ONE ledger file: audit/docs/feature-matrix.rows-<range>.audit.md
(per §12 with one block per row: claimed status, evidence strings, code actually
found, executed-or-not, verdict).
