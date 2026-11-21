# AUDIT: crates/demux/fixtures/ps/ps-vob-dts.vob (part 2 of 2 — bytes 82680..192512)
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-21 |
| Size / chunk | 192 512 bytes total; part 2 = bytes 82 680..192 512 (109 832 B, per the 467-newline chunk boundary: the file has 933 newline bytes, part 1 ends after the 467th) |
| File verdict | REAL (asset verified bit-level against every pin) |
| Reachable from shipped entry point? | The FORMAT is shipped-reachable (containers/mod.rs "mpegps" arm → ps::open; ps.rs:908 opens this fixture in tests); the fixture itself is test data |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
This fixture is the best-evidenced asset I have audited in this wave: I parsed every
byte of the 192 512-byte file with an independent Python MPEG-PS walker and it
matches EVERY pin the test suite commits — 94 pack headers, 2 system headers, 68
video PES (0xE0, 135 397 payload bytes, md5 `130873a4…7269` byte-identical to the
committed digest), 24 DTS PES on substream 0x88 (inside the documented 0x88..=0x8F
DTS carriage range; stripped payload exactly 48 128 bytes with md5
`de454125…20b0` byte-identical to the pin), 4 nav PES at exactly 980/1018/980/1018
with the pinned 0x00/0x01 PCI/DSI lead bytes and zero-filled bodies, and PTS ranges
(48 754→135 090 video, 48 754→136 114 DTS) exactly as the lane table records. Zero
orphan bytes: every byte in the file belongs to a start-code-delimited structure.
The two defects are integrity-provenance gaps, not content problems: nothing pins
the FILE's own hash (the payload md5s pin the payload, the container bytes — packs,
SCR, system headers — pin nothing), and the stream ends without the MPEG_program_end
code.

## 2. What actually works (verified)
- Independent byte-level parse (python3, this audit, READ-ONLY): whole file = 94
  packs + 2 system headers + 68 E0 PES + 24 BD/sub-0x88 PES + 4 BF PES;
  start-code coverage 192 512/192 512 bytes — **zero uncovered/orphan bytes**.
- Video payload: 135 397 B, md5 `130873a40a8090129373005692be7269` — equals the
  committed pin at crates/demux/tests/ps_fixtures.rs:137 (and is the SAME digest as
  the ac3/lpcm lanes' video — same video content, three audio variants).
- DTS payload: raw PES bodies 48 224 B; stripping the DVD substream framing
  (substream id + 3-byte header = 4 B × 24 PES) yields 48 128 B with md5
  `de45412596ea9fc909b4553055e420b0` — equals the committed pin at
  ps_fixtures.rs:138 EXACTLY. The first PES body is `88 04 00 01 7f fe 80 01 …` —
  substream 0x88 + header + the DTS 16-bit sync 0x7FFE8001 at body offset 4.
- Nav packets (correctly parsed as private_2 with NO PES extension — payload at
  start-code+6): @38 (980 B, lead 0x00), @1024 (1018 B, lead 0x01), @120870 (980 B,
  lead 0x00 — PART 2), @121856 (1018 B, lead 0x01 — PART 2); all rest-zero — exactly
  the shape `vob_nav_packets_are_real_and_zero_padded_by_ffmpeg` pins (read,
  ps_fixtures.rs:315-339; that test runs on the ac3 lane, same muxer family).
- PTS values: video first/last 48 754/135 090 (24 of 68 PES stamped — the unstamped
  B-frame tail the docs record), DTS 48 754/136 114 (all 24 stamped) — equal to the
  lane's `first_pts`/`last_pts` pins (ps_fixtures.rs:143-146).
- Baseline execution: `tests/ps_fixtures.rs` 4/4 ok —
  `fixture_stream_tables_and_packet_counts_match_the_pins`,
  `fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit`,
  `lpcm_packets_strip_the_full_dvd_prefix`,
  `vob_nav_packets_are_real_and_zero_padded_by_ffmpeg`
  (audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2176-2185).
- Not an orphan: referenced by crates/demux/tests/ps_fixtures.rs:127-146 (LANES[2]),
  crates/demux/src/ps/tests.rs:526 (fuzz + walk lane),
  apps/cli/src/containers/ps.rs:908 (open-path lane: 24 dts packets, last PTS
  136 114 — matching my parse), docs/formats/audio-dts.md:76,
  docs/formats/container-mpegps.md:52, docs/worklog/archive-2026-10.md.

## 3. Findings

### F-001 [S4] No file-level integrity pin — container bytes (packs/SCR/system headers) are unpinned
Where: crates/demux/fixtures/ps/ps-vob-dts.vob (whole file) vs the pin machinery
Code (the pin that exists — crates/demux/tests/ps_fixtures.rs:135-139):
```rust
        digest: [
            "",
            "130873a40a8090129373005692be7269",
            "de45412596ea9fc909b4553055e420b0",
        ],
        bytes: [0, 135_397, 48_128],
```
What is wrong: The committed pins cover PES counts, payload byte totals, payload
md5s, and PTS values — I verified all of them and they hold. But nothing pins the
file itself: no sha256 in any manifest (tests/fixtures/generated/MANIFEST.toml
covers only the generated first-vertical AVI/MP4 corpus; `cargo xtask fixtures
--check` — baseline #12 PASS — generates/checks that corpus, not
crates/demux/fixtures/ps). A regenerated .vob from a different ffmpeg version with
identical payload bytes but different pack headers, SCR stamps, mux rate, or
stuffing would pass every pin. The nav payload is pinned by shape only (lead byte +
zero-fill + sizes), not digest — adequate for zero-filled bodies, but it is the
weakest link in the chain.
Why it matters: The differential claims in docs/formats/container-mpegps.md:52
("md5 bit-exact") are payload-scoped; a reader could take them as file-scoped
integrity. The fixture is committed binary with no in-tree generator (provenance is
documentary: "ffmpeg 7.1.5, committed fixtures"), so the only reproducibility
anchor is the pin set.
What correct looks like: A one-line sha256 of each ps fixture in a committed
manifest (the generated corpus's MANIFEST.toml precedent), checked by a gate.
Confidence: HIGH — Verification: find for MANIFEST.toml (only
tests/fixtures/generated); read tools/xtask/src/cmd/fixtures.rs:1-50 (generates
AVI/MP4 first-vertical fixtures only); rg "ps-vob-dts" across repo (all consumers
listed above; none pins a file hash).

### F-002 [S4] No MPEG_program_end (0x000001B9) — the stream stops at the last PES
Where: crates/demux/fixtures/ps/ps-vob-dts.vob (end of file, offset 192 502..192 512)
Code:
```
(python3 parse this audit: `MPEG_program_end (B9) present: False`;
 last structure = the final DTS PES, then EOF)
```
What is wrong: The file ends without the PS end code. Real-world PS files often
omit it too, and the demuxer's EOF handling is exercised by exactly this shape
(the truncation-lane tests cover harder cases) — but as a FIXTURE it silently
encodes "no end marker" into every differential that uses it, without a doc note
saying so.
Why it matters: A consumer of the fixture cannot tell deliberate end-marker
omission from a truncated commit; the docs table (container-mpegps.md:52) does not
mention it.
What correct looks like: One doc-table note ("no B9 — muxer omits it; EOF is the
end signal") or a committed sibling with B9 for the end-marker lane.
Confidence: HIGH — Verification: python walker scanned for 0xB9 start code —
absent; whole-file coverage arithmetic (192 512/192 512) proves the last bytes are
PES payload, not a missing-code gap.

## 4. Per-function table (structural elements, part 2 = bytes 82 680..192 512)
| Element | Count in part 2 | Count in part 1 | Notes |
|---|---|---|---|
| Pack headers (0xBA) | 48 | 41 | 94 total — matches the "94 PES … packs" doc table row (container-mpegps.md:52 counts PES; packs verified here). |
| System headers (0xBB) | 1 | 1 | Standard dvd-muxer pair. |
| Video PES (0xE0) | 25 (payload 48 448 B; PTS 78 784→135 090, 16 stamped) | 39 (77 585 B; PTS 48 754→67 523, 6 stamped) | 68 total — pin EXACT. Part sums miss the boundary-straddling PES's tail (7 364 B counted once in part 1's walk only) — chunk artifact, not file damage: whole-file total 135 397 = pin. |
| DTS PES (0xBD, substream 0x88) | 22 (raw 44 189 B; PTS 56 434→136 114, all stamped) | 1 (2 016 B; PTS 48 754) | 24 total — pin EXACT. Audio is interleaved but back-loaded: 22 of 24 DTS PES sit in part 2's 109 KB — consistent with the 2 s DTS payload vs 25-frame video pacing. |
| Nav PES (0xBF) | 2 (@120 870: 980 B lead 0x00; @121 856: 1018 B lead 0x01) | 2 (@38: 980; @1024: 1018) | 4 total — sizes and lead bytes match the pinned test shape exactly (verified with correct private_2 parsing: payload at start-code+6, NO PES extension). |
| DTS sync (0x7FFE8001) | present (first PES of file, body offset 4) | — | Genuine 16-bit-mode DTS framing on substream 0x88 (documented DTS range 0x88..=0x8F, container-mpegps.md carriage table). |

## 5. Test honesty
| Test consuming this fixture | Would it fail if the fixture (or parser) were broken? | Reason |
|---|---|---|
| fixture_stream_tables_and_packet_counts_match_the_pins (ps_fixtures.rs) | YES | PES counts per track (4/68/24) asserted — my independent counts match. |
| fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit | YES | md5 of concatenated payload per track asserted — my independent md5s match both digests. |
| vob_nav_packets_are_real_and_zero_padded_by_ffmpeg | YES (shape) | Sizes 980/1018, lead bytes, zero-fill — all verified present in this file. |
| ps/tests.rs:523-532 lane (fuzz + open + walk) | YES (survival + walk) | Runs fuzz_demux_ps over these bytes; absence-of-panic contract. |
| apps/cli/src/containers/ps.rs:906-912 lane | YES | Asserts 24 dts packets and last PTS 136 114 through the REAL CLI open path — both match my parse. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "ps-vob-dts.vob: nav+mpeg2video+dts, 0+25+94 \| 4nav+68v+24dts" | docs/formats/container-mpegps.md:52 | REAL | My parse: 4/68/24 PES; ffprobe-side counts are documentary (no ffprobe in sandbox — UNVERIFIED but consistent with 94 packs/DTS-AU repacketization arithmetic: 48 128 B ÷ 512 B/AU ≈ 94). |
| "dts 48 128 B md5 bit-exact" | container-mpegps.md:52 + ps_fixtures.rs:138 | REAL | Independently recomputed md5 of stripped payload = the pinned digest, byte-identical. |
| "we expose 4 … Data packets (sizes 980/1018 alternating, measured)" | docs/worklog/archive-2026-10.md | REAL | 980/1018/980/1018 verified at offsets 38/1024/120870/121856. |
| "0x88..=0x8F→dts" carriage | container-mpegps.md / archive worklog | REAL | All 24 audio PES ride substream 0x88; DTS sync verified. |
| PTS spans (48 754→135 090 video; 48 754→136 114 dts) | ps_fixtures.rs:143-146 | REAL | Independently extracted from PES headers — exact match. |
| File-level provenance "ffmpeg 7.1.5 sha256-pinned fixture" (pattern claim) | container-mpegps.md:42 | PARTIAL | Payload-pinned, yes; file-sha256-pinned, no (F-001). |

## 7. Dead / duplicated / unreachable code
- Not applicable (binary asset). No orphan bytes (0/192 512 uncovered — verified).
- The fixture is shared evidence across three lanes (ac3/dts/lpcm carry the SAME
  video payload md5) — intentional reuse, not duplication.

## 8. Dependencies & flags
- Committed binary asset; no build involvement; consumed by integration tests
  (crates/demux/tests/ps_fixtures.rs), in-crate tests (ps/tests.rs), and the CLI
  open-path test (apps/cli/src/containers/ps.rs:908). Not covered by the xtask
  fixtures gate (see F-001).

## 9. Open questions for the next agent
- F-001's fix (a ps-fixture sha256 manifest) should cover all four files in
  fixtures/ps/ (ps-mp2-video-audio.mpg, ps-vob-ac3.vob, ps-vob-dts.vob,
  ps-vob-lpcm.vob) — A03's part-1 audit of this same file will want the same pin.
- The ffprobe-side counts (0+25+94) are documentary in this sandbox (no ffprobe
  binary); the orchestrator could re-run the differential with ffmpeg present if
  that column ever becomes load-bearing. REQUEST-ORCH-RUN: not needed — the
  payload-side pins are fully verified without it.
