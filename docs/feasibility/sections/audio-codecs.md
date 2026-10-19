# Feasibility §4 — Audio codecs (decode)

Task ID: 0.C2-I (Wave 0.C, investigation). Date of verification: **2026-10-19**.
Feeds: `docs/feasibility-report.md` §4 (doc 02 §5 item 4). Source tables: doc 04
§3 (audio decoders) + §8.6 audit items 1 (Symphonia AAC/Opus) and 5 (truehdd).
Risk linkage: R-06 (Opus), R-10 (AC-3/DTS/WMA/TrueHD), R-11-adjacent none.
Baseline scope: VLC 3.0.23 Linux census — `audio decoder` capability = 18
registrations across 16 real decoders (docs/inventory/modules-linux.json,
`by_capability.audio decoder`), counted with the doc 04 rollup as 35 total
"decoders" (18 audio + 14 video + 3 hw).

**Method.** Every crate marked **[RV]** below was re-verified today
(2026-10-19) via the crates.io HTTP API (`https://crates.io/api/v1/crates/<name>`,
User-Agent `lumen-phase0-audit`) and, for the Symphonia/truehdd/oxisynth
findings, via `raw.githubusercontent.com` fetches of the current `main`/`master`
branch files (exact URLs in §7). Anything not marked **[RV]** keeps doc 04's
2026-10-19 marks and its original `[VERIFY]` status. Purity designations remain
"verified promising, not approved" (doc 04 §1.1): the Phase 0 automated
dependency audit (doc 04 §8.6 item 10) still has to run `cargo tree`, tarball
scans and cargo-deny on every crate named here before any is *approved*.

---

## 1. Per-codec verdict table

Verdict vocabulary mirrors doc 04 §1.1; "v1 path" uses the task's three values:
`software-decode` (compliant pure-Rust decode ships in v1), `partial` (ships
with a named, matrix-visible gap), `blocked` (no compliant path in v1; typed
error, never silence). Effort scale S/M/L/XL per doc 04 §1.4.

| # | Codec | Prio | Pure-Rust availability (re-verified) | v1 path | Parity consequence |
|---|---|---|---|---|---|
| 1 | PCM (s16/s24/s32/f32/u8, planar, BE/LE) | P0 | `symphonia-codec-pcm` 0.6.1, MPL-2.0, published 2026-08-13, upstream status "Excellent" **[RV]** | software-decode | Full. WASM endianness + rare channel orders remain per-record tests (doc 04 row) |
| 2 | MP3 (MPEG-1/2/2.5 L-III) | P0 | `symphonia-bundle-mp3` 0.6.1, MPL-2.0, 2026-08-13, "Excellent", gapless; also MP1/MP2 **[RV]**. `minimp3` 0.6.1 re-confirmed to be bindings to the minimp3 C library — forbidden **[RV]** | software-decode | Full for common corpus; MAD-class edge cases (free-format bitrates) only measured on our differential corpus (doc 04 §3.2) |
| 3 | MP2 | P1 | same crate, upstream status "Great" **[RV]** | software-decode | Broadcast corruption tolerance = Phase 5 differential work, not a code gap |
| 4 | AAC-LC | P0 | `symphonia-codec-aac` 0.6.1, MPL-2.0, 2026-08-13, "Great"; ADTS reader (`AdtsReader`) ships inside the codec crate **[RV]** (§2.1) | software-decode | MP4/M4A + ADTS play; conformance breadth is our corpus job |
| 5 | AAC in MP4 vs ADTS (framing) | P0 | ASC-in-esds unwrapping glue on top of the same decoder (doc 04 row, unchanged) | partial | S-sized demux glue; see §2.1 — ADTS parsing itself already exists upstream |
| 6 | HE-AAC (SBR) / HE-AACv2 (PS) | P1 | **None.** Upstream status "—"; no `sbr.rs`/`ps.rs` modules exist in `symphonia-codec-aac@main`; no `he-aac`/`he-aac-v2` features in its Cargo.toml (README rows are aspirational, §2.2) **[RV]** | blocked (v1) | HE-AAC files decode as half-rate LC (audible) → must be *detected and reported*, not silently played (§2.2) |
| 7 | FLAC | P0 | `symphonia-bundle-flac` 0.6.1, "Excellent" **[RV]**; `claxon` 0.4.3, Apache-2.0, last release 2020-08-09 **[RV]** (stable, slow cadence; oracle role) | software-decode | Full; encode later via `flacenc` 0.5.1 (pure Rust) **[RV]** |
| 8 | ALAC | P1 | `symphonia-codec-alac` 0.6.1, "Great", gapless **[RV]**; legacy `alac` 0.5.0 (2018, ebarnard) kept as oracle per doc 04 **[RV]** | software-decode | Full for common files |
| 9 | Vorbis | P0 | `symphonia-codec-vorbis` 0.6.1, "Excellent" **[RV]**; `lewton` 0.10.2 (MIT/Apache-2.0, 2021, unmaintained) — demoted to oracle per doc 04 **[RV]** | software-decode | Full |
| 10 | Opus | P0 | **None published.** `symphonia-codec-opus` does not exist on crates.io (API returns `crate does not exist`, 2026-10-19) **[RV]**; upstream `main` holds a 1-byte-lib.rs placeholder **[RV]** (§2.3); `opus` 0.4.0 / `audiopus` are libopus C bindings — forbidden **[RV]** | blocked (v1) | WebM/YouTube-class audio unplayable; the R-06 decision, options in §4 |
| 11 | WMA 1/2 / Pro | P2 | None. crates.io `wma` NOT_FOUND (re-checked 2026-10-19) **[RV]**; WMA is absent from Symphonia's upstream codec table entirely (§2.4) | blocked | ASF/WMV audio dead in v1; Phase 7 decision point (doc 14 R-10) |
| 12 | AC-3 / E-AC-3 | P1 | None. crates.io `ac3` NOT_FOUND (re-checked) **[RV]**; liba52 is C (doc 04) | blocked (v1) → partial (P6) | Self-build L in Phase 6 (§5.2); typed `unsupported-decoder` until then |
| 13 | DTS (Coherent Acoustics) | P2 | None. crates.io `dts` exists but is unrelated data-transform tooling (re-checked) **[RV]** | blocked (later) | Phase 7 decision point; spec availability is itself a [VERIFY] (§5.3) |
| 14 | TrueHD (MLP) | P2 | `truehd` lib 0.7.2 + `truehdd` CLI 0.6.2, Apache-2.0, both published 2026-09-15 **[RV]**; self-described experimental (§3) | partial (P6) | Decodes, but research-grade: Phase 6 hardening is mandatory (§3) |
| 15 | ADPCM (MS/IMA/OKI/Yamaha…) | P1 | `symphonia-codec-adpcm` 0.6.1, "Good", default-on **[RV]** | software-decode | Exotic variants audited per record; differential corpus |
| 16 | WavPack | P3 | **Changed since doc 04.** Upstream placeholder empty (as Opus) **[RV]**; a third-party `symphonia-codec-wavpack` 0.1.1 (pierreaubert/symphonia-add-ons, published 2026-09-06) exists but is **GPL-3.0-or-later** **[RV]** — a copyleft conflict candidate for our MIT repo | blocked (later) | License question for doc 11 (§5.5); decode stays `missing` in v1 |
| 17 | Speex | P3 | None. crates.io `speex` NOT_FOUND (re-checked) **[RV]** | blocked (later) | Legacy VoIP streams stay `missing`; P3 |
| 18 | Musepack SV7/SV8 | P3 | None. crates.io `musepack` NOT_FOUND (re-checked) **[RV]** | blocked (later) | P3; M self-build if ever demanded |
| 19 | MIDI / synthesized | P3 | `rustysynth` 1.3.7, MIT, published 2026-10-01, pure Rust, **std-only**, *includes SMF parsing* **[RV]**; `oxisynth` 0.1.0, **LGPL-2.1**, 2025-05-25 **[RV]** (§ of subtitles doc, audit item 6) | blocked (later) | MIDI = `later`, not blocked; recommendation in the subtitles file §5 (MIDI's rendering is a synth question, ownership noted there) |
| 20 | ATRAC1/3, ATRAC-X | P3 | None. crates.io `atrac` NOT_FOUND (re-checked) **[RV]** | blocked (later) | Unchanged from doc 04 |

### 1.1 Census reconciliation (VLC's 18 audio-decoder registrations vs the table)

The Linux census (modules-linux.json, `audio decoder`) lists registrations doc
04's table does not name. Disposition so nothing is silently dropped:

| VLC module | What it is | v1 disposition |
|---|---|---|
| `araw` / `lpcm` | Raw + linear PCM | covered by row 1 (`symphonia-codec-pcm`) plus container fourcc mapping |
| `mad`, `mpg123` | Two MP3 paths (MAD, mpg123 — both C) | covered by row 2 (one Rust path, not two) |
| `faad` | libfaad2 AAC | rows 4–6 (LC yes, HE no) |
| `flac`, `vorbis`, `adpcm`, `speex` | 1:1 with rows 7, 9, 15, 17 | — |
| `avcodec` (audio side) | FFmpeg long tail (WMA, DTS, AC-3, TrueHD, cook, atrac3, …) | rows 11–14, 17, 20 — the honest hole |
| `opus` | libopus | row 10 |
| `g711` | G.711 A-law/µ-law | **not in doc 04 §3** — add: table-based codec, S self-build or [VERIFY] a pure crate; P3, trivially `later` |
| `aes3` | SMPTE ST 302M (PCM-in-AES3) | **not in doc 04 §3** — add: framing/de-interleave glue over PCM, S–M self-build; P2/P3, `later` [VERIFY spec detail] |
| `uleaddvaudio` | Ulead DV audio | **not in doc 04 §3** — P3, `later/blocked` [VERIFY format spec availability] |
| `spdif` | S/PDIF *pass-through* (not decode) | output-mode, not a decoder — doc 03 AudioSink scope; out of this section |
| `ddummy`×2, `stats` | test/dummy modules | no analog by design |

Three census-only formats (g711, aes3, uleaddvaudio) need doc 04 table rows
when the Lead next touches it — flagged in §8 [VERIFY] list, not silently added.

---

## 2. Symphonia deep-dive (doc 04 §8.6 item 1)

### 2.1 What Symphonia is, re-verified today

- Umbrella `symphonia` 0.6.1 on crates.io, published **2026-08-13**, MPL-2.0,
  repo `github.com/pdeljanov/Symphonia` (default branch **`main`**; the old
  `master` branch is stale at the 0.5.x line — a trap for citation, noted in
  §7) **[RV]**. 17.7 M total downloads, 9.0 M recent (crates.io, 2026-10-19).
- README (`main`, fetched 2026-10-19): *"100% safe Rust"*, *"pure Rust audio
  decoding and media demuxing library supporting AAC, ADPCM, AIFF, ALAC, CAF,
  FLAC, MKV, MP1, MP2, MP3, MP4, OGG, Vorbis, WAV, and WebM"*. MSRV for the
  0.6.x line: **1.85** (README "Minimum Supported Rust Version"). SIMD is
  now **on by default** (SSE/AVX/Neon via `rustfft`), a change from the 0.5
  era where it was opt-in — relevant to our reproducible-build story, flag to
  doc 11/toolchain.
- Project shape: single primary author (Philip Deljanov); AAC-LC decoder
  contributed by Kostya Shishkov (README "Special Thanks"). **Bus factor is a
  real supply-chain risk**; mitigation = our crates are pinned + hash-verified
  and the codec-api trait means any component can be swapped later.

### 2.2 AAC: what "Great" buys us, and the HE-AAC wall

- Upstream codec status table (`main` README, fetched 2026-10-19): **AAC-LC
  "Great"** (most streams play; inaudible glitches possible); HE-AAC (SBR) and
  HE-AACv2 (PS) both **"—"** (their vocabulary: "in work or not started").
- Sharper than the README: the `symphonia-codec-aac` crate on `main` contains
  exactly two modules — `mod aac; mod adts;` (`src/lib.rs`, fetched 2026-10-19)
  — i.e. the AAC-LC decoder **plus an ADTS framing reader** (`AdtsReader` is
  publicly exported). There is **no `sbr`/`ps` source file anywhere in the
  crate** (probed `src/aac/sbr.rs`, `src/aac/ps.rs` → 404) **[RV]**, and the
  crate's Cargo.toml has **no `[features]` section at all** — the README's
  `he-aac` / `he-aac-v2` feature flags do not exist in code; those README rows
  are aspirational **[RV]**. The upstream umbrella's `all-codecs` list contains
  no HE-AAC, Opus or WavPack entries **[RV]**.
- The ecosystem's own answer to HE-AAC today is a **C adapter**:
  `symphonia-adapter-fdk-aac` 0.2.0 (crates.io, updated 2026-05-16) — "AAC-LC,
  HE-AAC, & HE-AACv2 via `libfdk-aac`" **[RV]**. Forbidden for us (MP §9); its
  existence is evidence the pure-Rust HE-AAC gap is real and unsolved.
- Consequence for the v1 matrix: `aac = partial (LC)`. HE-AAC detection rule
  (doc 04 §3.2 honesty note, kept and sharpened): an ASC carrying
  `explicit_sbr`/`implicit_sbr` or PS signaling must surface a typed
  `unsupported-feature` event with "playing core-band audio at N/2 rate"
  wording — we do not silently hand HE-AAC to the LC decoder and call it
  playback. Self-build estimate for SBR/PS remains **M** (doc 04), queued
  against Phase 5+; not started by us today.

### 2.3 Opus: the placeholder finding (hard evidence)

- crates.io API (2026-10-19): `https://crates.io/api/v1/crates/symphonia-codec-opus`
  → `{"errors":[{"detail":"crate `symphonia-codec-opus` does not exist"}]}` **[RV]**.
- Upstream repo `main` (fetched 2026-10-19): `symphonia-codec-opus/` **is** a
  workspace member, but `src/lib.rs` is **1 byte (empty)** and its README
  states verbatim: *"This is a placeholder crate for Project Symphonia's
  work-in-progress Opus decoder"* **[RV]**. Version 0.0.1, depends on
  `symphonia-core 0.6.1` (path dep).
- The README's status-table row (`Opus | — | — | opus | Yes | symphonia-codec-opus`)
  claims a **default-on** `opus` feature; the umbrella Cargo.toml on `main`
  has **no `opus` feature and no opus dependency** — the table is describing a
  future state that the code does not match **[RV]**.
- Same shape for WavPack: `symphonia-codec-wavpack@main` is an empty 0.0.1
  placeholder **[RV]**; the *published* `symphonia-codec-wavpack` 0.1.1
  (2026-09-06) comes from a **different repo** (`pierreaubert/symphonia-add-ons`)
  and is **GPL-3.0-or-later** **[RV]** — not upstream Symphonia, and a license
  conflict candidate for an MIT product (doc 11's call, §5.5 here).
- The ecosystem's Opus answer today is likewise a C adapter:
  `symphonia-adapter-libopus` 0.3.0 ("Opus via libopus", updated 2026-05-16)
  **[RV]** — forbidden for us.
- **Verdict on audit item 1's Opus half:** there is no "in-work decoder" to
  wait for or contribute to — there is a reserved name and an empty file.
  Doc 04's "wait/contribute upstream" option must be re-scoped at 0.D3 (see
  §4); "symphonia-codec-opus not yet on crates.io" (doc 04 §3.1) stays
  factually true but now with the stronger, cited reason.

### 2.4 WMA/MP3 decode quality status per upstream table (as asked)

Re-fetched `main` README codec table, 2026-10-19: **MP3 "Excellent"** (their
top class: "all media streams play… passes compliance tests or matches a
reference implementation over a large corpus"), MP1/MP2 "Great", PCM
"Excellent", Vorbis "Excellent", FLAC "Excellent", ALAC "Great", ADPCM "Good",
AAC-LC "Great". **WMA does not appear in the upstream table at all** — it is
not "in work", it is not on Symphonia's roadmap (README features list names
no WMA crate; no `symphonia-codec-wma` workspace member or crate exists
**[RV]**). Doc 04's `missing` verdict for WMA is therefore *more* solid today:
no maintained pure-Rust WMA decoder exists and none is upstream-planned.

### 2.5 Ship-with vs self-build, per codec family

- **Ship with Symphonia (v1, Phase 2):** PCM, MP3/MP2, FLAC, ALAC, Vorbis,
  AAC-LC, ADPCM — plus its demux glue (isomp4, mkv, ogg, riff/wav, caf) and
  metadata readers. Our work: the `codec-api` adapter crate, container fourcc
  → layout mapping, gapless/priming plumbing for MP3/FLAC/ALAC/Vorbis, and a
  **differential conformance harness vs VLC 3.0.23** (doc 06 §7.1) —
  upstream's "Excellent" is *their* corpus claim, not ours; the feature
  matrix only moves on our evidence (doc 04 §7 closing rule).
- **Self-build (scheduled):** ADTS/esds AAC glue (S), HE-AAC SBR/PS (M, P5+),
  AC-3 (L, P6), TrueHD hardening (M, P6 — mostly testing, §3).
- **Self-build (unscheduled, honestly `blocked`/`later`):** Opus (decision
  §4), DTS (L), WMA (L), WavPack (M + license question), Speex (L),
  Musepack (M), ATRAC (L).

---

## 3. truehdd conformance (doc 04 §8.6 item 5)

**Current state, re-verified 2026-10-19** (crates.io + repo README
`raw.githubusercontent.com/truehdd/truehdd/main/README.md`):

- Two crates: `truehd` (the library, **0.7.2**) and `truehdd` (the CLI, 0.6.2),
  both published **2026-09-15**, **Apache-2.0**, repo `github.com/truehdd/truehdd`
  **[RV]**. Library downloads to date: ~3.4 k (small, young — doc 04's
  "research implementation" framing holds).
- The README's own banner: *"⚠️ Experimental — designed for research and
  development purposes. It is not intended for production environments or
  consumer playback systems."* **[RV]**. MSRV **Rust 1.95.0** — a very recent
  toolchain floor; our workspace MSRV policy must either match or vendor
  patches (flag to Lead/toolchain).
- What already exists and helps us (all cited from the README, fetched
  2026-10-19):
  - a **`verify` conformance command**: parses a whole stream, reports every
    fired rule with rule ID, severity, access-unit and bit position, per-rule
    tallies, FIFO-cap peaks, seamless-branch points, disc-authorability
    verdicts, JSON output mode, and a dedicated exit code 7 for
    non-conformance — i.e. upstream has a conformance *framework* already;
  - **damaged-stream behavior**: resync at next major sync, lost frames
    written as silence to keep A/V sync, `skippedFrames`/`concealedFrames`
    counters in a machine-readable JSON result — maps cleanly onto our typed
    event model for truthful reporting;
  - multi-presentation decode (presentations 0–3 incl. **Dolby Atmos/DAMF
    object audio** with `.atmos`/`.audio`/`.metadata` sets), **MLP for
    DVD-Audio**, warp-mode handling, Evolution frame HMAC verification
    (key-supplied), `--strict` mode;
  - CI badge on the repo.
- **What Phase 6 hardening must add (ours, on top of upstream's state):**
  1. **Decode-equality corpus.** `verify` checks *bitstream conformance*, not
     PCM equality. We need a corpus (movie-file .thd tracks we can license or
     self-encode; Dolby reference streams are **not** redistributable
     [VERIFY licensing]) differentially decoded vs VLC 3.0.23 (libavcodec
     MLP/TrueHD path) as oracle, per doc 06 §7.1 — sample-exact or
     tolerance-documented.
  2. **Fuzz target** `fuzz_codec_truehd` per doc 04 §8.5 rules (10-min clean
     CI run, committed seed corpus, coverage gate); upstream has no published
     fuzz corpus we can cite **[VERIFY upstream fuzzing state]**.
  3. **Seek edge cases:** access-unit-aligned seeking, major/minor sync-word
     search cost bounds, presentation index switch mid-stream, seamless
     branch splices (the README exposes `branches`/`invalidBranches` — our
     seek tests must cover both), MLP-vs-TrueHD mode transitions.
  4. **codec-api integration costs:** downmix policy (we take multichannel
     PCM; our mixer does the rest), sample-rate families (44.1/48/96/192),
     the Atmos object presentation (v1 ships **presentations 0–2 only**;
     Atmos metadata rendering is out of v1 scope — matrix note, not hidden).
  5. **Error taxonomy:** map skipped/concealed frames to typed
     `decoder-concealment` events (MP §8 — never fabricate success).
- Verdict: `truehd = partial` stands; the path to `implemented` is a testing
  project (M), not a codec-writing project. That is the best shape a "gap"
  can have — and the census confirms VLC's own TrueHD came from libavcodec
  (`avcodec` registration), so our parity story for TrueHD is *different
  implementation, tested against the same oracle*.

---

## 4. Opus strategy options (R-06) — decision framing for 0.D3

The R-06 premise is updated by §2.3: there is no unpublished-but-written
Symphonia Opus decoder to wait for. Options, honestly costed:

**(i) Blocked v1, Vorbis fallback where it exists.** Cost: none. Parity cost:
every Opus-only stream (most modern WebM audio, YouTube-class downloads,
WhatsApp/Telegram voice notes, many podcasts) returns a typed
`unsupported-decoder` error — visible, never silence. WebM files that carry a
Vorbis track (dual-audio encodes are rare but real) can fall back with a UI
notice (doc 14 R-06 mitigation already says this). This is the floor, not a
strategy.

**(ii) Self-build decoder (honest cost).** RFC 6716 (IETF Standards Track,
freely published) + RFC 8251 fully specify the decoder; the spec is public,
so clean-room legality is clean — the cost is engineering. The codec is a
SILK (LPC, LTP, shaped-noise, complex pitch lags) + CELT (MDCT bands,
PVQ spread/energy, anti-collapse) hybrid with a range coder, mode/bandwidth
switching, and stereo prediction (intensity + M/S). Building blocks exist in
pure Rust (rustfft or realmdft for MDCT; the range decoder is ~an afternoon),
but conformance-grade output — bit-exact enough that our differential harness
vs libopus passes at 16/24/48 kHz, all modes, FEC, discontinuous
transmission — is the real cost. Estimate: **L–XL** (doc 04 §3.1 number
kept): one strong engineer, 1–2 quarters including the conformance corpus
(Xiph's BSD-licensed `opus_testvectors` / conformance vectors [VERIFY exact
license of the specific corpus we use]) and fuzzing. It would be the only
maintained pure-Rust Opus decoder in existence — high strategic value,
high cost.

**(iii) Wait for Symphonia publication.** *Downgraded to non-viable as a
plan:* the upstream artifact today is a 1-byte placeholder (§2.3). "Wait"
has no observable progress signal (no branch, no PR, no timeline found on
the repo as of 2026-10-19 **[RV]**). "Contribute upstream" is really (ii)
with a different home — building the decoder *in* Symphonia's framework so
publication gives the ecosystem (and us) the result. That framing keeps
doc 04's "contribute rather than parallel-build" spirit but must not be read
as "cheap".

**(iv) WebCodecs `AudioDecoder` on web only.** A browser API, not a bundled
codec — policy-clean in principle (same class as the doc 09 §10 / ADR-001
WebCodecs discussion). Platform evidence already collected: WebKitGTK 2.52.6
Debian binary has `WebCodecsAudioEnabled` + `AudioDecoder` strings
compiled-in (docs/inventory/platform-linux.md §2.1, verified in-binary
2026-10-19); doc 09 §8.1 records WebCodecs API support Chrome/Edge 94+,
Firefox 130+ desktop, Safari 16.4+ — but the **AudioDecoder** sub-API
coverage per browser (notably Firefox and Android) is **[VERIFY]**. Limits:
does nothing for our native desktop builds (the primary VLC-parity surface),
and depends on the still-open WebCodecs-ADR scope (doc 04 §8.6 item 8).

**Recommendation framing [PROPOSAL] for 0.D3 (Lead decides):**
- v1 ships **(i)**: `opus = blocked` in the matrix with `blocked_reason`
  citing R-06 + this section; typed error; Vorbis-fallback UI note.
- Open **(ii)** as a Phase 5/6 project with a written go/no-go gate at
  Phase 5 entry (candidate scope: decoder-only, RFC 6716, no DTX/FEC
  encode-side concerns, 48 kHz + resampler mandatory, conformance corpus
  + fuzz from day one), with an explicit option to develop it as a
  `symphonia-codec-opus` contribution (upstream-first) so the ecosystem
  result is shared. Budget line: L–XL; do not start before the Phase 2
  vertical is green.
- Treat **(iv)** strictly inside the WebCodecs ADR scope (doc 04 §8.6
  item 8); it can upgrade *web-only* Opus from blocked to partial whenever
  that ADR passes, independent of (ii).
- **(iii)** is recorded as collapsed into (ii).

---

## 5. AC-3 / DTS / WMA / MP2 honesty paragraphs (R-10)

### 5.1 MP2 — the one free member
Covered by `symphonia-bundle-mp3` ("Great", §2.4). The remaining risk is not
availability but **broadcast-grade corruption tolerance** (radio/TS captures
with damaged headers): that is a Phase 5 differential-corpus task on our side,
not a dependency gap. Matrix: `mp2 = available` once our corpus passes.

### 5.2 AC-3 / E-AC-3 — a real but bounded self-build
No crate (re-verified NOT_FOUND). The spec (ATSC A/52/A; E-AC-3 in A/52B) is
publicly published by ATSC. Decoder-only, no FFT reuse problems (rustfft
covers the transforms). Doc 04's **L** estimate is credible for AC-3 proper;
E-AC-3 adds frame-syntax and dependent-substream complexity — call it L for
A/52 + M more for E-AC-3. Scheduled Phase 6 per doc 04 §8.4 / doc 14 R-10.
Until then: typed error, `ac3 = blocked`, `eac3 = blocked`.

### 5.3 DTS — blocked on spec availability before engineering
No crate (re-checked; `dts` crate is unrelated data tooling). The engineering
is L (doc 04), but the honest blocker is **specification access**: DTS
Coherent Acoustics documentation is not freely published the way ATSC/IETF
specs are; ETSI TS 102 114 carries parts for broadcast [VERIFY what is
actually obtainable and under what terms]. A clean-room implementation
without a citable spec is a research project, not a roadmap item. Decision
point Phase 7 (doc 14 R-10); until then `dts = blocked` with reason
"no compliant implementation; spec availability unresolved".

### 5.4 WMA — not even upstream-planned
§2.4: absent from Symphonia's roadmap entirely; `wma` NOT_FOUND re-verified.
Microsoft published no open WMA decoder spec; ffmpeg's WMA knowledge is
reverse-engineered (cannot be consulted — clean-room). Self-build L is the
*optimistic* number; realistic v-status is `blocked` for WMA Pro and
`blocked` for WMA1/2 unless someone funds a spec-grade reverse-engineering
effort we can't clean-room from ffmpeg. Phase 7 decision point stands.

### 5.5 WavPack — a new license wrinkle
Upstream placeholder empty (§2.3); the only published Symphonia-compatible
WavPack crate is GPL-3.0-or-later (§2.3). Our repo is MIT (ADR-006); whether
a GPL crate can ever enter the tree is a doc 11 policy question (strong
copyleft, static Rust linking — same analysis class as oxisynth's LGPL but
strictly harder). **Flag, not decision:** `wavpack = blocked` in v1 with
`blocked_reason = no permissively-licensed pure-Rust decoder; GPL candidate
referred to doc 11`.

---

## 6. What this means for the v1 feature matrix (summary)

- **Available (software-decode) in v1:** PCM family, MP3, MP2, FLAC, ALAC,
  Vorbis, AAC-LC, ADPCM — 8 of the 18 codec families VLC's census shows as
  software-decodable, but the *usage-weighted* P0 share is much higher
  (MP3/AAC/FLAC/Vorbis/PCM dominate real-world audio-only files; Opus is the
  glaring P0 exception) [VERIFY usage-share data — doc 04 §8.6 item 7].
- **Partial:** AAC (LC only, HE detected+reported), TrueHD (Phase 6 hardening).
- **Blocked:** Opus (R-06, §4), AC-3/E-AC-3 (until P6 self-build), WMA, DTS,
  WavPack, Speex, Musepack, ATRAC, MIDI (later via rustysynth path).
- Every blocked row carries a typed error + `blocked_reason` per doc 02 §4
  rules; no silent silence anywhere.

---

## 7. Evidence appendix (all URLs fetched 2026-10-19)

crates.io API (User-Agent `lumen-phase0-audit`), one per crate — response
fields used: `max_stable_version`, `newest_version`, `updated_at`,
`repository`, `description`, downloads; `/versions` suffix for license:

1. https://crates.io/api/v1/crates/symphonia — 0.6.1, updated 2026-08-13, repo pdeljanov/Symphonia.
2. https://crates.io/api/v1/crates/symphonia-codec-aac — 0.6.1; /versions → MPL-2.0.
3. https://crates.io/api/v1/crates/symphonia-codec-opus — **HTTP 200 with error body: `crate `symphonia-codec-opus` does not exist`** (the API's not-found shape).
4. https://crates.io/api/v1/crates/symphonia-bundle-mp3 — 0.6.1 (covers MP1/MP2/MP3).
5. https://crates.io/api/v1/crates/symphonia-bundle-flac — 0.6.1.
6. https://crates.io/api/v1/crates/symphonia-codec-pcm — 0.6.1.
7. https://crates.io/api/v1/crates/symphonia-codec-vorbis — 0.6.1.
8. https://crates.io/api/v1/crates/symphonia-codec-alac — 0.6.1.
9. https://crates.io/api/v1/crates/symphonia-codec-adpcm — 0.6.1.
10. https://crates.io/api/v1/crates/symphonia-codec-wavpack — 0.1.1, repo pierreaubert/symphonia-add-ons; /versions → **GPL-3.0-or-later** (NOT the pdeljanov repo).
11. https://crates.io/api/v1/crates/claxon — 0.4.3 (2020-08-09), Apache-2.0.
12. https://crates.io/api/v1/crates/lewton — 0.10.2 (2021-01-20), MIT/Apache-2.0.
13. https://crates.io/api/v1/crates/alac — 0.5.0 (2018).
14. https://crates.io/api/v1/crates/truehd — 0.7.2 (2026-09-15), Apache-2.0, repo truehdd/truehdd/tree/main/truehd.
15. https://crates.io/api/v1/crates/truehdd — 0.6.2 (2026-09-15).
16. https://crates.io/api/v1/crates/oxisynth — 0.1.0 (2025-05-25); /versions → LGPL-2.1 (analysis in subtitles §5).
17. https://crates.io/api/v1/crates/rustysynth — 1.3.7 (2026-10-01), MIT (details in subtitles §5).
18. https://crates.io/api/v1/crates/opus — 0.4.0, "Safe Rust bindings for libopus" (C — forbidden).
19. https://crates.io/api/v1/crates/audiopus — libopus binding (forbidden).
20. https://crates.io/api/v1/crates/minimp3 — 0.6.1, "bindings … for the minimp3 C library" (forbidden).
21. https://crates.io/api/v1/crates/flacenc — 0.5.1, pure Rust FLAC encoder.
22. https://crates.io/api/v1/crates/symphonia-adapter-fdk-aac — 0.2.0 (2026-05-16), C adapter, evidence for §2.2.
23. https://crates.io/api/v1/crates/symphonia-adapter-libopus — 0.3.0 (2026-05-16), C adapter, evidence for §2.3.
24. NOT_FOUND re-checks (2026-10-19): `ac3`, `wma`, `speex`, `musepack`, `atrac`; `dts` exists but is unrelated (data-transform tooling).

GitHub raw fetches (2026-10-19):

25. https://raw.githubusercontent.com/pdeljanov/Symphonia/main/README.md — status table (AAC-LC Great; HE-AAC/HE-AACv2/Opus/WavPack "—"; MP3 Excellent; MSRV 1.85; third-party-adapter table; MPL-2.0; "100% safe Rust").
26. https://raw.githubusercontent.com/pdeljanov/Symphonia/master/README.md — stale 0.5.x-era branch, kept as citation-hygiene note.
27. https://raw.githubusercontent.com/pdeljanov/Symphonia/main/Cargo.toml — workspace members incl. symphonia-codec-opus / -wavpack.
28. https://raw.githubusercontent.com/pdeljanov/Symphonia/main/symphonia/Cargo.toml — features list: **no `opus`/`wavpack`/`he-aac` features**; default = opt-simd + all-meta + adpcm/flac/mkv/ogg/pcm/vorbis/wav.
29. https://raw.githubusercontent.com/pdeljanov/Symphonia/main/symphonia-codec-opus/src/lib.rs — **1 byte**; …/symphonia-codec-opus/README.md — "placeholder crate … work-in-progress Opus decoder"; …/Cargo.toml — version 0.0.1, symphonia-core 0.6.1 path dep.
30. https://raw.githubusercontent.com/pdeljanov/Symphonia/main/symphonia-codec-wavpack/src/lib.rs — 1 byte (placeholder), version 0.0.1, MPL-2.0.
31. https://raw.githubusercontent.com/pdeljanov/Symphonia/main/symphonia-codec-aac/src/lib.rs — `mod aac; mod adts;` only; …/Cargo.toml — no `[features]`; 404 probes for `src/aac/sbr.rs`, `src/aac/ps.rs`.
32. https://raw.githubusercontent.com/truehdd/truehdd/main/README.md — experimental banner; Rust 1.95.0 MSRV; verify/decode command semantics; concealment counters; presentations 0–3; Apache-2.0.
33. `git ls-remote https://github.com/pdeljanov/Symphonia` — branch inventory (`main` default, `dev-0.6`, `rel-0.5`; tags through v0.6.1).

Repo-internal evidence:

34. docs/inventory/modules-linux.json — `audio decoder` = 18 registrations (§1.1 reconciliation source).
35. docs/14-risk-register.md — R-06 (lines ~53, ~107, ~175), R-10 (~57, ~110, ~183) as cited.
36. docs/inventory/platform-linux.md §2.1 — WebCodecs AudioDecoder compiled into WebKitGTK 2.52.6 (in-binary strings, 2026-10-19).
37. docs/09-web-wasm-target.md §8.1/§10 — WebCodecs support matrix + "web's OS-decoder" proposal.

## 8. Remaining [VERIFY] ledger (this section)

1. **[VERIFY-P0]** Full transitive-graph + tarball purity audit of every crate
   named here (doc 04 §8.6 item 10) — all [RV] marks above are
   existence/version/license/data verifications, **not** the §9 policy audit.
2. **[VERIFY]** HE-AAC share of real-world AAC files (drives how loud the
   `partial (LC)` consequence is) — depends on doc 04 §8.6 item 7
   usage-share data.
3. **[VERIFY]** Whether `symphonia-codec-pcm` covers G.711 A-law/µ-law (for
   the census-reconciliation row g711) — check crate docs/source before
   scheduling the S self-build.
4. **[VERIFY]** SMPTE ST 302M (`aes3`) spec obtainability + exact scope
   (de-emphasis, V/U/C/P bits) for the S–M self-build estimate.
5. **[VERIFY]** Ulead DV audio (`uleadvdaudio`) format documentation
   availability (P3).
6. **[VERIFY]** truehdd: Dolby reference-stream licensing for the Phase 6
   corpus; upstream fuzzing state; behavior on non-Atmos presentation-3
   streams in our containers (MKV/TS extraction path).
7. **[VERIFY]** WebCodecs `AudioDecoder` per-browser coverage (Firefox,
   Android) — needed if option (iv) is scoped into the WebCodecs ADR.
8. **[VERIFY]** Opus conformance-corpus license specifics (Xiph test
   vectors) before option (ii) budgeting.
9. **[VERIFY]** DTS specification obtainability/terms (ETSI TS 102 114 and
   any DTS-published docs) — gates §5.3's honest blocked stance.
10. **[VERIFY]** Symphonia SIMD-by-default (rustfft) interaction with our
    reproducible-build/wasm story (doc 11/toolchain).
11. **[VERIFY]** rustysynth SoundFont bundling policy + sf3 (compressed
    samples) support status — carried in the subtitles file §5 (audit item 6).

*Honesty statement: every [RV] claim above was fetched live on 2026-10-19 and
its URL recorded in §7; nothing was inferred from cached or third-party data.
Claims not marked [RV] retain doc 04's original verification status. No VLC
source was consulted; no git commit was made (AGENTS §3.8).*
