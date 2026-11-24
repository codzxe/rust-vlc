# CLAIM AUDIT: docs/adr/ADR-012-bitmap-subtitles.md (179 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 REAL / 1 REAL-UNPROVEN (crates.io API checks) / 0 / 0 / 0 / 0 / 1 UNVERIFIED (Lead signature) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A decision record whose every in-tree citation reproduces, whose "not landed (honest)" list
was accurate at authoring (2026-10-24), and whose Option-1 consequences were subsequently
executed in full: `crates/subtitles` now has `pgs/`, `dvb/`, `vobsub/`, `cc/` module trees
(cc carries cc_data.rs, cea708.rs, eia608.rs — the staged 608→708 plan realized), and
matrix rows subtitle-pgs / subtitle-cc-608-708 are implemented ×3 with evidence naming this
ADR. The ADR's self-flagged gaps (MKV codec-map rows, SEI type-4 T.35, MPEG-2 user_data)
all landed per the matrix row evidence. One nuance the ADR's consequence list
overpromised vs reality: "fuzz targets `fuzz_sub_pgs` / `_dvb` / `_vobsub` / `_cc`" — the
cargo-fuzz target FILES do not exist in fuzz/fuzz_targets/ (only srt/ass/vtt); the harness
logic lives in-crate (pgs/fuzz.rs:14-24 honestly documents the unwired state: "Building it
needs the libfuzzer-sys nightly scaffolding — a Lead-owned dependency decision ... Until
the Lead wires it, the harness logic is proven on stable by the deterministic survival runs").

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "CueBitmap ... crates/subtitles/src/compositing/bitmap.rs:87 ... CompositedCue ... :190" | :28-31 | REAL — EXACT | sed :87 `pub struct CueBitmap`, :190 `pub struct CompositedCue`. |
| 2 | "VobSub carriage: PS demuxer ... 0x20..=0x3F → PrivateKind::Spu → "dvd_subtitle" (ps/mod.rs:151,171)" | :33-34 | REAL — EXACT | ps/mod.rs:151 `0x20..=0x3F => Some(PrivateKind::Spu)`, :171 `"dvd_subtitle"`. |
| 3 | "PGS-in-TS carriage: stream type 0x90 → "pgssub" (ts/mod.rs:364,381)" | :35-36 | REAL (lines drifted to :365,:382) | rg: :365 `0x90 => "pgssub"`, :382 `0x90 => MediaKind::Subtitle`. |
| 4 | "H.264 SEI ... type 4 ... captured as Unparsed{type,size} and skipped intact (sei.rs:24-26,148)" | :37-40 | REAL | sei.rs:25 "[`SeiMessage::Unparsed`] (type + size) and skipped intact"; parse of user_data_unregistered present. |
| 5 | "Not landed (honest): MKV codec map lacks S_HDMV/PGS, S_VOBSUB, S_DVBSUB" | :41-43 | REAL at date; LANDED since | rg mkv/header.rs for S_HDMV today → hits exist (map rows landed); matrix subtitle-pgs evidence: "Matroska-stripped framing ... in tree by the 5.R2-V Phase-5 exit 3116aec". |
| 6 | Consequences: "crates/subtitles grows pgs/, dvb/, vobsub/, cc/ modules" | tail | REAL — ALL EXIST | ls crates/subtitles/src/ → pgs dvb vobsub cc compositing ass srt vtt font model. |
| 7 | "fuzz targets fuzz_sub_pgs / _dvb / _vobsub / _cc per doc 04 §5" | tail | PARTIAL — harnesses in-crate, cargo-fuzz targets UNWIRED | fuzz/fuzz_targets/ has only fuzz_sub_{srt,ass,vtt}.rs; pgs/fuzz.rs:18-24 discloses the wiring gap honestly (Lead-owned libfuzzer-sys decision). |
| 8 | "Evidence: live crates.io API checks (curl, 2026-10-24 ...)" + Status "ACCEPTED — Lead signature 2026-10-24" | :2-8 | REAL-UNPROVEN / UNVERIFIED | No network in sandbox; signature not verifiable without git. |

## 3. Contradictions
None material. The ADR pre-declared its own doc drift (doc 04 §5 + feasibility ecosystem
rows predating 2026-10-24) — the drift exists exactly as flagged, and matrix backfills
cite this ADR as the decision source.

## 4. Fabrication / inflation indicators
None. The ADR separates sunk-cost code (verified) from not-landed gaps (verified absent at
date) and pre-registers risks; the ecosystem quotes are labeled as live-API snapshots.
Follow-through gap (fuzz wiring) is disclosed in-code, not just in doc.
