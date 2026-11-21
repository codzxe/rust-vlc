# AUDIT: crates/demux/fixtures/ps/ps-vob-lpcm.vob#part1 (bytes 0–89 926)
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-11-21 |
| Size / "lines" | 241 664 B (exactly 118 × 2 048-byte DVD sectors) / 2 264 newline-"lines"; **part1 = lines 1–566 = bytes 0–89 926** (44 sectors) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO for the fixture itself (test-only asset; every consumer is `#[cfg(test)]`/integration-test — proof below). The **mpegps format** it exercises IS shipped-reachable: `apps/cli/src/containers/mod.rs:71` `"mpegps" => ps::open(stream, opts)` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `python3 /tmp/ps_parse.py` + `/tmp/lpcm_detail.py` + `/tmp/vid_detail.py` + `/tmp/scr_check.py` (independent ISO-13818-1 byte-walk — reproduced every pinned value); `ffprobe -show_streams/-show_packets` (7.1.5); `ffmpeg -map 0:1/-map 0:2 -c copy -f data` extractions; `rg` reference hunt; `sed -n` quote verification |

## 1. One-paragraph verdict
This is a genuine ffmpeg-`dvd`-authored MPEG-PS/VOB (mpeg2video 720×480 @ 24000/1001 + mono 48 kHz 16-bit DVD-LPCM + nav), sector-aligned to the byte, and **every number the test suite pins against it is real**: my independent parser (raw ISO-13818-1 walk, no Lumen code) reproduces the pinned counts (4 nav / 68 video / 48 LPCM PES), payload totals (135 397 B / 96 000 B), both md5 digests, first/last PTS (48 754 / 135 090 / 138 583), and packet sizes (2 008 … 1 424) exactly, and live ffmpeg 7.1.5 extractions confirm the byte-level claims (video bit-exact; audio 96 144 = 96 000 + 3 × 48). The defects are around the fixture, not in it: no authoring recipe or whole-file hash exists anywhere (unlike the flac/ogg/adpcm fixture families), this file's nav payload bytes are pinned by no test at all, the LPCM timestamps carry an undocumented ~1.3 % pacing stretch, and one test comment documents an idealized AU decomposition the file contradicts.

## 2. What actually works (verified)
— independent Python ISO-13818-1 walk (`/tmp/ps_parse.py`) → 118 packs, all 2048-spaced (pack i at byte i×2048, stuffing 0, mux-rate field constant 50 401, SCR 16 384→150 789, +146/pack), 2 system headers, 4 nav PES (980 B zero-filled lead-0x00 + 1 018 B lead-0x01 alternating), 68 video PES (0xE0), 48 LPCM PES (0xBD, substream 0xA0 × 48, no other substreams), 3 padding PES (0xBE), 0 PSM, 0 program-end code; file tail = the last padding PES filling sector 117 exactly.
— same walk → video payload 135 397 B md5 `130873a40a8090129373005692be7269`, LPCM payload after 7-byte prefix strip 96 000 B md5 `d7754037c4ff56d806ada9dc67877c59`, stripped sizes {2 008×1, 2 012×42, 2 016×4, 1 424×1}, first/last PTS video 48 754/135 090, LPCM 48 754/138 583 — **all identical to the pins** in `crates/demux/tests/ps_fixtures.rs:146-168`.
— `ffprobe -show_streams` (7.1.5, same family that authored the fixtures) → `dvd_nav_packet` + `mpeg2video 720x480 24000/1001` + `pcm_dvd 48000 Hz channels=1`; `-show_packets` → **0 nav + 25 video + 48 audio** — matches doc row and `ffprobe: [0, 25, 48]`.
— `ffmpeg -i <file> -map 0:1 -c copy -f data` → 135 397 B, md5 `130873a40a8090129373005692be7269` (**bit-exact** vs our PES concatenation); `-map 0:2 -c copy -f data` → **96 144 B**, first 16 bytes `0c 00 80 | 00 00 00 e3 01 df 02 b7 03 ae 04 85 05` = 3-byte AU header + our exact first sample bytes — the "+3 B × 48 pcm_dvd AU headers" claim verified at byte level.
— baseline `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt`: `:1839 ps::tests::fuzz_lite_clean_fixtures_drain_without_terminating_errors ... ok`, `:2179 fixture_stream_tables_and_packet_counts_match_the_pins ... ok`, `:2180 fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit ... ok`, `:2181 lpcm_packets_strip_the_full_dvd_prefix ... ok`.
— part1 structure (bytes 0–89 926): 44 packs; sector 0 = pack(14) + system header(24) + nav 980(986) + nav 1 018(1 024) = 2 048 exact; 41 video PES (7 PTS-stamped: 48 754…71 277); 2 LPCM PES (offsets 4 110, 26 638); 0 padding. Chunk boundary note: byte 89 927 falls **mid-PES** (video PES 87 078..90 112) — the line-based chunking of a binary is arbitrary; part2 (A04) inherits that PES's tail.

## 3. Findings

### F-001 [S3]
Where: crates/demux/fixtures/ps/ps-vob-lpcm.vob (whole file); evidence at docs/worklog/archive-2026-10.md:9162, tools/xtask/src/cmd/fixtures.rs:113-115, crates/demux/tests/ps_fixtures.rs:157-160
Code (worklog, provenance record):
```
- Inherited uncommitted: crates/demux/src/ps/{mod,error,fuzz,test_support}.rs (1.6 kloc) + 4 ffmpeg fixtures (mpg + ac3/dts/lpcm VOBs) — but NO tests.rs (the `mod tests;` decl pointed at a missing file), NO lib.rs wiring, and 5 mechanical compile errors
```
Code (xtask scope):
```
/// The committed fixture tree root (relative to the workspace root).
const FIXTURES_SUBDIR: &str = "tests/fixtures";
```
Code (test digest pin):
```
        digest: [
            "",
            "130873a40a8090129373005692be7269",
            "d7754037c4ff56d806ada9dc67877c59",
        ],
```
What is wrong: the ps fixture family has **no recorded authoring recipe** — `rg` across docs/worklog finds the differential measurement (archive-2026-10.md:9166) but no ffmpeg invocation, while the sibling families record theirs (ogg: :8587 "Generated ffmpeg-authored fixtures (lavfi sine, ffmpeg 7.1.5…)"; flac: :8528-8530 "recorded decode/dump commands and MD5s"; adpcm: :9332 exact per-flavor commands). `cargo xtask fixtures --check` (baseline PASS #12) only drift-gates `tests/fixtures/` (FIXTURES_SUBDIR, fixtures.rs:115 — zero vob/mpg references), so these files are outside every regeneration gate. And for THIS file the nav payload is byte-unpinned: `LANES[3].digest[0] = ""` (ps_fixtures.rs:158 — "the nav lane: unhashed control payload", skip at :258-259) and the nav byte/size assertions (`vob_nav_packets_are_real_and_zero_padded_by_ffmpeg`, ps_fixtures.rs:315) run on `LANES[1]` = ps-vob-ac3.vob, not this file; the CLI test (apps/cli/src/containers/ps.rs:964) pins only the nav **count** (4) and kind here.
Why it matters: the fixture cannot be regenerated or authenticated as a whole; contrary to the repo's own fixture policy ("no fixture of unknown provenance, ever" — docs/inventory/corpus-plan.md:60, :127), and a corruption of this file's nav sector (bytes ≈38–2 047) would pass every committed test (count stays 4, digest lane skipped, sizes unpinned on this file).
What correct looks like: commit the ffmpeg authoring command(s) for the four ps fixtures (the flac `tests/data/flac_bitexact_reference.txt` pattern) plus whole-file sha256 rows; extend the nav byte assertions to all VOB lanes or pin `LANES[3]` nav sizes {980, 1018}.
Confidence: HIGH — Verification: read (rg + sed) / ran (my parse + ffprobe/ffmpeg 7.1.5).

### F-002 [S4]
Where: crates/demux/fixtures/ps/ps-vob-lpcm.vob — LPCM PES timestamps (PES at offsets 4 110…239 630; measured: `/tmp/lpcm_detail.py` + `ffprobe -show_packets`)
What is wrong: the LPCM PTS cadence does not match payload duration: inter-stamp PTS deltas are 2 059–2 091 ticks whether the inter-stamp span carries one packet (2 012 B) or two (4 028 B), while one 2 012-B packet at the file's declared 48 kHz/mono/16-bit (ffprobe-confirmed) is 1 886 ticks. Net: stamped span = 91 164 ticks (ffprobe duration 1.012933 s) vs 90 000 ticks of actual sample data — a ~1.3 % stretched, internally overlapping timeline (e.g. unstamped PES 10 spans ffprobe 69 416–71 306 while the next stamp is 69 621).
Why it matters: nothing current is false — the docs/tests pin span ends as "last observed PES stamp" honestly (container-mpegps.md "we do not fabricate") — but no doc records this pacing quirk, and the future PCM-DVD decode card (owed by `ps/mod.rs:78-80`: "the DVD LPCM header bytes are skipped, not re-exposed as channel/rate facts `[PROPOSAL: …]`") will inherit an A/V-sync puzzle: audio PTS-to-PTS delta 23.2 ms ≠ audio per-packet duration 21.0 ms.
What correct looks like: a "known fixture quirks" row in docs/formats/container-mpegps.md recording the stretched LPCM stamp cadence (authoring-muxer artifact), next to the existing honest-divergences table.
Confidence: HIGH — Verification: ran (my PTS dump + ffprobe durations 1 882/1 886/1 890/1 335 vs deltas 2 059–2 091; ffprobe duration 1.012933 s vs 96 000 B ÷ 96 000 B/s = 1.000 s).

### F-003 [S4]
Where: crates/demux/tests/ps_fixtures.rs:163-165 (comment about this fixture)
Code:
```
        // ffmpeg keeps a 3-byte pcm_dvd AU header per packet:
        // 96 144 = 48 × (3 + 2 000) — the exact-delta assertion.
        ffmpeg_bytes: [0, 135_397, 96_144],
```
What is wrong: the "(3 + 2 000)" per-AU decomposition is an idealized average, not this file's measured truth: ffmpeg's per-AU sizes are 3 + {2 008, 2 012×42, 2 016×4, 1 424} (ffprobe packet sizes 2 011, 2 015×42, 2 019×4, 1 427 — my `ffprobe -show_packets` dump), i.e. the AU sample size is variable, not 2 000.
Why it matters: the pinned assertion itself (`ffmpeg_bytes - bytes == 3 * ours`, :282-291) is exact and correct; but the comment teaches a false per-AU model of the fixture, and a future reader "verifying" the pin by expecting 48×2 003 identical AUs will find a different file.
What correct looks like: "96 144 = 96 000 + 3 × 48 (one 3-byte pcm_dvd AU header retained per PES; AU sample sizes vary 1 424–2 016)".
Confidence: HIGH — Verification: ran (ffprobe sizes; arithmetic 2 008+42×2 012+4×2 016+1 424 = 96 000).
Note: the comment lives in ps_fixtures.rs (another agent's file); recorded here because it documents THIS fixture's measured truth.

## 4. Per-structure table (binary asset — per-function table adapted; part1 counts / whole-file counts)
| Structure | part1 | file | Verdict | Notes |
|---|---|---|---|---|
| Pack headers `00 00 01 BA` (MPEG-2) | 44 | 118 | REAL | all at i×2048, stuffing 0, SCR +146/pack monotonic 16 384→150 789, rate field 50 401 constant |
| System header `0xBB` (18 B) | 1 (offset 14) | 2 | REAL | one per nav sector (0 and 124 942) |
| Nav PES `0xBF` | 2 (980/1 018) | 4 (980,1 018,980,1 018) | REAL | 980 B all-zero lead 0x00; 1 018 B lead 0x01 rest zero — exactly the documented ffmpeg PCI/DSI shape |
| Video PES `0xE0` | 41 | 68 | REAL | 24 PTS-stamped, gaps exactly 3 753/3 754 = 24000/1001 fps; first payload = `00 00 01 B3 2D 01 E0 21` (720×480, 4:3, 23.976); payload 135 397 B |
| LPCM PES `0xBD`→substream 0xA0 | 2 (offsets 4 110, 26 638) | 48 | REAL | constant 7-byte prefix `a0 07 00 04 0c 00 80`; stripped payload 96 000 B; byte4 `0x0c` ⇒ 48 kHz/16-bit per ffmpeg pcm_dvd layout |
| Padding PES `0xBE` | 0 | 3 (119 017/175 951/241 076) | REAL | legal sector padding; demuxer damage-notes each (ps/mod.rs:1041-1044), pinned count==3 at apps/cli ps.rs:964 |
| PSM `0xBC` / program-end `0xB9` | 0 | 0/0 | REAL(absent) | file ends with the final padding PES filling sector 117 exactly |

## 5. Test honesty (tests consuming this fixture)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fixture_stream_tables_and_packet_counts_match_the_pins (ps_fixtures.rs:222) | YES | counts + first/last PTS pins match ground truth I reproduced independently; any demux count/stamp regression fails |
| fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit (:254) | YES (video+lpcm), NO (nav) | digests+sizes pinned and match my parse; nav lane skipped (`digest[0]=""`, :258-259) — see F-001 |
| lpcm_packets_strip_the_full_dvd_prefix (:296) | YES | 48/2 008/1 424/96 000 exact; a strip-off-by-one breaks sizes and both digests |
| vob_nav_packets_are_real_and_zero_padded_by_ffmpeg (:315) | N/A for this file | runs on LANES[1] (ps-vob-ac3.vob); this file's nav bytes unpinned (F-001) |
| fuzz_lite_clean_fixtures_drain_without_terminating_errors (ps/tests.rs:522) | WEAK alone | asserts only `terminal == None`; compensated by the CLI test below. Mutation-fuzz coverage for THIS file is zero — at 236 KiB it exceeds the "+FB" 200 KB budget (ps/tests.rs:499 lists only the two smallest siblings; note: ps-vob-dts.vob at 188 KiB is *under* the stated budget yet also unmutated — for A03) |
| vob_lanes_demux_and_refuse_undecodable_codecs_typed (apps/cli/src/containers/ps.rs:903, cfg(test)) | YES | pins 4/68/48 counts, `notes == 3` "padding-stream" (:964), span ends 135 090/138 583, typed `cli.codec_unavailable` refusal for pcm_dvd — the nav-count/PES-count/damage-note pins live HERE, not in the demux-crate suite |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ps-vob-lpcm.vob \| nav+mpeg2video+pcm_dvd, 0+25+48 \| 4nav+68v+48lpcm \| ours 96 000 B pure samples; ffmpeg 96 144 = +3 B × 48" | docs/formats/container-mpegps.md:53 | REAL | my parse (4/68/48; 96 000; md5 match) + ffprobe (0+25+48) + ffmpeg extraction (96 144 B) |
| "we expose 4 zero/0x01-lead Data packets (sizes 980/1018 alternating, measured)" | container-mpegps.md:62-64 | REAL (for this file too, though asserted only on the ac3 sibling) | my parse: 980(0x00-lead, all zero)/1 018(0x01-lead) ×2 |
| "0xBE padding damage-noted" | container-mpegps.md (error-paths section) | REAL | ps/mod.rs:1043-1044 `self.damage.note(ext, "padding-stream")`; this file has 3 such packets; pinned `notes==3` at apps/cli ps.rs:964 |
| "no corpus files (verified) — crates/demux/fixtures/ps/{ps-mp2-video-audio.mpg, ps-vob-{ac3,dts,lpcm}.vob}" | docs/plans/open-path-wiring.md:333 | REAL | `ls fixtures/ps` = exactly those 4 files |
| "Inherited uncommitted … 4 ffmpeg fixtures" | docs/worklog/archive-2026-10.md:9162 | REAL | consistent with the missing authoring recipe (F-001) |
| "96 144 = 48 × (3 + 2 000)" per-AU model | ps_fixtures.rs:163-164 | SHALLOW (idealized average) | measured per-AU 3+{1 424..2 016}; see F-003 |

## 7. Dead / duplicated / unreachable code
Not applicable (binary asset). Provenance chain instead: authored by ffmpeg `dvd` muxer (structure is unmistakable: 2 048-B sectors, nav pairs, pcm_dvd prefix), inherited uncommitted (worklog 9162), committed by card 5.I2. No shipped binary reads it — all three consumers are test code (ps_fixtures.rs, ps/tests.rs:527, apps/cli ps.rs:586/909 under `#[cfg(test)]`:556).

## 8. Dependencies & flags
None (data asset). Format wiring: container id `mpegps` → `ps::open` (apps/cli/src/containers/mod.rs:71); decode of pcm_dvd/mpeg2video is typed refusal by design (first vertical = mjpeg+pcm only, baseline engine-info).

## 9. Open questions for the next agent
- F-002's stretched LPCM timeline (91 164 vs 90 000 ticks) — the PCM-DVD codec card must decide gap-fill vs compact-then-sync policy; needs a doc row before that card starts.
- part2 auditor (A04): boundary context — byte 89 927 cuts video PES 87 078..90 112; padding PES #1 (119 017) and the second nav sector (124 928) are in your range; whole-file totals above are my cross-check, per-packet detail from 89 927 on is yours.
- Why is ps-vob-dts.vob (188 KiB < 200 KB) excluded from fuzz-lite mutation? (ps/tests.rs:497-499 comment says "the two smallest" but the budget rationale doesn't exclude it.) → A03 / ps/tests.rs owner.
- ps_fixtures.rs:214 swallows `Recovered` wholesale in the demux-crate differential; today the CLI test (ps.rs:964) backstops it — if that CLI test is ever moved/gated, the backstop silently disappears. → ps_fixtures.rs owner.
