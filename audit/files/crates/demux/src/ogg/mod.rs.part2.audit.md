# AUDIT: crates/demux/src/ogg/mod.rs#part2 (lines 493-984)
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 492 (chunk 493-984) / 20 |
| File verdict (chunk) | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → apps/cli/src/demux_host.rs:458 `containers::open` → containers/mod.rs:74 `"ogg" => ogg::open` → `OggDemuxer`; baseline doctest `crates/demux/src/ogg/mod.rs - ogg::OggDemuxer (line 406) ... ok` in orchrun-test-workspace-nofailfast.log.txt:6374) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | rg `ogg::` in baseline log (31 tests ok, lines 1802-1831) · rg `flac -i` in ogg/tests.rs + tests/ogg_fixtures.rs (ZERO FLAC-in-Ogg test coverage) · rg `FlacInOgg|flac_streaminfo` (definition part1:307, unit test part3:1461 — synthetic bare-block form) · page.rs parse_header read in full (truncation classification) · sed -n verification of every quote |

## Chunk boundaries
Part1 (1-492) = A10: consts, `OggCodec`, `Clock`, `identify`, `flac_streaminfo_rate`, `LogicalStream`, struct defs, `new`, `tracks()`. **Part2 (493-984, this audit)** = observability accessors, `fail`, `read_page`, `recover_sync`, `read_full_head`, `queue_recovered`, `step`, `open_stream`, `complete_packet` head (through line 984). Part3 (985-1477) = A03: `complete_packet` tail, `mint_track`, `stash_header`, seek/reanchor, `find_capture`, tests. `complete_packet` is SPLIT at 984/985 — I audit its head and cross-reference the tail.

## 1. One-paragraph verdict
This chunk is the demuxer's engine room — page reading with CRC gating, bounded sync recovery, chain/link mapping, granule bookkeeping, and packet assembly — and it is real: 31 ogg tests executed green in the baseline (bad-CRC drop+recovery, chained links, muxed serials, granule regression, continued-flag lies, truncated-page sticky, zero-copy pool bounds). The walk logic is careful (wrapping sequence comparison, RFC 3533 §6 last-terminal granule ownership, insert-only map invariants documented at each `get_mut`). The defects: the FLAC-in-Ogg clock gate in my range waits for a STREAMINFO packet on the SECOND header, but the Xiph ogg-mapping puts STREAMINFO in the FIRST (BOS) packet — on every real FLAC-in-Ogg file the clock is instead parsed out of a VORBIS_COMMENT header or an audio data packet (garbage rate, wrong duration/pts, zero test coverage); the module doc's "typed sticky truncation" claim holds only for the no-prior-damage body-overrun case (table-overrun is classified `bad_capture` and swallowed into recovery; recovery-path overruns are silently skipped); recovery re-reads and re-CRCs the page it just validated; and a serial-reuse chain boundary silently discards the old stream's open cross-page packet.

## 2. What actually works (verified)
- Page walk + damage policy executed: baseline `orchrun-test-workspace-nofailfast.log.txt:1814-1831` — `bad_crc_page_is_dropped_with_sync_recovery`, `mid_stream_garbage_recovers_at_the_next_capture`, `continued_packet_assembles_across_pages`, `only_the_last_packet_on_a_page_owns_the_granule`, `chained_links_boundary_discontinuity_and_fresh_tracks`, `mux_two_serials_two_tracks_interleaved`, `granule_regression_is_an_honest_discontinuity`, `minus_one_data_page_punches_a_visible_hole`, `sequence_discontinuity_is_damage_noted_and_continues`, `pool_stays_bounded_and_packets_are_zero_copy_views`, `truncated_page_is_a_typed_sticky_error` — all ok (demux lib suite 462/0/0, log lines 1489-1491).
- CRC construction (read_page 626-648): `head[0..22] + zeroed CRC field + head[26..27+nsegs] + body` — verified against RFC 3533 §5 layout; the CRC variant itself is pinned by `ogg::page::tests::crc_matches_the_ogg_variant_check_value` ok (log 1809).
- Assembly bounds: cross-page accumulation refuses `> MAX_PACKET_BYTES` typed (line 876-877, 897-898, 905-906) — `unterminated_lacing_run_is_a_typed_overrun` exists in tests.rs.
- Chain/link state machine traced by hand: muxed BOS mid-link (no damage, correct), all-ended + new serial → new link, serial reuse + BOS → new link — consistent with the module doc lines 10-16; `chained_links_boundary_discontinuity_and_fresh_tracks` ok (log 1815).

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/ogg/mod.rs:953-957 (gate, this chunk); 276-277 + 307-314 (part1, A10); 987-991 + 1461-1468 (part3, A03)
Code:
```
                } else if stream.codec == Some(OggCodec::FlacInOgg)
                    && stream.clock.is_none()
                    && stream.headers.len() == 1
                {
                    flac_clock_now = true;
                }
```
What is wrong: The gate arms the clock parse for the stream's SECOND completed packet (headers.len() == 1), on the belief that "the clock rides the STREAMINFO packet" (part1:277). Per the Xiph FLAC-in-Ogg mapping, the STREAMINFO block — with its sample rate — rides the FIRST (BOS) packet: `0x7F "FLAC" + major + minor + 2 count bytes + "fLaC" + 4-byte block header + STREAMINFO` (rate at packet offset ~26). `identify()` (part1:276-277) consumes that first packet as the magic probe and returns `None` clock. So on a real FLAC-in-Ogg file the gate fires on the SECOND header — normally a VORBIS_COMMENT block — and part3:988 applies `flac_streaminfo_rate` (part1:307-314), which has no magic check and reads a 20-bit "rate" from offsets 14-16 of whatever packet arrives: vendor-string bytes of the comment header (garbage nonzero) or, if the second header is lost to damage, bytes of the first AUDIO packet (garbage). The bare-block form the parser DOES handle (`[00 00 00 22] + …`, rate at 14) is the layout of a native FLAC block, not of any packet a conforming FLAC-in-Ogg file puts in the clock position.
Why it matters: Every real FLAC-in-Ogg (.oga) file gets a garbage sample-rate clock (or none): `track_duration` (554-568) reports a wrong duration at open, every granule-bearing packet stamp is wrong, and the seek mapping (`Clock::granule_of_nanos`, part1) lands in the wrong place. FLAC decode exists in this tree (`fuzz_codec_flac` seeds), so this is a live lane, not a dead arm. Coverage is zero: rg `flac` over `ogg/tests.rs` and `tests/ogg_fixtures.rs` → no hits; the only test is part3's `flac_streaminfo_rate_bits` (1461-1468), which feeds the synthetic bare-block shape and pins the wrong offset — the HEVC-fixture pattern again.
What correct looks like: Parse the rate out of the BOS packet itself in `identify()` (offsets: `fLaC` at 8, block header 12-16, rate bits at 26-28); keep the second-packet gate only as a fallback with a magic check (`starts_with("fLaC")` or the 0x7F mapping prefix) before trusting any bytes; add a mapping-form fixture.
Confidence: HIGH on the gate logic and zero coverage (code-traced, rg). HIGH on "STREAMINFO rides the BOS packet" (Xiph ogg_mapping; ffmpeg's oggparseflac.c parses it from the first packet). Not executed. Cross-file: part1 (A10) owns `identify`/`flac_streaminfo_rate`; part3 (A03) owns the apply tail and the unit test.

### F-002 [S3]
Where: crates/demux/src/ogg/mod.rs:605-624 (this chunk), 681 (this chunk); doc claim 45-47; page.rs:141-145 (cross-ref)
Code:
```
//! [`Recovered`](plugin_api::DemuxEvent::Recovered). A page that runs
//! past the end of the stream is a typed sticky truncation. Sequence
```
```
                    if matches!(
                        err.kind,
                        media_types::ErrorKind::Corrupt {
                            detail: media_types::error::CorruptDetail::Truncated { .. },
                        }
                    ) {
                        return Err(self.fail(err));
                    }
```
What is wrong: The unconditional doc claim is enforced on exactly one path. (a) A page truncated inside its SEGMENT TABLE never reaches the Truncated classification: `read_full_head` returns a short head, and `parse_header` (page.rs:141-144) returns `bad_capture` for `head.len() < 27+nsegs` — read_page's non-Truncated arm sends it to `recover_sync`, which scans to EOF and ends the stream CLEANLY with a `Recovered` event. (b) During recovery, a body-overrun truncated page is swallowed silently: `if let Ok(hdr) = page::parse_header(...)` (line 681) filters out the Truncated error and keeps scanning. Only the pristine-path body overrun (parse_header's second extent check, page.rs:154-157) is the typed sticky error the doc promises.
Why it matters: A truncated download cut mid-table (or any truncation after earlier damage) plays to a clean EOS with a `Recovered` event instead of surfacing the typed truncation — the swallowed-error class; the engine never enters its Error state for a file the module doc says is a typed refusal.
What correct looks like: In `parse_header`, classify a short window as `truncated_page` when the capture is valid and the page extent (header+table) exceeds `stream_len` — check extent before the head-length guard; and in `recover_sync`, propagate Truncated candidates instead of `if let Ok`.
Confidence: HIGH — Verification: read (both paths traced through page.rs:100-157); `truncated_page_is_a_typed_sticky_error` ok in baseline (log 1830) tests only the body-overrun shape.

### F-003 [S4]
Where: crates/demux/src/ogg/mod.rs:681-712 (this chunk)
Code:
```
                    if page::crc32(&whole) == hdr.crc {
                        candidate = Some(pos);
                        break;
                    }
```
```
                    self.walk = pos;
                    return self.read_page();
```
What is wrong: `recover_sync` fully validates a candidate (parses the header, reads the body via a fresh `read_prefix_at` Vec, CRCs the whole page), then instead of returning that page it sets `walk = pos` and recursively calls `read_page()`, which re-reads the header, re-fills the pool body, and re-CRCs the identical bytes — double I/O, double allocation, double CRC per recovered page.
Why it matters: Recovery is the damage path — the exact moment where work per byte matters (a 4 MiB budget of scanning); every successful recovery pays 2×. Not a correctness bug (deterministic reads give the same verdict).
What correct looks like: Return the already-validated `PageHeaderAndBody` (read the body through the pool in recover_sync and construct it) instead of recursing into `read_page`.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/demux/src/ogg/mod.rs:769-775 + 831-836 (this chunk)
Code:
```
        } else if header.bos {
            // Serial reuse with a fresh BOS: a chained link recycling
            // the serialno — fresh state, fresh track id.
            self.link += 1;
            self.pending.push_back(DemuxEvent::Discontinuity);
            self.open_stream(header.serial);
        }
```
```
        let had_pending = self
            .streams
            .get(&header.serial)
            .is_some_and(|s| s.pending.is_some());
```
What is wrong: On serial reuse, `open_stream` REPLACES the `LogicalStream` (pending → None) BEFORE the continued-flag honesty block reads `had_pending` — so if the old link's stream had an open cross-page packet, it vanishes with no damage note and no "open packet dropped" accounting (the `!header.continued && had_pending` arm at 840-848 can never see it).
Why it matters: A chain boundary cut mid-packet silently discards up to 256 KiB of a packet the demuxer already accepted; the damage log (the module's honesty surface) records nothing for it.
What correct looks like: Snapshot (or drop-with-note) the old stream's `pending` inside the reuse arm before `open_stream` replaces it.
Confidence: HIGH — Verification: read (ordering of the two blocks traced).

## 4. Per-function table (chunk 493-984)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `links` / `serial_numbers` / `header_packets` | 494-515 | REAL | — | Observability; cloned Vec per call (fine) |
| `granule_discontinuities` / `damage_total` / `serial_link` / `packet_counts` / `pool_bytes` | 517-552 | REAL | — | Test-observable (chained/mux/pool tests assert them) |
| `track_duration` | 554-568 | REAL | S2-adjacent | `Option::max` merge of walk + tail granules is correct — but feeds from F-001's garbage clock on FLAC-in-Ogg |
| `fail` | 572-575 | REAL | — | Sticky set |
| `read_page` | 581-653 | REAL | S3 | CRC layout verified; trailing-fragment clean-end; head27 short-read arm (594-597) dead (remaining ≥ 27 guaranteed at 585); F-002 truncation classification |
| `recover_sync` | 658-728 | REAL | S3/S4 | Bounded 4 MiB budget → typed sticky ResourceLimit; same window-advance pattern as mp3 (traced, no stall); F-002(b) swallow; F-003 double validation; recursion depth ≤ 2 (candidate must CRC-verify) |
| `read_full_head` | 731-734 | REAL | — | Trivial |
| `queue_recovered` | 736-741 | REAL | — | Skips zero-skip events |
| `step` | 745-920 | REAL | S4 | Chain/link state machine traced correct; wrapping seq compare correct; granule hole/regression bookkeeping matches doc lines 27-31; granule to LAST terminal run correct (RFC 3533 §6); overrun sticky; F-004 pending-discard ordering |
| `open_stream` | 924-933 | REAL | — | Insert-or-replace; `serial_order` dedup correct on reuse |
| `complete_packet` (head, →984) | 937-984 | REAL | S2 | F-001 gate; Unknown-magic fallthrough emits BOS as first data packet (correct opaque posture); FLAC two-stage design is the F-001 carrier; tail is part3 (A03) |

## 5. Test honesty (ogg tests live in tests.rs [A10] / clock_unit [part3]; executed in baseline, all ok)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `bad_crc_page_is_dropped_with_sync_recovery` / `mid_stream_garbage_recovers_at_the_next_capture` | YES | Drop + Recovered + continuation (log 1814, 1819) |
| `chained_links_boundary_discontinuity_and_fresh_tracks` | YES | Link count, Discontinuity, fresh track ids (log 1815) |
| `continued_packet_assembles_across_pages` / `continued_flag_lies_are_damage_noted` | YES | Cross-page assembly + both lie arms (log 1816-1817) |
| `only_the_last_packet_on_a_page_owns_the_granule` | YES | RFC §6 stamping rule (log 1824) |
| `granule_regression_is_an_honest_discontinuity` / `minus_one_data_page_punches_a_visible_hole` | YES | My chunk's granule bookkeeping (log 1818, 1820) |
| `mux_two_serials_two_tracks_interleaved` | YES | Mapping (log 1821) |
| `truncated_page_is_a_typed_sticky_error` | YES but NARROW | Covers only the pristine body-overrun shape; table-overrun and recovery-path swallow (F-002) untested (log 1830) |
| `flac_streaminfo_rate_bits` (part3:1461-1468) | NO for real files | Synthetic bare-block packet; pins the wrong offset; no mapping-form fixture anywhere (F-001) |

## 6. Doc claims touching this chunk
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "A page that runs past the end of the stream is a typed sticky truncation" | mod.rs:45-47 | PARTLY FALSE | F-002: only the pristine body-overrun path; table-overrun → bad_capture → recovery → clean EOS; recovery-path overruns swallowed |
| "Bad CRC, bad version, broken capture: the page is dropped and a bounded sync-recovery scan (≤ MAX_RECOVERY_SCAN_BYTES) hunts the next CRC-valid capture" | mod.rs:42-44 | REAL | read_page 642-648, recover_sync 658-728; tests ok (log 1814, 1819) |
| "Sequence discontinuities and continued-flag lies are damage-noted … never fatal, never hidden" | mod.rs:47-48 | REAL (one gap) | step 778-848; `sequence_discontinuity…`/`continued_flag_lies…` ok; the F-004 chain-boundary pending-discard is the hidden case |
| "the granule position is the position of the last packet completing on the page" (RFC 3533 §6) | mod.rs:20-27 | REAL | `last_terminal` rposition (855) + `only_the_last_packet…` ok |
| Zero-copy framing ("a packet completing inside one page is an O(1) slice of the pooled body") | mod.rs:50-57 | REAL | step 891-895 fast path; `pool_stays_bounded_and_packets_are_zero_copy_views` ok (log 1825) |

## 7. Dead / duplicated / unreachable code
- read_page's `head27.len() < HEADER_BYTES` arm (594-597) is unreachable (remaining ≥ 27 enforced at 585) — defensive, documented style.
- recover_sync's `body.len() < hdr.body_len` break (691-693) is defensive-dead (parse_header already bounds the extent).
- No dead accessors found in this chunk (all are test-asserted).

## 8. Dependencies & flags
No feature gates, no unsafe. `page::parse_header`/`crc32` from page.rs (A02's file). Pool: 2 slots × 256 KiB ceiling, lazy (riff.rs:542-549). Per-page CRC scratch Vec documented as accepted steady-state (mod.rs:58-61).

## 9. Open questions for the next agent
- F-001 spans three chunks: A10 (part1: identify/flac_streaminfo_rate) and A03 (part3: apply tail + clock_unit test) must cross-link; a fix belongs in identify() (parse the BOS packet) plus a mapping-form fixture.
- Does the probe (media-probe OggSignature) or the containers host (wave 13, apps/cli/src/containers/ogg.rs) consume `track_duration` at open? If yes, F-001's wrong duration is user-visible in `lumen open` metadata for .oga FLAC files.
- The trailing-fragment path (585-591) ends cleanly by design — confirm the engine treats Recovered-at-EOF as success (it does today per rc-smoke evidence shape); that is the F-002 posture question.
