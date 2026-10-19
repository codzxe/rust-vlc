# Subsystem Feasibility — Demuxers · Network · Outputs · Filters · Playlists · Sout · Devices

| | |
|---|---|
| **Task ID** | 0.C3-I (Phase 0, wave 0.C — coverage/feasibility) |
| **Card** | doc 13 §3 wave 0.C, row 0.C3-I |
| **Date of evidence** | 2026-10-19 (all live fetches and doc reads this date) |
| **Consumed inputs** | doc 01 §2/§4/§7 (crate registry, async policy, ADR register); doc 03 §2 + §7 (pipeline, first vertical); doc 04 §2 + §8.6 items 3/4/9 (containers, audit list); doc 13 §5–§10 (phase cards); doc 14 (risk register); `docs/inventory/modules-linux.json` (0.B2 census); `docs/inventory/platform-{linux,windows,macos}.md` (0.C5-a/b); `docs/dependencies/preaudit-foundation.md` (0.C4-R); `docs/inventory/ui-inventory.md` §2.11 (0.B3 filter defaults); `design/figma/*.svg` (panel contents, via element-id extraction) |
| **Method** | Doc reads per card scope + crates.io API re-verification (`UA "lumen-preaudit-0.C3-I (subsystem feasibility)"`, raw JSON in `tmp/c3i/`, gitignored) for exactly the crates where this section needed fresh purity/availability evidence: doc 04 [VERIFY]-adjacent items 3/4/9, the Phase-6 network stack, playlist/SD crates, and sout muxing. Tarball inspections (M4–M6 of `docs/dependencies/audit-method.md`) for `mpeg-ps 0.6.0`, `mp4parse 0.17.0`, `mpeg-pes 0.6.0`, `symphonia-format-isomp4 0.6.1`. No VLC source. No git commit. |
| **Verdict vocabulary** | same as preaudit-foundation: `pass` / `conditional (E2)` / `fail` / `exception-ADR`; subsystem-level: `self-build S/M/L/XL`, `crate-exists`, `missing`, `blocked` |

---

## 1. Demuxers

### 1.1 Family status summary (pointer, not duplication)

The per-container ground truth is doc 04 §2.1–§2.2 (verified 2026-10-19 there);
the formats-inventory records land per doc 13 waves 5.* (`docs/formats/*.md`).
This section only adds the subsystem rollup and the three items the audit list
assigns here. Baseline surface: VLC Linux census = **57 demuxer registrations**
(55 `demux` + 2 `demux_filter`; `modules-linux.json`, counting rule in-file).

| Family (doc 04 §2.1 rows) | Status | Self-build | Phase (doc 13) |
|---|---|---|---|
| WAV/FLAC/MP3/Ogg | pure-rust-exists (symphonia/claxon/ogg) | none | 2 (stretch) → 5.A/5.B |
| MP4/MOV/3GP incl. fragmented | partial — symphonia isomp4 primary, mp4parse oracle | M (quirks) | 2 (if MP4 wins D-001) / 5.C1 |
| Matroska/WebM | partial — symphonia mkv | M (cues/streaming EBML) | 5.D1–5.D2 |
| MPEG-TS | pure-rust-exists (mpeg2ts-reader; mpeg2ts for mux) | S (wrap+harden) | 5.I1 |
| MPEG-PS | pure-rust-exists but 2-day-old crate — §1.3 | M (harden + VOB streams) | 5.I2 |
| AVI | missing — no maintained crate | M | 2 (if AVI wins D-001) / 5.J1 |
| AAC ADTS/LOAS, .m4v elementary | partial/missing | S each | 5.C2 / 5.I3 |
| ASF/WMV, RealMedia | missing | M–L / M | 5.K3-I decision / later |
| AIFF, CAF | pure-rust-exists (symphonia riff/caf) | none | 5.A2 / later |

New since doc 04 (no status change required, recorded for the ledger): the
Rust media-ecosystem churns fast — e.g. a `transmux 0.27.7` crate (any-to-any
muxing hub, fMP4/CMAF-aware) and `multimux 0.13.0` exist on crates.io today
(search results, §9 [E20]); none of these is audited or proposed — they merely
corroborate that container-format activity in pure Rust is alive.

### 1.2 Fragmented-MP4 corpus plan (doc 04 §8.6 item 3 — closing evidence)

**What the crates actually do (verified today, tarball level):**

- `symphonia-format-isomp4` 0.6.1 (crates.io API: updated 2026-08-13 per
  endpoint [E16]; doc 04 §2.1 verified it 2026-10-19): tarball inspection
  confirms a player-shaped fragment path — `MoofSegment` with per-track `traf`
  indexing, `mfhd.sequence_number` handling, base-offset arithmetic keyed to
  the enclosing `moof` position, and `SidxAtom` in the demuxer surface
  (`src/stream.rs`, `src/demuxer.rs`). **This is the engine's fMP4 path.**
- `mp4parse` 0.17.0 (MPL-2.0, published 2023-05-29 [E17]): tarball inspection
  **sharpens doc 04's caution**: `read_mp4()` handles only `ftyp` + `moov` +
  `meta` at top level and **skips `moof`/`mdat`/`sidx` boxes**
  (`skip_box_content` in the top-level loop, `src/lib.rs` ~4190–4240; the
  "fragmented mp4 order: ftyp, moov, pairs of moof/mdat" comment documents
  awareness, not consumption). Its fragment machinery serves the AVIF path.
  Consequence: **mp4parse cannot cross-check fragment sample tables for us**;
  its oracle role for fMP4 is limited to init-segment (moov) structure. The
  doc 04 §2.2 phrase "moof support exists but completeness needs corpus
  testing" should be read as: in 0.17.0 the *public* API does not surface moof
  at all [record this correction in `docs/formats/container-mp4.md` when it
  lands].

**Fixture classes the corpus must contain** (each generated deterministically,
each with recorded expectations: sample count, per-sample PTS/DTS/size/offset/
sync-flag, duration, track list; each with truncated-at-256-byte-boundary and
bit-flip derivatives per doc 03 §7 fixture rules):

1. **Progressive baseline**: `ftyp/moov(stbl)/mdat` — non-fragmented anchor;
   guards against fragment work regressing the common case.
2. **Minimal fMP4**: `ftyp/moov(empty-stbl)/moof+mdat` — single fragment,
   single track, `default-base-is-moof` (tfhd flag 0x020000).
3. **Multi-moof sequences**: N alternating `moof/mdat` pairs (spec order:
   "ftyp, moov, pairs of moof/mdat (1–multiple), mfra" — the box-order note in
   mp4parse's own source, [E17]); sequence numbers 1..N.
4. **tfhd variants**: `base-data-offset-present` (explicit), absent (defaults
   to moof base), `default-sample-duration/size/flags-present`
   combinations; `sample-description-index` present/absent.
5. **tfdt variants**: box absent (= 0); present with non-zero
   `baseMediaDecodeTime` (mid-stream join / DASH live edge); v0 vs v1 (64-bit).
6. **Multi-track fragments**: one `moof` with two `traf` (A+V interleaved) vs
   alternating per-track `moof`s — the two layouts real packagers emit.
7. **trun variants**: multiple `trun` per `traf`; `first-sample-flags`;
   signed `sample_composition_time_offset` (B-frames, v1 trun and v0
   unsigned variants); non-sync sample flags mid-fragment.
8. **sidx**: single-level `sidx` over N segments (`earliest_presentation_time`,
   `first_offset`); **nested sidx** (`reference_type=1`) — DASH SegmentTimeline
   packages emit these; plus the `mfra/tfra` trailing index.
9. **DASH-adjacent**: `styp`-headed media segments (init segment + media
   segments concatenated); `emsg` and `prft` boxes present-and-skipped; CENC
   structures (`sinf/tenc`, `saiz/saio/senc`) present — v1 expectation:
   parse-without-crash + typed "encrypted: unsupported" error, unless
   ADR-001/WebCodecs scope says otherwise (ties to the doc 04 §8.6 item 8
   WebCodecs ADR).
10. **Live-edge pathologies**: non-monotonic `mfhd` sequence numbers; moof
    re-arrival (same sequence, different offsets); unknown-size `mdat` (0-size,
    extends-to-EOF) inside a fragment sequence.
11. **Hostile**: trun `sample_count` = absurd value (allocation-bomb guard —
    doc 03 §4.3 bounded-memory rule); sidx offsets past EOF; `moof` before any
    `moov` (must be a typed error, not a hang); truncated mid-trun-entry.

**How testkit generates them** [PROPOSAL, extends card 2.A3's pattern]: a
deterministic in-tree fMP4 *writer* in `testkit` (S–M): box emitter for the
structures above, synthetic H.264-less payloads (opaque sample bytes — the
corpus proves **demux/seek/index behavior**, not decode; decode correctness is
the codec cards' problem). Sample tables recorded as TOML expectations next to
each fixture. Generation must be byte-stable (hash-checked, per 2.A3 AC).
External conformance inputs (license-audited per R-37, listed in
`tests/fixtures/CORPUS.md`): DASH-IF test vectors (public reference streams,
per-file license check [VERIFY]), Apple HLS example streams (public; license
status [VERIFY]), MPEG/ISO conformance material where redistribution is
permitted [VERIFY]. Differential oracle: VLC 3.0.23 under ADR-007's external-
oracle rules (demux-level comparison via our own tooling: sample tables
extracted by `lumen info --dump-samples`-style debug output vs VLC's
`--demux=...` behavior — exact comparison harness owned by doc 10 §6).
Schedule: fixture classes 1–8, 11 land with card **5.C1**; classes 9–10 may
split to Phase 6 (6.C conformance) since DASH scheduling is what exercises
them — recorded as a 5.C1→6.C dependency note.

**Pass criteria**: every fixture demuxes to the recorded sample table exactly;
seek to arbitrary times inside fragment ranges lands within ±1 sample of the
computed anchor (doc 03 §6 seek contract); hostile class never exceeds bounded
memory/time (doc 03 §4.2 probe-depth analog); `fuzz_demux_mp4` seed corpus =
these fixtures (doc 04 §8.5 registry).

### 1.3 MPEG-PS scope (doc 04 §8.6 item 4 — closing evidence)

**Crate reality, verified today**: `mpeg-ps` 0.6.0 [E18] — MIT OR Apache-2.0,
published **2026-10-17 (two days old)**, `#![no_std]`, tarball: 0 native
files, no `build.rs`, deps = `broadcast-common ^10`, `mpeg-pes ^0.6`,
`thiserror ^2` (+optional serde). `mpeg-pes` 0.6.0 [E19]: also tarball-pure.
Scope (README "What it does / what it does not" + source): pack header
(42-bit SCR), system header (Table 2-40 incl. `stream_id_extension`), Program
Stream Map (§2.5.4, Table 2-41, CRC-32), PES via `mpeg-pes`,
`MPEG_program_end_code`; `Pack` walk/scan APIs with skip regions. **Explicit
non-goals**: Program Stream Directory (not implemented), PSM descriptors
(opaque bytes), MPEG-1 System Streams (recognized, rejected with
`Error::Mpeg1NotSupported`), no mux-from-scratch.

**What v1 needs (card 5.I2 scope) and where the crate stops:**

| Layer | v1 need | Covered by `mpeg-ps` 0.6.0? |
|---|---|---|
| Pack/system-header framing | yes (plain `.mpg` PS files) | **yes** |
| PSM stream typing | yes (stream_type → ES id map) | yes |
| PES extraction (MPEG-1/2 video, MP1/MP2 audio) | yes | yes (via mpeg-pes) |
| `private_stream_1` (0xBD) **substream split** — AC-3 (0x80–0x87), DTS (0x88–0x8F), LPCM (0xA0–0xAF) | yes — 5.I2 AC: "AC-3/DTS/LPCM private streams exposed" | **no** — 0xBD arrives as an opaque PES payload; the DVD substream header (substream id + PTS in first 4–6 bytes for AC-3/LPCM) is self-build **S** |
| `private_stream_2` (0xBF) = **nav packs** (PCI/DSI) in VOBs | v1: recognize + skip (no interpretation) | payload passed through as PES; PCI/DSI parsing itself **not needed v1** |
| IFO / cell / menu navigation | **no** — DVD-Video navigation is ADR territory (§7.3, item 9) | no (correctly out of scope) |
| MPEG-1 System Streams (`.mpg` MPEG-1) | open decision | **no** — crate rejects; self-build S–M **[VERIFY: driven by usage-share data, doc 04 §8.6 item 7]** |

**VOB/nav relationship to the DVD ADR** (made explicit): opening a `.vob`
*file* directly must work as a plain PS demux (streams exposed; decode
pending 5.K4-I AC-3/DTS decisions) — that is 5.I2 and it stops at the
container. Anything menu-shaped (IFO parsing for navigation, PCI/DSI-driven
button/highlight state, cell traversal, CSS) is the DVD-Video navigation
problem = **blocked, C-library-bound** (evidence in §7.3) and is decided by
the optical-disc ADR (doc 01 §7 register: **ADR-008**; the task brief's
"ADR-009" numbering is a mismatch — doc 01's ADR-009 is WASM threading;
flagged in §8 and worklog). MPEG-PS self-build estimate stays **M (harden)**
per doc 04 §2.1: substream split + corruption tolerance + the crate's extreme
youth (2 days; bus-factor unknown) argue for a thin-wrapper design behind the
doc 03 §4.3 Demuxer trait so a harden/fork path exists. Fuzz target
`fuzz_demux_ps` seeds: generated VOB-shaped packs + truncations.

### 1.4 First-vertical confirmation (D-001: AVI + MJPEG + PCM)

- doc 03 §7 recommends **AVI + MJPEG + PCM** [PROPOSAL] (candidate 2), WAV as
  the in-slice audio milestone; doc 04 §8.1 recommends **MP4 + PCM + MJPEG**;
  doc 13 §5 marks the pair `[DECISION-PENDING]` with **both cards written**
  (2.B2 = AVI variant A, 2.B3 = MP4 variant B) and fixture card 2.A3
  generating **both** fixture sets so the decision blocks nothing. D-001 is
  therefore not mine to close; it is a Lead call at wave-2.B launch.
- **Subsystem verdict: nothing in doc 03 §7 needs structural adjustment.**
  Confirmed consistent across docs: MJPEG decoder candidates (jpeg-decoder
  0.3.2 / zune-jpeg 0.5.x, doc 04 §4.1 row, purity `[VERIFY-P0]` carried by
  card 2.B4), PCM passthrough (symphonia-codec-pcm or self-build, card 2.B5),
  every-frame-keyframe seek honesty, and the testkit baseline-JPEG *encoder*
  for license-clean fixtures [PROPOSAL already in doc 03 §7].
- Three recorded notes (for the decision record, not changes): (a) if MP4
  wins, the Phase-2 vertical must pin **non-fragmented** MP4 fixtures — the
  fMP4 corpus (§1.2) lands at 5.C1 and must not gate the vertical;
  (b) if AVI wins, its self-build M is spent in Phase 2 and card 5.J1 becomes
  "polish (OpenDML >2 GB, indx, interleaving)" exactly as written; (c) either
  way the WAV milestone lands first inside the slice (doc 03 §7 candidate 1's
  retained role), de-risking audio-output early — consistent with 2.C3's
  ordering.

---

## 2. Network & streaming

Baseline surface: VLC Linux census = **57 access-family registrations**
(28 `access` + 16 `access_demux` + 11 non-playlist `stream_filter` +
`stream_directory`/`stream_extractor`; counting rule in `modules-linux.json`).
The v1 target per doc 13 Phase 6: HTTP(S), HLS, DASH, buffering, reconnect,
auth; RTSP = investigation + honest verdict; ftp `[PROPOSAL: blocked v1]`.

### 2.1 HTTP(S) client — the honest TLS story (all fetched 2026-10-19)

| Crate | Ver. | Facts (crates.io API, [E1]–[E6]) | Purity verdict |
|---|---|---|---|
| `hyper` | 1.12.0 | non-dev deps: bytes, http, http-body, tokio (+optional futures/h2/tracing…). No cc/build-native signals at this level | **pure Rust core, pass** (async — doc 01 §4.2 edge-only) |
| `reqwest` | 0.13.5 | `default = ["default-tls","charset","http2","system-proxy"]` and **`default-tls = ["rustls"]`** — the default TLS is now rustls, not native-tls. Optional TLS deps: rustls, native-tls, hyper-rustls, quinn (http3) | core pure; **every default TLS path pulls C** (below) |
| `ureq` | 3.4.2 | `default = ["rustls","gzip"]`; core deps pure (ureq-proto, base64, log, percent-encoding, utf8-zero); gzip via flate2 (default backend = miniz_oxide, pure [VERIFY-P0 pin-time]) | core pure; **default TLS pulls C** (below) |
| `rustls` | 0.23.46 | `default = ["aws_lc_rs","logging","prefer-post-quantum","std","tls12"]`; `ring` and `custom-provider` features exist | **not pure with either shipped provider** |
| `ring` | 0.17.14 | **`cc ^1.2.8` as a non-optional *build* dependency**; target-gated `libc` (aarch64/arm linux+android; aarch64 apple); `untrusted`, `getrandom`, `windows-sys` (aarch64 windows) | **honest answer to the task's [VERIFY]: ring compiles C and/or handwritten assembly on every supported target** (the `cc` build-dep is unconditional; ring's repo carries per-arch asm and C). **rustls+ring is NOT pure Rust** |
| `native-tls` | 0.2.18 | target-gated: **`openssl`+`openssl-sys`+`openssl-probe` on non-Windows/non-Apple**; `schannel` on Windows; `security-framework`(-sys) on Apple | Windows/macOS = **E2-class OS TLS**; **Linux = third-party C (OpenSSL) → fails the E-test** |
| `rustls-rustcrypto` | (alpha; last updated 2024-04-24, [E7]) | pure-Rust RustCrypto provider for rustls | technically pure, **immature/alpha** [VERIFY-P0] |

**The finding that must reach the feasibility report**: there is **no
production-grade fully-pure-Rust HTTPS path today**. rustls is Rust, but it
requires a crypto Provider and both providers it ships are C/asm-backed
(aws-lc-rs — its default, with `prebuilt-nasm` objects — and ring with its
unconditional `cc` build). OS-TLS via native-tls is E2-clean on Windows
(SChannel) and macOS (Security.framework) but is third-party C (OpenSSL) on
Linux. The web target needs no engine TLS (fetch/Range per doc 09 §6).

**[PROPOSAL] — new ADR input (suggest ADR-011, "TLS provider policy", or fold
into the ADR-004/E2 register)**: options for the Lead:
(a) accept rustls+ring as a *named third-party-C exception* (a new exception
class — E1-style, not E2; requires explicit ADR + audit of ring's build
graph per target); (b) per-OS TLS: schannel + security-framework (both E2) and
on Linux… blocked (no pure option) — incoherent as a cross-platform story;
(c) rustls `custom-provider` + rustls-rustcrypto — pure but alpha (HTTPS
interop risk unacceptable for v1 [VERIFY]); (d) v1 ships HTTP-only streaming +
defers HTTPS behind the ADR (honest matrix rows; weakens parity immediately).
Recommendation: (a) with a bounded audit (ring 0.17.x pinned, per-target
inventory of compiled C/asm), because HTTPS is table-stakes for a network
media player and VLC-parity requires it; the ADR must record that the product
is then not 100%-Rust-engine and the messaging rule (R-38 wording) applies.

**Client choice [PROPOSAL]**: `ureq 3.x` fits doc 01 §4.2's sync engine far
better than reqwest/tokio (doc 13 6.A1 already names "ureq or reqwest —
purity/async policy per doc 01 §4.2 [VERIFY]"): blocking API, bounded
redirects, no async runtime in the engine. Pin: `default-features = false` +
explicit feature set (`gzip`, TLS per ADR-011) so the provider choice is
visible in Cargo.toml. reqwest remains the fallback if HTTP/2 for
byte-ranging becomes measured-need (its rustls path has the same provider
problem).

### 2.2 HLS / DASH self-build estimates (Phase 6 waves 6.B/6.C)

- **HLS**: playlist parsing crates exist — `m3u8-rs` 6.0.1 (2026-07-30; deps
  nom 7 + chrono, pure [E8]) and `quick-m3u8` 0.8.0 (2026-02-17). No
  player-shaped HLS *client* crate found: `hls` 0.5.6 [E9] is a server-side
  component (deps include aws-sdk-s3, axum, xflv/xmpegts — wrong shape);
  `hls-runtime` 0.9.0 (2026-10-17) is a sans-IO **low-latency** HLS
  (RFC 8216bis) engine — niche profile, applicability to plain VOD/live HLS
  [VERIFY-P0]. **Estimate stands per doc 13**: media-playlist + segment
  scheduling + live window = M (6.B1); master playlists/variants = M (6.B2);
  records + Apple/IETF vectors = S (6.B3). Parse-only via m3u8-rs is a
  candidate but S-sized self-build keeps error taxonomy + fuzz uniform
  [PROPOSAL: self-build M3U8 parse, cross-check with m3u8-rs in tests].
- **DASH**: `dash-mpd` 0.21.0 (2026-10-04, 438k downloads) is a *downloader*
  (37 non-dev deps incl. ffprobe, governor — not an engine dependency);
  **`dash-mpd-core` 0.1.0** [E10] is the clean piece: MPD parse/serialize,
  12 pure deps (quick-xml, serde, chrono, iso8601, url, thiserror…).
  **Estimate**: MPD handling via dash-mpd-core as candidate-oracle with
  self-build M for the player-facing subset (Period/AdaptationSet/
  Representation, SegmentTemplate/List/Base, static+dynamic), scheduler +
  buffer-based ABR = M (6.C2) [PROPOSAL algorithm]. DASH-IF conformance
  vectors subset per 6.C2 AC; per-file license (R-37). **Coupling**: DASH
  media segments are fMP4 → §1.2 corpus classes 8–10 are a Phase-6
  prerequisite; sidx/byterange handling (SegmentBase `indexRange`) is shared
  with 6.A2's ranged reader.

### 2.3 RTSP — honest verdict (doc 13 card 6.G1-I input)

**Protocols do exist in Rust** (task question answered with citations):

- `retina` 0.4.21 [E11] (MIT/Apache-2.0, updated 2026-10-04, 361 KB): "high-
  level RTSP multimedia streaming library"; non-dev deps **all pure-Rust-
  named** (h264-reader, sdp-types, bitstream-io, futures, tokio, socket2,
  url…); default feature = `h265` (i.e., H.264 + HEVC RTP depacketization).
- `rtsp` 0.1.0 [E12] (2026-10-08): async RTSP 1.0/2.0 client+server; 0.1.0 —
  very young.
- `sdp` 0.17.2 (2026-07-20): pure-Rust SDP (plus retina's `sdp-types`).

**Verdict [PROPOSAL]: blocked v1, recorded — feasibility is real but the
prerequisites are not.** Three blockers: (1) **codec gap compounding**: RTSP
cameras overwhelmingly stream H.264/H.265 — exactly the codecs with no
pure-Rust software decoder (doc 14 R-01/R-02); RTSP playback quality would be
ADR-001-hw-only, i.e., the feature's value is chained to the riskiest
decision in the program; (2) **async boundary**: retina is tokio-based;
doc 01 §4.2 confines async to the `streaming` edge, so an RTSP adapter would
need an isolated async island with channel translation into the sync input
crate — buildable, but real design surface; (3) **compat quirks**: VLC's
`access_realrtsp` exists precisely because real cameras violate the spec
(live555-interop folklore) — an unbounded hardening tail we cannot scope
pre-corpus. Record in the 6.G1-I blocked-protocol registry:
`rtsp = blocked v1 (protocol crates exist and are pure — retina 0.4.21;
integration cost + hw-only codec payload + camera-quirk tail)`. Revisit
trigger: ADR-001 landed on all three OS + user demand signal. Also recorded
for the registry: `udp/rtp raw`, `satip`, `rist`, `sdp/sap`, `mms`,
`access_realrtsp`, `nfs/sftp/ftp` (VLC census names) — each gets its
one-line blocked/partial row with reason per 6.G1-I AC. **Task-brief note**:
the brief called this "R-31-class"; doc 14's R-31 is actually the wasm-size
risk — the blocked-protocol family has no register row yet, so
**[PROPOSAL] add one** (e.g. R-40 "blocked-protocol registry breadth") in
the 6.G1-I pass.

### 2.4 Reconnect / auth / remote playlists (waves 6.D/6.E — scope summary)

Bounded retry with backoff + resume-from-offset + generation-safe
cancellation (6.D2, [PROPOSAL] parameters); adaptive buffering with stall
prediction (6.D1 [PROPOSAL]); auth = HTTP basic + digest (6.E1 [PROPOSAL]),
credential-store = OS keychain [VERIFY — policy + crate per OS], redaction
tests (no secrets in logs/errors; media-types policy per doc 01 §2.5);
remote playlists + relative-URI resolution (6.E2); HTML directory-listing
parse [PROPOSAL decision]. All self-build S–M, no crate dependencies beyond
the HTTP client + TLS decision above.

---

## 3. Audio/video outputs

### 3.1 Audio output — cpal vs direct (ADR-005 inputs)

Evidence already on file (0.C4-R + 0.C5-a/b), consolidated:

| Path | Purity class | Key facts (sources) |
|---|---|---|
| `cpal` 0.18.2 facade | **conditional (E2)** | dep tree verified: every native dep is an OS-audio binding (alsa, coreaudio-rs, objc2-*-audio, ndk, wasm-bindgen…), no third-party media C (preaudit-foundation §2). Linux default host = **ALSA via `alsa ^0.11` (C)** with optional pipewire/pulseaudio/jack backends; README: "ALSA is needed even when using JACK, PipeWire, or PulseAudio" (platform-linux §4) |
| Direct WASAPI (Windows) | E2 canonical | cpal's own WASAPI host is written on `windows` 0.62 (no C lib). Shared mode + event-driven `AUDCLNT_STREAMFLAGS_EVENTCALLBACK` proven pattern; **exclusive/bit-exact NOT provided** (would be self-build on windows-rs, IAudioClient2/3) — platform-windows §5 |
| Direct CoreAudio (macOS) | E2 canonical | objc2 family / coreaudio-rs 0.14.2; callback runs on a CoreAudio-owned RT thread — **pull model, inverted vs WASAPI events** (cross-platform design constraint recorded by 0.C5-b); device-change notifications via AudioObjectID property APIs — platform-macos §5 |
| Direct PipeWire (Linux) | C-ABI (upstream-official) | `pipewire` 0.10.1 = pipewire-rs, wraps libpipewire via `pipewire-sys`/`libspa-sys` — upstream project's own bindings; classification (OS service?) is the open ADR-001-analog question (platform-linux §4) |
| Pure-Rust PulseAudio protocol | **pass** (protocol-level) | `pulseaudio` crate = only zero-C Linux client path; rides the compatibility protocol via pipewire-pulse — platform-linux §4.1 |

**ADR-005 synthesis [PROPOSAL]**: the doc 03 §4.9 audio-output trait already
isolates the choice; recommend deciding **cpal-first with per-OS direct
escape hatches** — i.e., v1 ships cpal (least code across 3 OS, callback
model matches doc 03 §4.9's pull-from-ring contract) **but** ADR-005
pre-authorizes the direct adapters for the two named cpal gaps: WASAPI
exclusive/bit-exact (windows-rs) and PipeWire-native routing; the Linux
default (ALSA host, no session mixing) is recorded as a known limitation with
the config-level backend switch as mitigation. Card 2.C3 already encodes
exactly this shape ("cpal [VERIFY-P0] purity; fallback: platform adapters
per doc 01 §2.7"). Risks to carry: R-29 (backend maturity/underruns),
platform-linux L6 (every non-pulseaudio path links C). Web target:
AudioWorklet (doc 09 §5, card 7.H3) — outside ADR-005's native scope but the
same pull-model contract.

### 3.2 Video output — wgpu rendering path (ADR-003/001 inputs)

- `wgpu` 30.0.1 = conditional (E2 graphics envelope; "pure-rust graphics
  API", native backends talk to OS graphics ABIs — preaudit-foundation §1/§2;
  default features include dx12/metal/vulkan/gles per platform-windows §6).
- Card 2.D1 (renderer-wgpu v0: RGBA8 upload, sRGB present, vsync, headless
  PNG-hash test) is crate-feasible today with zero new exceptions beyond the
  E2 register. The **top integration unknown** stays the Windows
  D3D11(MF decoder output) ↔ D3D12(wgpu) interop (platform-windows §6:
  copy path (a) simple-costs-a-copy, shared-handle import (b) [VERIFY wgpu 30
  hal support], D3D11 presentation path (c) [PROPOSAL out-of-scope unless
  ADR-003 decides]) — an ADR-001 spike item, not a Phase-2 blocker (Phase 2
  renders software-decoded frames).
- ADR-003 candidates (video-in-webview) get their decisive evidence from card
  2.D3, not from this section; recorded here only because the renderer is the
  subsystem: Linux WebKitGTK is the high-risk webview (R-04, platform-linux
  TL;DR), Windows Candidate-A feasible-with-conditions (WS_CLIPCHILDREN,
  white-flicker), macOS structurally cleaner. No change to those verdicts.

---

## 4. Filters

### 4.1 The census surface vs our v1

VLC Linux census: **110 filter-family registrations** (modules-linux.json) =
`audio filter` 16 + `video filter` 50 (incl. 5× `vaapi_filters`, 3×
`vdpau_*` hw-gated variants) + `audio converter` 9 + `video converter` 23
(swscale-class chroma conversions) + `audio resampler` 4 + `audio volume` 2 +
`audio renderer` 1 + `video blending` 1 + `glconv` 4. The design package's
player tooling (design/figma, element-id extraction today) maps to a small
named subset:

**v1 audio set** (four filters; all four have dedicated design panels):
`Audio - Equalizer.svg` (Preamp + 10 bands 60/170/310/600/1k/3k/6k/12k/14k/16k
Hz), `Audio - Compressor Settings.svg` (RMS/Peak 0.2, Attack 25 ms, Release
100 ms, Threshold −11 dB, Ratio 4:1, Knee 5 dB, Makeup 7 dB),
`Audio - Spatializer Settings.svg` (Size/Width/Wet/Dry/Damp — design shows
UI-scale values), `Audio - Stereo Widener.svg` (Delay 20 ms, Feedback,
Crossfeed, Dry-mix). Every value matches VLC's binary-observed defaults
except StereoWidener feedback (design 0.2 vs VLC 0.3 = ui-inventory D-3;
engine default 0.3, design value treated as placeholder). VLC module names:
`equalizer`, `compressor`, `spatializer`, `stereo_widen` — all present in the
census `audio filter` list.

**v1 video set** (three panels): `Video - Essential.svg` = `adjust`
(hue/brightness/contrast/saturation/gamma) + `sharpen` (sigma) + `gradfun`
(banding radius); `Video - Crop.svg` = top/bottom/left/right px;
`Video - Colors.svg` = `colorthres`/`extract` (color extraction + threshold +
similarity), `invert` (negate), `posterize`, `gradient`, saturation/sepia
family. All defaults binary-corroborated (ui-inventory §2.11 row 11).

**Insertion points** (per doc 03 §2.1 pipeline + §7 stub list): audio filters
insert in **audio-engine** (convert · filter · mix) between decode and the
SPSC ring — the first vertical ships the chain empty (doc 03 §7 "filters
(empty chain)"), so the insertion seam is designed-in from Phase 2; video
crop/aspect/colors insert at the **video-engine → renderer-wgpu** boundary.
[PROPOSAL] implementation shape: `adjust`/colors as **renderer-wgpu shader
uniforms** (one pipeline, zero extra copies; honors doc 03 §2.3 rule 3 —
renderer sees only frame handles + its own surface/param types), crop/aspect
as **geometry** (presentation-time only, no re-decode), audio DSP as
**audio-engine Rust modules**. This gives CPU-free video color controls and
keeps the renderer isolated.

**Pure-Rust DSP reality**: all four audio filters are textbook small DSP —
biquad IIR (shelf/peaking) for EQ, envelope-follower + gain for compressor,
freeverb-style network for spatializer, delay+crossfeed for stereo-widen —
comfortably **self-build, S total** [PROPOSAL: own code, zero new
dependencies]; helper crates exist if wanted (`biquad` 0.6.0, 2026-03-22;
`fundsp` 0.23.0, 2026-01-07 [E13] — purity [VERIFY-P0] if adopted) but the
parameter sets are small enough that vendor-code is not justified. Resampling
(audio converter family) is already in scope via 3.B1 (bounded audio
resample); VLC's `speex_resampler`/`soxr` have pure-Rust analog decisions
pending in Phase 3.

**Deinterlace**: 50-registration video-filter family includes deinterlace,
`vdpau_deinterlace`, `vaapi_filters` variants, `postproc`, `hqdn3d` — heavy
DSP and hw-coupled. **Later** [PROPOSAL: Phase 6+ decision card, no v1 row
beyond `blocked/later` with reason]; settings UI (4.E3) must render the
deinterlace control disabled-with-reason per config-never-invents (F1/F2).

**Wave-program gap found [PROPOSAL — escalate to Lead]**: doc 13 has **no
engine-side filter-implementation cards** — 4.F3 builds the *UI panels*
("fields present only when the engine supports them") and 7.E1's transcode
graph mentions a filter stage, but nothing implements equalizer/compressor/
spatializer/stereowiden/adjust. Suggest a Phase-4/5 wave (4 cards, S each, or
one M card `4.F4 audio-filters` + fold video-adjust into 4.F3's engine half)
so 4.F3's "no dead controls" AC has something real to bind to.

### 4.2 Census honesty for the matrix

v1 covers 4/16 audio-filter registrations + ~6–8/50 video-filter
registrations (adjust, sharpen, gradfun, crop≈`croppadd`-adjacent, extract,
invert, posterize, gradient). Feature-matrix rows must say `partial` with the
named set and `blocked_reason`/`notes` per doc 02 §4 rules R5/R6 — the other
~90 filter registrations (gaussianblur, motiondetect, mosaic, puzzle,
psychedelic, ball, oldmovie, vhs…) are novelty/analysis effects with no design
panel and no v1 claim.

---

## 5. Playlists & services-discovery

### 5.1 Playlist parsers (wave 5.O)

Census: **21 playlist-bucket registrations** (15 playlist `stream_filter`s +
4 `playlist export` + base entries). Crate status (fetched today):

| Format | Crate evidence | v1 plan (5.O1/5.O2 cards) |
|---|---|---|
| M3U/M3U8 | `m3u` 1.0.0 exists but **2017-frozen** [E14] | self-build S (4.D1 already ships M3U v0; 5.O1 polishes attributes + M3U8) |
| PLS | `pls` 0.2.3 (2025-10-07) — pure, small | candidate; **[PROPOSAL: self-build S]** for uniform errors + fuzz (crate as cross-check oracle) |
| XSPF | `xspf` 0.4.2 (2025-08-07) — pure | candidate; same self-build recommendation |
| CUE | `cue` 3.0.1 (2025-05-21) = **bindings to libcue (C)** [E15] → fails the E-test outright | **self-build S** (REM/INDEX/gaps; card 5.O2 as written) |
| WPL/IFO-as-playlist/others | none found | out of v1; registry rows later |

All four are days-of-work parsers; the deciding factor is error taxonomy +
fuzz-target uniformity (doc 04 §8.5: every parser gets a target), not crate
availability. Directory loading/probing = 5.O3 (S) behind the access trait.

### 5.2 Services-discovery (census 26 registrations)

Members (modules-linux.json): `lua` SD, `udev`×3, `podcast`, `avahi`
(mDNS/DNS-SD renderer discovery), `xcb_apps`, `pulselist` (PulseAudio sink
enumeration), `sap`, `mediadirs`×3, `mtp`, `upnp`, + renderer/probe/services
groups. Rust reality check (fetched today):

- **UPnP**: `rupnp` 3.0.0 (2025-04-25) — async (hyper/ssdp-client/roxmltree;
  tokio optional), pure-named deps. Feasible but async-edge + niche.
- **mDNS/DNS-SD**: `mdns-sd` 0.21.6 (**updated 2026-10-19**, no async
  runtime dependency) and `simple-mdns` 0.7.0 — both pure. Feasible.
- **podcast**: plain HTTP + XML (roxmltree) — feasible S.
- **SAP/mDNS-browser-style discovery**: multicast + SDP; niche.
- **udev**: `udev` crate = libudev C bindings [VERIFY classification:
  arguably OS service]; alternatives: plain inotify/dir-watch [PROPOSAL].

**[PROPOSAL] v1 scope recommendation: minimal — none of the network SD
modules in v1.** Rationale: (1) doc 13 card 7.D1 itself frames SD as
"investigation verdict (implement subset or record blocked)" — Phase 7 tail;
(2) the design package surfaces a file browser/library, not a network-
discovery tree (no design panel to satisfy); (3) each module drags an async
or C-ABI dependency for low user value. Concretely: v1 = directory loading
(5.O3) only; Phase 7 optional tier = local volumes/removable-media events
(7.D1's core, via dir-watch + platform device events — **[VERIFY]** udev
vs inotify classification) and podcast (S, if the library pane wants it);
**blocked v1**: UPnP, mDNS/Chromecast-style renderer discovery, SAP, MTP,
xcb_apps (each gets a registry row with reason; mdns-sd's existence is
recorded so the revisit is cheap).

---

## 6. Transcode / stream-output (sout family)

Census: VLC's sout = **46 registrations** = `sout access` 10 (file, http,
udp, livehttp, shout, rist, ftp, chromecast, avio, dummy) + `sout mux` 11
(ts, ps, mp4×2, ogg, asf, avi, wav, mpjpeg, avformat, dummy) + `sout stream`
23 (transcode, standard, duplicate, display, rtp, bridge, mosaic, record,
chromecast…) + `vod server` 2 (rtsp, rtp). The whole family is built around
**avcodec transcoding** — the one dependency class our program forbids.

**v1 scope (doc 13 wave 7.E, doc 01 §8 Q2 for muxer ownership)**:
- 7.E1 transcode graph core: decode → filter → encode pipeline model, job
  lifecycle, bounded resources; **v1 encoders = WAV PCM + stream-copy only**.
- 7.E2 recording: record-to-WAV from any decodable input; **MP4 remux
  (H.264/AAC stream copy, no transcode claim)**; honest matrix rows. This is
  the user-visible feature (design right-click menu "Record R" / Shift+R,
  ui-inventory §6 crosswalk).
- 7.E3 conversion UI (WAV / MP4-remux targets only) + stream-output subset =
  local file/pipe ES sink [PROPOSAL].

**Crate support for the v1 muxing surface** (fetched today): WAV encode via
`hound` 3.5.1 (doc 04 §2, already verified there); MP4 writing via `mp4`
0.14.0 [E21] (reader+writer; deps byteorder/bytes/num-rational/serde — pure
at this level; **last crates.io release 2023-08-01 → maintenance [VERIFY-
P0]**); TS muxing later via `mpeg2ts` 0.6.1 (2026-08-11, doc 04 §2 row);
FLAC encode for a later tier via `flacenc` 0.5.1 (2025-12-18, pure — doc 04
§2.2 verified). Muxing lives in the `transcode` crate per doc 01 Q2's
proposal (open decision — noted as an ADR-input remainder, not mine to
close).

**Honest gap statement** (for the feasibility report's sout row): v1 = 3
capabilities out of VLC's 46-module family (~7% by count): PCM encode,
lossless remux, file/pipe sink. **No** network stream-output (rtp/rtsp/
chromecast/shout), **no** mosaic/bridge/duplicate graph machinery, **no**
video/audio transcode (every encode beyond PCM is blocked by the encoder-
side mirror of the decoder gap: rav1e exists for AV1 [perf/patent posture
ADR-010], x264-class is forbidden-territory C). The matrix and convert dialog
must show exactly the two targets and nothing more (7.E3 AC: "no capability
shown that is not implemented"). Full sout parity is a post-v1 program
decision, likely permanently partial.

---

## 7. Devices

### 7.1 Capture (dshow / decklink / v4l2 / dv / avcapture)

Census: `access_demux` family carries v4l2×2, dv1394, dtv, dvb, imem;
platform modules dshow/avcapture on win/mac. MP §14: "DVB and capture
hardware cannot be verified without the hardware"; MP §10: web cannot do
capture at all. Rust reality (fetched today): `decklink` 0.1.0 (2023-03-20) =
Blackmagic **NDI bindings** (C); v4l2 access = C-ABI crates in the
ecosystem; no pure path that matters. **Verdict: blocked v1 —
hardware-gated** (cannot be honestly verified without devices, per MP §13
hardware-evidence rules and doc 09 §8.2's hardware-in-hand principle). No
doc 13 phase 2–8 card exists for capture (correct); rows go to the
blocked-protocol/device registry (6.G1-I) with reason "hardware-gated; no
pure-Rust path; untestable in CI". Revisit = never in v1 program unless
hardware + policy appear.

### 7.2 Disc (DVD / Blu-ray)

ADR numbering: the repo's register (doc 01 §7) = **ADR-008 "Optical disc
support scope"**; the task brief's "ADR-009" is a numbering mismatch (doc
01's ADR-009 = WASM threading; doc 14 R-13 uses ADR-009 that way too). This
section writes ADR-008 inputs. Blu-ray: VLC uses libbluray (census demux list
includes `libbluray`); crates.io `bluray` = NOT_FOUND today. **Blu-ray menus
blocked**; partial fallback [PROPOSAL]: BDMV `BDMV/STREAM/*.m2ts` files open
as MPEG-TS (5.I1) with honest "no menus" error/state — record as the
format's `partial` note. DVD: see the μADR below.

### 7.3 Audit item 9 — three micro-ADR decision briefs [PROPOSAL]

**μADR-A — Kate subtitles in Ogg.**
*Context*: VLC ships a `kate` spu decoder (census `spu decoder` list); Kate
is an Ogg-wrapped subtitle codec with karaoke/scroll/motion features.
*Evidence*: crates.io `kate` = NOT_FOUND (2026-10-19); doc 04 §5 row "Ogg
Kate P3 missing, M self-build"; libkate (the reference) is C. *Options*:
(a) self-build M (bitstream documented; render via our subtitle compositor
5.L1); (b) typed-unsupported track (Ogg demux recognizes the Kate mapping,
emits honest error); (c) ignore (mapping not even recognized). *Usage
share*: believed near-zero outside Ogg-advocate archives [VERIFY — doc 04
§8.6 item 7 data]. *Recommendation*: **drop from v1 via option (b)** —
low value per unit effort; matrix row `kate = blocked (no pure-Rust decoder;
niche)`; revisit only if item-7 share data surprises. *Owner*: card 5.P2-I
record; ADR text rides ADR-008's subtitle annex or stands alone in the
blocked-registry.

**μADR-B — FFV1.**
*Context*: lossless archival video codec (Matroska/MOV payloads; RFC 9043
documents it). VLC decodes it via avcodec. *Evidence*: crates.io `ffv1` =
**0.0.0 RESERVED placeholder since 2017-09-19** [E22]; no pure-Rust decoder
exists (doc 04 §4.1 row P3/later, self-build M). *Options*: (a) self-build M
decoder (range coder + contexts, versions 0/1/3; RFC is public; slices →
later threading); (b) `later` with blocked-registry row; (c) v1 attempt —
rejected: no design surface, no corpus in hand. *Patents*: RFC 9043
informational; believed unencumbered **[VERIFY — belongs to ADR-010's
checklist]**. *Recommendation*: **`later` (Phase 6+ decision card)** — FFV1
matters to film-archive users, not to the parity share; if a concrete
use-case appears, schedule the M self-build alongside the Theora-class
decisions. *Owner*: 5.P2-I record + ADR-010 input.

**μADR-C — DVD navigation (libdvd\*).**
*Context*: MP §14: "DVD and Blu-ray copy protection and menus involve C
libraries and legal questions. Decide each by ADR." Census: `dvdnav`,
`dvdread` as access_demux modules; VOB payload = MPEG-PS (§1.3).
*Evidence (fetched today)*: crates.io `dvdnav` NOT_FOUND, `dvdread`
NOT_FOUND; `dvdcss` 0.1.2 (2021) = Rust bindings **to libdvdcss (C)** — an
E1-class third-party C library, not an OS service. *OS-exception analysis*:
**no modern OS ships DVD navigation as a service** — Windows' DirectShow DVD
Navigator is legacy/deprecated [VERIFY status], macOS DVDPlayback framework
is deprecated [VERIFY], Linux has none; libdvdnav/libdvdread are the only
implementation and are third-party C. CSS descrambling adds regional legal
exposure (circumvention law — doc 11 territory). Therefore **no E2 path
exists**; adopting libdvd* would be a forbidden-class exception requiring a
policy change, not an audit pass. *Recommendation*: **DVD-Video navigation =
blocked v1 and absent a policy change, blocked v2+**; v1 ships: (1) VOB/PS
*file* demux per §1.3 (streams exposed; AC-3/DTS/LPCM decode pending 5.K4-I);
(2) VobSub subtitles (vobsub 0.2.3 harden per doc 04 §5; deps are 2017-era —
harden = fork-level work [VERIFY]); (3) matrix rows `dvdnav = blocked
(C-library + legal; see ADR-008)`. Optional [PROPOSAL]: self-build **S**
IFO *metadata* parsing (durations/chapters for the media-info panel) with an
explicit no-navigation claim. *Owner*: ADR-008 (optical disc scope) with
ADR-010 legal input.

---

## 8. Cross-subsystem synthesis

### 8.1 Subsystem × availability × risk × phase

| Subsystem | Availability verdict | Risk | Phase (doc 13) |
|---|---|---|---|
| Demuxers — WAV/FLAC/MP3/Ogg/MKV/TS | crate-exists (symphonia et al.) | L | 2 (stretch) → 5.A–5.I |
| Demuxers — fMP4 | crate-exists (symphonia isomp4; mp4parse oracle limited) + corpus self-build S–M | M | 5.C1 (+6.C coupling) |
| Demuxers — MPEG-PS/VOB | crate-exists but 2-day-old + S self-build (substreams) | M | 5.I2 |
| Demuxers — AVI | missing → self-build M | M | 2 or 5.J1 (D-001) |
| Network — HTTP client | ureq core pure; **TLS has no pure production path** (new finding) | **H** | 6.A (ADR-011 input first) |
| Network — HLS/DASH | parse crates exist; player clients self-build M+M | M | 6.B–6.C |
| Network — RTSP | crates exist (retina, pure) but blocked v1 (codec gap + async + quirks) | M (recorded, not taken) | 6.G1-I registry |
| Audio output | cpal conditional-E2; direct E2 paths proven; Linux purity only via pulseaudio-protocol | M (R-29, L6) | 2.C3 → ADR-005 |
| Video output / renderer | wgpu conditional-E2 | M (R-04; D3D11↔12 interop) | 2.D1/2.D3 → ADR-003/001 |
| Filters — audio 4 | self-build S total, zero deps | L | Phase 4/5 **[card gap — §4.1]** |
| Filters — video essential/crop/colors | shader/geometry in renderer | L | 4.F3 (engine half needed) |
| Filters — deinterlace et al. | blocked/later | M (accepted gap) | 6+ |
| Playlists (M3U/PLS/XSPF/CUE) | mixed crates; self-build S×4 | L | 4.D1 → 5.O |
| Services-discovery | pure crates exist (mdns-sd, rupnp) but minimal v1 | L (scoped out) | 7.D1 verdict |
| Transcode / sout | v1 = PCM + stream-copy + file/pipe (3/46 modules) | M (honest partial) | 7.E1–7.E3 |
| Devices — capture | blocked (hardware-gated, no pure path) | — (not taken) | registry rows |
| Devices — disc | DVD nav blocked (C + legal); Blu-ray menus blocked; TS/PS file fallback partial | M (legal) | ADR-008 + 5.I1/5.I2 |

### 8.2 Three biggest subsystem risks (ranked, my sections only)

1. **TLS provider purity (new, found today).** Every default HTTPS path in
   Rust pulls C: rustls defaults to aws-lc-rs (C + prebuilt nasm), its ring
   provider has an unconditional `cc` build-dep, native-tls is OpenSSL-C on
   Linux; the only pure provider (rustls-rustcrypto) is alpha. This gates
   6.A1 and therefore all of Phase 6, and it is a *policy* hole (E1-class
   exception) rather than an engineering one. Mitigation: ADR-011 [PROPOSAL]
   with bounded ring audit, or HTTP-only v1 (parity cost).
2. **Linux audio path (ADR-005's hardest cell).** Every option except the
   PulseAudio-protocol crate links a C library (platform-linux §4/L6); cpal's
   Linux default (ALSA) lacks session mixing users expect. Mitigation:
   cpal-first + pre-authorized direct adapters; honest config-level backend
   switching; R-29 carries it.
3. **Streaming-video payload gap (R-01/R-02 × Phase 6).** HLS/DASH/RTSP
   deliver H.264/HEVC-dominant catalogs while our software decode for those
   is blocked — network playback quality is chained to ADR-001 landing on
   all three OS. Not new (R-01/02 own it) but the streaming subsystem
   inherits it wholesale; the mitigation is the same hw-first truthfulness
   plus AV1 (rav1d asm-off) as the software flagship.

### 8.3 ADR inputs produced

- **ADR-001** (hw decode): no new evidence (platform docs own it); §3.2
  reiterates the D3D11↔D3D12 interop spike as the top Windows unknown.
- **ADR-005** (audio backend): §3.1 decision matrix + cpal-first-with-
  escape-hatches recommendation [PROPOSAL] + the WASAPI-exclusive and
  PipeWire-native pre-authorizations.
- **ADR-008** (optical disc): the three μADR briefs of §7.3 (Kate, FFV1,
  DVD nav) + Blu-ray m2ts-fallback note + IFO-metadata-only option.
- **ADR-010** (patents): FFV1/RFC-9043 check item; CSS legal exposure.
- **New [PROPOSAL] ADR-011** (TLS provider policy) — §2.1's option ladder.
- **ADR-007 note**: external tools for fMP4 corpus cross-checks must ride
  the same external-oracle rules as VLC (DASH-IF/Apple vector licenses
  [VERIFY]).
- **Risk register additions [PROPOSAL]**: blocked-protocol registry row
  (6.G1-I family; the brief's "R-31-class" label was a mis-ID — doc 14's
  R-31 is wasm size); ring/aws-lc C-in-TLS row unless ADR-011 supersedes.

### 8.4 Inputs to feasibility-report assembly (0.D3)

Pull from this file: §1.2 corpus plan (5.C1/6.C), §1.3 PS scope table,
§2.1 TLS finding (headline), §2.3 RTSP verdict, §3.1 ADR-005 matrix,
§4.1 filter card gap (escalation), §6 sout 3/46 honesty statement, §7.3
three μADRs, §8.1 table. The subsystem headline for the report: **every
subsystem has a viable v1 path; the two policy-grade holes are TLS purity
and Linux audio purity; the two honest ceilings are sout (~7% of VLC's
family) and network-video codecs (hw-only).**

---

## 9. Evidence appendix

All crates.io API fetches 2026-10-19, UA `lumen-preaudit-0.C3-I (subsystem
feasibility)`, raw JSON in `tmp/c3i/` (gitignored); tarballs via
`https://crates.io/api/v1/crates/<n>/<v>/download` (audit-method M4 fallback;
static.crates.io 403 from this sandbox — recorded limitation, audit-method
§7).

- [E1] `https://crates.io/api/v1/crates/reqwest` + `/0.13.5` + `/0.13.5/dependencies` — 0.13.5 (2026-09-08); default = default-tls(charset/http2/system-proxy), default-tls=[rustls]; TLS deps optional (rustls, native-tls, hyper-rustls, quinn).
- [E2] `https://crates.io/api/v1/crates/hyper` + `/1.12.0/dependencies` — 1.12.0 (2026-10-06); non-dev deps bytes/http/http-body/tokio (+optional). Pure core.
- [E3] `https://crates.io/api/v1/crates/ureq` + `/3.4.2` + `/3.4.2/dependencies` — 3.4.2 (2026-09-13); default=[rustls,gzip]; core deps ureq-proto/base64/log/percent-encoding/utf8-zero; TLS/flate2/cookies optional.
- [E4] `https://crates.io/api/v1/crates/rustls` + `/0.23.46` — 0.23.46 (2026-10-14); default=[aws_lc_rs,logging,prefer-post-quantum,std,tls12]; ring + custom-provider features.
- [E5] `https://crates.io/api/v1/crates/ring/0.17.14/dependencies` — 0.17.14 (2025-03-11); **cc ^1.2.8 build, non-optional**; libc (target-gated aarch64/arm linux+android, aarch64 apple); windows-sys (aarch64 windows).
- [E6] `https://crates.io/api/v1/crates/native-tls/0.2.18/dependencies` — openssl+openssl-sys+openssl-probe on non-win/non-apple; schannel (win); security-framework (apple).
- [E7] `https://crates.io/api/v1/crates/rustls-rustcrypto` — pure-Rust provider; updated 2024-04-24 (alpha-class).
- [E8] `https://crates.io/api/v1/crates/m3u8-rs` + `/6.0.1/dependencies` — 6.0.1 (2026-07-30); deps chrono + nom7(optional).
- [E9] `https://crates.io/api/v1/crates/hls` + `/0.5.6/dependencies` — 0.5.6 (2026-10-16); server-shaped (aws-sdk-s3, axum, xflv, xmpegts) — not a client candidate.
- [E10] `https://crates.io/api/v1/crates/dash-mpd` + `/0.21.0/dependencies`; `.../dash-mpd-core/0.1.0/dependencies` — core: 12 pure deps (quick-xml, serde, chrono, iso8601, url, thiserror…); full: 37 deps incl. ffprobe.
- [E11] `https://crates.io/api/v1/crates/retina` + `/0.4.21` + `/0.4.21/dependencies` — 0.4.21 (2026-10-04), MIT/Apache-2.0, 361 KB; deps h264-reader/sdp-types/bitstream-io/tokio/futures… (pure-named); default feature `h265`.
- [E12] `https://crates.io/api/v1/crates/rtsp` + `/0.1.0/dependencies` — 0.1.0 (2026-10-08); futures-util/log/tokio/tokio-util.
- [E13] `https://crates.io/api/v1/crates/biquad`; `.../fundsp` — 0.6.0 (2026-03-22); 0.23.0 (2026-01-07).
- [E14] `https://crates.io/api/v1/crates/m3u` — 1.0.0, created 2017-01-06 (frozen). Also `pls` 0.2.3 (2025-10-07), `xspf` 0.4.2 (2025-08-07).
- [E15] `https://crates.io/api/v1/crates/cue` — 3.0.1 (2025-05-21) = "High-level bindings for the libcue library" (C).
- [E16] `https://crates.io/api/v1/crates/symphonia-format-isomp4` + tarball 0.6.1 — MoofSegment/trafs/mfhd/SidxAtom verified in `src/stream.rs`/`src/demuxer.rs`; 0 native files, no build.rs.
- [E17] `https://crates.io/api/v1/crates/mp4parse` + `/0.17.0` + tarball — MPL-2.0, published 2023-05-29; `read_mp4` top-level loop handles ftyp/moov/meta and **skips moof/mdat/sidx** (src/lib.rs ~4190–4240); 0 native files, no build.rs.
- [E18] `https://crates.io/api/v1/crates/mpeg-ps` + `/0.6.0` + `/0.6.0/dependencies` + tarball — MIT OR Apache-2.0, **published 2026-10-17**; no_std; 0 native, no build.rs; deps broadcast-common/mpeg-pes/thiserror(+serde opt); README scope/non-goals as quoted in §1.3.
- [E19] `https://crates.io/api/v1/crates/mpeg-pes/0.6.0/dependencies` + tarball — pure; PES incl. private_stream_1/2 stream-ids, opaque payload.
- [E20] crates.io search `q=dash+mpd`, `q=hls` (2026-10-19) — ecosystem scan: dash-mpd-cli, quick-m3u8 0.8.0, hls-runtime 0.9.0, ll-hls-runtime 0.3.1, transmux 0.27.7, multimux 0.13.0, media-doctor 0.11.0.
- [E21] `https://crates.io/api/v1/crates/mp4` + `/0.14.0/dependencies` — 0.14.0 (2023-08-01); reader+writer; deps byteorder/bytes/num-rational/serde/serde_json/thiserror.
- [E22] NOT_FOUND checks (2026-10-19): `kate` (errors: crate does not exist), `dvdnav`, `dvdread`, `bluray`; `ffv1` = 0.0.0 RESERVED (2017-09-19); `dvdcss` 0.1.2 (2021-01-20) = libdvdcss bindings; `decklink` 0.1.0 (2023-03-20) = NDI bindings; `mpeg2ts-reader` 0.18.2 (2025-01-29), `mpeg2ts` 0.6.1 (2026-08-11), `vobsub` 0.2.3 (2017-05-22, ancient dep tree), `flacenc` 0.5.1 (2025-12-18); `sdp` 0.17.2 (2026-07-20); `rupnp` 3.0.0 (2025-04-25, hyper/roxmltree/ssdp-client, tokio optional); `mdns-sd` 0.21.6 (2026-10-19, "no async runtime dependency"); `simple-mdns` 0.7.0 (2026-05-15).
- Doc cross-refs (read this date): doc 01 §2.7/§4.2/§7; doc 03 §2/§7; doc 04 §2/§8.1–8.6; doc 13 §5–§10 cards (2.A3/2.B2–2.B5/2.C3/2.D1–2.D3/4.F3/5.C1/5.I1–5.I3/5.J1/5.K4-I/5.O1–5.O3/5.P2-I/6.A–6.I/7.A–7.E/7.D1); doc 14 rows R-01/02/04/13/29/31/32/37/38; modules-linux.json (0.B2); platform-linux.md §4/§6.3, platform-windows.md §5–§7, platform-macos.md §5/§7 (0.C5-a/b); preaudit-foundation.md (0.C4-R); ui-inventory.md §2.11 row 11 + D-3 (0.B3); MASTER-PROMPT §10/§13/§14; design/figma panel SVGs (element-id extraction).
- Design-package note: `design/figma/Audio - *.svg`, `Video - {Colors,Crop,Essential}.svg` carry text-as-paths; labels were extracted from element `id`/`data-name` attributes (method recorded here for reproducibility).

## 9.1 Remaining [VERIFY] ledger (numbered, this section's additions)

1. `flate2` default backend = miniz_oxide (pure) at the ureq pin — confirm in
   Cargo.lock at Phase 1 (6.A1).
2. `rustls-rustcrypto` maturity/interop — if ADR-011 option (c) is taken.
3. `mp4` 0.14.0 maintenance (last release 2023-08-01) — or self-build the
   MP4 remux writer in transcode (7.E2 decision input).
4. ureq/reqwest exact resolved feature graph at pin (provider visibility).
5. ring per-target C/asm inventory (files, arches) — required content of the
   proposed ADR-011 bounded audit.
6. Usage-share data (doc 04 §8.6 item 7) for: MPEG-1 PS scope, Kate, FFV1,
   PLS/XSPF-vs-self-build (drives several §5/§7 recommendations).
7. DASH-IF / Apple HLS example-stream license terms for CORPUS.md (R-37).
8. DirectShow DVD Navigator and macOS DVDPlayback deprecation status (μADR-C
   OS-exception analysis).
9. FFV1 patent posture — add to ADR-010 checklist (RFC 9043).
10. retina codec coverage beyond H.264/H.265 + camera-quirk corpus
    feasibility — only if RTSP is revisited post-ADR-001.
11. `biquad`/`fundsp` purity — only if adopted over self-build DSP.
12. `mpeg-ps` 0.6.x follow-up: API stability + bus factor after its first
    month (re-check at 5.I2 launch; thin-wrapper mitigates).
13. `hls-runtime` (LL-HLS, RFC 8216bis) applicability to standard HLS v1 —
    else self-build stands.
14. udev-vs-inotify classification for 7.D1 device events (OS-service
    test) + `udev` crate C-ABI classification.
15. OS keychain crates for 6.E1 credential store (windows-rs /
   security-framework / libsecret classification).
16. `vobsub` 0.2.3 harden depth (2017-era deps: nom 2, image 0.13,
   error-chain) — fork-level estimate [VERIFY at 5.N2].
17. flate2/miniz_oxide + h2 quinn graphs — covered by the Phase-1 cargo-deny
   sweep, listed here because §2's verdicts depend on them.

---

*End of section. Doc 04's own [VERIFY] marks are preserved elsewhere; this
file only sharpens the three audit items assigned to it (3, 4, 9) and adds
network-stack evidence. No VLC source was consulted; no git commit was made
(AGENTS §3.8).*
