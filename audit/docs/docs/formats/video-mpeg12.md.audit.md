# CLAIM AUDIT: docs/formats/video-mpeg12.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R4) |
| Claims extracted | 10 |
| REAL 7 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 3 |
| Internal contradictions found | 1 (matrix-status wording drift) |

## 1. Blunt verdict
The record's landed code is all present: the six-module mpeg12 parser with the
typed `mpeg12.decode.blocked` refusal and `descriptor_gate`, `sequence_changes`
and `slices_with_mb_address` facts, all five committed fixtures (incl. the
escaped-4:2:2 and TS lanes), 34 unit tests, and zero external deps (crates=[]).
Its candidate survey (oxideav-mpeg12video etc.) is crates.io fact, unverifiable
offline — and the doc says so explicitly ("could NOT re-query crates.io ...
flagged here rather than silently re-asserted"). One wording drift: the matrix
row today reads `blocked`, not the `not_started` the front matter describes.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "the sequence-layer parser (crates/codecs/src/video/mpeg12/ ...)" landed | :4-6 | REAL | `ls crates/codecs/src/video/mpeg12/` -> bitreader.rs error.rs facts.rs headers.rs mod.rs startcode.rs. |
| 2 | "decode refuses with `ErrorKind::Unsupported` at `mpeg12.decode.blocked`" | :51-52 | REAL | error.rs:109 `ErrorContext::new("mpeg12.decode.blocked")`; mod.rs:19 "(`mpeg12.decode.blocked`); no `Decoder` impl lives under this". |
| 3 | "`facts::descriptor_gate`" gates mpeg1video/mpeg2video | :49-50 | REAL | mod.rs:42 "descriptor gate ([`facts::descriptor_gate`])"; error.rs:134 test `descriptor_gate_accepts_exactly_the_two_ids`. |
| 4 | Fixture inventory: 5 committed ffmpeg-authored files | :87-90 | REAL | crates/codecs/tests/fixtures/mpeg12/ contains exactly mpeg1-ipb.m1v, mpeg2-422.m2v, mpeg2-in-ts.ts, mpeg2-interlaced-ntsc.m2v, mpeg2-main-ipb.m2v. |
| 5 | "`sequence_changes` ... the mid-stream header change" + `slices_with_mb_address` evidence | :85,96 | REAL | facts.rs:51 `pub sequence_changes: bool`, :61 `pub slices_with_mb_address: u32`, :89 sets it true. |
| 6 | crates = [] — none admitted | :21 | REAL | `rg 'oxideav|mpeg12' crates/codecs/Cargo.toml` -> only a comment (zero deps note :26); no dep entries. |
| 7 | "fuzz_codec_mpeg2 ... does not exist until a decoder is admitted; fuzz-lite ... lives in facts.rs today" | :18-20,106-107 | REAL | `ls fuzz/fuzz_targets | rg mpeg2` -> none; facts.rs carries 6 #[test]s incl. fuzz-lite rounds (module doc). |
| 8 | "Differential ... 10/10 PASS, 0 FAIL (table in the 5.H1 worklog entry)" | :36,58-66 | UNVERIFIED | Run artifact in a worklog entry; the named fixtures/lanes all exist in-tree, but the differential itself needs live ffprobe — not re-runnable. |
| 9 | "380 lib + 43 integration suites, 0 FAILED" | :67-68 | UNVERIFIED | cargo runs out of budget; baseline shows codecs-suite shape consistent (0 failed workspace-wide except the 2 known). |
| 10 | "matrix ... carries ... via `not_started` rows" | :10-12 | STALE-wording | feature-matrix.toml row `codec-video-mpeg1-2`: `status = { blocked ×4 }`, `evidence = ""`, phase_target = 5 — the doc's own status (blocked) matches; the "not_started" phrasing does not match today's row (status evolved to blocked, evidence still empty — the doc is still the unlinked evidence). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| matrix rows `not_started` | matrix rows now `blocked` ×4 | Tree state is `blocked` — matches the doc's status line; the "not_started" wording is stale/loose. Evidence field still empty (record not linked) — the doc's role as unlinked evidence persists. S4 drift. |

## 4. Fabrication / inflation indicators
None — the record earns trust by printing its own limitations in detail
(no macroblock decode, AVI/MP4 lanes not run through own demuxers, extensions
counted-not-parsed, crates.io survey stale) and each limitation checks out
against the code shape (headers-only parser, no Decoder impl).
