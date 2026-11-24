# FILE AUDIT: tools/bench-vs-vlc/mock_vlc.py (121 loc) — script

REAL, honest-by-construction mock oracle. The header is exemplary:
"THIS IS NOT VLC AND NEVER PRODUCES A MEASUREMENT" — exists to exercise
the real-oracle code path where the pinned VLC binary is absent; every
artifact it feeds is labeled [MOCK] by run.py's report writer
(mechanically-enforced), JSON records oracle.kind="mock".
Behavior: --version prints the pinned 3.0.23 shape WITH a MOCK marker;
normal invocation accepts documented flags. Clean-room note: emulates
only the CLI shape from the repo's own pinned baselines.
Findings: S3:1 — the mock's [MOCK] labeling depends on run.py's writer
discipline (cross-file contract, unverifiable here); S4:1 — no unit tests
committed for the script itself. Verdict: REAL (test infrastructure).
Cross-ref: this is the honest counterpart to the project's wider
evidence problem — mockness DISCLOSED here vs undisclosed elsewhere.

| Auditor | W19-A04 (orchestrator completion, W19-S2) |
