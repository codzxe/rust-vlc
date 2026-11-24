# CLAIM AUDIT: docs/investigations/rtsp-rtp-feasibility.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 5 |
| REAL 3 / REAL-UNPROVEN 1 / UNVERIFIED 1 (offline ecosystem) | |

## 1. Blunt verdict
A defer-decision investigation whose central behavioral claim (RTSP/RTP/UDP typed-refused by the SchemeRegistry) is verified in the live code — with line drift (doc cites schemes.rs:560-563 for the refusal test rows; the rows now sit at 695-697 — the code evolved after wave 13, the substance unchanged). The doc is honest about being offline: every ecosystem claim is [VERIFY]-marked in the text itself.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "The RTSP/RTP family ... is typed-refused today by the streaming port's `SchemeRegistry`" | Summary | REAL | schemes.rs:20-22 module doc "every other scheme (ftp:, rtsp:, data:, …) → the typed refusal SchemeError::Unsupported"; refusal rows exist |
| 2 | "refusal table test rows: `ftp`, `rtsp://cam.example.org/stream.sdp`, `rtp://224.1.1.1:5004`, `udp://@239.255.12.42:1234`" (at :560-563) | Evidence | REAL (line-drifted) | the exact rows exist at schemes.rs:695-697 (`("rtsp://cam.example.org/stream.sdp", "rtsp")` etc.); doc's line numbers stale |
| 3 | "Record the decision in the feature matrix blocked rows (done alongside this doc)" | Recommendation 2 | REAL | matrix rows protocol-rtp/rtsp/udp cite this very doc in evidence (feature-matrix.toml:1301,1315,1320) — closed loop |
| 4 | "`retina` (Scott Lamb / Moonfire NVR) — the closest thing to a real pure-Rust RTSP client ... [VERIFY current crate version...]" | Ecosystem | UNVERIFIED | offline, self-marked |
| 5 | "RFC 2326 / 7826 / 3550 / 3551 ... [VERIFY exact RFC numbers...]" | Protocol scope | REAL-UNPROVEN | RFC numbers standard knowledge — 2326 (RTSP 1.0), 7826 (RTSP 2.0), 3550 (RTP) are correct; codec RFC numbers marked [VERIFY] themselves |

## 3. Contradictions
None of substance; only citation line-drift (S4).

## 4. Fabrication / inflation indicators
None — the doc recommends doing NOTHING and keeps the refusal, with re-evaluation triggers named. The cheapest honest verdict there is.
