# CLAIM AUDIT: docs/formats/video-h264.md

| Field | Value |
|---|---|
| Auditor | W19-A08 (orchestrator completion, W19-S2) |
| Claims extracted | 12 |
| REAL | 7 | REAL-UNPROVEN | 1 | SHALLOW | 1 | STUB | 0 | PAPER-ONLY | 0 | FALSE | 3 | UNREACHABLE | 0 |
| Internal contradictions found | 3 |

## 1. Blunt verdict
A records-era doc (evidence 2026-10-25) whose central claim — "There is
no software H.264 decode" — was false by 2026-11: rust_h264 0.6.1 was
admitted, locked, and SHIPPED as the avcC-seeded SW decoder (W6/W13,
executed). The doc's honest hardware-first narrative is real history
that was never updated.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: hw decode-path seam delivered (card 5.F2); playback hw-only pending Phase-7 adapters" | :3-4 | FALSE-STALE | shipped build decodes h264 in software via rust_h264 (W13-A01/A04 executed; pipeline_host.rs:346) |
| 2 | "There is no software H.264 decode, and none is scheduled" | :8-9 | FALSE-STALE | rust_h264 =0.6.1 in Cargo.lock; baseline mp4_h264_family_lane 100%; 17/17 h264 differential (W18-A08 row 023 REAL) |
| 3 | "the 2026 wave — rusty_h264-decoder 0.16.1 et al. — stays out until it passes our corpus" | :10-12 | FALSE-STALE | rust_h264 admitted via h264-software-decoder-audits.md record (W17-A05 verified record vs lock) |
| 4 | "the 5.F1 clean-room parser (NAL/Annex-B/AVC/avcC, SPS/PPS, slice headers, SEI) is LANDED and corpus-verified" | :5-6 | REAL | W6-A02/A04/A10 audits: parser spec-conformant, corpus-gated |
| 5 | "ADR-001 (hw-first for H.264/HEVC ... truthful active-decoder reporting)" | :~15 | REAL | ADR-001 exists; note the irony: the "truthful reporting" ADR governs the file whose registries now lie (W13) |
| 6 | "software_decode = blocked on every platform, wasm32 = blocked" | :19-20 | FALSE-STALE | matrix row 023 = REAL sw decode (linux); wasm32 blocked remains true |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "no software H.264 decode" | lock: rust_h264 0.6.1; shipped binary plays h264 MP4 | Doc stale |
| "hw-only" status | engine-info prints mjpeg/pcm only | BOTH registries lie in different directions (W13-A07) |
| "stays out until it passes our corpus" | h264_software records recommend admission | Admission happened after doc date |

## 4. Fabrication / inflation indicators
Not fabrication — undated staleness. The doc lacks any "superseded by"
marker while being flatly wrong at HEAD; readers relying on it would
believe MP4 video decode is impossible in the shipped build.
