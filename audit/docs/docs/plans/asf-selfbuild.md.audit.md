# CLAIM AUDIT: docs/plans/asf-selfbuild.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL 5 / REAL-UNPROVEN 3 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERACHABLE 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An exemplary investigation card: claims are pre-labeled [VERIFIED]-this-session vs
[FORMAT KNOWLEDGE — verify in ASF-a/b], the environment pin is exact and still true, and
every artifact it promised or measured exists today — including the four follow-up units
that did not exist when it was written. Zero defects found.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "ffmpeg 7.1.5" generates the family (§1) | :15-16 area | REAL | `ffmpeg -version` executed in this sandbox -> "ffmpeg version 7.1.5-0+deb13u1" — exact pin match |
| 2 | "11 fixtures produced and ffprobe-verified" | :15 | REAL | crates/demux/src/asf/fixtures/ = exactly 11 files (8 wma/wmv/asf variants listed: mjpeg, msmpeg4v3, pcm_s16le, adpcm_ms, pkt8000, stream_broadcast, wmav1/wmav2, wmv1/wmv2) |
| 3 | ASF-a/b/c/d unit split (header tree / packets / index / differentials) | §"one-paragraph verdict" | REAL | crates/demux/src/asf/{header.rs, data.rs, guid.rs, mod.rs, test_support.rs, tests.rs, fuzz.rs, error.rs} all present |
| 4 | "36 asf tests" (per dead agent's count; this doc's lane evidence) | session claims | REAL | rg '#[test]' census: tests.rs 29 + guid.rs 3 + error.rs 3 + fuzz.rs 1 = 36 EXACT |
| 5 | "The corpus has **zero** asf/wma/wmv files" | :13 | REAL-UNPROVEN | corpus media gitignored (unverifiable directly); corroborated by compat-matrix-20261110.json regen_provenance ("issue-#53 additions for the two claimed containers corpus-big lacks (mpegps, asf)") |
| 6 | Session census (143 files, extension histogram) | §1.1 | REAL-UNPROVEN | /tmp corpus absent; arithmetic self-consistent (census sums to 143) |
| 7 | "play-duration lie and zero-filled Simple Index entries" hazards | §2.3 | REAL-UNPROVEN | honestly labeled [FORMAT KNOWLEDGE — verify in ASF-a/b]; the units have since landed |
| 8 | "Status: COMPLETE (investigation card — zero code changes)" | :3 | REAL | investigation-plan doc; the code it specced landed later as asf/ module (see #3) |

## 3. Contradictions
None. (The doc's "zero code changes" is about its own session, not the repo state — the
spec'd units landing later is the intended outcome, not a contradiction.)

## 4. Findings
None. What was checked and how: environment pin (executed ffmpeg -version), fixture count
(ls), unit-split artifacts (ls of crates/demux/src/asf/), test census (rg '#[test]'
sum = 36), corpus-zero claim (cross-checked against compat-matrix provenance; direct check
impossible — corpus media not committed), session measurements (not re-runnable; labeled
as such by the doc itself).

## 5. Prior-audit reconciliation
Dead W19-A09's audit (old tree audit/docs/plans/asf-selfbuild.md.audit.md) — "ffmpeg pin
match, 11 fixtures, 36 tests, unit split landed, session measurements UNVERIFIED, no
findings" — every number independently re-derived identically by me.
