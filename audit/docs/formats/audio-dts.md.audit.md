# CLAIM AUDIT: docs/formats/audio-dts.md
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 6 concrete claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 REAL / 0 / 0 / 0 / 0 / 0 / 1 (oxideav-dts crates.io existence) |

## 1. Blunt verdict
Honest blocked-with-record doc; every in-tree claim verified against code. Same external-
survey caveat as the AC-3 record (crates.io claims unverifiable in-tree).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "matrix …: `blocked` ×4, empty `evidence` — this record is that evidence" | L6-7 | REAL | feature-matrix.toml:773-774 `status = { windows = "blocked", … }` / `evidence = ""` |
| 2 | "PS: private-stream-1 `0x88..=0x8F` → DTS routing with the substream+filler strip" | L74-75 | REAL | crates/demux/src/ps/mod.rs:149 `0x88..=0x8F => Some(PrivateKind::Dts),`; strip=4 (mod.rs:159) |
| 3 | "fixture `ps-vob-dts.vob` … pinned in `crates/demux/tests/ps_fixtures.rs`" | L76-77 | REAL | fixtures/ps/ps-vob-dts.vob exists; ps_fixtures.rs:127 `file: "ps-vob-dts.vob",` |
| 4 | "TS: stream type `0x82 → "dts"`, MediaKind::Audio" | L78-79 | REAL | crates/demux/src/ts/mod.rs:362 `0x82 => "dts",` + :381 MediaKind::Audio arm (rg-verified) |
| 5 | "Raw ES: no `.dts` reader" | L81 | REAL | ls crates/demux/src/es/ → aac/annexb/h264/hevc/mp3 only |
| 6 | "No DTS material exists anywhere in corpus-v4 … only DTS bytes in-tree are the committed 5.I2 VOB fixture" | L88-90 | REAL | python3 over corpus-v4-smoke-manifest-20261023.json: dts files [], vob files [] |
| — | "fuzz_target = `fuzz_codec_dts` (planned name only)" | L11 | REAL | ls fuzz/fuzz_targets/: no fuzz_codec_dts (12 codec targets, none dts) |
| — | "Decode refuses at the registry level — typed-refusal status quo" | L84 | REAL | baseline mp4-smoke engine-info: decoders mjpeg, pcm_s16le only |

## 3. Contradictions
None found. The doc's own drift section (L115-123) flags matrix/feasibility staleness
consistent with the AC-3 record's self-report.

## 4. Fabrication / inflation indicators
None. oxideav-dts/vuio-codec-dts existence claims (L31-43) UNVERIFIED — external network
survey, no in-tree artifact. "94 PES" fixture detail and "bit-exact vs ffmpeg -c copy"
rest on the committed fixture + demux PS tests (ps/tests.rs:234-240 exercises the DTS
substream strip) — not re-executed here (UNVERIFIED execution, code verified).
