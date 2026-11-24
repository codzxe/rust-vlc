# CLAIM AUDIT: docs/formats/video-h264.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 4 / 0 / 1 / 0 / 0 / 2 / 0 |

## 1. Blunt verdict
The record's parser + hw-seam claims verify. But its headline claim — "There is no
software H.264 decode, and none is scheduled" — is FALSE against the current tree:
`rust_h264` 0.6.1 is Lead-admitted and wired into the CLI decode path (2026-11-15,
post-dating the record's 2026-10-25 evidence date). Both the record AND the matrix row
(whose evidence string still cites only 5.F1/5.F2 cards) are stale on the software
decoder; the code is ahead of both.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "The 5.F1 clean-room parser (NAL/Annex-B/AVC/avcC, SPS/PPS, slice headers, SEI) is LANDED and corpus-verified" | L5-7 | REAL | crates/codecs/src/video/h264/parser/ exists (ls); matrix evidence cites 19/19 + 26/26 + 185/185 slice corpus counts |
| 2 | "5.F2 adds the hardware consumption point: codecs::video::h264::hw ... + codecs::video::h264::decode" | L7-9 | REAL | crates/codecs/src/video/h264/{hw/,decode.rs} exist |
| 3 | "There is no software H.264 decode, and none is scheduled before ADR-001's ... software track produces a validated decoder" | L9-11 | FALSE (stale) | apps/cli/src/pipeline_host.rs:346: `"h264" \| "avc1" => self.seeded(track, config, H264SwDecoder::new)`; Cargo.toml:69-73 `codec-h264-sw = ["dep:rust_h264"]` "the Lead-admitted" admission |
| 4 | "no production-grade pure-Rust H.264 decoder exists today" (quoting doc 04) | L11-12 | FALSE (stale) | rust_h264 0.6.1 admitted + differential evidence crates/codecs/tests/h264_sw_differential.rs cited in pipeline_host.rs:332-335 |
| 5 | "playback hw-only pending Phase-7 adapters" | L3-4 | SHALLOW (stale) | hw/ modules exist; sw path now also live — the hw-only framing is outdated |
| 6 | "Date of evidence: 2026-10-25" | L14-15 | REAL | self-consistent; explains the staleness (rust_h264 landed 2026-11-15) |
| 7 | matrix proposal: codec-video-h264 = hw-only | L15-17 | STALE | matrix row reads `partial` on win/linux/macos citing 5.F1/5.F2 evidence — neither doc nor matrix mentions the sw decoder that the CLI now uses |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "no software H.264 decode" | pipeline_host.rs:346 seeds H264SwDecoder; Cargo.toml codec-h264-sw | code wins — software decode IS live in the CLI path |
| hw-only proposal for the matrix | matrix row = partial; CLI error text claims "landed: ... h264" | code + CLI text both reflect the sw decoder; matrix/record lag |

## 4-5. Fabrication / matrix rows
The record was honest for its date; the defect is un-refreshed staleness on exactly the
row this record governs. codec-video-h264 matrix row audit is rows-XXX chunk owner's file;
finding recorded here for the cross-check. Commands: ls h264; rg pipeline_host.rs;
rg Cargo.toml.
