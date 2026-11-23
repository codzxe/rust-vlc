# AUDIT: crates/demux/src/ogg/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Lines / functions | 1476 / 44 (+5 unit tests, 1 doctest) |
| File verdict | REAL (with 2 measured S2 perf defects on hostile input) |
| Reachable from shipped entry point? | YES: lumen-cli `containers/mod.rs:68 "ogg" => ogg::open` -> `apps/cli/src/containers/ogg.rs:81 OggDemuxer::new` -> this file |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 3 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 ogg::` (31 passed, exit 0); `ffmpeg`-generated libtheora/vorbis/opus corpus + `LUMEN_CORPUS=/tmp/w04corpus/media ./target/debug/deps/corpus_ogg-23921e099139d244 --test-threads=1 corpus_oggs_survive_the_fuzz_harness` (ok, 14.76s); adversarial timing runs (below); python3 CRC + Theora-header bit-level verification (below) |

## 1. One-paragraph verdict
This is a genuinely implemented Ogg demuxer: page framing, CRC gating, segment/lacing packet assembly, chaining, granule-hole accounting and seek all do real work, and I executed them against real ffmpeg-authored Ogg bytes plus hostile crafted files. The page/CRC math is correct (CRC variant independently recomputed; Theora header offsets empirically confirmed against a fresh libtheora encode). The two real defects are CPU blowups reachable from a shipped entry point: (1) the seek scan (`scan_landing`) re-reads and re-parses page headers byte-by-byte across the whole file with no bound on failed parses — measured 15.86 s (debug) for an 8 MiB file with a garbage tail; (2) the damage-recovery scan (`recover_sync`) CRC-verifies a full ≤65 KiB page at every 4-byte `OggS` occurrence — measured 10.2 s (debug) for 4 MiB of dense `OggS` spam. Neither the 200k-page seek cap nor the 4 MiB recovery budget bounds CPU. Additionally, state is insert-only: chained links mint tracks and stream state that are never freed (unbounded resident growth), and legal >256 KiB continued packets kill the whole demux with a sticky error.

## 2. What actually works (verified)
- All 31 prebuilt ogg unit tests pass: `./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 ogg::` -> `test result: ok. 31 passed; 0 failed` (binary newer than source).
- CRC-32 variant independently recomputed in python3 (poly 0x04C11DB7, MSB-first, init 0, no xorout): `crc32(b"123456789") == 0x89a1897f` matches page.rs:292.
- Theora identification offsets verified against a **fresh ffmpeg 7.1.5 libtheora encode** (`rgbtestsrc 240x180@25` -> `.ogv`): BOS packet is exactly 42 bytes; FRN BE32@22 = 25, FRD BE32@26 = 1 (ffprobe says 25/1); KFGSHIFT occupies bits 326–330 = exactly `(b40&3)<<3 | b41>>5` = 6, and EOS granule 9281 decodes to 146 frames = 5.840 s = ffprobe duration. The unit-test fixture's tail `00 03 0d 40 00 c0` is byte-identical to the real encoder's output.
- Fuzz harness over real+adversarial corpus (4 real lanes + 4 crafted hostile files, truncation+flip sweeps): no panic, no hang, exit 0 (timings in F-001/F-002).
- Vorbis LE24 rate @12..15 verified: my generated 8 kHz vorbis file parses rate 8000 through `identify`.

## 3. Findings

F-001 [S2]
Where: crates/demux/src/ogg/mod.rs:1217-1230 (scan_landing loop)
Code:
```rust
        while at + page::HEADER_BYTES as u64 <= len {
            let head27 = self.reader.read_prefix_at(page::HEADER_BYTES, at)?;
            if head27.len() < page::HEADER_BYTES {
                break;
            }
            let head = self.read_full_head(at, head27[26])?;
            let Ok(hdr) = page::parse_header(&head, at, len) else {
                at += 1;
                continue;
            };
            pages += 1;
```
What is wrong: on every byte where a page header does not parse (garbage, non-`OggS` capture, bad version) the scan advances by ONE byte, and each step performs `read_prefix_at(27)` **plus** `read_full_head` (up to 27+255=282 bytes) — two heap `Vec` allocations, two seeks, two reads (riff.rs:307 `read_prefix_at` allocates per call). `MAX_SEEK_SCAN_PAGES` (mod.rs:107, "the SlowLinear honesty bound") only counts *successfully parsed* pages, so the failure path is entirely unbounded: O(len × ~309 bytes) per `seek`/`reanchor` call. The loop also never exits early after the landing granule is passed, so even a valid landing found at byte 500 forces a full-file walk. Why it matters: `seek` is on the shipped CLI open path (`--start-time` via the pure seek query, apps/cli/src/containers/ogg.rs); a 1 GiB `.ogg` with a garbage tail (truncated download, appended ID3) stalls minutes per seek. Measured: 8 MiB file (valid vorbis head + 8 MiB zero tail) through the fuzz harness's 3 reanchor probes = **15.86 s** wall (debug build, in-memory source); clean 4-lane baseline = 0.01 s. What correct looks like: check the capture pattern on `head27` before reading the segment table; resync only on `OggS` occurrences (`find_capture`); count *bytes scanned* against a budget; break when `hdr.granule > target_granule` for the reference serial. Confidence: HIGH — Verification: ran (timing above) + read.
Confidence: HIGH — Verification: ran

F-002 [S2]
Where: crates/demux/src/ogg/mod.rs:680-699 (recover_sync candidate CRC) with budget at 663-664
Code:
```rust
                let head = self.read_full_head(pos, head27[26])?;
                if let Ok(hdr) = page::parse_header(&head, pos, len) {
                    let head_len = page::HEADER_BYTES + hdr.segments.len();
                    let mut whole = Vec::with_capacity(head_len + hdr.body_len);
                    ...
                    if page::crc32(&whole) == hdr.crc {
                        candidate = Some(pos);
                        break;
                    }
```
What is wrong: every 4-byte-aligned `OggS` inside the recovery window gets a full candidate page read + CRC over up to 27+255+65025 bytes before being rejected. The bound `MAX_RECOVERY_SCAN_BYTES` (4 MiB, mod.rs:99) caps *bytes skipped*, not CPU: worst case is ~1M candidates × 65 KiB CRC ≈ 65 GiB of byte processing per recovery event, and recovery is re-entered every time the walk loses capture (unbounded number of events per file). Measured: 4 MiB of dense `OggS` (vorbis head pages + `b"OggS"×1M`, each candidate parsing with nsegs=0x53) through one fuzz pass = **10.2 s** wall (debug), vs 0.04 s for the same-size page-aligned CRC-bad file. The module doc (lines 40-48) and fuzz.rs (lines 26-30) claim bounded recovery "never a whole-file loop" — bytes yes, CPU no. What correct looks like: cap candidate CRC work (e.g. total bytes CRC'd per recovery), or require the capture to be followed by a plausible page before reading the body. Confidence: HIGH — Verification: ran (timing above).
Confidence: HIGH — Verification: ran

F-003 [S3]
Where: crates/demux/src/ogg/mod.rs:924-933 (open_stream) + 1073-1121 (mint_track)
Code:
```rust
    fn open_stream(&mut self, serial: u32) {
        let track = TrackId::new(self.next_track);
        let stream = StreamId::new(self.next_track);
        self.next_track += 1;
        if !self.streams.contains_key(&serial) {
            self.serial_order.push(serial);
        }
        self.streams
            .insert(serial, LogicalStream::new(serial, self.link, track, stream));
```
What is wrong: `streams` is insert-only (the code's own comments call it "the map is insert-only") and `tracks` is append-only with no pruning of ended links and no steady-state page budget — `MAX_OPEN_PAGES` (10k) bounds only `open()`, not `next_event`'s walk. A chained file (or a file spamming BOS pages reusing one serial, ~29 bytes each) mints one `Track` (heap strings + TrackTiming) plus one `LogicalStream` (headers Vec, up to 256 KiB stashed headers) per chain, all retained for the demuxer's lifetime: memory grows superlinearly with a crafted file and linearly-with-overhead with a legitimately long chain. Not a crash, but an unbounded-resident-state defect the open-path budget was written to prevent. What correct looks like: drop (or cap) state for links whose streams have ended; bound track minting in the steady state. Confidence: MEDIUM — Verification: read (growth path traced; not heap-profiled — UNVERIFIED amplification factor).
Confidence: MEDIUM — Verification: read

F-004 [S3]
Where: crates/demux/src/ogg/mod.rs:876-906 (overrun -> sticky fatal) and 85-90 (the policy)
Code:
```rust
                        if acc.len() + run_len > MAX_PACKET_BYTES as usize {
                            overrun = Some((acc.len() + run_len) as u64);
```
What is wrong: `MAX_PACKET_BYTES = 256 KiB`; a continued packet exceeding it returns `error::packet_overrun` through `self.fail()` — a **sticky** error that permanently kills the demuxer. A single Ogg packet may legally be arbitrarily large (255-run lacing across many pages): high-bitrate Theora keyframes and long-format FLAC-in-Ogg packets can exceed 256 KiB, and a valid file then hard-fails instead of degrading. The `[PROPOSAL]` doc comment admits the tradeoff, but sticky-fatal on valid input is the wrong severity — damage-noting and dropping the packet (the file's own stated damage policy, lines 40-48) would preserve the rest of the stream. Confidence: MEDIUM — Verification: read (no >256 KiB-packet fixture available to run; UNVERIFIED against a real encoder).
Confidence: MEDIUM — Verification: read

F-005 [S4]
Where: crates/demux/src/ogg/mod.rs:248-262 (identify)
Code:
```rust
    if first_packet.starts_with(&[0x01, b'v', b'o', b'r', b'b', b'i', b's'])
        && first_packet.len() >= 15
    {
```
What is wrong: neither Vorbis (version must be 0, bytes 7..11) nor Opus (version must be 1, byte 8 per RFC 7845 §5.1) is version-checked; an OpusHead with version 9 still yields `Clock::Opus48000` and a "opus" track. Opaque-tolerant by design, but the clock is then taken from an unvalidated header. Blast radius: misidentified codec name/clock on nonconforming streams. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

F-006 [S4]
Where: crates/demux/src/ogg/mod.rs:50-61 (zero-copy framing doc)
Code:
```rust
//! pool) — zero copy, zero alloc. Only cross-page (continued) packets
//! copy: their runs concatenate into one buffer before the pool
//! adopts it (`from_vec`), and header packets stash one shared view
//! each. Steady-state allocations per page: the CRC scratch
//! (header-sized, reusable) and the segment-table `Vec` — the
```
What is wrong: the "CRC scratch (header-sized, reusable)" does not exist. `read_page` (line 628) allocates a fresh `Vec::with_capacity(head_len + header.body_len)` (up to ~65 KiB, i.e. body-sized, not header-sized) per page and drops it; `recover_sync` (line 683) allocates another. The doc's steady-state allocation claim is false in both size and lifetime. (The 5.X2 note at line 59-61 admits the extension, but describes the current state wrong.) Confidence: HIGH — Verification: read (lines 628, 683).
Confidence: HIGH — Verification: read

F-007 [S4]
Where: crates/demux/src/ogg/mod.rs:211-213
Code:
```rust
            Clock::Theora { num, den, .. } => {
                TimeBase::new(den, num).unwrap_or(TimeBase { num: 1, den: 1_000 })
            }
```
What is wrong: dead fallback arm — `num`/`den` are validated `> 0` at construction (line 273), so `TimeBase::new` cannot refuse nonzero coprime inputs; the `unwrap_or` masks a would-be programming error as a silent 1 ms timebase. The comment admits it ("the dead zero arm"). Same pattern at 1092-1095. Hygiene: assert or restructure instead of a silent wrong clock. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

F-008 [S3]
Where: crates/demux/src/ogg/mod.rs:1245-1247 (scan_landing terminal) with 1232-1236
Code:
```rust
        let Some((granule, page_at)) = landing else {
            return Err(error::seek_out_of_range());
        };
```
What is wrong: `reanchor`/`seek` to time 0 fails with `seek_out_of_range` when the file's first non-negative reference granule is > 0 and no page carries granule ≤ 0 (landing is only assigned inside `if hdr.granule <= target_granule`, and `prev_granule` starts at 0 but is only *assigned* to `landing` inside that arm — a page with granule 0 never appears). Files whose header pages carry granulepos −1 throughout hit this: seek-to-start errors instead of landing at offset 0. Edge case on real muxers (ffmpeg writes granule 0 on header pages), but the failure mode is a typed refusal on a legal query. Confidence: MEDIUM — Verification: read + traced; not executed against such a fixture (UNVERIFIED by run).
Confidence: MEDIUM — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe | 120-122 | REAL | — | 4-byte capture sniff; used by registry |
| OggCodec::descriptor | 150-162 | REAL | — | static names; typed err arm dead |
| OggCodec::is_header_packet | 166-178 | REAL | — | magic prefixes; FLAC 0x00 0x22 mapping included |
| Clock::base | 197-215 | REAL | S4 | dead unwrap_or fallback (F-007) |
| Clock::nanos | 221-235 | REAL | — | u128 math, no overflow; None on negative |
| Clock::granule_of_nanos | 239-243 | REAL | — | inverse; verified in unit test + my run |
| identify | 247-295 | REAL | S4 | no version checks (F-005); Theora offsets empirically verified |
| be32 | 297-302 | REAL | — | bounds guaranteed by len>=42 check |
| flac_streaminfo_rate | 307-314 | REAL | — | 20-bit rate @14..16, len-checked |
| LogicalStream::new | 351-369 | REAL | — | plain init |
| StickyShape::of / mint | 381-392 | REAL | — | replayable sticky error, cloned |
| OggDemuxer::new | 462-482 | REAL | — | pool 2×256 KiB; reader via SourceReader |
| tracks/links/serial_numbers | 489-504 | REAL | — | observability accessors |
| header_packets | 509-515 | REAL | — | opaque passthrough seam |
| granule_discontinuities/damage_total/serial_link/packet_counts/pool_bytes | 517-552 | REAL | — | counters |
| track_duration | 558-568 | REAL | — | max(walk, tail probe); None without clock |
| fail | 572-575 | REAL | — | sticky set |
| read_page | 581-653 | REAL | S4 | fresh ~65 KiB CRC Vec per page (F-006); truncation fatal vs other damage recovered — typed split verified |
| recover_sync | 658-728 | REAL | S2 | candidate CRC amplification (F-002); budget enforced on bytes only |
| read_full_head | 731-734 | REAL | — | 27+nsegs ≤ 282 bytes, bounded |
| queue_recovered | 736-741 | REAL | — | skips 0-byte notes |
| step | 745-920 | REAL | S3 | chain/BOS-reuse mapping, granule holes, orphan runs, overrun sticky (F-004) |
| open_stream | 924-933 | REAL | S3 | insert-only, never freed (F-003) |
| complete_packet | 937-1069 | REAL | — | identify/FLAC clock/header split/stamping; verified vs unit tests + real files |
| mint_track | 1073-1121 | REAL | S3 | append-only tracks (F-003) |
| probe_tail | 1125-1154 | REAL | — | 128 KiB window; slice-guarded by parse_header length check |
| open | 1159-1182 | REAL | — | 10k-page budget; no_streams typed |
| scan_landing | 1188-1254 | REAL | S2 | per-byte rescan, unbounded on garbage (F-001); seek-to-0 edge (F-008) |
| reference_stream | 1257-1263 | REAL | — | first clock-bearing serial |
| stash_header | 1273-1282 | REAL | — | 256 KiB budget, damage-noted |
| find_capture | 1285-1287 | REAL | — | windows(4) |
| nanos_to_stamp | 1291-1298 | REAL | — | u128, clamped |
| Demuxer::start | 1301-1312 | REAL | — | sticky replay; clones tracks |
| Demuxer::next_event | 1314-1342 | REAL | — | drains pending, EOS terminal |
| Demuxer::seek | 1344-1351 | REAL | S2 | pure-query claim holds positionally; cost is F-001 |
| Demuxer::seekability | 1353-1359 | REAL | — | SlowLinear honest |
| Demuxer::reanchor | 1361-1380 | REAL | S2 | commits walk, clears pending, discontinuity mark; cost F-001 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| clock_unit::clock_conversions_are_exact | YES | exact nanos/granule round trips incl. Theora shift math |
| clock_unit::identify_parses_the_magic_set | YES | magic->codec/clock pairs; ran green |
| clock_unit::theora_header_offsets_pin_the_corpus_layout | YES (partially) | asserts FRN/FRD/KFG on a hand-typed array; I independently confirmed the array matches real libtheora output (tail bytes identical, fps/shift verified on a fresh encode) |
| clock_unit::flac_streaminfo_rate_bits | YES | 20-bit field decode |
| clock_unit::find_capture_ignores_short_tails | YES | trivial but real |
| ogg::tests (26 tests, ran green) | YES | damage/chaining/seek/overrun behaviors each pinned (bad CRC drop, chain boundary, continued lies, granule regression, −1 hole, mux, seek landing, sequence gap, pool bounds, truncated page, unknown codec, unterminated lacing) |
| module doctest (406-422) | YES | constructs from demo_stream, asserts 1 track + first packet |
| tests/corpus_ogg.rs differential | NO — NEVER RUNS in baseline | corpus dir `/tmp/my-project/corpus-v4/media` absent; baseline `corpus_oggs_match_the_ffmpeg_truth ... finished in 0.01s` = the loud-skip path; the md5 differential asserts nothing in this tree (see that file's audit) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bounded sync-recovery scan (≤ MAX_RECOVERY_SCAN_BYTES) ... never a whole-file loop" | mod.rs:40-48, 97-99 | HALF-TRUE | bytes bounded, CPU unbounded — F-002 measurement |
| "Steady-state allocations per page: the CRC scratch (header-sized, reusable)" | mod.rs:58-61 | FALSE | fresh body-sized Vec per page at 628/683 — F-006 |
| "stamps only the last-completing packet of each granule-bearing page" | mod.rs:20-31 | REAL | complete_packet:1027-1034 `owns_granule` from `last_terminal` (851-855); verified by unit test only_the_last_packet... (ran green) |
| "Theora ... offsets pinned empirically against libtheora-encoded fixtures" | mod.rs:266-269 | REAL | re-verified on fresh ffmpeg 7.1.5 encode (see §2) |
| "seek/reanchor are honest SlowLinear page-header scans" | mod.rs:401-402 | SHALLOW | "honest" about mechanism, dishonest about cost: no bound on the non-page path (F-001) |

## 7. Dead / duplicated / unreachable code
- `Clock::base` Theora `unwrap_or` fallback (211-213) and mint_track's 1/1000 fallback (1092-1095): dead arms by the code's own comments.
- `OggCodec::descriptor`'s `map_err` (161): unreachable on static ASCII names.
- No unreachable!()/todo!() in this file. All pub API reachable from tests/CLI observability seams.

## 8. Dependencies & flags
Depends on `crate::avi::riff::{PayloadSlots, SourceReader}` and `crate::avi::header::DamageLog` (cross-module reuse, not a layering violation — documented). `plugin_api`, `media_types` (TimeBase/Timestamp/Stamp). No feature gates in this file; release lane builds it (baseline cargo-build-release-lumen-cli.txt exit 0).

## 9. Open questions for the next agent
- Measure heap amplification of F-003 (BOS-spam chain file) — I traced the growth but did not heap-profile (cargo-build-budget).
- Does any real encoder emit continued packets > 256 KiB (F-004)? A corpus lane with `-b:v` high theora would settle it.
- ogg/fuzz.rs FUZZ_EVENT_BUDGET=500k: the seek probes (reanchor) are where F-001 bites the harness; a crafted 100 MB file inside the harness budget would take minutes — next agent may quantify release-build cost.
- `crc_valid` in page.rs has no non-test caller in ogg (used by fuzz/other demuxers?) — page.rs is another agent's file.
