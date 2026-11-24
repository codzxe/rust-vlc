# CLAIM AUDIT: docs/formats/audio-aac.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R5) |
| Claims extracted | 10 |
| REAL 5 / REAL-UNPROVEN 3 / UNVERIFIED 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Solid record whose in-tree claims all verify: crate pinned exactly (symphonia-codec-aac
=0.6.1), both self-built modules exist (codecs::audio::aac with adts/decoder/fuzz, demux::adts
with tests), all four typed-refusal codes exist in error.rs, the bitexact reference file is
committed, and the matrix rows say exactly what the doc says (aac-lc implemented/partial;
he-aac blocked ×4 with empty evidence). Differential/throughput/test-gate numbers are
point-in-time measurements not re-runnable here; crate-internals claims (0 unsafe, forbid)
are UNVERIFIED offline.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | crates = [symphonia-codec-aac =0.6.1 (MPL-2.0)] | :12 | REAL | crates/codecs/Cargo.toml:109 `symphonia-codec-aac = { version = "=0.6.1", optional = true }`; Cargo.lock entry version 0.6.1 + checksum. |
| 2 | Lock diff is exactly 4 nodes (codec-aac, symphonia-common, symphonia-metadata, regex-lite) | :13-14 | REAL | Cargo.lock greps: symphonia-codec-aac + all 3 dependency names present (rg -c -> 3/3). |
| 3 | self_built = codecs::audio::aac (adts parse + resync + codec-api adapter + fuzz) + demux::adts | :19-21 | REAL | ls crates/codecs/src/audio/aac/ -> adts.rs decoder.rs error.rs fuzz.rs mod.rs; ls crates/demux/src/adts/ -> mod.rs tests.rs fuzz.rs test_support.rs. |
| 4 | fuzz_target = fuzz_codec_aac (harness landed; libFuzzer CI owed) | :11 | REAL | crates/codecs/src/audio/aac/fuzz.rs + crates/codecs/tests/aac_fuzz_survival.rs (rg --files-with-matches); CI wiring absent as admitted. |
| 5 | HE/policy typed refusals: aac.he_sbr, aac.profile, aac.channel_config_pce, aac.channel_count | :38-41 | REAL | error.rs:146-147 `unsupported("aac.he_sbr", "HE-AAC (SBR/PS) is honestly absent")`; decoder.rs:40-54 documents all four codes (rg/sed read). |
| 6 | PCM MD5s recorded per differential row (tests/data/aac_bitexact_reference.txt) | :60-61 | REAL | crates/codecs/tests/data/aac_bitexact_reference.txt exists (find). |
| 7 | status implemented for AAC-LC; HE-AAC blocked row unchanged | :8-9 | REAL | feature-matrix.toml:665-684 aac-lc `status = { windows="implemented", linux="partial", macos="implemented", web="not_started" }` (evidence cites THIS record); :685+ he-aac `blocked ×4`, evidence `""`. |
| 8 | Differential table (6 rows, 1-LSB maxΔ, 0094 typed channel_count, dirty-lane 350-vs-351 divergence) | :51-67 | REAL-UNPROVEN | measured-at-evidence-date; matrix evidence string (:681) repeats the same numbers incl. "dirty-lane 350-vs-351 divergence" — self-consistent, not re-executed (cargo-build budget). |
| 9 | Throughput 66.8M samples/s = 758× realtime (0059) | :68-69 | REAL-UNPROVEN | point-in-time release-build measurement; no bench re-run in this sandbox. |
| 10 | Tests at card gates: 5.C2a 271/0/13; 5.C2b 331/0 (244 unit + 30 adts + corpus) | :73-74 | REAL-UNPROVEN | current `#[test]` census in aac+adts src = 48 (unit subset only; gate totals include integration suites in tests/); cannot re-run full gate (load discipline); baseline cargo-test-workspace.txt shows codecs packages green. |

## 3. Contradictions
None. HE-AAC honesty (blocked row with empty evidence) matches the doc's "honestly partial/
blocked" and the code's typed refusal.

## 4. Fabrication / inflation indicators
None — the doc enumerates five honest gaps including a disclosed divergence vs ffmpeg
(351st short packet) rather than claiming a clean sweep.

## 5. Unverified externals
- symphonia-bundle-aac "does not exist on crates.io" (:14-15) — offline, no registry query.
- `forbid(unsafe)` in codec-aac+metadata and 0 unsafe sites in common (:16-18) — cargo
  registry sources not present in this sandbox; no rustc available.

Commands run: rg symphonia-codec-aac Cargo.toml + grep Cargo.lock; ls aac/adts modules; rg fuzz_codec_aac/he_sbr/channel_*/aac.profile; find aac_bitexact_reference.txt; sed 665-700p feature-matrix.toml; rg -c "#\[test\]" aac+adts; rg faad modules-linux.json (:2425 "AAC audio decoder (using libfaad2)" — VLC oracle exists, lumen-side differential honestly pending).
