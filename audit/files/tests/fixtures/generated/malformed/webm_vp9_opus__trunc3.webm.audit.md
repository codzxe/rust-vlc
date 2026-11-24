# AUDIT: tests/fixtures/generated/malformed/webm_vp9_opus__trunc3.webm
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (malformed WebM derivative: truncation) |
| Size / sha256-prefix | 13950 bytes / f5959d3f52078a4c (verified locally) |
| File verdict | REAL (consumed) — not in MANIFEST |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256, header sniff, rg consumers |

## 1. Verdict
Real WebM truncation derivative (clean `webm` EBML header, cut at 13950 of
the parent). Consumed identically to its flip16 sibling.

## 2. What actually works (verified)
- Consumer (verbatim, crates/demux/tests/mkv_fixtures.rs:359-363):
```
        "webm_vp9_opus__trunc0.webm",
        "webm_vp9_opus__trunc1.webm",
        "webm_vp9_opus__trunc2.webm",
        "webm_vp9_opus__trunc3.webm",
        "webm_vp9_opus__trunc4.webm",
```
- Matrix evidence: docs/corpus/compat-matrix-20261110.raw-rows.jsonl rg hit (exact name).

## 3. Findings
F-001 [S3] Not in tests/fixtures/generated/MANIFEST.toml — unpinned, no drift gate. Confidence: HIGH.
