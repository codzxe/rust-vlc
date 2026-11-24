# CLAIM AUDIT: docs/formats/audio-dts.md
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-R4) |
| Claims extracted | 11 |
| REAL 8 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 3 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A decision record that does exactly what it says: no code changed, carriage
claims verified in-tree (TS 0x82, PS 0x88..=0x8F, committed VOB fixture with
the 94-PES pin), the blocked ×4 matrix row with empty evidence matches, no DTS
dependency anywhere in the workspace, and no DTS material in the corpus — all
as stated. Its external crates.io survey (oxideav-dts 0.0.2, vuio fork) is
network fact, unverifiable here. Its "Doc drift for the Lead" section is
accurate: the matrix blocked_reason and feasibility row 13 are still stale in
exactly the way the doc describes.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | matrix row codec-audio-dts: `blocked` ×4, empty evidence | :6-7 | REAL | feature-matrix.toml:765 `id = "codec-audio-dts"`; status all four "blocked"; `evidence = ""` (sed verified). |
| 2 | TS: stream type 0x82 → "dts", MediaKind::Audio | :78-79 | REAL | crates/demux/src/ts/mod.rs:362 `0x82 => "dts",`; :381 audio-kind set including 0x82. |
| 3 | PS private-1 0x88..=0x8F → DTS routing with substream+filler strip | :74-75 | REAL | crates/demux/src/ps/mod.rs:48 "`0x80..=0x87` AC-3, `0x88..=0x8F` DTS"; ps/tests.rs:231-240 routing test (substream 0x88, strip 4). |
| 4 | "fixture ps-vob-dts.vob ... (94 PES) pinned in crates/demux/tests/ps_fixtures.rs" | :76-77 | REAL | ps_fixtures.rs:127 `file: "ps-vob-dts.vob"`; comment "ffprobe: dvd_nav_packet + mpeg2video + dts; packets 0 + 25 + 94"; binary committed at crates/demux/fixtures/ps/ps-vob-dts.vob. |
| 5 | "Raw ES: no .dts reader (es/ = aac/annexb/h264/hevc/mp3)" | :81-82 | REAL | `ls crates/demux/src/es/` -> aac.rs annexb.rs error.rs fuzz.rs h264.rs hevc.rs mod.rs mp3.rs tests.rs — no dts. |
| 6 | "No DTS material exists anywhere in corpus-v4 ... the only DTS bytes in-tree are the committed 5.I2 VOB fixture" | :88-90 | REAL | `rg -n 'dts' tests/fixtures/CORPUS.md` -> no hits; only fixture is crates/demux/fixtures/ps/ps-vob-dts.vob (find verified). |
| 7 | crates = [] — none admitted | :12-13 | REAL | `rg 'dts|oxideav' Cargo.toml crates/codecs/Cargo.toml Cargo.lock` -> no hits. |
| 8 | "Ride the AC-3 record's actions: IEC 61937 passthrough card (both codecs)" | :44-48,109-110 | REAL | docs/formats/audio-ac3.md:151-153 "4. **S/M card proposal (for the Lead): IEC 61937 passthrough output mode** ... AC-3 and DTS ride it". |
| 9 | VLC pinned build configured --disable-dca (formats.json dts row rationale) | :49-51 | REAL | docs/inventory/formats.json:953 "doc04 §3 P2 (DVD/DTS-CD niche; --disable-dca in this build's configure)". |
| 10 | Drift note: feasibility audio-codecs row 13 "dts = unrelated tooling" is stale | :121-123 | REAL | docs/feasibility/sections/audio-codecs.md:45 row 13 "None. crates.io `dts` exists but is unrelated data-transform tooling (re-checked)" — unchanged, i.e. the flagged drift exists exactly as described. |
| 11 | oxideav-dts 0.0.2 (2026-05-03, 68 downloads) + vuio-codec-dts 0.0.1 fork (43 dl) | :30-41 | UNVERIFIED | Network/crates.io facts; no local copy, no lockfile entry (consistent with non-admission, but the survey itself is not re-checkable offline). |

## 3. Contradictions
None. The matrix/feasibility staleness it flags is still present (the doc
declares that update out of its scope for the Lead), so the tree state is
consistent with the record instead of contradicting it.

## 4. Fabrication / inflation indicators
None. A refusal record with zero code motion and verifiable negative space:
no dep, no ES reader, no corpus material, typed refusal at registry level —
all four negatives confirmed by search.
