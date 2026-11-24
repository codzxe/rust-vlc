# CLAIM AUDIT: docs/formats/video-wmv-vc1.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 5 / 1 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
An honest nothing-implemented decision record. Its negative claims verify: no WMV/VC-1
decoder exists in the tree, the matrix row matches (not_started/blocked/unsupported with
empty evidence), and the per-platform reasoning (macOS blocked: no VideoToolbox VC-1
exposure) is consistent with the hw-decode design.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: deferred (later) — decision record" | L3 | REAL | nothing in crates/codecs/src/video/ named wmv/vc1 (ls: av1 dnx h264 hevc mjpeg mpeg12 vp9 only) |
| 2 | "Nothing implemented" (§Parse/Decode) | body | REAL | rg for vc1/wmv decode modules -> none |
| 3 | matrix row claim: "codec-video-wmv-vc1 (win/linux not_started, macos blocked, web unsupported)" | L14-15 | REAL | matrix row verified: `status = { windows = "not_started", linux = "not_started", macos = "blocked", web = "unsupported" }`, evidence = "" |
| 4 | "web = unsupported (not in the WebCodecs registry)" | L13 | REAL-UNPROVEN | external spec fact; consistent with ADR framework |
| 5 | "macOS = blocked (no VideoToolbox exposure, no pure software decoder)" | L12 | REAL-UNPROVEN | platform API fact; consistent with platform-macos crate scope |
| 6 | "The container half (ASF demux) is a separate Phase-6 proposal (5.K3-I → container-asf.md)" | L15-16 | REAL | docs/formats/container-asf.md exists (53 files in formats dir) |

## 3-5. Contradictions / fabrication / matrix rows
None / none — a PAPER-ONLY-negative record that claims nothing, and it is accurate.
Commands: ls crates/codecs/src/video; matrix row sed.
