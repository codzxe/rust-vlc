# AUDIT: apps/cli/src/demux_host.rs#part1 (lines 1–499)
| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-21 |
| Lines / functions | 499 (of 999; chunk boundary pinned from audit/waves/wave-00/assignments.tsv:770: part1 = 1–499 [this audit], part2 = 500–999 [W13-A03, owns SelectDemuxer + the tests module]) / 15 items |
| File verdict | REAL (the open path's front half; executed end-to-end by baseline integration tests and MP4 smoke transcripts) |
| Reachable from shipped entry point? | YES (call chain: `lumen` main.rs:9 → run.rs:126 `Some(CliCommand::Open{..})` → dispatch.rs:82 `host.open_session` → host.rs:904 `open_source(...)` → this part's `open_source_with` → containers::open; `queue_plan_from_product_limits` ← host.rs:971; `wire_error` ← every container host. `bench.rs:270` decode-bench also calls `open_source`.) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:4 |
| Commands run | Read demux_host.rs 1-250/251-500 (full part); sed -n 63-66/213-218/377-384/478-495 (quote verification); sed -n 195-205 product.toml (limits values + line numbers); rg `queue_plan_from_product_limits|PRODUCT_QUEUE_PACKETS` apps/* (callers: host.rs:971-973 production; 970-992 tests [part2]); rg `fn redacted` crates/media-types/src/ids.rs:243 + Read 236-265 (redaction rules); Read commands/src/error.rs:141-167 hit (`CommandError::new` routes message through `redact` — REFUTES a suspected path-leak finding); Read exit.rs 1-90 (single error printer + V8 claim); grep baseline orchrun-test-workspace-nofailfast.log.txt:2922-2935/2935-2960/3029-3050 (command_surface 7/7, container_conformance 19/19, unsupported_path 16/16 — all ok); rg test bodies in apps/cli/tests/unsupported_path.rs:88-92,303-308 (redaction + directory tests). |

## 1. One-paragraph verdict
This is the front of the hostile-question lane — argv → canonical `file:` URI → access → probe → container registry — and it is the best-evidenced code audited so far in this wave: 16/16 unsupported-path tests, 19/19 container-conformance family lanes, and the MP4 smoke transcripts all execute exactly these functions, and the MP4's failure point is provably NOT here (tracks 2 ready, duration, MediaInfoReady all delivered before the downstream decode refusal). The engineering is disciplined: one shared canonical tail for the three file spellings, typed refusals at every stage, redaction enforced centrally by `CommandError::new` (verified in commands/src/error.rs:163-167 — my initial suspicion that `canonical.display()` at 327/348 leaks machine paths is REFUTED by that constructor and by the passing `error_messages_redact_user_paths` test). What remains are four hygiene defects: an RFC-7845 version byte never checked in `audio_geometry`, a needless `#[allow(clippy::too_many_arguments)]` on a 5-argument function, a stale `product.toml:199-200` line citation (values right, lines wrong), and a duplicated URL parse with an `unreachable!` in the http lane.

## 2. What actually works (verified)
- The whole open path ran in baseline: `tests/unsupported_path.rs` 16/16 ok (orchrun-test-workspace-nofailfast.log.txt:3029+) — covers `missing_file_is_inaccessible` (file_uri canonicalize failure), `directory_source_is_inaccessible` (the is_file pre-check, exit 66), `blocked_scheme_url_is_unsupported_for_host` (the scheme refusal at 428-438), `file_url_opens_while_blocked_schemes_refuse` (the file: URL lane 418-426), `http_url_opens_over_a_real_loopback_socket` (open_http_source + the mock HttpFetch seam over a REAL loopback socket), `error_messages_redact_user_paths` (redaction chain).
- `tests/container_conformance.rs` 19/19 ok (log:2935+) — every wired family lane (mp4, mp4_h264, mkv, es_mp3/mp2/flac, ogg, adts, …) opens through this part's `open_source`.
- MP4 smoke (baseline INDEX.md §MP4 SMOKE / CORRECTION v2): `lumen open sample.mp4` → `event: tracks 2 ready`, `TimingDurationKnown`, `MediaInfoReady` — probe + registry + facts pre-pass (this part) succeed; the 4:4:4 refusal `h264.sw.unsupported` and the Error→Idle→exit-0 behavior are downstream (pipeline_host/provider — W13 cross-refs), not here.
- Constants mirror product.toml faithfully: `queue_packets_audio = 1024` / `queue_packets_video = 512` (product.toml:203-204) == `PRODUCT_QUEUE_PACKETS_AUDIO/VIDEO` (71-73), consumed at host.rs:971-973.
- `wire_error`'s stage-prefix strip matches `EngineError`'s Display format exactly (`"[{stage}] {summary} ({code})"`, media-types/src/error.rs:294) — verified by read.

## 3. Findings

### F-001 [S4] — `audio_geometry` accepts OpusHead without checking the RFC 7845 version byte
Where: apps/cli/src/demux_host.rs:213-218
Code:
```
        if data.starts_with(b"OpusHead") && data.len() >= 19 {
            // RFC 7845 §5.1: channels at byte 9 of the 19-byte
            // fixed identification header; the output rate is
            // always 48 kHz (§4.2).
            return Some((u32::from(data[9]), 48_000));
        }
```
What is wrong: RFC 7845 §5.1 requires `Version` (byte 8) to be 1; a blob with any other version is not a valid identification header. The check skips it: any ≥19-byte blob starting with the magic is treated as Opus geometry, so a malformed/corrupt `extra_data` (or a future format that embeds the magic) yields a wrong channel count for the audio limb sizing.
Why it matters: `audio_geometry` sizes the output limb (`audio_geometry()` is the "generic audio-limb truth" per its own doc); a wrong channel count mis-sizes buffers or the wall-clock limb for the whole session. Blast radius is small today because the blob is minted by our own Ogg harvest, whose demuxer presumably validates — but this function advertises itself as the defensive parser for the blob, not for our demuxer's goodwill.
What correct looks like: `if data.starts_with(b"OpusHead") && data.len() >= 19 && data[8] == 1 {`
Confidence: HIGH — Verification: read (RFC 7845 §5.1 layout; byte 8 is Version).

### F-002 [S4] — Needless `#[allow(clippy::too_many_arguments)]` on a 5-parameter function
Where: apps/cli/src/demux_host.rs:377-384
Code:
```
#[allow(clippy::too_many_arguments)]
fn open_source_with(
    source: &commands::catalog::types::SourceSpec,
    start_time_us: Option<i64>,
    no_video: bool,
    no_audio: bool,
    fetch: Option<std::sync::Arc<dyn streaming::fetch::HttpFetch>>,
```
What is wrong: clippy's `too_many_arguments` default threshold is 7; this function has 5 parameters. The allow suppresses nothing on any current clippy version (baseline #5/#6 clippy clean — the lint was never firing here).
Why it matters: A dead `#[allow]` is exactly the pattern the checklist flags — it accretes, hides future real lints if the threshold is lowered, and teaches readers that allows are decoration. The repo's own gate culture (`unsafe-lints 39/39 inherit`, INDEX.md) treats allows as load-bearing.
What correct looks like: Delete the attribute.
Confidence: HIGH — Verification: read (parameter count) + baseline clippy clean.

### F-003 [S4] — Stale line citation: "product.toml:199-200" is actually product.toml:203-204
Where: apps/cli/src/demux_host.rs:63-66 vs product.toml:203-204
Code:
```
/// The product's queue packet caps — the `[limits]` figures
/// product.toml ships (`queue_packets_audio = 1024`,
/// `queue_packets_video = 512`, product.toml:199-200). The CLI loads
```
```
queue_packets_audio = 1024      # bounded audio packet queue (doc 03 §4). schema 64..=8192
queue_packets_video = 512       # bounded video packet queue. schema 64..=4096
```
What is wrong: The values are correct, but the cited lines are off by four: the `[limits]` keys live at product.toml:203-204 (sed-verified; 199-200 is the blank line + `[limits]` header).
Why it matters: This is a load-bearing cross-reference in a constant-mirror whose whole justification is "mirrors the shipped values" — a verifier following the citation lands on the wrong lines and must re-derive trust. The same doc block is the anchor for the future product-config join (`[PROPOSAL]`).
What correct looks like: `product.toml:203-204`.
Confidence: HIGH — Verification: sed -n 195-205p product.toml.

### F-004 [S4] — http lane parses the same URL twice, second parse guarded by `unreachable!`
Where: apps/cli/src/demux_host.rs:480-493
Code:
```
    streaming::fetch::Url::parse(url).map_err(|err| {
        CommandError::new(
            Stage::Input,
            ErrorCode::InvalidArgument,
            format!("the http(s) URL was refused by the fetch port: {err}"),
        )
        .with_detail("scheme", url.split(':').next().unwrap_or("?"))
    })?;
```
```
        None => streaming::schemes::SchemeRegistry::new().open_http(
            &streaming::fetch::Url::parse(url)
                .unwrap_or_else(|err| unreachable!("the URL parsed one call ago: {err}")),
```
What is wrong: The URL is parsed once for validation (discarding the result), then parsed AGAIN in the production branch. The `unreachable!` is sound today (`Url::parse` is pure over the same `&str`), but it is a panic path whose correctness depends on an invariant ("parse is deterministic") that the code re-establishes by re-doing the work instead of keeping the value.
Why it matters: Every future edit that makes the first parse conditional, or `Url::parse` stateful, turns line 493 into a reachable panic in the production http lane. Parse-once-and-reuse removes both the duplicate work and the panic.
What correct looks like: `let parsed = streaming::fetch::Url::parse(url).map_err(...)?;` then pass `&parsed` to `open_http`.
Confidence: HIGH — Verification: read.

## 4. Per-function table (part1 items only; SelectDemuxer and the tests module are part2, W13-A03)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PRODUCT_QUEUE_PACKETS_AUDIO/VIDEO` (consts) | 71-73 | REAL | S4 (F-003) | Values match product.toml:203-204; citation lines stale; honest `[PROPOSAL]` for the config join. |
| `queue_plan_from_product_limits` | 85-97 | REAL | — | Thin typed wrapper over `QueuePlan::from_product_limits`; reachable (host.rs:971); error path tested in part2 (lines 984-992). |
| `wire_error` | 108-124 | REAL | — | Single EngineError→CommandError seam; stage-prefix strip matches Display format (media-types error.rs:294); root-cause append; `code` detail attached. Used by every container host. |
| `access_refusal` | 130-138 | REAL | — | Dedupes the redacted name; only called on the file/file-URL lanes (396, 420, 424). |
| `TrackFact` (struct) | 144-170 | REAL | — | Data type; `extra_data` doc contract matches es.rs/mp4.rs usage. |
| `TrackFact::span_end_us` | 175-180 | REAL | — | Ticks→ns→µs via NANO_BASE; truncating division (toward zero) — irrelevant for normal non-negative spans. |
| `TrackFact::audio_geometry` | 202-220 | REAL | S4 (F-001) | PCM passthrough; Vorbis via the codec crate's own parser (no local grammar); OpusHead inline parse misses the version byte. |
| `MediaFacts` (struct) + `duration_us` + `fact_for` | 225-247 | REAL | — | Max-of-spans duration; linear find — fine at track counts. |
| `OpenOptions` / `OpenedMedia` (structs) | 256-282 | REAL | — | Pure data; the `Box<dyn Demuxer>` seam documented. |
| `file_uri` | 296-305 | REAL | — | canonicalize failure → 66; executed (`missing_file_is_inaccessible` ok, log:3029+). |
| `file_uri_from_canonical` | 322-352 | REAL | — | is_file + UTF-8 + constructor refusals; raw `canonical.display()` in messages is safe ONLY because `CommandError::new` redacts (commands/src/error.rs:167) — verified by `error_messages_redact_user_paths` ok. Executed (directory/file-URL tests ok). |
| `open_source` | 362-369 | REAL | — | Public wrapper; the argv→probe→registry entry. |
| `open_source_with` | 378-459 | REAL | S4 (F-002) | Three access spellings → one tail; scheme refusal typed (428-438); probe non-consuming; dispatch to registry at 458. Executed by 35 integration tests + smoke. |
| `open_http_source` | 474-499 | REAL | S4 (F-004) | Real ureq/rustls transport by default; mock seam for tests over a real loopback socket (test ok). Double parse + `unreachable!`. (Closing brace at 500 is part2's first line — boundary noted.) |

## 5. Test honesty
No inline tests in part1 (the demux_host tests module lives in part2, lines 635-998 — W13-A03). Part1's behavior is covered externally:
| Test (external) | Would it fail if the code were broken? | Reason |
|---|---|---|
| unsupported_path.rs 16 tests (baseline ok, log:3029+) | YES | Each lane and refusal has a dedicated binary-level test: missing file, directory, blocked scheme, file: URL, loopback http, garbage probe, grammar failures, redaction. |
| container_conformance.rs 19 tests (log:2935+) | YES | Every family lane opens through `open_source`; a registry/probe regression fails the corresponding lane. |
| error_messages_redact_user_paths (unsupported_path.rs:303-308) | YES | Asserts the redacted form appears AND `/home/someone` does not — a negative assertion, not a tautology. |
| MP4 smoke transcripts (INDEX.md) | YES | `tracks 2 ready` / `TimingDurationKnown` / `MediaInfoReady` prove the probe+registry+facts pre-pass for the exact litmus file. |

## 6. Doc claims touching this part
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a `file:` URL source resolves through `input` and runs the same canonical tail" (open `C:\x.mp4` == `open file:///C:/x.mp4`) | demux_host.rs:10-19, 313-314 | REAL | `file_url_opens_while_blocked_schemes_refuse` ok (log:3029+); code path 418-426 shared via `file_uri_from_canonical`. |
| "an `http(s)` URL opens through the streaming crate's ranged resource" | demux_host.rs:13-18, 461-467 | REAL | `http_url_opens_over_a_real_loopback_socket` ok (log:3029+); NOTE: this is the dependency the architecture gate flags — baseline #11: "lumen-cli depends on streaming, not declared in boundaries.toml" (standing S-level, recorded; not re-filed here). |
| "every other scheme is the honest host-capability refusal naming the blocked streaming lane" | demux_host.rs:18-19 | REAL | `blocked_scheme_url_is_unsupported_for_host` ok; code 428-438. |
| "The engine's own `start()` call then returns the cached tracks (the demuxer's idempotent re-start)" | demux_host.rs:32-33 | UNVERIFIED (cross-agent) | Demuxer idempotency lives in the demux crates (W-demux agents); CLI side just calls `start()` at pre-pass. |
| "[limits] figures product.toml ships … product.toml:199-200" | demux_host.rs:63-66 | REAL values / FALSE citation (F-003) | sed product.toml:203-204. |
| `wire_error` "the single bridge … message through the command layer's redaction-safe constructor" | demux_host.rs:48-51, 99-106 | REAL | commands/src/error.rs:163-167 `redact(&message.into())`; redaction test ok. |

## 7. Dead / duplicated / unreachable code
- No dead code in part1. The retired `encode_path` mentioned in `file_uri`'s doc (288-289) is genuinely absent (rg over the file: no other definition).
- Duplication: the scheme-detail idiom `url.split(':').next().unwrap_or("?")` appears twice (415, 486) — micro-duplication, folded into F-004's neighborhood, not filed separately.

## 8. Dependencies & flags
- `playback_engine` (QueuePlan, wire_stage/wire_error_code), `media_probe` (registry + probe_stream), `input` (FileAccess, resolve_file), `streaming` (fetch/ranged/schemes — the boundary-gate violation, baseline #11), `codecs::audio::vorbis::parse_identification` (no local grammar copy — verified), `plugin_api`, `media_types`, `commands`. No feature gates. No unsafe. The `HttpFetch` injection seam is test-only-but-honest (a real socket, per its doc and the loopback test).

## 9. Open questions for the next agent
- W13-A03 (part2): the `SelectDemuxer` watermark/reanchor rules carry the `--start-time` and seek claims of this part's module doc (34-44) — verify the never-drop-on-Missing and watermark-clear-on-reanchor behaviors there.
- Engine/pipeline owners: module doc line 36-44 claims in-session seeks clear the skip watermark "at the landing" — trace in the engine's native seek flow (card 3.A1) whether a backward seek below `--start-time` actually escapes starvation (the doc admits the risk).
- Demux agents: `TrackFact.span_end_ticks` consumers rely on `span.end.timestamp().ok()` — confirm demuxers mint `Present` span ends, not `Missing`, for the duration math (`duration_us`) to be non-trivially correct.
