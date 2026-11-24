# CLAIM AUDIT: docs/formats/network-http.md

| Field | Value |
|---|---|
| Auditor | W19-A08 (orchestrator completion, W19-S2) |
| Claims extracted | 11 |
| REAL | 6 | REAL-UNPROVEN | 2 | SHALLOW | 1 | STUB | 0 | PAPER-ONLY | 0 | FALSE | 2 | UNREACHABLE | 0 |
| Internal contradictions found | 2 |

## 1. Blunt verdict
A truthful record of the 6.A* library stack that is STALE about the CLI:
its front-matter claim that the CLI open path "does not dispatch through
the registry yet" was overtaken by events (the shipped CLI DOES open
http:// via ureq — W10/W13 executed) and its status line "implemented
(library)" undersells shipped reality.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: implemented (library stack)" | :3 | SHALLOW | understates: the lane is SHIPPED and CLI-reachable (W10-A01 trace: open_http_source:474 -> UreqClient) |
| 2 | "The CLI open path does not dispatch through the registry yet" | :5-6 | FALSE-STALE | W13-A09 executed: 3 access lanes incl. http(s) ranged through demux_host; doc 05's `c://x` drift also disproven live |
| 3 | "today every SourceSpec::Url still refuses at apps/cli/src/demux_host.rs (UnsupportedForHost ...)" | :6-8 | FALSE-STALE | same: http arm exists (demux_host.rs:399-441), hls/dash refuse but http opens |
| 4 | "fetch port (6.A1a), admitted ureq client (6.A1b/6.A1c), ranged seekable resource (6.A2a)" landed and tested | :3-5 | REAL | W10-A02/A06 audits: ranged.rs reachable, ureq=3.5.0+rustls, loopback e2e green |
| 5 | "no-range linear-scan fallback (6.A2b)" | :4 | REAL | W10-A01: linear-scan confirmed in trace |
| 6 | "fuzz_target = fuzz_scheme_classify ... deterministic sweeps" | :~18 | REAL | W10: in-crate fuzz + sweeps exist |
| 7 | "evidence Linux-only (mock-server tests + the two --ignored live-net tests)" | :~14 | REAL | W10-A05: MockClient e2e; 2 live tests permanently #[ignore]d |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "CLI does not dispatch yet" | W13 executed `lumen open` http lane | Doc stale (predates w31.3 wiring) |
| status "implemented (library)" | matrix row 095 says not_started | BOTH wrong; shipped (W18-A10 S0) |

## 4. Fabrication / inflation indicators
Stale-as-current: last_audit 2026-10-31 predates the CLI wiring; the doc
was honest at write time and was never refreshed. Same defect class as
video-h264.md.
