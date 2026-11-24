# CLAIM AUDIT: docs/formats/video-h264-software.md
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A02) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 / 0 / 0 / 0 / 0 / 0 / 0 (2 of 6 REAL-at-date-since-flipped) |

## 1. Blunt verdict
Honest blocked-record whose own "next step" was executed five days later: the
Lead DID admit rust_h264 0.6.1 (2026-11-15), the matrix row flipped to
`implemented` (win/linux/macos), and the admission landed with exactly the
differential-gate shape this doc predicted. The record is now STALE as a
status snapshot (says blocked ×4; the tree says implemented) — the doc
promised "status flips are evidence-driven"; they were.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "row status `blocked` ×4, ... admission is still pending" | :9-12 | REAL-at-date, SUPERSEDED | feature-matrix row now `status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }`; evidence cites "rust_h264 =0.6.1 Lead-admitted 2026-11-15" |
| 2 | "Nothing implemented for software decode ... decode behind `H264HwDecoder`" | :27-31 | SUPERSEDED | crates/codecs/src/video/h264/sw.rs:2 "`rust_h264` engine (0.6.1, exact pin) behind the codec-api" — software decode landed |
| 3 | "no decoder crate is a workspace dependency (audited in throwaway projects)" | :18-19 | SUPERSEDED | Cargo.lock:769-770 `name = "rust_h264" / version = "0.6.1"`; codecs Cargo.toml:73 `codec-h264-sw = ["dep:rust_h264"]` |
| 4 | audits record exists, 2026-11-09, candidate table | :33-43 | REAL | docs/dependencies/records/h264-software-decoder-audits.md:19-21: rust_h264 RECOMMEND [PROPOSAL], rusty_h264 fallback, openh264 **REJECT** — verbatim match incl. openh264 "vendored Cisco C++/NASM" |
| 5 | "if accepted, the admission lands with a corpus + fuzz + differential gate pass" | :54-56 | REAL (executed) | matrix evidence: "differential crates/codecs/tests/h264_sw_differential.rs: 6 libx264 lanes x 2 entry forms ... bit-exact vs ffmpeg 7.1.5 whole-dump MD5 + hostile battery"; file exists on disk |
| 6 | "the audits' numbers ... are upstream claims quoted in the audits record, not measurements in this tree" | :61-63 | REAL-at-date | honest scope-limit; post-admission differential measurements now exist (matrix evidence), exactly distinguishing upstream claims from tree measurements |
| 7 | "outdates `docs/formats/video-h264.md`'s 'none is scheduled' framing" | :5-6 | REAL (context) | video-h264.md is the parser/hw record; sw admission is separately documented here — no conflict found |
| 8 | "admission record docs/dependencies/records/rust_h264-0.6.1.md" (matrix evidence) | — | REAL | file referenced by matrix row + h264/mod.rs:19; consistent chain |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| blocked ×4, decision pending (:10-13) | matrix: implemented ×3, admission 2026-11-15 | Matrix is true — the decision landed after this record's last_audit (2026-11-10) |

## 4. Fabrication / inflation indicators
None — a blocked record that explicitly refuses to self-serve a status flip.
Findings: none (staleness is the doc's dated nature, disclosed in-file).
