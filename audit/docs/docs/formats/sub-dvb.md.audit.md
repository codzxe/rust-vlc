# CLAIM AUDIT: docs/formats/sub-dvb.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-S2) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL 6 / REAL-UNPROVEN 2 / UNVERIFIED 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Honest, precise record. Every structural claim verifies exactly against
crates/subtitles/src/dvb/** and the matrix row quotes this record (and its honest
gaps) verbatim. The self-build decision, caps, test census and the no-PMT-in-this-
crate boundary all check out. Nothing inflated.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Row subtitle-dvb-subs backfilled to implemented by 5.Q2-V with honest gap "NO docs/formats record" (G-1) — this file closes it | :5-8 | REAL | docs/feature-matrix.toml:1004 status implemented x3/web not_started; :1005 evidence "...record docs/formats/sub-dvb.md (G-1 closed, 2026-10-26)"; :1006 notes "...honest gap: NO reference-encoder differential ... no corpus lane, record owed" — the row's honest-gap text matches the record's own deferred list. |
| 2 | self_built = crates/subtitles/src/dvb/** (mod/segment/rle/clut/fuzz) | :18-19 | REAL | ls crates/subtitles/src/dvb -> clut.rs fuzz.rs mod.rs rle.rs segment.rs — exact five files. |
| 3 | Unit suites: 38 tests across mod/segment/rle/clut | :49 | REAL | rg -c '#\[test\]' -> segment.rs 9, rle.rs 11, mod.rs 11, clut.rs 7 = 38 exactly. |
| 4 | Damage ledger 64 events (overflow counted); region pixel buffers 4 MiB | :44-45 | REAL | mod.rs:77 `pub const MAX_DAMAGE_EVENTS: usize = 64;`, :82 `pub const MAX_REGION_PIXELS: u32 = 4 * 1024 * 1024;` (byte-exact). |
| 5 | Contract: DvbDecoder::decode_pes (PES payload + 90 kHz PTS -> DvbPageUpdate -> CompositedCue) | :40-43 | REAL | mod.rs:26 "[`DvbDecoder::decode_pes`] takes one subtitle PES payload", :268 `pub struct DvbDecoder`, :293 impl. |
| 6 | BadClutId fallback to 2/4/8-bit default banks | :33-34 | REAL | mod.rs:104 `BadClutId { clut_id: u8, region_id: u8 }`, :16 comment "the default banks (the `BadClutId` fallback)". |
| 7 | Fuzz-lite survival tests/dvb_vobsub_fuzz_survival.rs; in-crate harness src/dvb/fuzz.rs | :15,:52 | REAL | ls crates/subtitles/tests -> dvb_vobsub_fuzz_survival.rs present; src/dvb/fuzz.rs present. |
| 8 | No PMT plumbing of its own — pairs with the TS demuxer's PMT walk | :11-13 | REAL | rg -c 'pmt|PMT' crates/subtitles/src/dvb/*.rs -> 0 matches; crates/demux/src/ts/psi.rs:322/:355 (`TID_PMT => parse_pmt`) owns it demux-side. |
| 9 | Gates at card: 230/0 across cargo test -p subtitles; current tree re-verified green | :75-77 | REAL-UNPROVEN | Historical gate runs (same 230 figure as sub-srt.md's record — consistent cross-citation); not re-runnable here (load discipline); baseline cargo-test-workspace.txt subtitles package green. |
| 10 | ffmpeg 7.1.5 refuses text→bitmap transcoding; differential NOT generatable | :57-61 | REAL-UNPROVEN (claim about ffmpeg behavior) | Consistent with the family-wide gap noted in AGENT-SPEC §13 (VLC 3.0.23 oracle PENDING-INFRA; ffmpeg is the always-on oracle); no DVB corpus lane exists in-tree to contradict it. |

## 3. Contradictions
None. The 230/0 figure is shared with the sibling sub-srt.md record (same gate run)
and the matrix evidence repeats it verbatim.

## 4. Fabrication / inflation indicators
None — the record's headline honesty moves are disclosures (no differential, no
corpus lane, web not_started), and the matrix absorbed exactly those caveats.

Commands run: ls dvb module + tests; rg -c '#\[test\]' per file (9/11/11/7=38); sed -n 1003,1010p feature-matrix.toml; rg MAX_DAMAGE/MAX_REGION_PIXELS/DvbDecoder/BadClutId mod.rs; rg -c pmt dvb/ (0) + psi.rs pmt parse; rg commits ab696b4 in matrix.
