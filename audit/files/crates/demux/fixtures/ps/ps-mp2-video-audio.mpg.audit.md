# AUDIT: crates/demux/fixtures/ps/ps-mp2-video-audio.mpg
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-21 |
| Lines / functions | 75 776 bytes (binary) / 50 top-level MPEG-PS items: 10 packs, 1 system header, 32 video PES (0xE0), 5 audio PES (0xC0), 2 padding PES (0xBE), 0 program-end codes |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (baseline: real `lumen` binary opens it — `apps/cli/tests/unsupported_path.rs:125-131` `mpegps_container_opens_and_accepts_the_session` … ok, nofailfast log line 3065; call chain: `lumen open` → input file → media-probe `mpegps` signature (file starts `00 00 01 BA`) → `apps/cli/src/containers/ps.rs` host → `demux::ps::PsDemuxer`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `ls/stat/file` (75 776 B, MPEG-1 system multiplex, matches manifest.tsv row 461); `git log --follow` (landed in 261b224c, card 5.I2); `rg -n "ps-mp2-video-audio"` (9 code sites, 4 docs); `python3` × 3 independent MPEG-PS walks (item tiling, PES counts, payload md5, PTS, pack tail-bits); `ffprobe -show_entries stream/packet` (identities, 25+39 packets, PTS extents); `ffmpeg -map -c copy` extractions piped to `md5sum`/`wc -c` (video 63 898 B / audio 8 150 B bit-exact); `ffmpeg -f s16le` (89 856 B = 44 928 samples = 39 × 1 152 exactly); baseline log greps (all 6 referencing suites green) |

## 1. One-paragraph verdict
This fixture is exactly what every pin in the repo says it is, and I verified every
pinned constant two independent ways (my own from-scratch MPEG-1 PS byte walk, and
re-derivation with the sandbox ffmpeg 7.1.5 — the same Debian build family that
authored it). Counts (32 video + 5 audio PES), payload byte totals (63 898 + 8 150),
both payload md5s, PTS extents (video 48 600→135 000; audio 47 618→118 149 vs
ffprobe-interpolated 136 957), ffprobe's 25v+39a re-packetization, the pack byte-7
`..01` ffmpeg marker divergence on all 10 packs, the unstamped-PES single 0x0F byte,
and the "4 of the 39 frames span a PES boundary" arithmetic (38×209 + 1×208 = 8 150;
all four inter-PES boundaries land mid-frame: offsets 156/103/50/194 mod 209) —
all TRUE. The 50 items tile the file perfectly: zero holes, zero unaccounted bytes,
EOF lands exactly on a packet boundary. The two findings are evidence-provenance
defects, not content defects: (1) the fixture's generation command is recorded
nowhere and the xtask fixture gate does not cover it; (2) no test anywhere pins
that this clean fixture yields zero damage events — the differential explicitly
drops `Recovered` events.

## 2. What actually works (verified)
- My own Python MPEG-1 PS walk (start-code scan, MPEG-1 pack 12 B, PES lengths,
  0xFF-stuffing + 0x0F/PTS flags): 32× 0xE0 (63 898 payload B, md5
  `b235981565a1b5f08783097bb9f40627`), 5× 0xC0 (8 150 B, md5
  `903e70465aecd25c6e5cf6c9f38b3daf`), first/last PTS video 48 600/135 000, audio
  47 618/118 149 — every value equals the pin in `crates/demux/tests/ps_fixtures.rs:77-99`.
- `ffmpeg -map 0:v:0 -c copy -f data -` → 63 898 B, md5 `b235981565a1b5f08783097bb9f40627`;
  `ffmpeg -map 0:a:0 -c copy -f mp2 -` → 8 150 B, md5 `903e70465aecd25c6e5cf6c9f38b3daf`
  — the docs' "bit-exact" claim is literally true, re-derived live.
- `ffprobe -show_packets`: 25 video + 39 audio packets; video PTS 48 600→135 000,
  audio 47 618→136 957 — matches the pinned ffprobe columns and the documented
  "ffprobe interpolates unstamped MP2 AUs to 136 957" divergence.
- `ffmpeg -map 0:a:0 -f s16le -` → 89 856 B = 44 928 samples = 39 frames × 1 152
  (Layer II) — the CLI lane's decode-count claim (`apps/cli/src/containers/ps.rs:758-762`)
  re-derived exactly.
- Pack tail bits: all 10 packs have `pack[11] & 0x03 == 0b01` — the ffmpeg-vs-ISO
  marker divergence the unit test claims was "measured" (`crates/demux/src/ps/tests.rs:78-83`)
  is real on every pack of the real file.
- Item tiling: `python3` walk proves items begin exactly where the previous ends,
  last padding PES ends at byte 75 776 == file size; no holes, no orphan bytes,
  no embedded foreign content. (An earlier pass of my own script had a
  length/SID index bug that fake-produced "1 819 trailing 0xFF bytes" — re-run
  with correct arithmetic dissolves it; those bytes are the interior of the final
  padding_stream packet. Recorded to preempt a future auditor's false positive.)
- Baseline (cited, not re-run): all six referencing suites green in
  `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` — ps_fixtures
  4/4 (log 2176-2183), ps unit incl. both fuzz-lite tests (log 1835-1860),
  containers::ps 4/4 (log 2830-2833), mpeg12_corpus 3/3 (log 1142-1148),
  registry_roundtrip 3/3 (log 3422-3428), unsupported_path 16/16 (log 3029-3046,
  real spawned `lumen` binary, exit 0, "ok: accepted session=1").

## 3. Findings
### F-001 [S3]
Where: crates/demux/fixtures/ps/ps-mp2-video-audio.mpg (asset) + tools/xtask/src/cmd/fixtures.rs:115 + docs/worklog/archive-2026-10.md:9162
Code:
```
/// The committed fixture tree root (relative to the workspace root).
const FIXTURES_SUBDIR: &str = "tests/fixtures";
```
```
- Inherited uncommitted: crates/demux/src/ps/{mod,error,fuzz,test_support}.rs (1.6 kloc) + 4 ffmpeg fixtures (mpg + ac3/dts/lpcm VOBs) — but NO tests.rs ...
```
What is wrong: The fixture's ffmpeg generation command is recorded nowhere in the
repo, and the repo's own fixture-integrity gate excludes it. `cargo xtask fixtures
--check` (baseline gate-fixtures___check.log.txt: "result: no drift — 55 file(s)
verified under /home/z/my-project/audit-workspace/tests/fixtures") covers only
`tests/fixtures`; `crates/demux/fixtures/ps/` is outside its scope. The worklog
provenance is worse than thin: the 4 ps fixtures were *inherited uncommitted* from
a lost prior session (archive-2026-10.md:9162) — their authoring session is not in
the record at all, and no `ffmpeg …` command line for them exists anywhere
(searched scripts/, tools/xtask, docs/worklog, docs/formats; contrast the AVI
heavy fixture whose command IS documented at archive-2026-10.md:7842 and the IVF
fixtures at 9239).
Why it matters: 30+ pinned constants across 6 test suites (counts, md5s, PTS
extents, span-end ticks, nav shapes) bind to these exact bytes. If the blob is
ever lost, corrupted, or needs extending (e.g., a new exact-delta lane), it cannot
be regenerated deterministically — only luck with the identical ffmpeg 7.1.5
Debian build reproduces it. Reproducibility is the entire value proposition of a
committed differential fixture.
What correct looks like: a generation recipe (the ffmpeg command, e.g. lavfi
testsrc2+sine → mpeg2video+mp2 → `-f mpeg`) committed next to the fixtures (a
`fixtures/ps/README.md` or an xtask recipe) and either extending
`xtask fixtures --check` to cover `crates/demux/fixtures/` or recording why it
cannot (ffmpeg version pinning).
Confidence: HIGH — Verification: read (rg over scripts/, tools/, docs/; xtask
fixtures.rs source; gate log) + ran (ffmpeg re-derivation confirms ffmpeg 7.1.5
authorship empirically: `-f data`/`-f mp2` extractions bit-exact).

### F-002 [S3]
Where: crates/demux/tests/ps_fixtures.rs:213-215 (consumer of this fixture)
Code:
```
        match ps.next_event().must() {
            DemuxEvent::Packet(p) => packets.push((p.track.handle(), p.data.to_vec())),
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => break,
```
What is wrong: No test anywhere pins that demuxing this *clean* fixture emits ZERO
damage events. The differential (`ps_fixtures.rs`) drops `Recovered`/`Discontinuity`
on the floor, and the only other clean-fixture test
(`ps::tests::fuzz_lite_clean_fixtures_drain_without_terminating_errors`, line 519-537)
asserts only `terminal == None` — a `Recovered` event is not terminal. My own byte
walk shows the fixture is perfectly tiled with zero damage sources (no resyncs, no
skipped stream ids except positional padding, EOF exactly at a packet boundary and
no program-end code, so the `walk == length` path at ps/mod.rs:976-981 fires with
`self.walk < length` false → no trailing-garbage note), so "zero damage" is the
expected truth — but it is asserted by nothing.
Why it matters: Mutation reasoning — if the walk regressed to damage-note the
clean EOF/padding handling (e.g., noting trailing-garbage whenever the program-end
code is absent, which this fixture lacks), every committed suite still passes and
users of every clean ffmpeg-authored MPEG-PS file get spurious
`Recovered{skipped, "trailing-garbage"}` events. The fixture is the natural place
to pin "clean ⇒ zero events" and that pin is missing.
What correct looks like: one assertion in the differential's `run_lane` (or the
clean-fixture unit test): collect `Recovered`/`Discontinuity` events and
`assert!(events.is_empty())` for all four committed fixtures.
Confidence: HIGH — Verification: read (both test files, ps/mod.rs EOF path) +
my independent walk proving the zero-damage expectation; the demuxer itself not
executed by me (baseline ran it: 4/4 + fuzz-lite green, but no assertion examines
events) — the *gap*, not the behavior, is the finding.

## 4. Per-item table (binary asset — every top-level item class; no functions exist)
| Item class | Count | Payload bytes | Verdict | Notes |
|---|---|---|---|---|
| MPEG-1 pack (00 00 01 BA, 12 B) | 10 | — | REAL | All 10: SCR 0→115 799, mux rate 2 647 751 (×50 B/s), tail bits `01` (ffmpeg divergence, pinned by ps/tests.rs:78-91); rate value itself pinned by no test (harmless) |
| System header (00 00 01 BB) | 1 | 12 B body | REAL | At offset 12, length 12; skipped positionally, covered by unit test `padding_stream_and_system_header_are_skipped_positionally` (baseline ok) |
| Video PES (0xE0) | 32 | 63 898 | REAL | 20/32 PTS-stamped (48 600…135 000); ES starts `00 00 01 B3` (sequence header); bit-exact vs ffmpeg `-c copy`; first PES 2 012 B, rest 2 042/2 030/922 |
| Audio PES (0xC0) | 5 | 8 150 | REAL | 4/5 stamped (47 618…118 149); PES 5 is the 15-byte unstamped (single 0x0F byte) tail fragment; 39 Layer II frames, 4 spanning boundaries (arithmetic closes exactly) |
| Padding stream (0xBE) | 2 | 3 094 (skipped) | REAL | At 68 512 (1 114 B) and 73 761 (2 009 B); second one ends exactly at EOF 75 776 |
| Program end (00 00 01 B9) | 0 | — | — | Absent; EOF-at-boundary path terminates the walk — undocumented shape fact (see F-002) |

## 5. Test honesty (suites consuming this fixture — baseline green, cited)
| Test | Would it fail if the fixture (or walk) were broken? | Reason |
|---|---|---|
| ps_fixtures `fixture_stream_tables_and_packet_counts_match_the_pins` | YES | Counts + PTS extents pinned numerically; any byte change altering PES count or stamps fails it |
| ps_fixtures `fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit` | YES | md5 over concatenated payload — I reproduced both digests independently; single-byte flip fails |
| ps::tests `fuzz_lite_mutations_of_the_committed_fixtures_never_panic` | WEAK | No-panic + typed-error only (120 rounds, 8 byte-flips into first 512 B); outcome-blind by design |
| ps::tests `fuzz_lite_clean_fixtures_drain_without_terminating_errors` | PARTIAL | Asserts terminal==None only; damage events unpinned (F-002) |
| containers::ps `mpg_lane_demuxes_splits_and_decodes_mp2_exactly` | YES | Span-end ticks 135 000/118 149, AU split 39, decode 44 928 samples — all re-derived by me |
| containers::ps `start_time_skips_packets_with_slow_linear_seekability` | YES | Per-track packet survival after 0.6 s skip pinned (2+3 drop) |
| unsupported_path `mpegps_container_opens_and_accepts_the_session` | YES | Real binary, exit 0 + "accepted session=1"; broken probe/demux fails it |
| mpeg12_corpus `fixture_transport_lanes_ts_and_ps` | YES | Field assertions + `bad_headers == 0` + ffprobe differential (ffmpeg-gated at runtime) |
| registry_roundtrip `every_landed_container_probes_to_its_format_id` | YES | File must probe to `mpegps` through the real signature registry |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "mpeg2video+mp2, 25v+39a \| 32v+5a \| video 63 898 B + mp2 8 150 B md5 == ffmpeg `-c copy` bit-exact" | docs/formats/container-mpegps.md:50 | REAL | My walk + live ffmpeg extraction (§2) |
| "5 audio PES carry 8 150 payload bytes = exactly 39 Layer II frames, and 4 of the 39 frames span a PES [boundary]" | apps/cli/src/containers/ps.rs:25-27 | REAL | 38×209 + 1×208 = 8 150; boundary offsets 156/103/50/194 (mod 209) all mid-frame; 39×1 152 = 44 928 samples = 89 856 s16le B (live) |
| "byte 7 of every pack in ps-mp2-video-audio.mpg is `..01`" | crates/demux/src/ps/tests.rs:81-82 | REAL | All 10 packs: `pack[11] & 0x03 == 0b01` (my walk) |
| "mpg audio 118149 vs ffprobe-interpolated 136957" | archive-2026-10.md:9166; ps_fixtures.rs:96-98 | REAL | ffprobe live: audio last PTS 136 957; last stamped PES 118 149 |
| "no corpus files (verified) — crates/demux/fixtures/ps/{…}" | docs/plans/open-path-wiring.md:333 | REAL | rg: fixture referenced only by the 9 code sites + docs listed here; no tests/fixtures corpus entry |

## 7. Dead / duplicated / unreachable code
Not applicable (binary asset). The asset itself is referenced (9 code sites, 4 docs,
manifest.tsv row 461: 75 776 B — size matches `stat`); not an orphan. Sibling
fixtures ps-vob-{ac3,dts,lpcm}.vob are outside this assignment.

## 8. Dependencies & flags
None: plain committed bytes, no feature gates. Consumed cross-crate via relative
paths (`crates/media-probe/tests/registry_roundtrip.rs:147`
`"../demux/fixtures/ps/ps-mp2-video-audio.mpg"`, and
`apps/cli/tests/unsupported_path.rs:128` via `../../crates/demux/…`) — read-only,
no build-time dependency; sandbox ffmpeg 7.1.5 present for oracle re-derivation.

## 9. Open questions for the next agent
- F-002's pin ("clean fixture ⇒ zero Recovered events") belongs naturally in
  ps_fixtures.rs — whoever audits `crates/demux/src/ps/mod.rs` fully should confirm
  no other site already asserts it (I found none).
- The VOB siblings carry the same unrecorded-generation-command defect as F-001;
  their auditor should cite this file rather than re-derive the provenance search.
