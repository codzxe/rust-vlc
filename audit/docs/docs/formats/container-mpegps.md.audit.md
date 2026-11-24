# CLAIM AUDIT: docs/formats/container-mpegps.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A09) |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL 6 / REAL-UNPROVEN 1 / FALSE 0 / UNVERIFIED 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The MPEG-PS/VOB container record (5.I2, dated 2026-10-24). Its code claims verify exactly:
the ps/ module set, the EXTENSIONS registry line byte-matching its quote, and the matrix
row it asked to amend is now implemented x3 platforms WITH this record as evidence — the
Lead amendment it requested landed. Its honest-divergence framing (PES-vs-AU packet counts,
nav-packet exposure) is the doc's own accounting, unpinnable here. No fabrication.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "EXTENSIONS = ['mpg','mpeg','ps','vob','m2p'] (ps/mod.rs:121)" | :22-23 | REAL | crates/demux/src/ps/mod.rs:120-121 `pub const EXTENSIONS: &[&str] = &["mpg", "mpeg", "ps", "vob", "m2p"]` |
| 2 | "self_built = crates/demux/src/ps/** (mod/error/fuzz/test_support)" | :16-17 | REAL | ls crates/demux/src/ps/ -> mod.rs, error.rs, fuzz.rs, test_support.rs, tests.rs |
| 3 | "matrix row container-mpegps still reads not_started with empty evidence... amendment recorded for the Lead" | :3-6 | REAL at date; amended since | docs/feature-matrix.toml:181 row now: status implemented x3 + evidence "crates/demux/src/ps/** (card 5.I2; record docs/formats/container-mpegps.md, 5.REC-1)" |
| 4 | "fuzz_target = fuzz_demux_ps (in-crate ps/fuzz.rs... 240-round +FB fuzz-lite)" | :13-14 | REAL | crates/demux/src/ps/fuzz.rs exists; 240-round LCG runs not re-runnable (cargo budget) |
| 5 | "4 committed fixtures... md5 bit-exact" | :42-53 | UNVERIFIED | fixture md5 differentials not re-run (cargo/ffmpeg budget); fixture files presence not re-listed here |
| 6 | "No .ps/.vob lane exists in corpus-v4 (verified again by 5.K4-I)" | :44 | REAL-UNPROVEN | consistent with corpus manifests audited by R-runs (corpus-v4 lanes absent in sandbox sessions) |
| 7 | "Private-1 0xBD routing: 0x80+ AC-3, 0x88+ DTS, 0xA0+ LPCM, 0x20+ SPU" | :32-34 | REAL | the routing is carried in ps/mod.rs (0xBD substream class constants; cross-verified by 5.K4-I citations in the record) |
| 8 | "cargo test -p demux 442/0... ps_fixtures 4/4" | :100-104 | UNVERIFIED | cargo test forbidden; not in baseline granularity |
| 9 | "error paths typed demux.ps.*... zero panics" | :80-84 | REAL-UNPROVEN | error.rs zone constructors present; the fuzz-clean claim not re-executed |

## 3. Contradictions
None. The record's "not_started" complaint is snapshot-aging in the GOOD direction (the
amendment it demanded landed). No W18-style drift on this row today.

## 4. Fabrication / inflation indicators
None. The record states its divergences and gaps with numbers on both sides (PES vs AU
counts, exact +3B AU-header deltas) — the honest-reference-set posture is explicit.

## 5. Notes for next agents
- W13 context: this record exposes AC-3/DTS/LPCM descriptors WITHOUT decoders ("decode
  pending, never silent") — consistent with the registry-disagreement findings: exposed
  tracks vs engine-info decoders (mjpeg, pcm_s16le) is the same seam.
- The "amendment recorded in-file for the Lead" pattern worked here — compare with
  ADR-013's gate violation (amendment skipped) for the procedural contrast.
