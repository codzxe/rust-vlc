# AUDIT: apps/cli/src/demux_host.rs
| Field | Value |
|---|---|
| Auditor | W13-A09 |
| Date | 2025-07-14 |
| Lines / functions | 999 / 19 non-test + 10 tests |
| File verdict | REAL (with a dispatcher-level S1: it forwards unvalidated bytes into a panicking probe) |
| Reachable from shipped entry point? | YES: main.rs:14 argv -> run.rs:193 `parse::open_command` -> dispatch.rs:82 `host.open_session` -> host.rs:904 `open_source` -> demux_host.rs:368 `open_source_with` (executed: `./target/debug/lumen open` exit 101/65/0) |
| Findings | S0: 0 S1: 1 S2: 0 S3: 2 S4: 3 |
| Commands run | `./target/debug/lumen open audit/scratch/W13-A09/ts188.bin` -> exit 101 panic; ts187/ts189 -> exit 65; `cargo test -p lumen-cli --lib demux_host` -> 10 passed, 0 failed |

## 1. One-paragraph verdict
This is the real open dispatcher, not a stub: three access lanes (path, `file:` URL, `http(s)` via ranged resource) all funnel into one shared `probe_stream` -> `crate::containers::open` pipeline, and all 15 probed format ids have wired arms (containers/mod.rs:52-73). The wrapper logic (track selection, start-time skip, reanchor watermark clearing) is genuine and behaviorally tested. Its fatal flaw is what it does NOT do: nothing between `FileAccess::open` and the probe inspects a single content byte, so the known TS-188 probe off-by-one (media-probe phase5.rs:161) is reachable verbatim through `lumen open` — I reproduced exit 101. The dispatcher trusts a probe that panics on a 188-byte 0x47-leading file.

## 2. What actually works (verified)
— verified by `cargo test -p lumen-cli --lib demux_host` -> `test result: ok. 10 passed; 0 failed` (fixtures, mockhttp loopback socket, clamp semantics, typed refusals all asserted).
— verified by `./target/debug/lumen open ts187.bin` / `ts189.bin` -> exit 65 `[demux.corrupt]: corrupt structure (demux.ts.no-pat)` (the non-188 boundary sizes behave typed).

## 3. Findings
F-001 [S1]
Where: apps/cli/src/demux_host.rs:442-458 (dispatch) -> crates/media-probe/src/signature/phase5.rs:159-163 (panic site; cross-file, also W05-A04/W05-A09)
Code:
```
    // 2. Probe (the 2.A2 bounded window; non-consuming).
    let mut stream = stream;
    let registry = media_probe::ProbeRegistry::with_builtins();
    let config = media_probe::ProbeConfig::default();
    let report = media_probe::probe_stream(&registry, &mut *stream, &config)
        .map_err(|err| wire_error(&err))?;
```
What is wrong: The probe is the FIRST consumer of source bytes; the only preceding checks are path-level (canonicalize L297, `is_file` L323, UTF-8 L330) — zero content validation guards the probe. The TS signature panics at exactly 188 bytes: `if prefix.len() >= TS_PACKET_BYTES { if prefix[TS_PACKET_BYTES] != 0x47` indexes one past a 188-byte window (guard must be `>` or `>= TS_PACKET_BYTES + 1`). Why it matters: any local 188-byte file starting with 0x47 crashes the whole process — executed: `./target/debug/lumen open audit/scratch/W13-A09/ts188.bin` -> `thread 'main' panicked at crates/media-probe/src/signature/phase5.rs:161:16: index out of bounds: the len is 188 but the index is 188`, EXIT=101. Controls: 187B and 189B both exit 65 typed (`demux.ts.no-pat`) — the bug is a single-byte window off-by-one, not a general TS failure. It also contradicts media-probe's own house rule ("no `unwrap`/`expect`/`panic!` ... in non-test code", lib.rs:39-40). What correct looks like: fix the bound in phase5.rs (media-probe owner agent); demux_host needs no change but deserves a defense-in-depth note that the dispatcher offers no pre-probe content sanity check.
Confidence: HIGH — Verification: ran (exit 101 reproduced; Wave 5 W05-A04 independently executed the same).

F-002 [S3]
Where: apps/cli/src/demux_host.rs:514-562
Code:
```
    /// (converted per-track through the harvested bases) are dropped here
...
    fn before_skip(&self, packet: &media_types::Packet) -> bool {
...
        if !self.bases.iter().any(|(id, _)| *id == packet.track) {
            return false;
        }
        match pts.convert(&media_clock::NANO_BASE) {
```
What is wrong: The doc claims conversion "through the harvested bases", but the harvested `TimeBase` tuple element is never read (`|(id, _)|`) — conversion uses the packet pts's own embedded base; `bases` is only a track-membership filter. The stored bases are dead data beyond membership, and a packet whose track is absent from `bases` bypasses the skip filter entirely. Why it matters: comment/code mismatch on a correctness-critical filter; if a demuxer ever emits packets for a track not in its `start()` list, those packets silently ignore `--start-time`. What correct looks like: either convert via the harvested base for the matching track (honoring the doc), or drop the base from the tuple and fix the doc.
Confidence: HIGH — Verification: read.

F-003 [S3]
Where: apps/cli/src/demux_host.rs:488-495
Code:
```
        None => streaming::schemes::SchemeRegistry::new().open_http(
            &streaming::fetch::Url::parse(url)
                .unwrap_or_else(|err| unreachable!("the URL parsed one call ago: {err}")),
            config,
        ),
```
What is wrong: Production `unreachable!` on a re-parse of the same string (parsed at L480, result discarded, parsed again here). Provably dead only while `Url::parse` is a pure function of its input; the double parse exists because the first result is thrown away. Why it matters: a panic path shipped in the release binary guarding redundant work; any future parse non-determinism (encoding hooks, normalization flags) turns an http URL into a remote panic. What correct looks like: bind the first `Url::parse` result and reuse it — removes both the unreachable! and the duplicate work.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: apps/cli/src/demux_host.rs:63-73
Code:
```
/// product.toml ships (`queue_packets_audio = 1024`,
/// `queue_packets_video = 512`, product.toml:199-200). The CLI loads
```
What is wrong: Stale citation — the values live at product.toml:203-204, not 199-200. The constants themselves are a hand-maintained mirror of config values (honestly flagged `[PROPOSAL]`), which will drift silently when product.toml changes and nothing enforces the mirror.
Confidence: HIGH — Verification: read + `rg -n queue_packets product.toml` -> 203/204.

F-005 [S4]
Where: apps/cli/src/demux_host.rs:377-378
Code:
```
#[allow(clippy::too_many_arguments)]
fn open_source_with(
```
What is wrong: The lint fires at 8+ arguments; this function takes 5. Dead attribute.
Confidence: HIGH — Verification: read.

F-006 [S4]
Where: apps/cli/src/demux_host.rs:566-576
Code:
```
            .filter(|track| match track.kind {
                MediaKind::Video => !self.disable_video,
                MediaKind::Audio => !self.disable_audio,
                _ => false,
            })
```
What is wrong: Subtitle/data tracks are silently erased from the track list before the engine ever sees them; the doc comment (L510-513) describes the engine dropping them "at the unwired-track boundary", but the wrapper drops them first. Undocumented-in-place behavior divergence (the module doc at L29-33 does not mention it either).
Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| queue_plan_from_product_limits | 85-97 | REAL | — | maps or typed-refuses; test pins floor/ceiling |
| wire_error | 108-124 | REAL | — | stage strip + chain root append; used by host.rs/bench.rs |
| access_refusal | 130-138 | REAL | — | redacted URI naming, no double-append |
| TrackFact::span_end_us | 175-180 | REAL | — | ticks->us via Timestamp convert |
| TrackFact::audio_geometry | 202-220 | REAL | — | PCM verbatim; Vorbis via codecs parser; OpusHead RFC 7845 byte 9, 48 kHz — correct |
| MediaFacts::duration_us | 239-241 | REAL | — | max of declared extents |
| MediaFacts::fact_for | 245-247 | REAL | — | linear find, cold path |
| file_uri | 296-305 | REAL | — | canonicalize + typed inaccessible |
| file_uri_from_canonical | 322-352 | REAL | — | is_file, UTF-8, UNC refusal via shared constructor |
| open_source | 362-369 | REAL | — | thin wrapper over open_source_with(None) |
| open_source_with | 378-459 | REAL | S1(F-001) | THE dispatcher; no content validation before probe |
| open_http_source | 474-500 | REAL | S3(F-003) | fetch-port parse + ranged resource; unreachable! on re-parse |
| SelectDemuxer::new | 529-543 | REAL | — | plain constructor |
| SelectDemuxer::before_skip | 548-562 | REAL | S3(F-002) | membership-only use of bases; unjudgeable passes through |
| SelectDemuxer::start | 566-576 | REAL | S4(F-006) | kind filter; erases subtitle/data |
| SelectDemuxer::next_event | 578-590 | REAL | — | skip loop; forwards non-packet events |
| SelectDemuxer::seek | 592-594 | REAL | — | pure forward |
| SelectDemuxer::seekability | 596-604 | REAL | — | forwards, no invention |
| SelectDemuxer::reanchor | 606-618 | REAL | — | clears skip watermark at landing — correct backward-seek semantics, test-pinned |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parent_fixture_opens_with_expected_facts | YES | asserts container id, 2 tracks, duration, codec names, frame_ns window, pcm pair |
| start_time_clamps_to_duration | YES | asserts clamped anchor + zero surviving packets |
| select_demuxer_is_generic_over_a_non_avi_demuxer | YES | non-AVI instantiation + watermark drops all packets |
| garbage_file_is_a_probe_stage_refusal | YES | stage/code pinned on garbage |
| magic_broken_derivative_is_refused_at_probe | YES | flip16 fixture, "sniffed" in message |
| missing_file_is_inaccessible | YES | stage/code pinned |
| blocked_scheme_urls_refuse_naming_the_lane | YES | rtsp named, no "Phase 6" |
| http_url_opens_through_the_ranged_resource | YES | real loopback socket, Range header asserted in request log |
| file_url_sources_open_like_path_sources | YES | both RFC 8089 spellings, canonical URI equality |
| queue_plan_join_maps_product_limits_or_refuses_typed | YES | pins 1024/512 and both schema bounds |
All 10 executed: `cargo test -p lumen-cli --lib demux_host` -> 10 passed. No #[ignore], no tautologies found.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bounded, non-consuming window read" probe | demux_host.rs:21-23 | REAL | probe_stream restores position (media-probe lib.rs:204-209); tests pass |
| "every other scheme is the honest host-capability refusal" | demux_host.rs:18-19 | REAL | L428-438 UnsupportedForHost + rtsp test |
| product.toml:199-200 holds the queue figures | demux_host.rs:64-65 | FALSE (line numbers) | actual product.toml:203-204 (values correct) |
| "the probe and the container registry are shared verbatim" for http | demux_host.rs:15-18 | REAL | open_source_with shares L442-458 for all lanes; http test proves |

## 7. Dead / duplicated / unreachable code
- `bases` TimeBase elements unread (F-002).
- `#[allow(clippy::too_many_arguments)]` dead (F-005).
- The `unreachable!` at L493 is provably dead today (F-003) but ships in release.

## 8. Dependencies & flags
Crosses: commands, media-types, media-probe, playback-engine, plugin-api, streaming, input, codecs, media-clock, demux (test-only). No feature gates in this file; the release lane (baseline cargo-build-release-lumen-cli.txt exit 0) compiles all of the above.

## 9. Open questions for the next agent
- Who owns the phase5.rs fix — media-probe's auditor (W05 confirmed it); has a fix landed by W13? (Still panics in the debug binary I ran — NOT fixed as of this audit.)
- Does the desktop app funnel `open` through this same path (apps/desktop dispatch)? See my desktop/dispatch.rs audit this wave.
