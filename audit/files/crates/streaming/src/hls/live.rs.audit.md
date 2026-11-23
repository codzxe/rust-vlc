# AUDIT: crates/streaming/src/hls/live.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Lines / functions | 1473 / 42 (21 production, 6 test helpers, 15 tests) |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point calls it) |
| Reachable from shipped entry point? | NO — only `crates/streaming/tests/{hls_live_e2e,hls_conformance_e2e,net_interrupted}.rs` and its own `#[cfg(test)]` call `open_live` (rg `open_live`, all hits test-only). CLI `open <url>`: apps/cli/src/parse.rs:510-518 `SourceSpec::Url` → apps/cli/src/demux_host.rs:399-441 http(s) arm → `open_http_source` (demux_host.rs:474-500) → `streaming::ranged::RangedResource::open` → `media_probe::probe_stream`. No HLS probe exists (rg `-i 'm3u|EXTM3U' crates/media-probe/src` → 0 hits), so an `.m3u8` URL's playlist text is fetched as a ranged byte resource and fails at the probe stage; `open_live`/`open_vod`/`open_master` are never invoked. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 4 |

## 1. One-paragraph verdict
The live sliding-window session is genuinely implemented: absolute-sequence window mapping, live-edge join, demand-driven reload with a stall budget, opt-in RFC 8216 §6.3.4 pacing through an injected clock, typed gap/regression/encryption refusals, ENDLIST termination with exact size, and a discontinuity record. All 15 unit tests pass (I ran them). The sliding-window math itself is sound (media sequence mapping, overlap filtering, stall accounting — I re-derived every reload case below). But none of it is reachable from the shipped binary: the module doc itself admits at L55-58 that live playback through existing demux hosts "is a **later integration**", and the shipped CLI has no HLS wiring at all. Secondary defects: the pacing fallback in `reload_wait_ms` is dead in every reachable call, the live-edge wallclock anchors the whole window regardless of tag position (doc flags [VERIFY]), and `#EXT-X-MEDIA-SEQUENCE` omission on reload kills the session as "window-regression".

## 2. What actually works (verified)
— `cargo test -p streaming --lib hls::live` (from repo root) → `15 passed; 0 failed; 0 ignored; 199 filtered out`. Slide mapping, ENDLIST size, stall budget error shape, pacing schedule `[2000,1000,1000]`, gap refusal, regression refusal, discontinuity record, refusal table, seek policy, timestamp-map supersedes — all asserted with exact stats/byte expectations; these tests would fail if the reload math broke.

## 3. Findings
F-001 [S2]
Where: crates/streaming/src/hls/live.rs:682-765 (and 50-58)
Code:
```
//! refuses unknown-length sources at open
//! (`demux.source.unknown-length`), which means live playback through
//! the existing demux hosts is a **later integration** (a streaming
```
What is wrong: `open_live` (and the entire live session) has zero production callers; the shipped `lumen open http://…/pl.m3u8` path never reaches it. The typed refusal vocabulary the module advertises (`streaming.hls.vod-playlist`, `.empty-playlist`, `.encrypted`, …) is never what a user of the shipped binary sees for an HLS URL — they get a ranged-HTTP fetch of the playlist text followed by a probe-stage "unknown signature" failure. Why it matters: 1473 lines of tested engine code are dead weight in the release lane; any doc/matrix reader believing "HLS live is refused typed" or "HLS plays" is wrong about the binary. What correct looks like: either a CLI open-lane branch that probes content-type/`#EXTM3U` and routes to `open_live`/`open_vod`, or matrix/docs stating plainly "not wired into any shipped host". (The feature-matrix row `network-hls` marks status "partial" with crate-level evidence only — it does not claim binary playback, so this is unwired-real, not fraud.)
Confidence: HIGH — Verification: read + rg call-chain (`open_live` hits: live.rs tests + 3 streaming test files only).

F-002 [S3]
Where: crates/streaming/src/hls/live.rs:543-550
Code:
```
        if model.media_sequence < self.media_sequence {
            return Err(mint(
                ErrorKind::Corrupt {
                    detail: CorruptDetail::InvalidStructure { offset: None },
                },
                "streaming.hls.window-regression",
```
What is wrong: `MediaPlaylist.media_sequence` defaults to 0 when the tag is absent (crates/playlists/src/hls.rs:313 `media_sequence: media_sequence.unwrap_or(0)`). A reload from a server that published `#EXT-X-MEDIA-SEQUENCE:N` on the opening playlist and then omits the tag (RFC 8216 §4.3.3.2 makes the tag optional per-file; only the *first* playlist of a session is required to carry it consistently) parses as 0 < N → the session dies as `window-regression` (Corrupt) on the first reload. VLC tolerates this. Why it matters: kills live playback against a class of real encoders. What correct looks like: treat an absent tag as "sequence unchanged" (or compare against the previous *window's* last segment URI+seq, as VLC's adaptive module does).
Confidence: MEDIUM — Verification: read (parser default confirmed at hls.rs:313); not executed against a real encoder.

F-003 [S3]
Where: crates/streaming/src/hls/live.rs:476-494
Code:
```
        let seconds = self
            .window
            .target_duration
            .unwrap_or_else(|| {
                self.pending
                    .iter()
                    .filter_map(|segment| segment.duration)
                    .fold(0u64, |acc, duration| acc.max(duration.ceil() as u64))
                    .max(1)
            })
```
What is wrong: the doc (L473-475) claims a window without TARGETDURATION "falls to its largest `EXTINF` (rounded up)". Dead path: `reload()` is only ever called from `serve()`'s final `else` arm, reached only when `self.pending` is empty (serve():450-461), and `reload_wait_ms` runs at reload() entry — so `self.pending.iter()` always folds over an empty queue → the fallback is always `0.max(1)` = 1 s. The EXTINF fallback can never observe a duration. Why it matters: misleading comment + pacing for non-compliant no-TARGETDURATION playlists is a fixed 1 s regardless of EXTINF (under-waits vs. §6.3.4). What correct looks like: fold over `self.window.segments`, not `self.pending`.
Confidence: HIGH — Verification: read + traced both call sites of `reload` (only serve():461).

F-004 [S3]
Where: crates/streaming/src/hls/live.rs:346-357
Code:
```
        let window_ms: u64 = self
            .window
            .segments
            .iter()
            .filter_map(|segment| segment.duration)
            .map(|duration| (duration * 1000.0).round() as u64)
            .sum();
        Some(map.local_ms + window_ms as i64)
```
What is wrong: `live_edge_wallclock_ms` adds the ENTIRE window's EXTINF total to `LOCAL`, which is only correct when `#EXT-X-TIMESTAMP-MAP` anchors at the window's first segment. A tag placed mid-playlist (legal) overcounts by the durations of the segments preceding it. The doc admits the anchor assumption is unverified (L341-345 `[VERIFY]` against a real-world corpus) — an honest flag, but the number shipped is wrong for that shape. Why it matters: any consumer syncing subtitles to `live_edge_wallclock_ms` gets a skewed edge. What correct looks like: sum only the EXTINFs of segments from the tag's position to the window end.
Confidence: HIGH — Verification: read; the [VERIFY] marker is the module's own admission.

F-005 [S4]
Where: crates/streaming/src/hls/live.rs:584-585
Code:
```
        let window_max = model.media_sequence + model.segments.len().saturating_sub(1) as u64;
        self.max_seen_seq = self.max_seen_seq.max(window_max);
```
What is wrong: for an EMPTY reload window (parser accepts zero segments; only `open_live` refuses empty, reload() never re-checks), `len().saturating_sub(1)` = 0 so `window_max` = base — `max_seen_seq` advances to a sequence number that was never published. When the server then publishes that first real segment at the new base, the productive check (`media_sequence + i > self.max_seen_seq`, :576-581) misses it → counted as stalled → false `streaming.hls.live-timeout` after the budget. Why it matters: empty-window playlists (ad-break placeholders) mis-bill the stall budget. What correct looks like: `window_max` should only account for windows with ≥1 segment (`len().checked_sub(1)` → skip update on empty).
Confidence: MEDIUM — Verification: read + manual derivation; no test covers empty reload windows (confirmed by test list).

F-006 [S4]
Where: crates/streaming/src/hls/live.rs:436-449 (serve) with 554-559 (reload)
Code:
```
            } else if self.gap_pending {
                // The window slid past unfetched segments: the byte
                // stream would need a gap — never served silently.
                // Bytes already in hand answer this read; more
                // demanded past the gap refuses typed.
```
What is wrong: `gap_pending` is never cleared, so after one lost-window slide the session is permanently terminal even though the rebuilt `pending` queue holds fetchable post-gap segments (reload():563-574 still populates them — dead bytes). Contract-honest (byte-exact concatenation cannot skip), but the retained pending entries are unreachable state. What correct looks like: on gap, clear `pending` and document the terminal state.
Confidence: HIGH — Verification: read; rg `gap_pending` — set at :558, read at :436/:618, never reset.

F-007 [S4]
Where: crates/streaming/src/hls/live.rs:517-533
Code:
```
        self.timestamp_map = timestamp_map;
        self.stats.requests += 1;
        self.stats.playlist_reloads += 1;
```
What is wrong: a reload that refuses (encrypted/byterange/map arriving mid-live, :532-540) has already mutated stats and `timestamp_map` before returning Err; the pending queue was NOT rebuilt, so a caller that retries `read()` re-fetches the playlist on every attempt — no budget bounds the refusal-retry loop (the stall budget only counts unproductive *successful* reloads, :589-604). Why it matters: a read-loop consumer against a mid-stream-encrypted source generates one playlist GET per read call. What correct looks like: count refusals against the stall budget or latch a terminal state.
Confidence: MEDIUM — Verification: read + control-flow trace.

F-008 [S4]
Where: crates/streaming/src/hls/live.rs:563-574 (also 584, 723, 727)
Code:
```
        self.pending = model
            .segments
            .iter()
            .enumerate()
            .filter(|(i, _)| model.media_sequence + *i as u64 >= self.next_abs_seq)
            .map(|(i, seg)| PendingSegment {
                abs_seq: model.media_sequence + i as u64,
```
What is wrong: unchecked `u64` addition on a parsed `MEDIA-SEQUENCE` (attacker-controlled, up to `u64::MAX` via `parse_tag_uint`). No `[profile.release] overflow-checks` override exists in the workspace Cargo.toml, so the release build wraps silently: `MEDIA-SEQUENCE: 18446744073709551615` with any segment index wraps the absolute sequence to small numbers, corrupting the slide/gap/regression logic (no crash, no memory unsafety — wrong typed refusals at worst). Why it matters: hostile-input robustness; the debug build would panic here. What correct looks like: `checked_add` with a typed Corrupt refusal on a hostile base.
Confidence: HIGH — Verification: read + Cargo.toml profile check (no overflow-checks key).

## 4. Per-function table (file >100 LOC — every function)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HlsLiveConfig::Default | 151-162 | REAL | — | budgets wired to consts (109/115) |
| Debug for HlsLiveConfig | 167-178 | REAL | — | clock elided, `is_some` only |
| HlsLiveConfig::validate | 187-209 | REAL | — | zero/over-ceiling typed refusals, tested |
| HlsLiveStream::window | 305-308 | REAL | — | advisory accessor |
| HlsLiveStream::media_sequence | 312-315 | REAL | — | accessor |
| HlsLiveStream::stats | 318-321 | REAL | — | Copy accessor |
| HlsLiveStream::timestamp_map | 329-332 | REAL | — | per-reload refreshed |
| HlsLiveStream::live_edge_wallclock_ms | 346-357 | SHALLOW | S3 | F-004: whole-window sum, anchor unverified |
| HlsLiveStream::segment_boundaries | 363-366 | REAL | — | grows per fetch |
| HlsLiveStream::fetch_pending | 370-400 | REAL | — | HTTP status mapping, boundary record, seq cursor |
| HlsLiveStream::absorb | 406-420 | REAL | — | budget rule; invariant retained_from+len==fetched_upto holds in all 3 branches (re-derived) |
| HlsLiveStream::serve | 427-468 | REAL | S4 | F-006 gap terminal; partial-read contract |
| HlsLiveStream::reload_wait_ms | 476-494 | SHALLOW | S3 | F-003: EXTINF fallback dead (pending always empty) |
| HlsLiveStream::reload | 499-606 | REAL | S3/S4 | F-002 regression-vs-default-0; F-005 empty-window max_seen; F-007 refusal retry unbounded |
| SourceStream::uri | 610-612 | REAL | — | |
| SourceStream::size | 614-619 | REAL | — | Some only when ended+drained+ungapped |
| SourceStream::supports_seek | 621-626 | REAL | — | `true`; policy enforced in seek() |
| SourceStream::read | 628-630 | REAL | — | capped at MAX_READ_BYTES |
| SourceStream::seek | 632-657 | REAL | — | backward-within-retained / forward-within-fetched typed |
| SourceStream::position | 659-661 | REAL | — | |
| open_live | 682-765 | REAL-UNPROVEN | S2 | F-001 unreachable from shipped binary; logic itself tested |
| tests: LiveRouter::new/playlist_fetches/fetch | 790-850 | REAL | — | reload-mutable router, enforces response budget |
| drain / read_exactly / fast_config | 855-888 | REAL | — | helpers |
| 15 test fns | 892-1472 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| live_open_joins_back_from_the_edge | YES | asserts exact bytes BBBBCC, segments_fetched=2, no pre-join fetch |
| window_slide_maps_by_absolute_sequence | YES | 2 slides, exact 12-byte concat, reload/stall counts |
| endlist_terminates_and_fixes_the_total | YES | size()==Some(10) only after drain; empty post-end read |
| stalled_reloads_fail_typed_live_timeout | YES | exact ResourceLimit{budget:2, attempted:3}, 4 playlist fetches |
| paced_reloads_wait_the_rfc8216_cadence | YES | asserts sleep schedule [2000,1000,1000] + virtual clock 4000 |
| paced_productive_reload_waits_the_full_cadence | YES | sleeps==[1000] on productive reload |
| slid_out_segments_refuse_typed_at_the_gap | YES | code + lost count 1 + size None |
| window_regression_refuses_typed | YES | Corrupt kind + code |
| discontinuity_is_recorded_across_the_seam | YES | boundary offset/duration/flag + stats |
| live_refusal_table_is_typed | YES | 6-code table + discontinuity OK |
| encryption_arriving_on_reload_refuses | YES | prefix plays, then typed refusal |
| live_seek_is_honest | YES | free rewind, unfetched-forward and unretained-backward refusals |
| live_config_bounds_refuse_typed | YES | zero + over-ceiling |
| timestamp_map_parses_applies_and_supersedes_per_reload | YES | exact ms values, supersedes on reload |
| timestamp_map_absent_is_none_not_a_guess | YES | both surfaces None |
No tautologies found. Mutation reasoning: changing the slide filter, stall accounting, or absorb budget rule breaks specific assertions.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "live playback through the existing demux hosts is a later integration" | live.rs:55-58 | REAL | rg open_live — test-only callers; CLI path goes RangedResource→probe |
| "Reloads that waited out the RFC 8216 cadence first" counted | live.rs:221-223 | REAL | paced_reloads counter + test |
| "falls to its largest EXTINF (rounded up)" pacing fallback | live.rs:473-475 | FALSE | F-003: pending always empty at reload_wait_ms time |
| "A window that moves backwards refuses typed" | live.rs:46-48 | REAL | reload():543-550 + test |
| matrix network-hls "partial" + crate-level evidence | docs/feature-matrix.toml:1441-1444 | REAL (but omits "not wired into CLI") | call-chain in header table |

## 7. Dead / duplicated / unreachable code
- Whole module unreachable from shipped binary (F-001) — reachable only from streaming-crate tests.
- `reload_wait_ms` EXTINF fallback (F-003) — dead in all reachable calls.
- Post-gap `pending` entries (F-006) — populated but never fetched.
- `#EXT-X-DISCONTINUITY-SEQUENCE` is not parsed anywhere (rg in playlists crate: 0 hits) — per-segment flags only; acceptable for the byte surface but the discontinuity-sequence continuity story is absent.

## 8. Dependencies & flags
Uses `crate::clock::WallClock` (FakeClock in tests), `playlists::MediaPlaylist`, `plugin_api::SourceStream`, `media_types` errors, `crate::fetch`. No feature gates inside this file; crate-level features audited under streaming lib.rs (another agent).

## 9. Open questions for the next agent
- Does any desktop/web host route `.m3u8` to the HLS sessions? (apps rg says no — only a ranged.rs comment in apps/web/src/io/seekability.rs:175.)
- media.rs/master.rs (VOD/ABR sessions): same unreachable-from-CLI status? Same audit trail applies.
