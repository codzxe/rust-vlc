# AUDIT: tests/fixtures/generated/malformed/webm_vp9_opus__flip16.webm
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (malformed WebM derivative: 16 seeded header bit-flips) |
| Size / sha256-prefix | 27259 bytes / 8e86e9f95da53d6a (verified locally) |
| File verdict | REAL (consumed) — not in MANIFEST |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256, header sniff (`\x1aE\xdf\xa3...webm` — DocType 'webm' intact), rg consumers |

## 1. Verdict
Real WebM derivative of the VP9+Opus parent (EBML header with `webm`
DocType). 16 seeded header bit-flips. Consumed by
crates/demux/tests/mkv_fixtures.rs (line 366), the compat-matrix walker
(`.webm` extension) and named in docs/corpus/compat-matrix-20261110.raw-rows.jsonl.

## 2. What actually works (verified)
- Consumer (verbatim, crates/demux/tests/mkv_fixtures.rs:364-371):
```
        "webm_vp9_opus__flip1.webm",
        "webm_vp9_opus__flip4.webm",
        "webm_vp9_opus__flip16.webm",
        "webm_vp9_opus__garbage0.webm",
    ] {
        let path = dir.join(name);
        assert!(path.exists(), "{name} missing");
```
- Matrix evidence: raw-rows.jsonl names this exact file (rg hit).

## 3. Findings
F-001 [S3] Not in tests/fixtures/generated/MANIFEST.toml — unpinned (see
mkv sibling audit for the full argument). Confidence: HIGH.
