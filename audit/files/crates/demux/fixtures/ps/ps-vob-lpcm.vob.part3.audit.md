# AUDIT: crates/demux/fixtures/ps/ps-vob-lpcm.vob#part3 (bytes 167,829–204,678)
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-11-21 |
| Kind | chunk of a binary fixture (line-based chunking: "lines" 1129–1692 of 2264 = bytes 167,829–204,678, 36,849 B = 15.2% of 241,664; region starts mid-packet 107 B before its first complete pack and ends 122 B into the last packet — line cuts of a binary are arbitrary; parts 1/2/4 belong to other agents) |
| Lines / functions | 78 source-lines-equivalent N/A — binary; 18 complete 2048-B sectors + 2 partial edges audited |
| File verdict | REAL (ffmpeg-authored DVD-VOB, MPEG-2 PS; every structural claim and every pinned test number in my chunk reconciled bit-exact from raw bytes by this audit) |
| Reachable from shipped entry point? | N/A (test asset) — referenced and executed by: crates/demux/tests/ps_fixtures.rs (3 tests), crates/demux/src/ps/tests.rs:527 (fuzz-lite), apps/cli/src/containers/ps.rs:909 (conformance); all green in baseline `orchrun-test-workspace-nofailfast.log.txt:2176-2184, 1839, 2833` |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `file` → "MPEG sequence, v2, program multiplex"; `wc -c/-l` → 241,664 B / 2,264 newlines (manifest.tsv:464 says 2,265 lines — off-by-one vs `wc -l`, audit-tooling artifact); python3 byte scans (pack/SCR/PTS/PES decode — scripts inline below, all numbers in this audit are their output); `rg -n "ps-vob-lpcm"` over crates/ apps/ docs/ scripts/ tools/ (4 code/doc reference sites, 0 generator scripts); `sed -n` on every quoted source range |

## 1. One-paragraph verdict
This chunk is the LPCM-dense middle of a genuine ffmpeg-authored DVD-VOB: 18 perfect 2048-byte
sectors, each = 14-B MPEG-2 pack header (SCR strictly monotone 69,622→100,968, constant mux
rate 25,200×50 B/s = 10.08 Mbps, zero stuffing) + one PES packet filling the sector exactly —
17 × private-stream-1 LPCM (all substream 0xA0, all with the identical 6-byte DVD LPCM header
`07 00 04 0c 00 80`, PES length 2028, PTS on 16 of 17) and 1 × mpeg2video (0xE0, len 1851)
closed out by a legal 0xBE padding packet (171 B) — sector arithmetic to the byte. Every
number the test suite pins for this fixture I recomputed independently from raw bytes:
48 LPCM packets, Σ(PES_len−3−hdr_len−7) = 96,000 sample bytes exactly, first/last PTS
48,754/138,583, first packet 2,008 B, final packet 1,424 B. The only defects are provenance-
and documentation-level: the PTS stamp cadence has an undocumented anomaly (below) and no
regeneration recipe exists for the fixture anywhere in the repo.

## 2. What actually works (verified)
— Structure: python3 scan → 18 packs in-chunk at exact 2048-B spacing; every sector fully
accounted (17 × [14+6+2028=2048] LPCM, 1 × [14+6+1851=1857]+[6+171=177] video+padding);
SCR monotone, mux rate constant, no pack stuffing; zero strings ≥8 printable chars and zero
runs ≥16 zero bytes in the chunk (no hidden metadata, no secret payload — pure PCM sample
data shaped like a smooth waveform).
— Test-pin reconciliation (bit-exact, recomputed from bytes): 48 × 0xBD packets file-wide;
ours-bytes Σ = 96,000 (= ps_fixtures.rs:159 `bytes: [0, 135_397, 96_000]`); first/last LPCM
PTS 48,754 / 138,583 (= ps_fixtures.rs:164-165 and containers/ps.rs:909 pin `138_583`);
first packet ours = 2,008 and last = 1,424 (= ps_fixtures.rs:308-309 pins); ffmpeg delta
96,144 − 96,000 = 144 = 3 × 48 (= the exact-delta assertion, ps_fixtures.rs:161-162, 281-290).
— Executed evidence (baseline, not re-run): `orchrun-test-workspace-nofailfast.log.txt:2176-2184`
— tests/ps_fixtures.rs 4/4 ok, including `fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit`
and `lpcm_packets_strip_the_full_dvd_prefix`; `:1839` — `ps::tests::fuzz_lite_clean_fixtures_drain_without_terminating_errors … ok`;
`:2833` — `containers::ps::tests::vob_lanes_demux_and_refuse_undecodable_codecs_typed … ok`.
— File is exactly 118 × 2048 B (241,664 = 118 sectors, no partial tail) — clean sectorized VOB.

## 3. Findings

### F-001 [S4]
Where: crates/demux/fixtures/ps/ps-vob-lpcm.vob, part3 PTS series (bytes 167,936+); consumer semantics in crates/demux/src/ps/mod.rs:78-81
Code (decoded from bytes — PTS column extracted by this audit's script):
```text
packet idx (file-wide)  PTS(90kHz)
… 16: 84 250   17: 86 338   18: — (unstamped, hdrlen 2)
19: 88 429    20: — (unstamped)   21: 90 518 …
1-AU gaps ≈ 2 059-2 091 ticks; the 4 gaps that span an unstamped AU are the SAME size
(≈2 089), not 2×. Per-AU average = 89 829/47 = 1 911.26 ticks = 21.236 ms for ~2 000 B AUs.
```
```rust
//!   (stamps only); the DVD LPCM header bytes are skipped, not
//!   re-exposed as channel/rate facts `[PROPOSAL: the 6-byte header
//!   carries the audio facts — owed to the PCM-DVD codec card]`.
```
(ps/mod.rs:78-80)
What is wrong: the four unstamped LPCM AUs (file-wide indices 10, 20, 30, 41) occupy zero
stamp-time — a stamp-to-stamp gap spanning two AUs advances the same ~2,089 ticks as a
single-AU gap — so PTS is not a linear function of AU index. The implied payload rate is
96,000 B / (89,829+~1,911)/90,000 s ≈ 94.2 kB/s, which matches no clean DVD-LPCM format
(48 kHz/16-bit/mono = 96 kB/s is 1.9% off; 44.1 kHz/16-bit/stereo = 176.4 kB/s is 87% off;
the constant header's byte 5 = 0x00 says 16-bit/48 kHz, implying 1,875 ticks per ~1,006-sample
AU, not 1,911). This is an ffmpeg-muxer artifact baked into the fixture, and the repo's
otherwise-thorough "Honest divergences" section (docs/formats/container-mpegps.md:60-65)
does not record it.
Why it matters: any future duration/bitrate arithmetic test built on this fixture inherits
a ~2% stamp-vs-sample drift; the header facts that would resolve the format are explicitly
not decoded (the mod.rs PROPOSAL above), so the anomaly is currently undetectable by design.
What correct looks like: one honest-divergence row in container-mpegps.md stating the
measured cadence (unstable per-AU stamps: 4 timeless AUs, ~94.2 kB/s implied vs 96 kB/s
nominal), so the PCM-DVD codec card starts from recorded truth; regenerate the fixture only
with a recorded command (F-002).
Confidence: HIGH for the byte-level facts (decoded by script, reproducible); MEDIUM for the
format identification (no spec in sandbox — header semantics UNVERIFIED against ISO/IEC
10918/DVD-Video book; consistent with ffmpeg `pcm_dvd` output).

### F-002 [S4]
Where: crates/demux/fixtures/ps/ps-vob-lpcm.vob (whole asset); provenance chain in docs/worklog/archive-2026-10.md:9162 and audit/runs/baseline/gate-fixtures___check.log.txt
Code:
```text
- Inherited uncommitted: crates/demux/src/ps/{mod,error,fuzz,test_support}.rs (1.6 kloc) +
  4 ffmpeg fixtures (mpg + ac3/dts/lpcm VOBs) — but NO tests.rs …
```
(worklog:9162)
```text
result: no drift — 55 file(s) verified under /home/z/my-project/audit-workspace/tests/fixtures
```
(gate-fixtures___check.log.txt, the baseline `cargo xtask fixtures --check` run)
What is wrong: no regeneration recipe exists for this fixture — `rg "ps-vob"` over scripts/
and tools/ finds zero hits; the doc names the authoring tool ("Corpus evidence (5.I2,
ffmpeg 7.1.5 …)", container-mpegps.md:42) but never the command line; the fixtures arrived
as uncommitted files from a dead prior session (worklog:9162). The repo's fixture-integrity
gate covers a different tree (`tests/fixtures`, 55 files) and does not verify
`crates/demux/fixtures/ps/` at all.
Why it matters: the 241,664 bytes are pinned only by the md5 digests committed in
ps_fixtures.rs — a change-detector, not a reproducer. Nobody can prove the pinned
ffmpeg-side numbers (96,144 B; digest d7754037…) came from this data rather than from a
different ffmpeg build; a future regeneration breaks all pins with no recorded way to
reproduce the original.
What correct looks like: a checked-in generation script (ffmpeg command with -f dvd,
input spec, ffmpeg version pin) or a FIXTURES.md provenance block next to
crates/demux/fixtures/ps/, plus extending the xtask fixtures manifest to cover this tree.
Confidence: HIGH — Verification: rg repo-wide (no recipe), read worklog:9162, read baseline
gate log (path differs).

## 4. Per-sector / per-packet structure table (the per-function equivalent for a binary)
| Structure (part3, rel offsets) | Count | Verdict | Notes |
|---|---|---|---|
| MPEG-2 pack headers `00 00 01 ba 44…` | 18 | REAL | Exact 2048-B spacing; SCR 69,622→100,968 monotone; mux rate 25,200 constant (10.08 Mbps DVD constant, ~14× the actual ~86 kB/s content rate — mux-rate field is nominal, players time off SCR/PTS); stuffing 0. |
| Private-1 0xBD LPCM packets | 17 | REAL | All: PES len 2028, MPEG-2 flags `80 80`, hdrlen 6 (PTS+1 stuffing byte) on 16, one hdrlen 2 (no PTS, 2 stuffing bytes); substream 0xA0; identical 6-B LPCM header `07 00 04 0c 00 80` on every packet (constant format); ~2,012-B sample payload each (16-bit big-endian ramp-like waveform, verified smooth). |
| Video 0xE0 packet | 1 | REAL | PES len 1851, no PTS (legal; flags2=0x00), hdrlen 1 (single 0xFF stuffing byte); carries mpeg2video ES bytes (contains in-band start codes — not packet starts). |
| 0xBE padding packet | 1 | REAL | 171 B of 0xFF; legal sector-alignment padding closing the video sector exactly to 2048. Note: demuxer classifies 0xBE as damage-noted (doc line "0xBE padding damage-noted") — a clean ffmpeg file yields damage notes (see §9). |
| 0xBF nav / 0xBB system header / PSM | 0 | N/A (this chunk) | 4 nav + 2 system headers live outside part3 (parts 1/2/4). |
| Unaccounted bytes between packets | 0 | — | Every complete sector consumed to the byte: 17×(14+2034) and 1×(14+1857+177) = 18×2048. |

## 5. Test honesty (tests consuming this fixture; would they fail if broken?)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ps_fixtures::lpcm_packets_strip_the_full_dvd_prefix | YES | Pins first packet 2,008 B, last 1,424 B, count 48, total 96,000 (ps_fixtures.rs:307-311). Mutation: strip 6 instead of 7 → first packet 2,009 ≠ 2,008 → fails. Independently reconciled from raw bytes by this audit. |
| ps_fixtures::fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit | YES | Per-lane byte totals + md5 digest + the exact ffmpeg delta 3×48 (ps_fixtures.rs:261-290). Strip/count/digest mutations all caught. (Name oversells for this lane: it is a pinned-delta, not bit-equality — the body is honest about it, lines 279-290.) |
| ps_fixtures::fixture_stream_tables_and_packet_counts_match_the_pins | YES | Asserts per-track counts (48), kinds, descriptors, first/last PTS ticks (48,754/138,583) — all reconciled from bytes. |
| ps::tests::fuzz_lite_clean_fixtures_drain_without_terminating_errors | NO (for semantics) | Absence-of-panic + `terminal == None` only (ps/tests.rs:533); a strip or PTS regression passes. Honest per its name; the pin tests above carry the semantics. |
| containers::ps::vob_lanes_demux_and_refuse_undecodable_codecs_typed | PARTIAL | Asserts container id, 3 tracks, codec descriptors, packet count 48, last PTS 138,583, typed decode refusals (ps.rs:909-950); does not sum payload bytes, so a strip change that keeps counts passes. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ps-vob-lpcm.vob | nav+mpeg2video+pcm_dvd, 0+25+48 | 4nav+68v+48lpcm" | docs/formats/container-mpegps.md:53 | REAL | 48 × 0xBD decoded from bytes; 68 raw 0xE0 and 4 × 0xBF occurrences file-wide (whole-file scan) match the pinned counts; lanes green in baseline :2176-2184. |
| "ours 96 000 B pure samples; ffmpeg 96 144 = +3 B × 48 pcm_dvd AU headers — exact delta pinned" | container-mpegps.md:53 | REAL | Σ(PES_len−3−hdrlen−7) = 96,000 recomputed from bytes; 96,144−96,000 = 144 = 3×48; consistent with repo strip 7 vs ffmpeg strip 4 (doc:55-56 "substream+3 filler … strip 4"). |
| "0xA0..=0xA7 LPCM (a 6-byte DVD LPCM header follows the substream byte)" | crates/demux/src/ps/mod.rs:49-50 | REAL | All 48 packets: `a0` + `07 00 04 0c 00 80` (constant). |
| "the DVD LPCM header bytes are skipped, not re-exposed" | ps/mod.rs:78-80 | REAL | The header is indeed opaque constant bytes; no channel/rate facts anywhere in the demux path (rg). |
| "4 ffmpeg fixtures … inherited uncommitted" | docs/worklog/archive-2026-10.md:9162 | REAL | The provenance statement; see F-002 for the missing recipe. |
| Manifest row "241664 / 2265 lines / vob / 4 refs" | audit/waves/wave-00/manifest.tsv:464 | REAL | Size exact; line count 2,265 vs `wc -l` 2,264 (trailing-newline convention, immaterial); 4 references confirmed. |

## 7. Dead / duplicated / suspicious content
- No dead/duplicated structure: every sector accounted; no orphan bytes, no unexplained
  start codes in this chunk (the one raw `00 00 01 e0` sequence is the packet itself; video
  ES in-band codes are payload, correctly not counted by the length-driven parser).
- No suspicious content: zero printable strings ≥8 chars, zero zero-runs ≥16 in the chunk.
- Cross-tree note: `cargo xtask fixtures --check` (baseline PASS) does not cover this
  fixture's tree (F-002).

## 8. Dependencies & flags
- No build dependencies (binary asset, git-tracked, no LFS pointer — 241,664 real bytes).
- No feature flags gate its consumers: ps_fixtures.rs is a plain integration test;
  containers/ps.rs conformance test is `#[cfg(test)]` in the CLI crate; all ran in the
  default-features baseline run cited above.

## 9. Open questions for the next agent
- The demuxer "damage-notes" legal 0xBE sector padding (doc: "0xBE padding damage-noted");
  this clean ffmpeg fixture produces 3 such notes (CLI test counts them, ps.rs:~935). Is
  "damage" the right classification for a conforming muxer's alignment padding? → ps/mod.rs
  auditor (W-wave owner of crates/demux/src/ps/mod.rs).
- F-001's format question: decode `07 00 04 0c 00 80` against the DVD-Video LPCM spec when
  the PCM-DVD codec card lands (the mod.rs PROPOSAL); expected: frame-count/FAUP/info bytes
  + 16-bit/48 kHz; then re-derive the nominal 96 kB/s and record the ~2% stamp drift.
- Parts 1/2/4 of this file (bytes 0-167,828 and 204,679-241,664) are other agents' chunks —
  my whole-file scans (counts 48/68/4, Σ bytes, PTS endpoints) corroborate their regions but
  were not per-sector audits of them.
