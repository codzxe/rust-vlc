# AUDIT: tests/integration/first-vertical/evidence/transcripts/corrupt-garbage-appended.json
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture/evidence data; consumed by the tests named below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/size/prefix/byte-diff batch; cat/od for content |

Data audit: 3,847 B, 195 lines JSON, sha256 b034f8cfb960b569…, schema transcript/1, scenario corrupt-garbage-appended, fixture generated/malformed/avi_mjpeg_pcm__garbage0.avi. Content verified: session.open accepted -> full event trail (seq strictly increasing 1..N, session.state-changed Idle->Opening->Buffering, duration-known 2,000,000 us, media.info-ready) -> scheduler observables (60 pushed / 60 presented / 132 repeats) -> final_png digest (307,541 B, sha 698af4ee…, 320x240). Honest mock labels (hardware_class "mock", devices null-audio/software-render). CURRENCY: the evidence.rs:37-60 currency gate (committed_transcripts_match_fresh_runs) is RED in the orchestrator baseline (cargo-test-workspace.txt:2764, panicked at evidence.rs:47 "the committed transcript for `open` drifted") — the gate short-circuits at scenario `open`, so THIS file's own freshness is UNVERIFIED (regeneration requires cargo; not installed here).

F-001 [S3]: evidence currency is stale-or-unverified repo-wide: the gate that would prove this transcript fresh fails earlier in the suite; no per-file proof exists. Confidence: HIGH — Verification: read (baseline citation).