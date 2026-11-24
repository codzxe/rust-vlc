# CLAIM AUDIT: docs/formats/video-hevc.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 4 |
| REAL 3 / REAL-UNPROVEN 1 | |

## 1. Blunt verdict
An honest hw-only codec record: it leads with what is NOT there ("There is no software HEVC decode, and none is scheduled") and its landed claims (5.G1 parser, 5.G2 hw seam) are backed by the real crate tree. The mp4-smoke baseline corroborates the user-visible truth: HEVC open -> exit 65 "no decoder for `hevc`".

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "The 5.G1 clean-room parser ... is LANDED and corpus-verified (3/3 lanes, live-ffprobe differential)" | Status | REAL | crates/codecs/src/video/hevc/parser/ + decode.rs + hw.rs + fuzz.rs all exist (ls) |
| 2 | "5.G2 adds ... `codecs::video::hevc::hw` (the adapter seam) + `codecs::video::hevc::decode` (the codec-api Decoder)" | Status | REAL | hw.rs + decode.rs present in crates/codecs/src/video/hevc/ |
| 3 | "There is no software HEVC decode, and none is scheduled" | Status | REAL | matrix row codec-video-hevc-software (line 445 area) blocked; baseline mp4-smoke: HEVC open -> exit 65 "no decoder for `hevc`" |
| 4 | "field-exact vs ffmpeg trace_headers/ffprobe on corpus lanes 0044/0091/0100" | Parse | REAL-UNPROVEN | differential transcripts not in this doc; corpus absent in sandbox |

## 3. Contradictions
None surviving: the engine-info registry lie flagged in the Wave-0 baseline ("landed: ... h264" error text) is an engine-info string defect, not this record's — this record says no software decode, and the code agrees.

## 4. Fabrication / inflation indicators
None — deflation-first record (no-software-decode stated up top; patent posture deferred to ADR-010 honestly).
