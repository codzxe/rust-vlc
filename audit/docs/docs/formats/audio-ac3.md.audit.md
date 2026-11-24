# CLAIM AUDIT: docs/formats/audio-ac3.md
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-R3; replaces a prior audit at the wrong path audit/docs/formats/… missing the `docs/` segment) |
| Claims extracted | 12 |
| REAL 7 / REAL-UNPROVEN 3 (network/corpus claims) / still-live-handoff 1 / UNVERIFIED 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A decision record whose "landed state" section is fully verifiable in the tree and checks
out at the byte level (PS private-1 map, TS stream types, ES-gap, registry refusal, matrix
blocked ×4 with empty evidence). Its crates.io survey and corpus-lane claims are
network/snapshot-dependent (UNVERIFIED here). Its three "Doc drift for the Lead" items are
still unapplied — the matrix row's blocked_reason still carries the pre-record text.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "PS: private-stream-1 0x80..=0x87 → PrivateKind::Ac3 ... DTS rides 0x88..=0x8F" | :104-108 | REAL | crates/demux/src/ps/mod.rs:48 `0x80..=0x87 AC-3, 0x88..=0x8F DTS`; :134-135 `Ac3` variant; :148-149 range match — verbatim |
| 2 | "TS: stream types 0x81 → "ac3", 0x84 | 0x87 → "eac3", MediaKind::Audio" | :109-110 | REAL | crates/demux/src/ts/mod.rs:361, :364, :381 — all three lines verbatim |
| 3 | "Raw ES: no .ac3 reader yet (crates/demux/src/es/ = aac / annexb / h264 / hevc / mp3)" | :111-112 | REAL | ls es/ -> aac.rs annexb.rs error.rs fuzz.rs h264.rs hevc.rs mod.rs mp3.rs tests.rs — exactly that set, no ac3 |
| 4 | "Decode refuses at the registry level (no AC-3 decoder registered)" | :113-114 | REAL | rg ac3 in codecs/src/audio/mod.rs + lib.rs -> 0 hits; codec registry has no ac3 module (baseline engine-info decoders: mjpeg, pcm_s16le) |
| 5 | "status = blocked ... matrix carries status = blocked ×4 with empty evidence — this record is that evidence" | :6-9, :22 | REAL | feature-matrix.toml:759-760 `status = { windows = "blocked", ... }`, `evidence = ""` |
| 6 | "fixtures ps-vob-ac3.vob ... pinned in crates/demux/tests/ps_fixtures.rs" | :106-107 | REAL | crates/demux/fixtures/ps/ps-vob-ac3.vob present (ls); ps_fixtures.rs in tests/ (tree listing) |
| 7 | "crates = [] — none admitted ... 2026 candidate wave ... all below the validation bar" | :13-14, :76-97 | REAL-UNPROVEN | crates.io counts (6782 dl etc.) not verifiable offline; CORROBORATED: records/oxideav-ac3-0.0.11.md exists (dependency-record audit tree) and rg oxideav in all Cargo manifests = 0 hits (W17-R1 ac3 vta audit same result) |
| 8 | "corpus: 0001-audio-only.ac3 ... all 5 .ts files probed: h264-only or aac" | :118-126 | UNVERIFIED | corpus media dir not present in this snapshot (find corpus/ -> absent); claim self-flagged as "Honest gap" |
| 9 | "VLC oracle: spdif (passthrough packing) ... decode NOT exercised [V3 re-probe]" | :130-135 | REAL-UNPROVEN | formats.json/citations not re-run; consistent with the pinned-baseline posture recorded in ac3 vta audits |
| 10 | "Doc drift ... matrix codec-audio-ac3-eac3.blocked_reason → amend ... backfill evidence to this record" | :159-167 | REAL-and-still-live | feature-matrix.toml:761 blocked_reason STILL pre-record text ("No pure crate (ac3 NOT_FOUND 2026-10-19...)", no candidate-wave/patent-gate mention); evidence STILL "" — the handoff was never applied |
| 11 | "ADR-010 [PROPOSAL] is still open and gates any distribution" | :66-67 | REAL | ADR-010 record exists (docs/adr/ADR-010-codec-patent-shipping.md; audit tree corroborates [PROPOSAL] status citations in KNOWN-LIMITS) |
| 12 | "fuzz_target = fuzz_codec_ac3 ... does not exist until a decoder is admitted" | :11-12 | REAL | rg fuzz_codec_ac3 in fuzz corpus targets — none (fuzz/ dir target list has no ac3 target) |

## 3. Contradictions
None. The record's blocked-status verdict matches the matrix's blocked row exactly.

## 4. Fabrication / inflation indicators
None — the record *amends an earlier premise* (0.C2-I "None found" missed the 2026 wave)
against itself, documents its own evidence gaps ("no ac3-in-TS lane exists", "no E-AC-3
lane at all"), and marks legal claims [VERIFY] instead of asserting them.

## 5. Findings
F-001 [S4-doc-drift] :165-167 — the three Lead handoffs (doc 04 §3.1/§7, feasibility rows,
matrix blocked_reason/evidence backfill) remain unapplied in the current tree; the matrix
row still contradicts this record's "this record is that evidence" intent by carrying
evidence = "". Confidence HIGH (rg on the row, exit with old text).

## 6. Reachability
N/A (decision record). The carriage code it documents (PS/TS ac3 exposure) is shipped via
crates/demux (L2) — reachable from lumen-cli open paths.
Commands: rg on feature-matrix.toml:751-762, ps/mod.rs, ts/mod.rs, es ls, codecs rg,
Cargo-manifest oxideav rg (0 hits), ls fixtures.
