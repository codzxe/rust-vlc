# apps/cli/src/demux_host.rs — W13-A01 audit

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A01 |
| Date | 2026-11-16 |
| Lines | 999 (≈20 functions + a 10-test module) |
| File verdict | **REAL** — the open path, wrapper, facts pre-pass and error bridge are all real and executed; the harvest omits one field the snapshot layer needs (F-001) and mirrors two product constants (F-002) |
| Reachable from shipped entry point | YES — two entry chains: `lumen open …` → run.rs:132 → dispatch.rs:81 → host.rs:725 `open_source` (362-369); `lumen decode-bench …` → run.rs:144 → bench.rs:270 `open_source` |
| Findings | F-001 [S3], F-002 [S4], F-003 [S4] |

## 1 One-paragraph verdict

The open path is real and it is the player's single shared spine: file path or `file:`/`http(s)` URL → access stream → bounded probe → the container-host registry — and the executed MP4 run proves the whole chain (probe named `mp4`, registry constructed the demuxer, the facts pre-pass harvested the codec-config blobs, the engine decoded both limbs). The hostile question about the avcC harvest is answered YES with citations: the harvest is real code reading a real demuxer accessor, and the end-to-end proof is that the same binary decoded h264 from that blob. The one substantive omission in the harvest: `MediaFacts` collects tracks, codec ids, bases, PCM geometry, extra_data blobs, dimensions — everything except the demuxer's `SeekSupport`, which is why host.rs:1128 can hard-code `seekable: true` without any truth source contradicting it in-tree. The error bridge is single-sourced and the wrapper's semantics (selection, skip watermark, reanchor-forwarding) are as documented.

## 2 What actually works (verified)

- **The full open chain, cited** (the hostile question, answered with the executed evidence): argv `open audit/runs/baseline/sample-h264-aac.mp4` → run.rs:132 `repl_mode` → run.rs:193 `parse::open_command` → dispatch.rs:81-87 → host.rs:725 `open_source(&source, …)` → demux_host.rs:392-397 (`file_uri` → `FileAccess::open`) → demux_host.rs:444-448 (`media_probe::probe_stream` over the builtin registry) → demux_host.rs:458 `crate::containers::open(&container, stream, &opts)` → containers/mod.rs:67 `"mp4" => mp4::open(stream, opts)`. Executed proof: the orchestrator's smoke run opened the file, answered `tracks 2 ready`, and played to `Ended (100.0%)`, EXIT=0 (audit/runs/baseline/mp4-smoke-open-advance.txt); my own live runs of the same binary answered `container: mp4`, `#0 video h264 320x240`, `#1 audio aac 1ch 44100Hz`.
- **The avcC/config_blob harvest is REAL** (the hostile question): the facts pre-pass structure lives here (`TrackFact.extra_data`, 158-165: "Out-of-band codec initialization bytes (the esds ASC, the ALAC magic cookie, the Vorbis three-header chain, the ADPCM WAVEFORMATEX body)"), and the mp4 family's harvest is containers/mp4.rs:186-226 `harvest_facts` → :190 `demuxer.sample_entry_bytes(track.id)` — a real `pub fn` on `Mp4Demuxer` (crates/demux/src/mp4/mod.rs:669) — → :237 `"h264" => find_child_box(entry, b"avcC")` (config record payload without the box header, ffmpeg's extradata shape) → :192 `config_blob(codec, entry)` into `TrackFact.extra_data` → consumed by pipeline_host.rs:184-192 `seeded_config` → :328 `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)`. The demuxer mints the `h264` descriptor for `avc1|avc2|avc3|avc4|avcp` entries (crates/demux/src/mp4/mod.rs:440), so harvest key and dispatch arm line up. End-to-end proof: the executed decode itself (the h264 limb configured from the avcC blob and presented to Ended; conformance test container_conformance.rs:426 asserts no `cli.codec_unavailable`). My own python box-walk of the same file (45,682 bytes, video stsz 75 samples / timescale 12800, audio stsz 131 / 44100) confirms the trak structure the harvest walks.
- **The three access spellings are one pipeline**: `File` path (392-397), `file:` URL through `input::resolve_file` + the shared canonical tail (418-425), `http(s)` through `RangedResource` on the production ureq transport (427, 474-500) — all feed the same probe + registry. The http lane is pinned by `http_url_opens_through_the_ranged_resource` (876-921, a REAL loopback socket with Range requests asserted) in the 1001-pass suite; the file-URL lane by `file_url_sources_open_like_path_sources` (931-958).
- **The wrapper's semantics**: `SelectDemuxer::start` filters disabled kinds (566-576); `before_skip` drops only stamps that provably predate the watermark and passes unjudgeable ones (548-562); `reanchor` forwards the demuxer's compute-then-commit landing and clears the skip watermark so a backward seek below `--start-time` does not starve (606-618); `seekability` forwards the inner truth, never invents (596-604).
- **The error bridge** `wire_error` (108-124) is the single EngineError→CommandError conversion (stage/code via the engine's published mappings, chain-root appended) — every executed typed refusal in my live runs (`[session.not-found]` chapter refusal, `plugin.demux.reanchor.unsupported` seek failure) came through it with the right stage tags.

## 3 Findings

### F-001 [S3] The facts harvest omits seekability — the one field the snapshot layer needs to answer honestly
Where: demux_host.rs:224-233
```rust
pub struct MediaFacts {
    /// The demuxed track list (unfiltered — selection truth is the
    /// engine's wired-limb answer).
    pub tracks: Vec<Track>,
    /// Per-track facts, parallel to `tracks`.
    pub facts: Vec<TrackFact>,
    /// The probe's format id (`avi`).
    pub container: String,
}
```
Wrong: `SelectDemuxer::seekability()` (596-604) forwards a REAL per-container truth (`Fast { indexed }` for AVI idx1, `SlowLinear` for fragmented mp4, `None` for the ts family), but nothing captures it at open time and nothing carries it in `MediaFacts` — so the class-S `media.get-info` half (host.rs:1104-1135) has no source for `seekable` and hard-codes `true` (host.rs:1128, host.rs audit F-002). The open path is the composition root that reads the demuxer BEFORE the engine owns it ("the CLI is the composition root, so it reads the demuxed track list … BEFORE the demuxer moves into the engine", module doc 29-33) — the same window in which seekability should have been harvested. After the engine takes ownership the host can still ask the engine, but the facts struct is the established carrier and it is missing the field.
Correct: add `seekable: SeekSupport` (or a bool fold) to `MediaFacts`, harvest it in each container module's `open()` where the demuxer is at hand, and let `media_info` report it.
Confidence: high — struct fully quoted; `seekability` exists and is forwarded; no `seekable` field anywhere in the facts struct (grep verified single assignment site at host.rs:1128).
Severity S3 — it is the root cause of a wrong user-facing answer on exactly the unseekable families.

### F-002 [S4] Product queue caps mirrored as hand constants
Where: demux_host.rs:71-73
```rust
pub const PRODUCT_QUEUE_PACKETS_AUDIO: u64 = 1024;
/// The video half of the same mirror.
pub const PRODUCT_QUEUE_PACKETS_VIDEO: u64 = 512;
```
The `[PROPOSAL]` block (63-70) honestly records the product-config load as the rebrand-propagation card's join. Registered debt, not a lie. S4.

### F-003 [S4] One `unreachable!` in the "never panics" app
Where: demux_host.rs:490-495
```rust
        None => streaming::schemes::SchemeRegistry::new().open_http(
            &streaming::fetch::Url::parse(url)
                .unwrap_or_else(|err| unreachable!("the URL parsed one call ago: {err}")),
            config,
        ),
```
Wrong: the app's top-level claim is "Std-only, no panics: every failure is a typed `CommandError`" (main.rs doc 3-5). This arm re-parses a URL that `Url::parse` accepted 10 lines earlier (480) and asserts its impossibility with `unreachable!`. `Url::parse` is a pure function of the same input, so the double-parse cannot disagree in practice — but the correct never-panic shape is to parse once and carry the value. S4 (provably dead today; a contract violation in principle).

## 4 Per-function table

| Function (line) | Role | Verdict |
|---|---|---|
| `queue_plan_from_product_limits` (85-97) | product caps → engine QueuePlan; typed refusal outside schema range | REAL — pinned (968-998) |
| `wire_error` (108-124) | THE EngineError→CommandError bridge | REAL — every live refusal rode it |
| `access_refusal` (130-138) | attach redacted URI name to access refusals | REAL |
| `TrackFact::span_end_us` (175-180) | declared extent in µs via base conversion | REAL |
| `TrackFact::audio_geometry` (202-220) | generic limb truth: pcm pair, vorbis ID parse, OpusHead RFC 7845 | REAL — live `aac 1ch 44100Hz` via the mp4a declaration pair (containers/mp4.rs:200-203) |
| `MediaFacts::duration_us` (239-241) | greatest declared track extent | REAL — `3.000000s` live |
| `MediaFacts::fact_for` (245-247) | fact row by track id | REAL |
| `file_uri` (296-305) | canonicalize → inaccessible typed on failure | REAL — pinned (822-838) |
| `file_uri_from_canonical` (322-352) | shared canonical tail: is_file pre-check, UTF-8 refusal, constructor errors bridged | REAL — pinned (931-958) |
| `open_source` (362-369) | public open entry (no test seam) | REAL — executed |
| `open_source_with` (378-459) | the three access lanes + probe + registry dispatch | REAL — executed; F-003's unreachable lives here |
| `open_http_source` (474-500) | ranged resource open over production ureq / injected fetch | REAL — pinned (876-921) |
| `SelectDemuxer::new` (529-543) | wrap started demuxer + bases + watermark + flags | REAL |
| `SelectDemuxer::before_skip` (548-562) | provable-predated stamp drop; unjudgeable pass-through | REAL |
| `SelectDemuxer::start` (566-576) | track list minus disabled kinds (subtitle/data filtered out) | REAL |
| `SelectDemuxer::next_event` (578-590) | loop-drop packets before the watermark | REAL — pinned (684-724) |
| `SelectDemuxer::seek` (592-594) | forward pure query | REAL |
| `SelectDemuxer::seekability` (596-604) | forward inner truth — the un-harvested field (F-001) | REAL |
| `SelectDemuxer::reanchor` (606-618) | forward landing + clear watermark | REAL — mp4's inner default refusal surfaces here (live `plugin.demux.reanchor.ununsupported`) |
| tests (622-999) | 10 tests: parent fixture facts, start-time clamp, generic wrapper over WAV, garbage probe, magic-broken, missing file, blocked scheme, http loopback, file URL, queue plan | REAL — strong, in the 1001-pass suite |

## 5 Test honesty

- In-file tests (622-999) are genuinely strong: they pin the parent AVI facts, the start-time clamp with a real packet-level assertion (`start_time_clamps_to_duration`, 684-724), the wrapper's genericity over a non-AVI demuxer (732-775), probe-stage refusals for garbage and magic-broken files, the blocked-scheme naming, the http lane over a REAL loopback socket with Range-request proof, the file-URL canonical equivalence, and the queue-plan bounds.
- apps/cli/tests/container_conformance.rs drives `open_source` through the REAL binary for every wired family (mp4 lane at :426) — in the 1001-pass suite.
- apps/cli/tests/unsupported_path.rs covers the refusal shapes (exit-65/66/69 families) — in the suite.
- NOT covered: any assertion tying `media.get-info`'s `seekable` to the demuxer's actual seek support (the F-001 gap has no test).

## 6 Doc claims

- docs/feature-matrix.toml:1288 (streaming-http evidence): "w31.3: apps/cli/src/demux_host.rs open_http_source (the open path's http arm, the production ureq transport + RangedResource) + the loopback e2e through the REAL binary … + the in-process mock proof (demux_host.rs http_url_opens_through_the_ranged_resource)" — CONSISTENT: the cited function (474-500) and the cited in-process test (876-921) exist as described.
- docs/feature-matrix.toml:152/:428 (mp4/h264 e2e) depend on this file's `open_source` + the mp4 harvest — CONSISTENT per the executed run.
- The module doc (1-52) describes the generalized open path accurately (three lanes, registry, wrapper, reanchor forwarding); the doc's `TrackFact` bullet at 160-165 ("`None` for AVI") is accurate for AVI and superseded by the per-container harvests the registry owns.

## 7 Dead/duplicated/unreachable

- `open_source_with`'s `unreachable!` (F-003) — the one panic-shaped arm in the app.
- `PRODUCT_QUEUE_*` mirrors (F-002) — registered duplication of product.toml:199-200.
- Nothing else dead: every function is reachable from `open`/`decode-bench`/tests.

## 8 Dependencies & flags

`input`, `media-probe`, `media-types`, `plugin-api`, `playback-engine` (wire mappings), `commands`, `demux` (test-only, via `demux::wav`/`demux::avi::test_support`), `streaming` (production ureq transport + mockhttp test seam). No codec features are consumed here — the codec posture enters at pipeline_host.rs (`codec-h264-sw` default-on, crates/codecs/Cargo.toml:47-53; apps/cli/Cargo.toml resolves default features). The streaming dependency is unconditional in apps/cli/Cargo.toml (`streaming = { path = "../../crates/streaming" }`), matching the always-compiled http lane.

## 9 Open questions

- Which wired families answer `SeekSupport::None` or refuse `reanchor`? Live-observed: mp4 (reanchor default refusal). Conformance header names the ts family (`demux.ts.seek.unsupported`). A per-family seekability matrix would make F-001's fix trivial and host.rs F-002's blast radius known — UNVERIFIED beyond those two data points (the other container modules are outside my file set).
- `SelectDemuxer::start` filters subtitle/data tracks out of the engine's track list entirely (573 `_ => false`) while the module doc (511-516) says the engine drops their packets "at the unwired-track boundary" — consistent because the engine never sees the tracks, but the doc's two sentences describe the packet-drop from the engine's perspective. Wording only.
