# AUDIT: tests/integration/first-vertical/evidence/transcripts/corrupt-truncated-open.json
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture/evidence data; consumed by the tests named below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/size/prefix/byte-diff batch; cat/od for content |

Data audit: 777 B, 36 lines JSON, sha256 ad0ea6c0b15f6dee…, schema transcript/1, scenario corrupt-truncated-open, fixture generated/malformed/avi_mjpeg_pcm__trunc1.avi (the 52-byte magic/avih cut). Content verified: one command session.open -> typed refusal result {form: error, stage: demux, code: corrupt, message "[demux.corrupt]: truncated data: expected 88 bytes, found 52 (demux.avi.truncated)"}, empty events, observables final_state Idle / position_us 0. Consistent with the trunc1 expectation family (typed open refusal). Same currency caveat: freshness gate RED at scenario `open` before reaching this file (baseline :2764).

F-001 [S3]: same stale-or-unverified evidence currency as the sibling transcript. Confidence: HIGH — Verification: read (baseline citation).