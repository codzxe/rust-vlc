# CLAIM AUDIT: docs/formats/audio-flac.md
| Field | Value |
|---|---|
| Auditor | W19-A07 (recovered by W19-S2) |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL 6 / TRUE-at-time-since-amended 1 / REAL-UNPROVEN 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Solid record. The flac module layout, claxon pin, purity record (sha256 byte-exact),
the VLC-differential-pending file, the bit-exact reference data and the bench test all
exist exactly as claimed; the matrix row has since absorbed the record verbatim
(including the 539x realtime figure). Only historical gate counts are not re-runnable,
and the self_built line count drifted +31 lines (2413 vs 2382) since the record.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Matrix `status = not_started` rows + empty evidence are STALE vs the landed decoder — backfill proposed | :6-8 | TRUE-at-time-since-amended | docs/feature-matrix.toml:707-709 now `status = { windows="implemented", linux="implemented", macos="implemented", web="not_started" }`, evidence = "...record docs/formats/audio-flac.md, 5.REC-2: 4 fixtures byte-exact + STREAMINFO MD5, corpus 0046, 539x realtime" — the proposed backfill landed citing this record. |
| 2 | self_built = codecs::audio::flac (2 382 lines: decoder/metadata/seek/error/fuzz) | :15-16 | REAL (structure) / drift | ls crates/codecs/src/audio/flac -> decoder.rs error.rs fuzz.rs metadata.rs mod.rs seek.rs (all named files present); wc -l -> 2413 total (vs 2382 claimed; +31 drift, understatement direction immaterial). |
| 3 | crates = [claxon =0.4.3 (Apache-2.0)]; purity record docs/dependencies/records/claxon-0.4.3.md | :14,:18 | REAL | crates/codecs/Cargo.toml:86 `claxon = { version = "=0.4.3", optional = true }`; record file exists; :18 sha256 `4bfbf56724aa9eca8afa4fcfadeb479e722935bb2a0900c2d37e0cc477af0688` matches the record's own sha256 (lines 18/117). |
| 4 | tests/flac_bitexact_reference.txt records commands + MD5s | :39-40 | REAL | crates/codecs/tests/data/flac_bitexact_reference.txt exists, 121 lines, provenance header "every fixture's PCM essence is authored by the external ffmpeg 7.1.5-0+deb13u1 encoder". |
| 5 | tests/flac_bench.rs — throughput row | :45 | REAL | crates/codecs/tests/flac_bench.rs exists (1 #[test]); sibling files flac_bitexact.rs (4 tests = the "4 fixtures"), flac_fuzz_survival.rs, flac_seek.rs, flac_vlc_differential.rs all present. |
| 6 | 47 552 483 inter-channel samples/s ≈ 539x realtime at 88.2 kHz | :43-44 | REAL-UNPROVEN | Arithmetic verified: 47,552,483 / 88,200 = 539.06x ✓; the measured figure itself is a historical release-mode run, not re-runnable (load discipline). |
| 7 | flac_vlc_differential.rs exists with pinned dump command + version gate; no vlc binary in sandbox | :58-60 | REAL | File exists; AGENT-SPEC §13 baseline confirms no VLC oracle in sandbox (rc-smoke; HEVC litmus "no decoder" path). |
| 8 | Tests at card gate: 208/0/11 ignored; workspace at wave-7 close 3040/0 | :49-51 | REAL-UNPROVEN | Historical gate counts; baseline cargo-test-workspace.txt shows 4744/2/51 workspace-wide today (post-record growth), codecs package green. |
| 9 | Honest gaps: Ogg-FLAC not exercised end-to-end; 32-bit depth no fixture; wasm claxon [VERIFY-P1]; libFuzzer CI unwired | :61-69 | REAL | All negative claims; no in-tree fixture lane for ogg-flac exists (ogg demuxer separate); web column not_started in matrix row; fuzz/fuzz_targets/fuzz_codec_flac.rs exists but no CI campaign wiring (family-wide, per 5.Q1). |

## 3. Contradictions
None. The record's figures (4 fixtures, corpus 0046, 539x) are repeated 1:1 in the
matrix evidence string.

## 4. Fabrication / inflation indicators
None. Encode explicitly deferred (v1 out of scope); the throughput figure is rounded to
3 significant digits with the raw samples/s given.

Commands run: rg -A7 codec-audio-flac matrix; ls + wc -l flac module (2413); find flac_bitexact_reference.txt/flac_bench.rs; head of reference data; rg -c '#\[test\]' flac test files; rg claxon Cargo.toml + sha256 in purity record; arithmetic recompute 47552483/88200=539.06.
