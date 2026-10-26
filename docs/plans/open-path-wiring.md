# Open-Path Wiring — Investigation & Seam Design (6.OPW-0)

Status: COMPLETE (investigation card — zero code changes; see §6).
Branch: `wip/p6-opw0`. Card: 6.OPW-0 (Phase 6, Wave 1).
Method: every claim below was verified in this worktree this session; file:line cites are from
`wip/p6-opw0` at base `f3e5b8c` (merge p6-w1 6.A1b).

**The one-paragraph verdict.** The open path is *already* container-agnostic below the probe and
above the demuxer: `input` access and `media-probe` are generic, `playback-engine` consumes a
`Box<dyn plugin_api::Demuxer>` plus a host `DecoderProvider`/`FrameConsumer`, and the CLI's
`HostDecoderProvider` already dispatches **every** Phase-5 codec by registry descriptor. Exactly
**one** site is AVI-hardcoded — the format gate in `apps/cli/src/demux_host.rs:298-310` and the
AVI-specific pre-pass behind it. The seam is therefore a per-container *construction + facts
harvest* adapter behind a single registry match, plus a one-time generalization of the AVI skip
wrapper and a one-time `extra_data` seeding rule in the decoder provider. No new trait, no new
crate.

---

## 1. The current open path (verified map)

### 1.1 Entry → dispatch

| Hop | Site (file:line) | What happens |
|---|---|---|
| binary `main` | `apps/cli/src/main.rs:19` | `lumen_cli::run::run(&argv, stdin, stdout, stderr, interactive)` — thin main, everything is the library. |
| mode split | `apps/cli/src/run.rs:66-108` | `parse::parse_argv` (:73); three modes: argv one-shot (:145→`one_shot` :151), REPL (`repl_mode` :180), `--control-stdio` (:108). All funnel through `lumen_dispatch`. |
| open verb parse | `apps/cli/src/parse.rs:231` | `parse_open` builds `Command::SessionOpen { source, options }`; `SourceSpec::File{path}` / `SourceSpec::Url{url}` live in `commands::catalog::types`. |
| THE dispatch entry | `apps/cli/src/dispatch.rs:62` | `lumen_dispatch(host, request)` → `route` (:75). `Command::SessionOpen` arm at **:81-101** calls `host.open_session(source, start_time, no_video, no_audio)`. |

### 1.2 `CliHost::open_session` (the composition)

`apps/cli/src/host.rs:647-729`:

1. `open_source(&source, start_time_us, no_video, no_audio)` — **:661** (the path §1.3 maps).
2. Audio limb: `pipeline_host::open_audio_session(channels, rate, anchor_ns)` when a PCM audio
   fact survived selection (**:666-687**); else `pipeline_host::wall_clock(anchor_ns)` (:685).
3. `CliFrameConsumer::new(audio_limb, facts, clock, …)` — **:688-693**.
4. `HostDecoderProvider` built over the facts; consumer attached (between :693 and :699).
5. `SessionConfig::new(uri, Box::new(opened.demuxer), provider, consumer, clock)` — **:699-705**;
   `config.container = Some(facts.container)` (:710, card 3.C2); queue plan from product limits
   (:716-719); `engine.open(config)` (:726-729).

`SessionConfig` (`crates/playback-engine/src/session.rs:450-501`): `source: SourceUri`,
`demuxer: Box<dyn plugin_api::Demuxer>` (**:462**), provider/consumer/clock fields,
`queues: QueuePlan` (:472). `Engine::open` at `session.rs:1048`.

### 1.3 `open_source` — the path and the gate

`apps/cli/src/demux_host.rs:270-350` (`open_source`):

| Step | Site | Detail |
|---|---|---|
| URI build | `demux_host.rs:229-262` (`file_uri`) | `std::fs::canonicalize` (:230), non-regular-file → `input.inaccessible` (:241-247), percent-encoded `file:` URI → `SourceUri::parse` (:255). URL sources refuse `UnsupportedForHost` "needs the streaming crate (Phase 6)" (**:278-285**). |
| 1. Access | `demux_host.rs:288-291` | `input::FileAccess::new().open(&uri)` → `Box<dyn SourceStream>`. `Access` trait: `crates/plugin-api/src/access.rs:119-131`; `SourceStream`: `access.rs:29`; impls: `crates/input/src/file.rs:181` (`Access for FileAccess`), `:207` (`SourceStream for FileStream`). |
| 2. Probe | `demux_host.rs:293-296` | `media_probe::ProbeRegistry::with_builtins()` + `probe_stream` (lib entry `crates/media-probe/src/lib.rs:193`). Builtin registry: **`crates/media-probe/src/signature/mod.rs:127-136`** — exactly 10 signatures: Avi, Mp4, Wav, Aiff, Ogg, Mkv, Adts, Ts, Ps, Ivf. Format ids (`crates/media-probe/src/report.rs:37-85`): `avi`, `mp4`, `wav`, `aiff`, `ogg`, `matroska`, `adts`, `mpegts`, `mpegps`, `ivf`. Probe is content-sniff only — extension is a hint, not truth (`media-probe/src/lib.rs:57-59`). |
| **THE GATE** | **`demux_host.rs:298-310`** | `if container != "avi"` → `CommandError` `Stage::Probe` / `ErrorCode::Unsupported`, message "format \`{container}\` probes but has no open-path wiring yet … D-018 AVI first vertical", `.with_detail("format", container)`. **This is the single AVI-only site.** |
| 3. Demux + pre-pass | `demux_host.rs:312-314` | `AviDemuxer::new(stream)` (`crates/demux/src/avi/mod.rs:360`), `demuxer.start()` (:313 — idempotent re-start later), `harvest_facts(&demuxer, &tracks)` (:314 → :353-405; `AviDemuxer::stream_format` :389 → `StreamFormat::MotionJpeg`/`Pcm` arms :357-385). |
| 4. Start-time | `demux_host.rs:316-332` | duration clamp (:317), pure `demuxer.seek(target)` query (:330), `skip_until_ns` watermark (:331). |
| 5. Wrap | `demux_host.rs:333-343` | `AviHostDemuxer { inner, skip_until_ns, bases, disable_video, disable_audio }` → `OpenedMedia { demuxer, facts, start_position_ns, uri }` (:193-202). |

**`AviHostDemuxer`** (`demux_host.rs:418-501`) implements `plugin_api::Demuxer`
(`crates/plugin-api/src/demux.rs:52-121`): `start()` filters disabled *and non-A/V* kinds
(:448-458 — subtitle/data tracks never reach the engine from this host), `next_event()` drops
pre-skip packets (:460-472), `seek`/`seekability`/`reanchor` forward to the inner demuxer with the
skip watermark cleared at landing (:474-500). **Nothing in this wrapper is AVI-specific except the
`inner: AviDemuxer` field type** — the §4 seam generalizes exactly this.

### 1.4 Engine drive (read-only: names + edges)

- Pipeline = `crates/playback-engine/src/pipeline.rs`: demux (`plugin_api::Demuxer`) →
  per-track `packet_queue::BoundedQueue<Packet>` from `QueuePlan` → `codec_api::Decoder` obtained
  from the **`DecoderProvider`** seam (`pipeline.rs:120-133`) → frames out through the
  **`FrameConsumer`** seam (`pipeline.rs:140-148`). Tick model: bounded per poll —
  `budget.demux_events`, `budget.packets_per_limb`, `budget.flush_calls_per_limb`
  (`pipeline.rs:30-43`); full queue ⇒ packet held `pending`, never blocks.
- Decoder construction: `Pipeline::obtain_decoder` (`pipeline.rs:1620-1651`) — cache hit
  re-`configure` (:1632-1638), miss asks the provider with a **fresh `DecoderConfig` carrying
  `extra_data: None`** (:1642-1648; `DecoderConfig` fields at `crates/codec-api/src/config.rs:79-95`).
  ⇒ **any codec that needs out-of-band config must get it from the host provider, not the engine**
  (this drives §4.3).
- Pump: `CliHost` is the whole scheduler (`apps/cli/src/host.rs:1-44` module doc):
  `pump_pipeline` (:356) = engine poll + output drain; full `pump` (:366) adds time advance —
  audio quanta rendered from the null device move the shared counter, which moves the
  `MediaClock`, which gates video presents; `advance(ticks)` (:406) is the run-to-end driver.
  Clock types from `crates/media-clock` (`MediaClock`, `PlaybackClock`, `MonotonicSource`,
  `host.rs:86`).
- Output half: `apps/cli/src/pipeline_host.rs` — `CliFrameConsumer`: audio → `AudioPipeline` →
  null-sink ring (the session clock's truth), video → I420→RGBA host conversion →
  `PresentationScheduler` → bounded `PresentationQueue` → software `Renderer`
  (`pipeline_host.rs:14-33` module doc; `apps/cli/src/lib.rs` module map). Video/native-seek wiring
  proved by `apps/cli/tests/native_seek.rs`.

### 1.5 Refusal sites (all typed, all tested)

| Refusal | Site | Test |
|---|---|---|
| probe-miss (garbage) | `media_probe` no-hit → `probe.no-demuxer` | `apps/cli/tests/unsupported_path.rs:48-60` |
| **format probes but unwired** | `demux_host.rs:298-310` | `unsupported_path.rs:99-105` (asserts `format \`mp4\` probes` + `no open-path wiring`) — **every wiring unit must re-target this row** (§5 convention) |
| URL source | `demux_host.rs:278-285` | `unsupported_path.rs:109-114` |
| codec unavailable | `pipeline_host.rs:255-262` (`cli.codec_unavailable`, lists landed set) | decoder-level contract tests |
| no decodable track (all kinds refused/unwired) | engine `NoDecodableTrack`, names `config.container` (host.rs:710) | `playback-engine` suite + CLI `--no-video --no-audio` lanes |

---

## 2. Demux public API surface per container (all verified)

**Uniform core — the collision-free foundation.** Every container module exposes
`<X>Demuxer::new(Box<dyn plugin_api::SourceStream>) -> Result<_, EngineError>` and implements
`plugin_api::Demuxer` (start/next_event/seek + `seekability`/`reanchor` where supported), emitting
L0 `Track` lists and `Packet`s. One exception: `es/annexb` takes an extra codec parameter.

| Container (probe id) | `new` | `impl Demuxer` | Facts accessors the host harvest needs | `seekability` |
|---|---|---|---|---|
| avi (`avi`) | `crates/demux/src/avi/mod.rs:360` | `avi/mod.rs:891` | `stream_format(TrackId)` → `StreamFormat::{MotionJpeg, Pcm}` (`avi/mod.rs:389`; PCM descriptor via `header.rs` `codec_descriptor()`, used at `demux_host.rs:378`) | `avi/mod.rs:956` (idx1 → Fast) |
| mp4 (`mp4` — mov/m4a/3gp brands) | `mp4/mod.rs:558` | `mp4/mod.rs:1933` | `sample_entry_bytes(TrackId)` (`mp4/mod.rs:617` — the coded sample description: esds ASC for aac, ALAC cookie, avcC…); `chapters()` :602; `is_fragmented()` :595; codec descriptor map `mp4/mod.rs:440-450` (`mp4a`→`aac` :449, `alac` :450, `mjpa/mjpb/jpeg`→`mjpeg` :443, `av01`→`av1` :446, `avc1…`→`h264` :440, `hvc1/hev1`→`hevc` :441) | `mp4/mod.rs:1964` (indexed → Fast) |
| mkv (`matroska` — webm/mka/mk3d) | `mkv/mod.rs:149` | `mkv/mod.rs:716` | `track_entries()` (`mkv/mod.rs:190`) → `TrackEntry` (`mkv/header.rs:180-203`): `codec_descriptor` (vocab map `header.rs:751-808`: `A_VORBIS`→`vorbis`, `A_FLAC`→`flac`, `A_ALAC`→`alac`, `A_AAC*`→`aac`, `A_MPEG/L3`→`mp3`, `A_PCM/INT/LIT`→`pcm_s16le`…, `V_MJPEG`→`mjpeg`, `V_AV1`→`av1`, `S_TEXT/*`→subtitle) + `private: TrackPrivate` (`header.rs:147-166`: raw `codec_id`, **`codec_private` bytes**, `default_duration_ns`, `channels`, `sampling_frequency`, `bit_depth`); `cue_index()` :196 | `mkv/mod.rs:823` (cues → Fast) |
| ogg (`ogg` — oga/ogv) | `ogg/mod.rs:443` | `ogg/mod.rs:1263` | `tracks()` :470, **`header_packets(serial) -> &[SharedBytes]`** (`ogg/mod.rs:490` — the Vorbis/FLAC header chain), `links()` :476 (chained streams), `serial_link` :514, `track_duration` :539 | `ogg/mod.rs:1316` (granule-based; None early) |
| wav (`wav` — rf64/bw64) | `wav/mod.rs:278` | `wav/mod.rs:1022` | `format() -> Option<WaveFormat>` (`wav/mod.rs:304`; `codec_descriptor()` `wav/format.rs:273-275` — `pcm_s16le/s24le/s32le/u8/f32le/f64le` :109-116, `adpcm_ima_wav` :207, `adpcm_ms` :208, `flac` :209), **`fmt_chunk_bytes()`** (:312 → WAVEFORMATEX body for ADPCM extra_data), `frames()` :355, `is_rf64_like()` :326 | `wav/mod.rs:1081` (Fast) |
| aiff (`aiff` — aif/aifc) | `aiff/mod.rs:195` | `aiff/mod.rs:640` | `comm() -> Option<CommChunk>` (`aiff/mod.rs:218` — channels/rate/bits), `comm_chunk_bytes()` :225, `ssnd_offset_block()` :231, `frames()` :251 | `aiff/mod.rs:692` (Fast) |
| ts (`mpegts`) | `ts/mod.rs:594` | `ts/mod.rs:1025` | `programs()` :629, `track_of_pid` :673, `TsStream::descriptor()` (`ts/mod.rs:349`), `last_pcr` :647, PES hook `set_pes_consumer` :622 (v0 ignores) | `ts/mod.rs:1160-1161` (**`SeekSupport::None`** — in-session seek refuses typed) |
| ps (`mpegps` — vob/mpg) | `ps/mod.rs:343` | `ps/mod.rs:924` | `stream_descriptors() -> Vec<(&str, MediaKind)>` (`ps/mod.rs:368`), `current_scr` :374, `is_mpeg2` :388 | `ps/mod.rs:1139-1140` (**`SlowLinear`**) |
| adts (`adts` — aac/adif) | `adts/mod.rs:377` | `adts/mod.rs:1121` | `stream_info() -> Option<AdtsStreamInfo>` (`adts/mod.rs:406`; struct :311-320: format/rate/channels/profile) | `adts/mod.rs:1212` (None until profile known) |
| es h264/hevc | `es/annexb.rs:363` — **`new(codec: AnnexbCodec, stream)`** | `es/annexb.rs:477` | codec known from construction (`AnnexbCodec`, h264 or hevc); raw ES ⇒ `Stamp::Missing` pts (`es/mod.rs:26-29`) | annexb: seek refuses typed (no timing, `es/annexb.rs:340` doc) |
| es mp3 | `es/mp3.rs:292` | `es/mp3.rs:626` | sample-rate from first frame header (in-band) | exact sample arithmetic (`es/mod.rs:28-29`) |
| es aac | reuses `crate::adts` verbatim (`es/mod.rs:17-18`) | — | — | — |
| ivf (`ivf`) | `ivf/mod.rs:325` | `ivf/mod.rs:553` | `stream_info() -> Option<IvfStreamInfo>` (`ivf/mod.rs:353`; struct :247-262: codec `av1`/`vp9`, width/height, rate/scale) | `ivf/mod.rs:665` (frame-index — measured SlowLinear through the wrapper, 6.OPW-ivf) |

Per-module test suites (the per-container regression backbone):
`crates/demux/tests/{corpus_mp4,mkv_corpus,mkv_fixtures,mkv_attachments_chapters,mkv_editions_tags,
corpus_ogg,ogg_fixtures,corpus_wav,wav_aiff_fixtures,ts_corpus,ts_pes_corpus,ps_fixtures,
adts_corpus,es_corpus,ivf_fixtures,corpus}.rs`.

**Bespoke remainder** (what a host adapter must actually write per container): the constructor
call, the facts harvest (columns 3–4 above), and the container's descriptor→codec vocabulary —
everything else (skip wrapper, engine wiring, codec dispatch, sinks, clocks) is shared code that
already exists.

---

## 3. Codec dispatch surface (verified)

**Host-side, container-agnostic, already multi-codec.** `HostDecoderProvider`
(`apps/cli/src/pipeline_host.rs:139-265`) implements the engine's `DecoderProvider`
(`crates/playback-engine/src/pipeline.rs:120-133`) and dispatches on `track.codec.as_str()`
(the registry descriptor the demuxers emit, §2):

| Descriptor | Decoder | Site |
|---|---|---|
| `mjpeg` | `codecs::video::mjpeg::MjpegDecoder::new()` | `pipeline_host.rs:174-178` |
| `flac` | `FlacDecoder::new()` | :185-189 |
| `mp3` \| `mp1` | `Mp3Decoder::new()` | :190-194 |
| `mp2` | `Mp2Decoder::new()` | :195-199 |
| `vorbis` | `VorbisDecoder::new()` | :200-204 |
| `aac` \| `aac-lc` | `AacDecoder::new()` (LC-only by card) | :210-214 |
| `alac` | `AlacDecoder::new()` | :215-219 |
| `adpcm_*` | `AdpcmDecoder::new()` | :220-224 |
| `av1` \| `av01` | `Av1Decoder::new()` | :225-229 |
| `pcm_*` | `PcmDecoder::with_format(channels, rate)` from **host facts** | :230-253 |
| anything else | typed `cli.codec_unavailable` refusal listing the landed set | **:255-262** |

All decoders are `new()` + `configure(&DecoderConfig)` (`codec_api::Decoder`:
`crates/codec-api/src/decode.rs:86-112`; `FrameSink` :186-192).

**The extra_data gap (the one real codec-binding change).** The engine mints
`DecoderConfig::new(codec, generation, threads, None, None)` — `extra_data` is always `None`
(`pipeline.rs:1632-1638` cache-hit path, `:1642-1648` miss path). But three landed codecs
*require* out-of-band config at `configure`:

- `VorbisDecoder::configure` — refuses without `extra_data` = the stream's **three header
  packets** concatenated (`crates/codecs/src/audio/vorbis/decoder.rs:356-372`);
- `AacDecoder` ASC mode — `extra_data` = AudioSpecificConfig (raw MP4/esds path)
  (`crates/codecs/src/audio/aac/decoder.rs:326-331, 406-417`); ADTS mode needs none;
- `AlacDecoder` — refuses without the ALAC **magic cookie** in `extra_data`
  (`crates/codecs/src/audio/alac/decoder.rs:33-35, 620-626`);
- `AdpcmDecoder` — parses its WAVEFORMAT (tag/channels/rate/block_align) **from `extra_data`**
  (`crates/codecs/src/audio/adpcm/mod.rs:346-358`);
- `PcmDecoder` — host facts channel (`with_format(channels, rate)`,
  `crates/codecs/src/audio/pcm/decoder.rs:618`; big-endian descriptors like `pcm_s16be` are
  supported — test at `pcm/decoder.rs:1507`, the AIFF case).

⇒ `DecoderConfig`'s fields are `pub` (`codec-api/src/config.rs:79-95`), and the provider receives
`&DecoderConfig` — so the host provider can (and must) **clone the engine's config and seed
`extra_data` from harvested facts** before `configure`. No engine change, no codec change.

**H.264/HEVC honesty boundary.** The h264/hevc modules are parsers + hardware-path seam only —
"no software H.264 decode here" (`crates/codecs/src/video/h264/mod.rs:1-12`; hevc mirrors it).
MPEG-1/2 video likewise has no `Decoder` impl (`crates/codecs/src/video/mpeg12/` — parser
modules only; verified: no `impl Decoder for` in that tree). So mp4/mkv/ts/ps/es files whose
video is h264/hevc/mpeg2 video open and demux, then refuse decode per-track honestly
(`pipeline_host.rs:255-262`) — or `NoDecodableTrack` when no track is decodable.

**Subtitles.** Not decoded on the open path: `AviHostDemuxer::start` filters to Audio/Video kinds
(`demux_host.rs:452-456`), and the engine's subtitle-kind wiring is an open-time *deselect*
(`pipeline.rs:1617-1619` names "the subtitle kind's open-time path: deselect"). The `subtitles`
crate (srt/vtt/ass/pgs/dvb/cc/vobsub render paths) is not consulted by the CLI host today. A
subtitle-rendering card is future work; per-container units should simply preserve the
filter-to-A/V behavior.

---

## 4. THE SEAM — per-container host registry (design)

### 4.1 Where it lives

**`apps/cli`** (the composition root). Rationale (verified): the CLI already depends on `demux`,
`codecs`, `codec-api`, `input`, `media-probe`, `playback-engine`, `plugin-api`, `media-types`
(`apps/cli/Cargo.toml` `[dependencies]`); the engine's seams (`Demuxer` box in `SessionConfig`,
`DecoderProvider`, `FrameConsumer`) are host-facing by design (`pipeline.rs:114-148`);
a new crate would add a `boundaries.toml` entry + Lead review for zero capability gain. Nothing
in `crates/*` needs to change.

### 4.2 What a container host is (reuse, do not invent)

The demuxer trait **already exists**: `plugin_api::Demuxer` (`plugin-api/src/demux.rs:52-121`) —
`start()` (track list), `next_event()` (packet pump), `seek()` (pure query),
`seekability()`, `reanchor()` (compute-then-commit). Every container already implements it (§2).
Therefore **no new trait is needed**. A "container host" is exactly one free function per
container with a fixed signature:

```rust
// apps/cli/src/containers/<fmt>.rs  (one file per implementation unit)
pub(crate) fn open(
    stream: Box<dyn plugin_api::SourceStream>,
    opts: &OpenOptions,            // { start_time_us, no_video, no_audio } — today's open_source args
) -> Result<OpenedMedia, CommandError>
```

whose body is the §1.3 steps 3-5 pattern, made concrete per container:
construct the demuxer → `start()` → harvest `TrackFact`s from the container's accessors (§2 col. 3)
→ optional pure `seek()` start-time query → wrap in the shared `SelectDemuxer` → return
`OpenedMedia`.

### 4.3 Shared-code changes (one-time, seam-prep; NOT per-unit)

1. **Generalize `AviHostDemuxer` → `SelectDemuxer<D: Demuxer>`** in `apps/cli/src/demux_host.rs`
   (rename + type parameter; behavior identical — the skip/selection/filter logic at
   `demux_host.rs:418-501` is already demuxer-generic). `OpenedMedia.demuxer` becomes
   `Box<dyn plugin_api::Demuxer>` (today `AviHostDemuxer`, `demux_host.rs:195`) — `host.rs:701`
   already boxes it, so the call site is a no-op shape change.
2. **Extend `TrackFact`** (`demux_host.rs:133-150`) with
   `extra_data: Option<media_types::packet::SharedBytes>` (container blob: esds ASC / ALAC cookie /
   Vorbis 3-header chain / WAVEFORMATEX body) alongside the existing `pcm` pair.
3. **Seed config in `HostDecoderProvider::decoder_for`** (`pipeline_host.rs:167-264`): build
   `DecoderConfig { extra_data: fact.extra_data.clone(), ..config.clone() }` before `configure`
   for the extra_data-consuming codecs (§3 list). PCM keeps its `with_format` facts path.
4. **Replace the gate with a registry call** (`demux_host.rs:298-310`):
   `crate::containers::open(&container, stream, &opts)`.

### 4.4 THE single registry

**File: `apps/cli/src/containers/mod.rs`** (new; `pub mod containers;` added once to
`apps/cli/src/lib.rs`). Its entire dispatch surface:

```rust
/// THE container-host registry. One entry per wired container family;
/// anything else falls to the honest refusal below (probe stage,
/// format named — the D-018-era message shape, demux_host.rs:298-310).
pub(crate) fn open(
    format: &str,
    stream: Box<dyn plugin_api::SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    match format {
        "avi" => avi::open(stream, opts),        // ← the entry shape:
        "mp4" => mp4::open(stream, opts),        //   `"<probe-id>" => <mod>::open(stream, opts),`
        // … one line per container family …
        other => Err(unwired_refusal(other)),    // keeps the honest typed refusal forever
    }
}
```

**Collision rule (the card's requirement):** wiring ONE container = creating
`apps/cli/src/containers/<fmt>.rs` + appending exactly **one `mod <fmt>;` declaration and one
match arm** to `containers/mod.rs`. No other shared file is touched by a wiring unit (the §4.3
changes land once, in the seam-prep commit or with the first unit, Lead-sequenced). Merge
conflicts are confined to two append points in one file and are trivial one-line re-merges;
entries are kept in the order of the §5 table.

Probe-id mapping (verified, §1.3/§2): `avi`, `mp4` (mov/m4a/3gp brands probe here), `matroska`
(mkv/webm/mka), `ogg` (oga/ogv), `wav` (rf64/bw64), `aiff` (aif/aifc), `mpegts`, `mpegps`
(vob/mpg), `adts` (aac/adif), `ivf`. Raw-ES (`.mp3/.mp2/.flac/.264/.265`) has **no** probe
signature today — see the 6.OPW-es row in §5.

### 4.5 How codec binding resolves per track

Unchanged from §3, plus the §4.3.3 seeding: `Track.codec` descriptor (emitted by each demuxer's
vocabulary map — mkv `header.rs:751-808`, mp4 `mod.rs:440-450`, wav `format.rs:202-209`) →
`HostDecoderProvider` match (`pipeline_host.rs:173-263`) → configure with facts-seeded
`extra_data`/PCM format → decode. Unlanded codecs (opus, h264, hevc, mpeg2 video, vp8/vp9, ac3,
dts) refuse typed at `pipeline_host.rs:255-262`; a file with no decodable track lands in the
engine's `NoDecodableTrack` naming the container (`host.rs:706-710`).

### 4.6 How the refusal stays honest

- Unwired container → registry default arm: same stage/code/detail as today
  (`demux_host.rs:298-310`), message keeps naming the format. Exit 65 (`apps/cli/src/exit.rs:49`).
- Wired container, unlanded codec → `cli.codec_unavailable` (§3).
- Wired container, nothing decodable → engine `NoDecodableTrack` (§1.5).
- **Test convention:** `apps/cli/tests/unsupported_path.rs:99-105` pins the refusal *with mp4 as
  the example*. Each unit that wires a format re-targets that test's fixture to a still-unwired
  probing format (mkv until it lands, then ts, …); the final unit (ps) re-pins it to the
  permanently-honest case: a probing-but-unwired *codec family* if any remains, else the
  probe-miss corpus files `0001-audio-only.ac3` / `0048-flv-video.flv` (verified present;
  neither has a signature — §1.3 registry list).

---

## 5. Per-container implementation-unit checklists

Corpus: `/tmp/my-project/corpus-v4/media` — 143 files, verified census: avi 44, mkv 31, mp4 24,
wav 12, ts 5, mov 5, ogg 3, flac 3, mp3 2, mp2 2, mka 2, m4a 2, aac 2, webm 1, ogv 1, h264 1,
flv 1, ac3 1, `265` 1. (No aiff / ivf / ps / m2v files — those lanes run on crate fixtures,
noted per row.)

Recommended unit order = dependency-lightest first (wav → aiff → adts → ogg → ivf → mp4 → mkv →
ts → ps → es). All units: same gate skeleton — demux crate tests stay green, new CLI lane opens +
plays + seeks (where seekable) over the corpus files, `unsupported_path` re-target (§4.6),
`cargo clippy --all-targets -D warnings` + `fmt --check` on `lumen-cli`.

| Unit | Files touched (beyond registry line) | Container facts source | Expected gates | Corpus lanes | Risk notes |
|---|---|---|---|---|---|
| **6.OPW-wav** | `containers/wav.rs` | `format()` wav/mod.rs:304 (+`fmt_chunk_bytes` :312 → adpcm extra_data) | `demux/tests/{corpus_wav,wav_aiff_fixtures}.rs`; CLI open/play/advance over wav lane; adpcm+pcm descriptors flow to decoder | 12 wav (incl. `0013-sr-chaos.wav`, `0004/0006/0016-audio-only.wav`) | extensible-GUID forms (`format.rs:334-369` AC paths); rf64 (`is_rf64_like` :326); PCM needs facts channel (existing pattern); seek is Fast |
| **6.OPW-aiff** | `containers/aiff.rs` | `comm()` aiff/mod.rs:218 (channels/rate/bits → pcm be descriptors) | `demux/tests/wav_aiff_fixtures.rs` | **no corpus files** (verified) — use `crates/demux/fixtures` + `tests/fixtures/generated`; consider adding 1-2 generated aiff fixtures to the corpus lane | big-endian pcm (`pcm_s16be` supported, pcm/decoder.rs:1507); AIFC compressed forms → honest codec refusal; corpus-lane gap is the honest limit |
| **6.OPW-adts** (+es-aac) | `containers/adts.rs` | `stream_info()` adts/mod.rs:406 | `demux/tests/adts_corpus.rs`; CLI lane over `.aac` | 2 aac | ADIF variant; LC-only (Main/SSR/LTP refuse — by card 5.C2a); raw `.flac`/`.mp3` NOT this unit (no probe signature — see es row) |
| **6.OPW-ogg** | `containers/ogg.rs` | `header_packets(serial)` ogg/mod.rs:490 → Vorbis 3-header chain extra_data | `demux/tests/{corpus_ogg,ogg_fixtures}.rs` | 3 ogg + 1 ogv (`0002`-class audio-only, ogv likely vorbis-in-ogv or theora video → honest refusal) | **first unit to exercise extra_data seeding (vorbis)**; chained streams (`links()` :476) — v0 opens first link, record honestly; opus refuses at codec dispatch |
| **6.OPW-ivf** | `containers/ivf.rs` | `stream_info()` ivf/mod.rs:353 (codec/w/h/rate/scale) | `demux/tests/ivf_fixtures.rs` | no corpus files (verified) — `crates/codecs/tests/fixtures/av1/*.ivf` are the natural lane | av1 decodes; vp9 refuses; timebase from rate/scale; DKIF-only probe |
| **6.OPW-mp4** (mp4/mov/m4a) | `containers/mp4.rs` | `sample_entry_bytes(track)` mp4/mod.rs:617 (esds→ASC, ALAC cookie); descriptor map :440-450 | `demux/tests/{corpus_mp4,mp4_fixtures}.rs`; CLI lanes incl. `0011-vfr-av.mov`, `0012-still-24h.mp4`, `0008-fps-extremes.mp4`, m4a/alac | 24 mp4 + 5 mov + 2 m4a | **extra_data ASC path for aac** (aac/decoder.rs:406-417); alac cookie (alac/decoder.rs:620-626); esds OTI quirks already demux-side (:1115-1121); fragmented mp4 (`is_fragmented` :595) — verify packet flow; h264/hevc video tracks refuse decode honestly; **this unit re-targets `unsupported_path.rs:99-105`** (mp4 is its fixture) |
| **6.OPW-mkv** (mkv/webm/mka) | `containers/mkv.rs` | `track_entries()` mkv/mod.rs:190 → `TrackEntry.private` (codec_private/header.rs:152, channels :163) + `codec_descriptor` :194 | `demux/tests/{mkv_corpus,mkv_fixtures,mkv_attachments_chapters,mkv_editions_tags}.rs` | 31 mkv + 2 mka + 1 webm (incl. `0007-chapter-bomb`, `0009-param-midchange`, `0017-meta-bomb-av`) | vorbis codec_private = header chain → extra_data; aac-in-mkv needs ASC from private (verify the demuxer emits it — honest check); editions/ordered-chapters = v0 out of scope (record); attachments skipped; `S_TEXT/*` tracks deselect at open (§3 subtitles) |
| **6.OPW-ts** | `containers/ts.rs` | `programs()`/`track_of_pid`/`TsStream::descriptor()` (ts/mod.rs:629/673/349) | `demux/tests/{ts_corpus,ts_pes_corpus,ts_fuzz_survival}.rs` | 5 ts (incl. `0000-param-midchange.ts`, `0018-ts-video.ts`) | **`SeekSupport::None` (ts/mod.rs:1160-1161) — native-seek lane must expect the typed NotSeekable refusal**; h264/mpeg2 video refuses decode (audio mp2/aac decodable); mid-file stream changes (0000) |
| **6.OPW-adts-es-ivf split note** | — | — | — | — | card program named `6.OPW-adts-es-ivf`; this table splits it into adts / ivf / es rows (disjoint files, no shared surface beyond the registry) |
| **6.OPW-es** (h264/hevc/mp3 raw) | `containers/es.rs` (+ **probe decision**) | `es/annexb.rs:363` (`AnnexbCodec`), `es/mp3.rs:292` | `demux/tests/es_corpus.rs` | 1 h264 + 1 `265` + 2 mp3 + 2 mp2 + 3 flac | **No probe signature exists for raw ES** (registry list §1.3 = 10, none for mp3/h264/hevc/flac) → today these probe-miss. Unit must add media-probe signatures (mpeg-audio syncword with layer-bit discrimination — the `AdtsSignature` precedent phase5.rs:127-147 already distinguishes ADTS layer-00 from MP3 layer-01; Annex-B start-code sniff for h264/hevc) **or** a Lead-approved extension fallback — this is the one design decision that touches a second crate (`media-probe`), keep it its own review point. h264/hevc have **no software decoder** (h264/mod.rs:1-12) ⇒ those files open, demux, and land in `NoDecodableTrack`/codec refusal — the unit's honest value is the mp3/mp2/flac ES lanes + the pipe-ready ES surface. Raw `.flac` probing would collide with the adts-style magic question — coordinate with the flac codec owner |
| **6.OPW-ps** (ps/vob) | `containers/ps.rs` | `stream_descriptors()` ps/mod.rs:368 | `demux/tests/ps_fixtures.rs` | no corpus files (verified) — `crates/demux/fixtures/ps/{ps-mp2-video-audio.mpg, ps-vob-{ac3,dts,lpcm}.vob}` | mpeg2/ac3/dts/lpcm: only mp2 audio decodes today; `SlowLinear` seek (ps/mod.rs:1139-1140) — bounded-budget behavior; **final unit: re-pin the refusal regression (§4.6)** |
| **playlists-local** | — | `crates/playlists` (m3u/pls/xspf parse, lib.rs §Scope) | — | — | **Recommend: NOT a 6.OPW unit.** The open path is single-source; playlists imply queue/next semantics (a session-model feature, doc 05 queue cards) and have no probe signature. Record as its own future card so the wave stays per-container-disjoint |

---

## 6. Honest limits / unverified

- Zero code changes made this session (doc + worklog only) — nothing to `cargo check`; the
  worktree compiles only what `main` shipped (not re-measured here; `f3e5b8c` is the green merge
  the wave program landed).
- Unverified this session: exact `seekability()` return *values* for wav/avi/adts/ogg/ivf/mp4
  beyond the cited arms (each `impl` block site is cited; the wiring units should assert the
  value they get, not trust this doc's shorthand).
- Unverified: whether `AacDecoder` accepts mkv's `A_AAC` codec_private as ASC verbatim (the
  mp4/esds ASC path is the tested one — aac/decoder.rs:406-417); the mkv unit must check the
  demuxer's private-blob shape against the decoder's expectation before claiming aac-in-mkv.
- Unverified: the exact set of descriptors each demuxer can emit for damaged/unknown tracks
  ("unknown" passthrough — mkv header.rs:191-193 says raw pass-through; those refuse at codec
  dispatch, which is the honest path).
- The `SelectDemuxer` genericization is a mechanical rename+type-param of verified code, but its
  clippy/test outcome is unmeasured until a unit lands it.
