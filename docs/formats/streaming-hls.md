# Format record — HTTP Live Streaming (RFC 8216 + Apple HLS authoring spec, self-build)

Status: **implemented (library stack; evidence Linux-only)** — the VOD
media-playlist session (6.B1a), the live sliding window with recorded
discontinuities (6.B1b), master playlists + variant switching with ABR
hysteresis (6.B2a/6.B2b), and the `X-TIMESTAMP-MAP` live-edge
derivation + conformance corpus (6.B3) are landed and tested in
`crates/streaming` + `crates/playlists`. The matrix row `network-hls`
stays **partial** on every platform: the code is platform-independent
but the evidence is mock-socket/Linux-only (no win/macos runs yet,
R4). Front-matter keys per doc 04 §6:

- id = `network-hls` (`docs/feature-matrix.toml`)
- status = implemented (library); matrix partial — evidence Linux-only
- phase = 6 (waves 6.B1–6.B3; commits d2de397, 56d1a05 + this card)
- fuzz_target = none yet — network fuzz targets are card 6.F1's
  (registry `doc 10 §6`); this record does not claim one
- crates = [] — zero external deps (the 2017-frozen `m3u8` crate was
  evaluated as a parse cross-check oracle and rejected for
  self-build, matrix note + 5.O1; `thiserror` is the playlists
  crate's pre-existing workspace dep)
- self_built = [`crates/playlists/src/hls.rs` (shared grammar),
  `crates/streaming/src/hls/{media,live,master,timestamp_map}.rs`]
- last_audit = 2026-11-09 (card 6.B3, issue #11)

## What landed when (the stack this record covers)

| Card | Landed | Where / evidence |
|---|---|---|
| 6.B1a | VOD media-playlist session: typed model, EXTINF/`MEDIA-SEQUENCE`/`TARGETDURATION`/`ENDLIST`, discontinuity records, unknown-length probe refusal | `hls/media.rs`; `tests/hls_vod_e2e.rs` (TS lane byte-exact + ADTS lane decoded to the ffmpeg oracle) |
| 6.B1b | LIVE session: sliding window by absolute sequence, live-edge join (`live_edge_back`, clamped at the window head), demand-driven reloads with a stalled-reload budget, `segment-lost-window` typed refusal, per-fetch boundary records | `hls/live.rs`; `tests/hls_live_e2e.rs` (slide + ENDLIST + real 44.1→22.05 kHz discontinuity seam over a real socket; stalled source fails `streaming.hls.live-timeout`) |
| 6.B2a | Master playlists: `EXT-X-STREAM-INF`/`EXT-X-MEDIA` parse, highest-BW-under-cap selection | `hls/master.rs`; `tests/hls_master_e2e.rs` |
| 6.B2b | Variant switching at segment boundaries: absolute-sequence alignment, ABR hysteresis band + dwell | `hls/master.rs`; `tests/hls_master_e2e.rs` |
| 6.B3 M1 | `#EXT-X-TIMESTAMP-MAP` grammar (either attr order, ISO-8601 UTC profile, 33-bit-wrap-exact PTS↔wallclock), superseding-tag-wins playlist scan, `live_edge_wallclock_ms()` | `hls/timestamp_map.rs` + `hls/live.rs`; 9 lib tests (commit 56d1a05; suite below) |
| 6.B3 M2 | **Conformance corpus** (this card): clean-room vectors over the real ffmpeg TS fixtures, served on a real socket | `tests/hls_conformance_e2e.rs` (table below) |

## The 6.B3 conformance corpus (clean-room)

No proprietary vendor suite is used or copied: every vector is
synthesized from the *documented* behavior of the public specs
(RFC 8216 §4.1/§6.3.4 + the Apple HLS authoring spec / IETF bis tag
grammar), pinned over this repo's own ffmpeg-generated fixtures
(`tests/fixtures/hls/ts_live/` — two real encoding lanes, generation
evidence in `hls_live_e2e.rs`), with hand-derived oracles. All six
ran green in the suite below.

| Vector | Spec behavior (clause) | Pinned by test |
|---|---|---|
| L1 | Live-edge join clamps at the first segment of a window shorter than `live_edge_back`; edge wallclock = `LOCAL` + window EXTINF | `live_edge_join_clamps_to_first_segment_of_a_short_window` |
| L2 | Window slide (`MEDIA-SEQUENCE` advances): no refetch, concatenation undisturbed — pinned by request counts (open + reloads + segments only) | `window_slide_never_refetches_or_disturbs_the_stream` |
| L3 | Reload cadence §6.3.4 "no sooner than one target duration": elapsed == wait must NOT sleep; early reload is paced out (`FakeClock`) | `reload_cadence_at_the_exact_target_duration_boundary` |
| D1 | `EXT-X-DISCONTINUITY` with one continuing timeline: seam recorded on the boundary, one PTS↔wallclock anchor across it, byte-exact concat | `discontinuity_seam_with_continuing_timestamps` |
| D2 | Encoder timestamp reset across a seam: superseding `X-TIMESTAMP-MAP` (counter near the 33-bit wrap + `LOCAL` jump) adopted on reload; wrap-exact conversions | `discontinuity_seam_with_encoder_timestamp_reset` |
| G1 | `#EXT-X-GAP`: unknown-tag tolerance (§4.1 MUST-ignore) — **honest v0: no gap skip/fence semantics yet** (pending below) | `gap_tag_is_tolerated_no_gap_semantics_yet` |

## Working subset + honest boundaries

- **Served**: plain TS media segments (VOD + live), master/variant
  selection + boundary switching, X-TIMESTAMP-MAP derivation.
- **Still refused, typed** (`streaming.hls.*`): encrypted segments
  (`EXT-X-KEY`), `EXT-X-BYTERANGE`, init sections (`EXT-X-MAP` /
  fMP4 lanes), a live open on an `ENDLIST` playlist (use the VOD
  session), window-regression and lost-window slides.
- **Parsed but not acted on**: `EXT-X-MEDIA` alternate renditions
  (recorded, never selected); `EXT-X-GAP` (played, never skipped).
- **Pending**: gap skip/fence semantics; live-window variant
  switching; gapless `X-TIMESTAMP-MAP` anchoring is an assumption
  `[VERIFY]` against a real-world corpus (the tag anchors at the
  first following segment — how encoders publish it in practice).

## Evidence (this audit's runs)

- `cargo test -p streaming` (2026-11-09, this card): **lib 173
  passed / 0 failed / 2 ignored** (live-net `--ignored` lanes) +
  e2e suites all green — `hls_conformance_e2e` 6/6, `hls_live_e2e`
  2/2, `hls_master_e2e` 3/3, `hls_vod_e2e` 2/2 (plus the non-HLS
  streaming suites in the same run).
- Fixtures: ffmpeg 7.1.5 segment outputs (s-/t-lanes), committed
  with their generation command in `hls_live_e2e.rs`.
- No differential vs VLC: the sandbox has no VLC HLS lane; the
  oracles are ffmpeg's own encoder output (byte-exact concatenation
  + decoded-sample counts), which is the honest available reference.

## Network scenarios (issue #14 netfuzz M2/M3, 2026-11-09)

The interrupted-network lane: hostile bytes and scripted wire faults
against the HLS stack, over the real-socket mockhttp transport. Two
fuzz targets + three scenario proofs, all deterministic (seeded
mutations, the M1 `FaultProxy`/`FakeClock` support).

### Fuzz targets (registry rows 34–35; stable sweeps as the runner)

| Target | Surface | Harness | Recorded 60 s clean run (this card, stable 1.99.1) |
| ------ | ------- | ------- | -------------------------------------------------- |
| `fuzz_hls_playlist` | M3U8 bytes → media/master models, ABR pick, `X-TIMESTAMP-MAP` conversions | `playlists::fuzz::fuzz_playlist_m3u` + `netfuzz::fuzz_hls_playlist` (`#[path]` from `crates/streaming/tests/netfuzz/fuzz.rs`) | `netfuzz_sweep_hls_playlist_60s`: **112 981 iterations in 60.000285029 s**, 0 panics |
| `fuzz_http_headers` | response bytes → the transport's response-head grammar + `Range:` value formatting | `streaming::mockhttp::fuzz_response_head` + `netfuzz::fuzz_http_headers` (same module) | `netfuzz_sweep_http_headers_60s`: **160 120 iterations in 60.000324337 s**, 0 panics |

- Seeds: the real HLS playlists of
  `crates/playlists/tests/fixtures/hls/` (master `multi`/`single`/
  `malformed`, `ts_vod`/`fmp4` media playlists) + authored response
  heads; committed under `fuzz/corpus/<target>/`.
- Mutations (deterministic xorshift64\*): truncations, 1–64 seeded
  bit flips, tag/head soup splices, head+tail splices, pure random,
  and an over-64-KiB arm exercising the head budget's typed
  `ProtocolViolation` refusal.
- **Recorded finding** (kept honest, src outside this wave's
  ownership): `TimestampMap::pts_for_wallclock_ms` overflows its i64
  intermediate when the wall value sits > ~10^14 ms from the tag's
  anchor — a panic under `overflow-checks`, a silent wrap without.
  The mapping is contracted for values near anchors; the harness
  clamps its probe to ±(i64::MAX / 2·90 000) ms of the anchor and
  documents the boundary for the follow-up card.
- The libFuzzer campaign itself still awaits the Lead's nightly
  wiring (fuzz/README's LEAD-DECISION); `cargo check` in `fuzz/`
  compiles all 35 targets on stable (verified this card).

### Interrupted-network scenarios (`tests/net_interrupted.rs`)

| Scenario | Scripted wire | Proven outcome |
| -------- | ------------- | -------------- |
| Mid-segment drop | `DropAfter{at:300}` on a ranged segment GET (then `TruncateAt` on a plain GET) | `RetryFetch` re-issues the identical request; resumed body **byte-exact** (`body[250..]` / full 1000 B); 1 fault fired, 2 wire fetches, ≤1 backoff sleep under the 500 ms first ceiling, zero real wait |
| Manifest reset | `ResetAfter` ×64 (persistent outage) at the manifest | typed `streaming.hls.fetch-failed` refusal (network cause); 4 retries + 1 exhausted + 5 bitten fetches; every virtual sleep under its exponential ceiling (500→4000 ms), total under the 7.5 s ladder sum; whole ladder < 5 s real time |
| Live edge under delay | `Delay{900}` + `Delay{400}` at live join (back=2) | joins at the edge, streams **byte-exact** vs the on-disk oracle (100 204 B, cat(s01,s02,t00,t01)); 0 segments lost to the sliding window; sleeps exactly `[900, 400]` virtual ms, zero real wait |

Evidence run (this card): `cargo test -p streaming` — lib 201/0/2 +
all e2e green, including `net_interrupted` 3/3 and `netfuzz_sweep`
2/2 (+ the two `--ignored` 60 s lanes above, run with
`-- --ignored --nocapture`).
