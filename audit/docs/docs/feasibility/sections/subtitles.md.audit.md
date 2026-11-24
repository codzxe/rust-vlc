# CLAIM AUDIT: docs/feasibility/sections/subtitles.md (435 lines, Wave-0 investigation 0.C2-I)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 10 |
| REAL 6 / REAL-UNPROVEN 1 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 3 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A high-quality feasibility doc: every external claim carries [RV]/[VERIFY] marks with URLs,
the census numbers are exactly reproducible from the committed inventory JSON, the risk
register actually contains the R-09/A9 rows the doc says it feeds, and its v1-path
predictions aged well (SRT/PGS now implemented, ASS partial — exactly the shapes it
proposed). The only unverifiable material is the crates.io metadata (sandbox has no
network) — 11 crate facts stay UNVERIFIED at source level. The doc's own honesty statement
("capability descriptions are quoted from crate metadata ... never inferred") is consistent
with its structure.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "VLC 3.0.23 Linux census — subtitle category = **32 modules**: `spu decoder` 22 + `sub source` 7 + `sub filter` 1 + `text renderer` 2 (docs/inventory/modules-linux.json)" | :9-12 | REAL | python3 over docs/inventory/modules-linux.json by_capability: spu decoder count=22, sub source=7, sub filter=1, text renderer=2 (tdummy, freetype) — sum 32. EXACT match. |
| 2 | §1.1 module dispositions (subsdec, libass, webvtt, ttml, substx3g, stl, kate, subsusf, spudec, svcdsub, cvdsub, dvbsub, aribsub, cc, scte27, scte18, textst, zvbi, avcodec, marq/rss/logo/mosaic/audiobargraph_v/remoteosd/dynamicoverlay, subsdelay, ddummy×2, stats) | :56-77 | REAL | Every name appears in modules-linux.json by_capability module lists (python3 dump cross-checked; e.g. spu decoder list contains scte18, subsdec, stl, aribsub, avcodec, libass, ttml, spudec, kate, substx3g, cc, ddummy×2, dvbsub, zvbi, webvtt, subsusf, scte27, textst, svcdsub, cvdsub, stats = 22; sub source has audiobargraph_v, remoteosd, marq, mosaic, logo, dynamicoverlay, rss = 7). |
| 3 | "libass is C — forbidden (doc 04, unchanged)" / "freetype is C — forbidden" | :32, :70 | REAL | `rg -n 'libass' Cargo.lock` -> 0 hits (nothing C-side imported); consistent with ADR-006 MIT and risk-register R-09 wording. |
| 4 | "R-09 (ASS styling) ... doc 14: 'trigger: unsupported-tag count in conformance'" | :7, :219-221 | REAL | docs/14-risk-register.md:100 "R-09 ... **trigger: unsupported-tag count in conformance**"; :153 "`ass.styling = partial (subset)` permanently in v1"; :243 "A9 ASS full styling parity not claimed in v1". Exact matches. |
| 5 | "matrix `ass.styling = partial (subset)`" / row 2 v1 path "partial — plain text v1 Phase 2, styling subset Phase 5" | :32, :29-31 | REAL (aged prediction) | docs/feature-matrix.toml:905-912 subtitle-ass-ssa: status partial/partial/partial/not_started with evidence "crates/subtitles/src/ass/** ... G-2: 267/0 lib tests incl. per-tag suites" — the proposed shape landed. |
| 6 | Row 1 SRT "software-decode (self-build S)" ; Row 9 PGS "software-decode parse ... iff compositing contract lands" | :27, :43 | REAL (aged prediction) | matrix subtitle-srt :892-902 implemented w/l/m (evidence crates/subtitles/src/srt/**); subtitle-pgs :957-965 implemented w/l/m (ADR-012 self-build, "RLE 0x00 escapes, BT.601-vs-BT.709-by-height palette ... the old [VERIFY] is resolved"). |
| 7 | Rows 4-8 (SubViewer/MPL2/SAMI/TTML/MicroDVD) later-phase self-builds | :33-39 | ACCURATE (as predictions; now not_started) | matrix rows subtitle-microdvd/subviewer/sami/ttml = not_started x4 with empty evidence — consistent with "Phase 5/6 later". |
| 8 | External crate facts: ass-parser 0.2.3 MIT 2025-04-11, cea608 0.0.1 builder-not-decoder, cc708/eia608 NOT_FOUND, oxisynth 0.1.0 LGPL-2.1 2025-05-25, rustysynth 1.3.7 MIT 2026-10-01 std-only | :27-47, :375-390, §7 | UNVERIFIED (network) | Sandbox offline; crates.io not re-quotable. Internal consistency: matrix row 870 (MIDI) cites "rustysynth 1.3.7 (MIT, pure, std-only, includes SMF parsing) ... oxisynth 0.1.0 is LGPL-2.1 and stays out-of-tree (feasibility subtitles §5 classification)" — the downstream consumer reproduced the same facts, one downstream consistency point in their favor. |
| 9 | "Feeds: `docs/feasibility-report.md` §5" | :3-4 | REAL | docs/feasibility-report.md exists (ls docs/*.md); :4 "feasibility sections (0.C1-I video, 0.C2-I audio+subtitles, 0.C3-I ...)" names this section family. |
| 10 | MIDI "v1 recommendation [PROPOSAL]: MIDI = later, not blocked ... no oxisynth dependency" | :347-350 | REAL (recommendation adopted downstream) | feature-matrix.toml:870 blocked_reason: "rustysynth 1.3.7 ... enables a LATER path with user-supplied .sf2; oxisynth 0.1.0 is LGPL-2.1 and stays out-of-tree (feasibility subtitles §5 classification). No v1 card." — verbatim adoption of this doc's posture. |

## 3. Contradictions
None found. The census numbers, risk-register rows, and matrix statuses all reconcile.
One nuance, not a contradiction: feasibility row 2 predicted "plain text v1 Phase 2, styling
subset Phase 5"; the matrix notes say the G-2 override-tag engine landed via issue #17 — the
subset arrived ahead of the predicted phase split, with "beyond-minimum tags still degrade
honestly" (matrix :912), which matches §2.2's degrade/drop/counters design.

## 4. Fabrication / inflation indicators
None detected. The doc marks its own unverified items ([VERIFY] ledger §6 with 12 entries),
distinguishes existence-checks from full audits (:325-330 "[RV] marks are existence/version/
license checks only"), and its census arithmetic is exactly reproducible. The "~95% of
styling value" estimate is kept [VERIFY] and made measurable (§2.3) rather than asserted.
