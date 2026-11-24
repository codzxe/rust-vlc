# AUDIT: tests/fixtures/generated/valid/ogg_chain_two_links.ogg
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 7041 bytes, 6 Ogg pages, 2 logical bitstreams |
| File verdict | REAL (chain verified page-by-page by me) |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 Ogg page walk (exit 0); rg consumer (exit 0) |

## 1. Verdict
Genuine two-link Ogg CHAIN, verified page-by-page: 6 `OggS` pages walking exactly to EOF
(pos == size). Link 1 (serial 1073372718): BOS(30 B) + headers page(12 segments, 2706 B,
granule 12) + EOS(14 packets, 401 B, granule 14). Link 2 (serial 1983438): BOS(30 B) +
page(14 segments, 3320 B) + EOS(14 segments, 336 B). Two BOS pages, two EOS pages, two
distinct serials — the chaining semantics the demuxer's chain contract pins. sha256
33cec020c090df71... (recomputed).

## 2. Consumers
crates/demux/tests/ogg_fixtures.rs — the header comment (line 4) says the "fixture asserts
the chain contract"; the valid-family truth table (line 126 region) pins it. Baseline:
demux PACKAGE exit_code=0 — the ogg family walk RAN and PASSED.

## 3. Findings
F-001 [S3] — systemic manifest-gap: outside the 28-file drift gate; no generator row.
CORPUS.md:10-11's tree-wide authorship claim FALSE here. Confidence: HIGH.

## 4. Open questions
None.
