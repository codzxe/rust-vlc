# AUDIT: crates/demux/src/ps/mod.rs
| Field | Value |
|---|---|
| Auditor | W05-A03 |
| Date | 2026-11-23 |
| Lines / functions | 1219 / 34 (incl. trait impl) |
| File verdict | REAL (one S2 arithmetic defect on malformed MPEG-1 PES; core walk verified by execution) |
| Reachable from shipped entry point? | YES — `lumen open` → apps/cli/src/containers/mod.rs:71 `"mpegps" => ps::open(stream, opts)` → apps/cli/src/containers/ps.rs:144 `PsDemuxer::new(stream)`; probe mirrored in crates/media-probe/src/signature/phase5.rs:186 |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 5 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b ps:: --test-threads=2` → 28 passed / 0 failed, exit 0 (prebuilt binary, mtime 18:08 > source 17:35). Baseline citations: audit/runs/baseline/cargo-test-workspace.txt:2389-2397 (ps_fixtures 4/4 ok), :2474 (doctest ps/mod.rs line 296 ok) |

## 1. One-paragraph verdict
The MPEG-PS core is a genuine, reachable, heavily-pinned demuxer: pack/PES header walks, private-1 AC-3/DTS/LPCM/SPU routing with per-kind strip offsets, nav-pack Data tracks, bounded resync, pooled payloads — all verified by 28 passing unit tests I ran plus the fixture suite in the orchestrator baseline. SCR/stamp bit arithmetic was hand-checked against ISO 11172-1/13818-1 bit budgets and matches (33-bit budgets exact; MPEG-2 `3/15/15` split correct; mux-rate fields read from the right bytes). The hostile focus areas hold up: pack/system-header bounds are checked (`end > length` at :483, `want > declared_len` at :533), LPCM/AC3/DTS strips (7/4/4) are pinned by tests + fixtures, and SCR math is correct. The one real defect: the MPEG-1 PES lane omits the header-vs-declared-length guard the MPEG-2 lane has, so a malformed MPEG-1 PES underflows `payload_len` — debug panic, release session death via `payload-too-large`. Secondary: fabricated PTS/DTS from zero bytes when MPEG-2 `header_data_length` is shorter than the flags claim, all 0xBD stamps routed to the first private stream, one lost `Recovered` event, and dead width checks.

## 2. What actually works (verified)
- `./target/debug/deps/demux-5e0c7194d741938b ps:: --test-threads=2` → `test result: ok. 28 passed; 0 failed` — covers pack round-trips both flavors, ffmpeg-delta pins (0x0F no-stamp byte, 3-byte AC-3 filler, never-enforced markers), private-1 routing for all four kinds, nav Data track, padding/system-header positional skip, every typed error path, resync, seek/reanchor, fuzz-lite over committed fixtures.
- Baseline cargo-test-workspace.txt:2392-2395: `fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit ... ok`, `lpcm_packets_strip_the_full_dvd_prefix ... ok`, `vob_nav_packets_are_real_and_zero_padded_by_ffmpeg ... ok` (fixtures ps-vob-lpcm/ac3/dts.vob, digests pre-verified Wave 4).
- SCR math verified by hand: MPEG-2 extraction sums 3+2+8+5+2+8+5 = 33 bits at correct offsets (matches test_support::mpeg2_pack encoder bit-for-bit); MPEG-1 mux-rate `(buf[9]<<14)|(buf[10]<<6)|(buf[11]>>2)` matches the '0010'+33+3 markers layout with 22-bit rate ending at buf[11] bit 2.

## 3. Findings
F-001 [S2]
Where: crates/demux/src/ps/mod.rs:558-610
Code:
```
        } else {
            let mut buf = [0u8; MAX_MPEG1_STUFFING + 10];
            let got_h = self.reader.read_prefix_into(&mut buf, pos + 6)?;
            let mut at = 0usize;
```
… and the unguarded subtraction:
```
        let data_start = pos + 6 + header_len;
        let payload_len = u64::from(declared_len) - header_len;
```
What is wrong: the MPEG-2 lane guards `if want > usize::from(declared_len)` (:532-536) but the MPEG-1 lane never compares its header extent (`at`, up to 16 stuffing + 10 stamp bytes) against `declared_len`. A legal-prefix MPEG-1 PES with `declared_len=2` and a 5-byte PTS header reaches :610 with `2 - 5`. Why it matters: debug builds panic ("attempt to subtract with overflow") — the fuzz-lite harness would crash on such input, and `cargo test` runs debug; the shipped release build wraps, and the wrapped `payload_len` (≈2^64) then fails `want > MAX_PACKET_BYTES` in `emit_pes_packet` (:890), returning `payload-too-large` — one malformed packet kills the whole session in a demuxer whose design goal is damage-noted positional skipping. The release scan walk position is accidentally still correct (the wrap cancels: `data_start + payload_len ≡ pos+6+declared_len`), but the event walk dies whenever the stream id is registered. The committed seeded fuzz (`fuzz_lite_mutations...`, fixed LCG seed, first 512 bytes only) does not hit this shape — it passed. What correct looks like: mirror the MPEG-2 guard — after the MPEG-1 header parse, `if at > usize::from(declared_len) { return Err(error::bad_pes_header(pos)); }`.
Confidence: HIGH — Verification: read (arithmetic traced; no executable repro possible without editing sources — parse_pes_full is private; cargo-build-budget for a scratch harness).

F-002 [S3]
Where: crates/demux/src/ps/mod.rs:549-556
Code:
```
            match buf[1] >> 6 {
                0b10 => p = Some(parse_stamp(&buf[3..8], pos)?),
                0b11 => {
                    p = Some(parse_stamp(&buf[3..8], pos)?);
                    d = Some(parse_stamp(&buf[8..13], pos)?);
                }
                _ => {}
            }
```
What is wrong: `hl` (buf[2], header_data_length) is never checked against 5/10 before slicing. With `hl=0` and the PTS flag set, `want=3` passes the `want > declared_len` guard for any `declared_len ≥ 3`, only `buf[0..3]` is read, and `buf[3..8]` slices the zero-initialized 258-byte array — parse_stamp returns `Ok(0)`: a fabricated PTS=0 (and fabricated DTS from `buf[8..13]` for `0b11` with `hl<10`). No panic, but a malformed header mints a false timestamp that propagates into track spans and packet PTS. What correct looks like: `0b10` requires `hl >= 5`, `0b11` requires `hl >= 10`, else `bad_pes_header`. Confidence: HIGH — Verification: read (slice indices vs `want` traced).

F-003 [S3]
Where: crates/demux/src/ps/mod.rs:1015-1019
Code:
```
                    None => {
                        self.damage.note(length - self.walk, "trailing-garbage");
                        self.eos = true;
                        continue;
                    }
```
What is wrong: this is the only EOS path that does not drain `damage.events` into `pending` — compare :982-987 (loop-top) and :1036-1039 (0xB9), both of which run `for (skipped, reason) in std::mem::take(&mut self.damage.events)`. After the `continue`, the loop top pops an empty `pending` and returns `EndOfStream`; the `Recovered{..,"trailing-garbage"}` event is silently dropped, contradicting the module's "never-silent" rule (stated at :993). Blast radius: one observability event on damaged-tail files. Confidence: HIGH — Verification: read (all four EOS paths compared).

F-004 [S3]
Where: crates/demux/src/ps/mod.rs:731-744
Code:
```
            let matches = match stream.key {
                StreamKey::Video(id) | StreamKey::Audio(id) => id == sid,
                StreamKey::Private1(..) => sid == 0xBD,
                StreamKey::Private2 => false,
            };
            if matches {
```
What is wrong: stamps are routed by stream id only. Every 0xBD PES — regardless of substream — matches the FIRST registered `Private1` stream and returns; a standard multi-audio DVD (AC-3 0x80 + DTS 0x88 + LPCM 0xA0, plus SPU 0x20) mixes all private-stream stamps into the first AC-3 track's extent and leaves DTS/LPCM/SPU tracks at `TimeSpan::UNKNOWN` (:854-859). Why it matters: track timing spans (UI duration, per-track extents) are wrong for exactly the DVD-Video content this demuxer targets. The committed fixtures each carry only one private audio family, so nothing pins the multi-stream case. What correct looks like: pass the substream byte into `note_stamps_for` and match `StreamKey::Private1(kind, sub)` exactly (the event walk already has the substream in hand at :1078-1084; the scan has it at :676-695). Confidence: HIGH — Verification: read.

F-005 [S3]
Where: crates/demux/src/ps/mod.rs:518-521
Code:
```
        let packet_end = pos + 6 + u64::from(declared_len);
        if packet_end > self.reader.length() {
            return Err(error::truncated(packet_end, self.reader.length()));
        }
```
What is wrong: a file truncated inside the LAST PES payload (a cut download / split VOB boundary) makes `parse_pes_full` fail inside `scan_streams`, so `start()` (:944-946) fails and the whole file refuses to open — despite every earlier packet being intact and the module advertising damage tolerance. The graceful `pes-payload-short` damage note in `emit_pes_packet` (:904-906) is unreachable through this lane (packet_end ≤ length guarantees the full read); only the unchecked 0xBF nav lane (:1052-1064) can reach it. Defensible as "honest truncation", but it is a hard open failure on realistic damage, inconsistent with the positional-skip treatment of every other defect class here. Confidence: MEDIUM (behavioral severity judgment; the control flow is HIGH-confidence read) — Verification: read + fuzz_lite truncation sweeps pass only because fuzz treats typed errors as success.

F-006 [S4]
Where: crates/demux/src/ps/mod.rs:279-281 and :451-453, :462-464
Code:
```
    if value >> 33 != 0 {
        return Err(error::bad_pes_header(pos));
    }
```
What is wrong: dead defensive code. The extractions at :274-278 and :444-450/:457-461 are OR-compositions of masked fields whose bit budgets sum to exactly 33; the maximum representable value is 0x1FFFFFFFF, so `>> 33` is always 0 and these branches can never fire (the same holds for the MPEG-1 SCR check at :462). ps/error.rs:48-50 advertises "SCR field bits outside their declared widths … every extraction is width-checked before it is trusted" — the checks exist but are unreachable, i.e. the advertised guard class guards nothing. Confidence: HIGH — Verification: read + hand-computed bit budgets (3+2+8+5+2+8+5=33; 3+8+7+8+7=33).

F-007 [S4]
Where: crates/demux/src/ps/mod.rs:661-667 (scan) and :1041-1047 (event walk)
Code:
```
                0xBC | 0xBB | 0xBE | 0xBF => {
                    let ext = self.length_of(pos)?;
                    if sid == 0xBF {
                        self.register_key(StreamKey::Private2, "dvd_nav_packet", MediaKind::Data);
                    }
                    pos = pos + 6 + ext;
                }
```
What is wrong: unlike the PES lane, the system-header/PSM/padding/nav skip never checks `pos + 6 + ext` against the file length, so an element whose declared length runs past EOF advances the cursor past the end and the loop-top silently absorbs it (`pos + 4 <= length` fails) — no typed truncation, no damage note in the scan pass. No OOB read occurs (read_prefix_into clamps), but the handling is asymmetric with the PES lane's typed `truncated`. Confidence: HIGH — Verification: read.

F-008 [S4]
Where: crates/demux/src/ps/mod.rs:769-771 (and :794-796)
Code:
```
        let got = match self.reader.read_prefix_into(&mut buf[..want], data_start) {
            Ok(got) => got,
            Err(_) => return "mpeg2video",
        };
```
What is wrong: I/O errors during the classification peek are swallowed into the default descriptor ("mpeg2video" / "mp2"). Documented default, benign blast radius (track label), but it is a swallowed error (`Err(_) =>`), the pattern the spec's checklist flags. Confidence: HIGH — Verification: read.

F-009 [S4]
Where: crates/demux/src/ps/mod.rs:1124-1133
Code:
```
        let last = self.last_scr.unwrap_or(0);
        if ticks < 0 {
            return Err(error::seek_out_of_range());
        }
        let clamped = u64::try_from(ticks).unwrap_or(0).min(last);
```
What is wrong: (a) the trait doc (crates/plugin-api/src/demux.rs:78-79) says seek errors when "the target is out of range"; this implementation never does — targets beyond the SCR extent clamp to `last_scr` (the CLI host openly relies on the clamp, containers/ps.rs:149-164, so this is a documented divergence, not a bug); (b) `u64::try_from(ticks).unwrap_or(0)` is dead after the `ticks < 0` check. The seek/reanchor split itself is contract-correct: plugin-api demux.rs:73-75 "Does **not** move the read position" — `seek` is pure, `reanchor` (:1143-1218) does the compute-then-commit walk and marks the discontinuity. Confidence: HIGH — Verification: read of both files.

F-010 [S4]
Where: crates/demux/src/ps/mod.rs:576-596
Code:
```
            match buf[at] >> 4 {
                0b0001..=0b0011 => {
```
What is wrong: the accepted MPEG-1 stamp vocabulary includes '0001' treated as DTS-only (:587-588) — not an ISO 11172-1 prefix — and rejects the spec's optional-field prefixes ('0110' buffer size, '1000' STD, etc.) via the `_` arm's `bad_pes_header` (:597-605). ps/error.rs:62-65 documents the same 0010/0011/0001/0000 vocabulary, so code and error doc agree with each other and both diverge from the ISO table; scope is explicitly ffmpeg-muxer layouts (module doc :10-14), so this is permissiveness/strictness debt, not a fixture-visible defect. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe | 125-127 | REAL | — | matches media-probe phase5.rs:186 mirror |
| PrivateKind::of_substream | 146-154 | REAL | — | ranges match DVD book; pinned by tests |
| PrivateKind::strip | 157-163 | REAL | — | Ac3/Dts 4, Lpcm 7, Spu 1 — fixture-pinned |
| PrivateKind::descriptor | 166-173 | REAL | — | ac3/dts/pcm_dvd/dvd_subtitle |
| StickyShape::of / mint | 244-255 | REAL | — | replayable sticky shape |
| stamp_of | 260-265 | REAL | — | Missing fallback leg |
| parse_stamp | 270-283 | REAL | S4 | dead `>>33` check (F-006) |
| PsDemuxer::new | 343-361 | REAL | — | pool 2×64KiB; SourceReader rejects unseekable/empty |
| stream_descriptors | 368-370 | REAL | — | host uses it for decode-pending exposure |
| current_scr / scr_extent / is_mpeg2 / damage_total / pool_bytes | 374-403 | REAL | — | observability getters, test-pinned |
| fail / replay | 406-417 | REAL | — | sticky discipline correct |
| parse_pack_at | 422-494 | REAL | S4 | SCR/mux-rate math hand-verified correct; `end>length` checked; dead width checks (F-006) |
| parse_pes_full | 497-619 | REAL | S2/S3 | MPEG-2 guarded, MPEG-1 underflows (F-001); short-hl fabricated stamps (F-002); truncation typed (F-005) |
| scan_streams | 624-714 | REAL | S4 | header-only pre-scan; 0xBC..0xBF no bounds check (F-007); substream peek ≤1B |
| length_of | 718-725 | REAL | — | u16 big-endian at pos+4 |
| note_stamps_for | 728-745 | REAL | S3 | 0xBD routed to first private stream (F-004) |
| register_key | 749-760 | REAL | — | first-appearance dedup |
| probe_video_codec | 766-785 | REAL | S4 | 64-byte peek; swallowed Err (F-008); default mpeg2video documented |
| probe_audio_codec | 789-806 | REAL | S4 | layer bits; swallowed Err; default mp2 |
| resync_from | 811-832 | REAL | — | 64KiB window, one bounded alloc, returns Err(resync_failed) only when window full & no code |
| stream_index_for | 835-845 | REAL | — | exact substream match |
| build_tracks | 849-877 | REAL | — | index→TrackId consistent with emit; spans from F-004-affected extents |
| emit_pes_packet | 881-921 | REAL | S2 | `payload_too_large` is where F-001's wrapped len detonates; pool fill + short-read note |
| Demuxer::start | 925-959 | REAL | — | head check, scan, idempotent replay, damage drain |
| Demuxer::next_event | 961-1114 | REAL | S3 | full walk; F-003 lost Recovered; per-packet damage drain |
| Demuxer::seek | 1116-1137 | REAL | S4 | pure query per contract; clamp-not-error divergence (F-009) |
| Demuxer::seekability | 1139-1141 | REAL | — | SlowLinear |
| Demuxer::reanchor | 1143-1218 | REAL | — | compute-then-commit; discontinuity flag; pending cleared |

## 5. Test honesty (crates/demux/src/ps/tests.rs, 28 tests + 4 fixture tests — all pass, my run)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mpeg1/mpeg2 pack round-trips + marker-bit pins (7) | YES | exact SCR/rate/stuffing assertions incl. the prior-session `b[11]&1` bug from the other side |
| stamp_bytes_roundtrip_with_the_parser | YES | 7 tick values incl. 2^32 and (1<<33)-1 |
| mpeg1 no-stamp / mpeg2 flags pins | YES | byte-exact header shapes asserted before draining |
| private1_ac3/dts/lpcm/spu (2 tests) | YES | assert descriptor, kind, strip-exact payload bytes |
| private1_unclassified…, private2_nav…, padding/system-header skip | YES | assert track sets + recovered reasons + payload identity |
| typed error paths (wrong_head/truncated_pack/bad_pack/bad_pes) | YES | assert error codes exactly |
| zero_pes_length / damaged_bytes_resync | YES | damage notes + packet survival pinned |
| seek_clamps…reanchor | YES | asserts landed tick + discontinuity via drain |
| fuzz_lite_mutations…never_panic | WEAK | absence-of-panic only; fixed LCG seed does not reach F-001's shape (would panic in debug if it did) |
| fuzz_lite_clean_fixtures… | YES (for its claim) | pins terminal==None on all 4 fixtures |
| ps_fixtures.rs (4, baseline) | YES | bit-exact payload match vs ffmpeg extractions |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "AC-3/DTS/LPCM/SPU exposed as honest tracks (decode pending — typed, never silent)" | mod.rs:1-3 | REAL | tests private1_* + containers/ps.rs refusal test (baseline :3047 `vob_lanes_demux_and_refuse_undecodable_codecs_typed ... ok`) |
| "PES_packet_length == 0 … is a damage-noted skip, never an unbounded read" | mod.rs:74-76 | REAL | :505-517 + test zero_pes_length… |
| "the differential suite pins the demuxed payloads against ffmpeg's own raw extractions bit-for-bit" | mod.rs:12-14 | REAL | baseline ps_fixtures 4/4 ok |
| "every extraction is width-checked before it is trusted" (SCR class) | ps/error.rs:48-50 | SHALLOW | checks exist but are dead (F-006) |
| "Unclassified substreams are skipped positionally with a Recovered event" | mod.rs:53-54 | REAL | :694 + :1096-1101 + test |
| "one ≤64-byte elementary peek per new audio/video id" | mod.rs:21-22 | REAL | probe_video_codec :767 (min(payload_len,64)) |
| seek "never moves anything" / reanchor is the acting half | plugin-api demux.rs:73-98 | REAL | PsDemuxer::seek pure; reanchor moves walk :1210-1213 |

## 7. Dead / duplicated / unreachable code
- Dead width checks: mod.rs:279-281, :451-453, :462-464 (F-006).
- Dead truncation branches in parse_pes_full (:525-527, :538-542): unreachable after `want > declared_len` + `packet_end ≤ length` both pass — harmless defensive code.
- `u64::try_from(ticks).unwrap_or(0)` :1132 dead leg (F-009b).
- `MAX_PACKET_BYTES` guard :890-894: unreachable via u16 lanes (comment admits); reachable only through F-001's wrapped value — i.e. the "typed ceiling" only ever fires on the underflow bug's debris.
- `DemuxEvent::Discontinuity` never emitted by this demuxer (fine — Recovered is the sanctioned damage event, plugin-api demux.rs:152-159).

## 8. Dependencies & flags
No feature gates on this file (plain `pub mod ps` in lib.rs:53; no cfg anywhere in ps/mod.rs — verified by read). Relies on crate::avi::riff (SourceReader/PayloadSlots — pool fill returns Arc-backed SharedBytes; 2-slot bound verified at riff.rs:588-669: pinned arenas spill to one exact-size buffer, so the 2-slot pool cannot corrupt queued packets) and crate::avi::header::DamageLog (:397-400 saturating note). No unsafe blocks in the file.

## 9. Open questions for the next agent
- containers/ps.rs (host layer, 913+ lines) — not in my assignment; my F-004 (private-stream timing) and F-005 (truncation → open failure) observability claims stop at the demuxer boundary; the host may paper over or amplify them.
- Does any engine lane feed `next_event` errors back into a retry (F-001 release behavior assumes the engine stops on first Err — verify in demux_host/engine session loop)?
- fuzz corpus: the committed fuzz-lite seeds never hit F-001/F-002 shapes; a targeted corpus (short declared_len MPEG-1 PES; PTS flag with hl=0) would panic the debug harness immediately.
