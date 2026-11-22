# AUDIT: apps/cli/src/demux_host.rs

| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-22 |
| Lines / functions | 1000 / 31 (20 production + 11 test fns/tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `main.rs:19` → `run::run` → `repl_mode` (run.rs:198 `lumen_dispatch`) → `CliHost::open_session` (host.rs:904 `open_source`) → this file. This IS the spec's `open_source`. |
| Findings | S0:0 S1:0 S2:0 S3:3 S4:6 |

Commands run:
- `sed -n '63,73p;202,220p;478,496p' apps/cli/src/demux_host.rs` (quote verification, exit 0)
- `sed -n '195,205p' product.toml` → `queue_packets_audio = 1024`, `queue_packets_video = 512` (mirror constants verified accurate today)
- `rg -n "extension|with_builtins|fn probe_stream" crates/media-probe/src` + read of `probe_stream` (lib.rs:193-236) and `ProbeRegistry::probe` (signature/mod.rs:175-213) → **content probing only; `meta.extension` is read nowhere except error messages (error.rs:32)**
- `rg -n "extra_data|esds|aac" apps/cli/src/containers/mp4.rs` → MP4 harvest exposes the AAC ASC blob (mp4.rs:17-22, 232)
- `rg -n "DEFAULT_MAX_LOOKAHEAD|DEFAULT_MIN_SCORE" crates/media-probe/src/config.rs` → 64 KiB / 60

## 1. One-paragraph verdict

This is the real open path: canonicalize → access → content probe → container-registry dispatch → facts pre-pass → selection/skip wrapper, with honest typed refusals at every stage (input/probe/demux). Demuxer selection is 100% content-based — the file extension is never consulted (verified: no builtin probe plugin reads `meta.extension`; only the error path does), so a renamed file does NOT break `open`; what breaks it is a content signature the registry does not know, which refuses at stage `probe` (tested with garbage and magic-broken fixtures). The wrapper's skip/selection logic is genuine and behaviorally tested. The defects: the audio-geometry lookup parses only PCM pairs, Vorbis ident headers and OpusHead — the MP4 harvest hands it a real AAC ASC blob and it declines to parse it, so every AAC session opens limb-less (a contributing cause of the litmus MP4's frozen-clock shape); the product queue caps are hand-mirrored constants; and a handful of brittle stringly seams.

## 2. What actually works (verified)

- **Extension vs content (the spec's hostile question): answered.** `open_source` → `media_probe::probe_stream` (crates/media-probe/src/lib.rs:193-211) reads a bounded 64 KiB window (`DEFAULT_MAX_LOOKAHEAD`), scores builtin signatures (`ProbeRegistry::probe`, signature/mod.rs:175-213: score → priority → shorter-prefix tie-break, min_score 60), restores the stream position (non-consuming), and answers a format id that drives `containers::open`. No code path consults the extension for identification — renaming `x.mp4` to `x.txt` still opens as MP4; a garbage file refuses at stage `probe` (test `garbage_file_is_a_probe_stage_refusal`, demux_host.rs:780-798). The extension surfaces only in refusal messages (error.rs:32).
- File / `file:`-URL / `http(s)` lanes share one downstream pipeline; the http lane is exercised in-test over a real loopback socket with ranged requests asserted (demux_host.rs:876-921).
- The `SelectDemuxer` skip watermark and reanchor-clearing are behaviorally pinned (`start_time_clamps_to_duration` asserts zero packets survive a target clamped to the end; `select_demuxer_is_generic_over_a_non_avi_demuxer` proves the wrapper over a non-AVI demuxer).
- Not executed (no cargo): verdicts from reading + W00 baseline (open path works: MP4 "opens (tracks parsed)").

## 3. Findings

### F-001 [S3]
Where: apps/cli/src/demux_host.rs:202-220
Code:
```rust
        let data = self.extra_data.as_ref()?.as_slice();
        if let Ok(ident) = codecs::audio::vorbis::parse_identification(data) {
            return Some((u32::from(ident.channels), ident.sample_rate));
        }
        if data.starts_with(b"OpusHead") && data.len() >= 19 {
```
What is wrong: `audio_geometry` — the generic seam that decides whether a session gets an audio limb (device + audio-master clock) — recognizes exactly three geometries: the PCM pair, the Vorbis identification header, and OpusHead. The MP4 container harvest hands it a real AAC `esds` AudioSpecificConfig blob (containers/mp4.rs:17-22, 232: `"aac" => asc_of_entry(entry)...`), which carries sampling_frequency_index and channel_configuration — parseable geometry that this function declines to parse.
Why it matters: every AAC-in-MP4 session (the audit's litmus family; W00: h264+aac MP4 opens, 0 frames decoded, clock frozen) opens limb-less on the wall clock. The doc's framing (demux_host.rs:196-199: "`None` for … audio without a geometry-bearing blob") is misleading: for MP4-AAC the blob exists and is handed over; the parser is simply missing an arm. With no AAC decoder today the practical impact is masked, but any future AAC decoder inherits a session that can never track an audio-master clock.
What correct looks like: an AAC ASC arm (5-byte minimum: object type, sampling index, channel config), or the doc admitting the gap is per-codec, not per-blob-availability.
Confidence: HIGH — Verification: read both files.
Severity: S3 today (no AAC decoder exists to expose it), S2 the day an AAC decoder lands.

### F-002 [S3]
Where: apps/cli/src/demux_host.rs:63-73
Code:
```rust
pub const PRODUCT_QUEUE_PACKETS_AUDIO: u64 = 1024;
/// The video half of the same mirror.
pub const PRODUCT_QUEUE_PACKETS_VIDEO: u64 = 512;
```
What is wrong: the product's queue caps are hand-mirrored Rust constants instead of loaded from product.toml. Verified accurate TODAY (`product.toml` `[limits]`: `queue_packets_audio = 1024`, `queue_packets_video = 512`), but any product.toml edit silently diverges from the shipped binary's posture.
Why it matters: duplicated config source of truth; the in-code `[PROPOSAL]` admits the load is owed and never landed (same defect as host.rs F-011 — counted once per file since both cite it).
What correct looks like: load `[limits]` at open (or at least a build-time equality test against product.toml).
Confidence: HIGH — Verification: read + sed of product.toml:199-200.

### F-003 [S4]
Where: apps/cli/src/demux_host.rs:213-218
Code:
```rust
        if data.starts_with(b"OpusHead") && data.len() >= 19 {
            // RFC 7845 §5.1: channels at byte 9 of the 19-byte
            // fixed identification header; the output rate is
            // always 48 kHz (§4.2).
            return Some((u32::from(data[9]), 48_000));
```
What is wrong: the OpusHead sniff trusts any 19+ byte blob starting with `OpusHead` — the version byte (offset 0, must be 0x01 per RFC 7845 §5.1) is not validated, and no channel-count sanity bound (0 and >255 are impossible per spec; 0 would size a zero-channel render buffer downstream).
What correct looks like: `data[0] == 0x01 && data[9] >= 1`.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: apps/cli/src/demux_host.rs:480-493
Code:
```rust
    streaming::fetch::Url::parse(url).map_err(|err| {
        ...
    })?;
    let config = streaming::ranged::RangedConfig::default();
    let resource = match fetch {
        Some(fetch) => streaming::ranged::RangedResource::open(fetch, url, config),
        None => streaming::schemes::SchemeRegistry::new().open_http(
            &streaming::fetch::Url::parse(url)
                .unwrap_or_else(|err| unreachable!("the URL parsed one call ago: {err}")),
```
What is wrong: the URL is parsed twice; the second parse's failure is an `unreachable!` justified only by the parser's purity (true today, brittle if the grammar ever gains state). The first parse's value is discarded instead of threaded through.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: apps/cli/src/demux_host.rs:108-111
Code:
```rust
    let full = error.to_string();
    let stage_prefix = format!("[{}] ", error.stage);
    let base = full.strip_prefix(&stage_prefix).unwrap_or(full.as_str());
```
What is wrong: `wire_error` de-tags the engine's Display string by string-prefix stripping — a stringly coupling to playback-engine's Display format. If the engine's Display changes, the stage tag leaks into user-facing messages (graceful, but silent). Cross-crate contract with no test pinning it here.
Confidence: MEDIUM — Verification: read; no test in this file pins the strip behavior.
Severity: S4.

### F-006 [S4]
Where: apps/cli/src/demux_host.rs:578-590
Code:
```rust
                DemuxEvent::Packet(packet) => {
                    if self.before_skip(&packet) {
                        continue;
                    }
```
What is wrong: while the skip watermark is live, one `next_event` call can loop unboundedly through the inner demuxer until a packet survives — for `--start-time` near the end, a single call can walk the entire remainder of the file (the engine's per-tick demux bound counts events, not inner iterations). Documented ("linear, bounded-by-file rewalk") and one-shot in practice, so S4 — but it makes the engine's "demux ≤ 64 events per tick" bound illusory during the initial skip.
Confidence: HIGH — Verification: read; the bound language is in host.rs module docs (host.rs:11-13).
Severity: S4 (documented, one-shot, file-bounded).

### F-007 [S4]
Where: apps/cli/src/demux_host.rs:555
Code:
```rust
        if !self.bases.iter().any(|(id, _)| *id == packet.track) {
            return false;
        }
```
What is wrong: linear scan of the bases vec per packet — bounded by track count (≤ 8 in practice), trivia, but this is the per-packet hot path.
Confidence: HIGH — Verification: read.

### F-008 [S3]
Where: apps/cli/src/demux_host.rs:196-200 (doc on `audio_geometry`)
Code:
```rust
    /// (ADTS: the config rides in-band and the demuxer exposes no
    /// blob — that family stays honestly limb-less until its
    /// harvest grows one), or for a blob that does not parse: the
    /// session then opens limb-less, never a guess.
```
What is wrong: the doc presents limb-less-ness as a property of what the DEMUXER exposes, but the MP4 case (F-001) shows it is equally a property of what this function PARSES — a blob that exists and does not parse here reads as "no geometry source". The "never a guess" honesty framing obscures an implementation gap.
Confidence: HIGH — Verification: read + mp4.rs blob verified.
Severity: S3 (misleading core-seam documentation on the exact family the project's litmus test fails).

### F-009 [S4]
Where: apps/cli/src/demux_host.rs:296-305
Code:
```rust
    let canonical: PathBuf = std::fs::canonicalize(path).map_err(|err| {
```
What is wrong: `std::fs::canonicalize` is platform-divergent (Windows returns `\\?\`-extended paths; macOS/ Linux resolve symlinks). The wave-30 fix note claims the shared constructor handles this — UNVERIFIABLE in this sandbox (no Windows run). Flagged for the wave-20 Windows trace, not scored as a defect here.
Confidence: LOW (UNVERIFIED) — Verification: read only.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const PRODUCT_QUEUE_* | 71-73 | REAL | S3 | mirror verified accurate today (F-002) |
| `queue_plan_from_product_limits` | 85-97 | REAL | — | typed reject, never clamp (tested both ends) |
| `wire_error` | 108-124 | REAL | S4 | stringly prefix strip (F-005); chain-root appended |
| `access_refusal` | 130-138 | REAL | — | redacted-name attachment, idempotence check |
| `TrackFact::span_end_us` | 175-180 | REAL | — | convert-fail → None |
| `TrackFact::audio_geometry` | 202-220 | SHALLOW | S3 | 3-of-N codec coverage; AAC ASC unparsed (F-001); OpusHead version unvalidated (F-003) |
| `MediaFacts::duration_us` | 239-241 | REAL | — | max of declared extents |
| `MediaFacts::fact_for` | 245-247 | REAL | — | linear find |
| `file_uri` | 296-305 | REAL | — | canonicalize → shared tail; platform note F-009 |
| `file_uri_from_canonical` | 322-352 | REAL | — | is_file pre-check, UTF-8 refusal, UNC refusal bridged |
| `open_source` | 362-369 | REAL | — | prod entry, no fetch seam |
| `open_source_with` | 378-459 | REAL | S4 | three lanes; blocked-scheme refusal names the lane (tested); probe stage between access and containers |
| `open_http_source` | 474-500 | REAL | S4 | double parse + unreachable (F-004); production ureq/rustls path only reachable when fetch=None (the test seam substitutes transport, everything else real) |
| `SelectDemuxer::new` | 529-543 | REAL | — | |
| `SelectDemuxer::before_skip` | 548-562 | REAL | S4 | unjudgeable stamps pass (honest); linear base scan (F-007) |
| `SelectDemuxer::start` | 566-576 | REAL | — | kind filter; `_ => false` drops subtitle/data (documented convention) |
| `SelectDemuxer::next_event` | 578-590 | REAL | S4 | skip loop can walk far in one call (F-006) |
| `SelectDemuxer::seek` | 592-594 | REAL | — | pure query forwarded |
| `SelectDemuxer::seekability` | 596-604 | REAL | — | forwarded, never invented |
| `SelectDemuxer::reanchor` | 606-618 | REAL | — | watermark cleared at landing (the backward-seek starvation fix — correct and documented) |
| test `fixture` | 625-629 | REAL | — | |
| test `parent_fixture_opens_with_expected_facts` | 634-674 | REAL | — | end-to-end through real access+probe; pins facts incl. frame_ns 33,333,333..334 |
| test `start_time_clamps_to_duration` | 684-724 | REAL | — | behavioral watermark proof (0 packets survive) |
| test `select_demuxer_is_generic_over_a_non_avi_demuxer` | 733-775 | REAL | — | proves generalization over WAV |
| test `garbage_file_is_a_probe_stage_refusal` | 780-798 | REAL | — | content-probe truth |
| test `magic_broken_derivative_is_refused_at_probe` | 803-819 | REAL | — | |
| test `missing_file_is_inaccessible` | 823-838 | REAL | — | |
| test `blocked_scheme_urls_refuse_naming_the_lane` | 845-866 | REAL | — | rtsp refusal, scheme named |
| test `http_url_opens_through_the_ranged_resource` | 876-921 | REAL | — | real loopback socket, ranged requests asserted |
| test `file_url_sources_open_like_path_sources` | 931-958 | REAL | — | empty-authority + localhost forms |
| test `queue_plan_join_maps_product_limits_or_refuses_typed` | 969-998 | REAL | — | floor/ceiling refusals |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parent_fixture… | YES | exact codec/facts/duration pins through the real path |
| start_time_clamps… | YES | zero-packet assertion breaks with any watermark leak |
| select_demuxer_is_generic… | YES | kind filter + skip behavior pinned over a second container |
| garbage/magic-broken/missing | YES | stage+code asserted per refusal |
| blocked_scheme… | YES | code+stage+message content |
| http_url_opens… | YES | requires a real socket, ranged header, wav facts |
| file_url_sources… | YES | canonical-URI equality across spellings |
| queue_plan_join… | YES | both refusal ends asserted |
No tautologies found. Not executed here (no cargo); W00 baseline corroborates the suite runs (1751/2/25).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "probe between access and demux" (doc 03 §4.2 shape) | module docs 8-27 | REAL | open_source_with 442-458 |
| "the bounded, non-consuming window read" | module docs 21-22 | REAL | probe_stream restores position (media-probe lib.rs:204-209) |
| extension route | signature/mod.rs:51-55 "content always wins" | REAL (content-only; extension NEVER consulted) | grep: only error.rs reads meta.extension |
| "never a guess" for limb-less sessions | 196-200 | MISLEADING | F-001/F-008: the MP4 AAC blob exists and is declined |
| http(s) opens through ranged resource (w31.3) | 461-467 | REAL | loopback test with Range assertions (UNVERIFIED against the production rustls client — no network run in sandbox) |

## 7. Dead / duplicated / unreachable code

- `open_source_with`'s `fetch` seam: reachable only from tests; production passes `None` — a deliberate, documented test seam in the production path (acceptable, but it is production code carrying a test-only parameter).
- No dead functions found; every symbol has callers (host.rs, containers/*, tests).

## 8. Dependencies & flags

No feature gates. Cross-crate seams: `input` (FileAccess), `media_probe` (builtins), `streaming` (ranged/schemes/mockhttp), `codecs::audio::vorbis::parse_identification` (no local grammar copy — verified), `playback_engine::{QueuePlan, wire_stage, wire_error_code}`.

## 9. Open questions for the next agent

- The litmus failure decomposition: with no h264/aac decoders (bench.rs:334), the frozen clock is over-determined — does the desktop shell's wall pump + limb-less wall clock produce a moving position for MP4-AAC despite zero decodes (host.rs `position_us` latches `last_presented_ns` → None → anchor → 0)? Wave 20 A05/A06 should pin which single fix (decoder vs geometry) moves the clock.
- `streaming::ranged::RangedConfig::default()` — timeouts/budgets live in the streaming crate (Wave 10); the open path sets none itself.
- Windows `\\?\` canonical paths through `SourceUri::from_file_path` (F-009) — needs a Windows run.
