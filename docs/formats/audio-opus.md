# Audio record — Opus (RFC 6716) decode

Decision record (card 5.K2-I, doc 04 §8.6 item 1's Opus half). No code
changed. Front-matter keys per doc 04 §6:

- id = `codec-audio-opus` (matches `docs/feature-matrix.toml`, which
  already carries `status = blocked` on all four platforms with an empty
  `evidence` — this record is that evidence)
- status = **blocked** (decode; container carriage is landed, below)
- phase = 6 for decode admission (matrix `phase_target = 5` recorded the
  ambition; feasibility 0.C2-I §4 schedules the work Phase 5/6)
- fuzz_target = `fuzz_codec_opus` (planned name, doc 04 §3.1 row — does
  not exist until a decoder is admitted)
- crates = [] — **none admitted**; the four admitted Symphonia engines
  (`symphonia-bundle-mp3`, `-codec-vorbis`, `-codec-aac`, `-codec-alac`,
  all pinned `=0.6.1` in `crates/codecs/Cargo.toml`) have no Opus sibling
- self_built = []
- last_audit = 2026-10-24 (this card's evidence pass; prior: feasibility
  0.C2-I, 2026-10-19)

## Verdict (the card's question: contribute vs self-build vs blocked)

**BLOCKED — upheld, with the recorded reason amended and the unblock plan
replaced.** Opus decode ships v1 as a typed `unsupported` gap (never
silence); the feasibility recommendation (0.C2-I §4 option i) stands.
What changes is the *plan*:

1. **Contribute-upstream: REJECTED as a distinct option.** There is no
   in-work decoder body to contribute to — upstream's
   `symphonia-codec-opus` is a reserved name around an empty file
   (evidence below). "Contribute" collapses into self-build with a
   different home (feasibility §4 iii said the same on 2026-10-19).
2. **Self-build (L–XL, RFC 6716 + RFC 8251): DEMOTED to fallback of last
   resort.** It was the recorded plan ("the only maintained pure-Rust
   Opus decoder in existence"); that premise is now false — see the new
   evidence below. Duplicating two existing pure-Rust implementations at
   L–XL needs a failure of both first.
3. **Primary unblock (new): validate-then-admit `ruopus`.** A from-scratch
   pure-Rust RFC 6716 decoder is **published on crates.io since
   2026-06-29** — this was missed by the 2026-10-19 survey (§1 row 10
   "None published" is stale) and is this card's material finding.

## Evidence (all fetched this session, 2026-10-24, unless dated)

**Upstream Symphonia — placeholder, unpublished.**
- crates.io API, exact names: `symphonia-codec-opus` → `{"errors":
  [{"detail":"crate `symphonia-codec-opus` does not exist"}]}`; same for
  `symphonia-bundle-opus`. (Matches 0.C2-I §2.3, 2026-10-19.)
- Repo `github.com/pdeljanov/Symphonia` (from the `symphonia` crate's
  repository field), `master` and `main` both: `symphonia-codec-opus`
  **is** a workspace member; `src/lib.rs` is a **1-byte empty file** on
  both branches; the directory README states verbatim *"This is a
  placeholder crate for Project Symphonia's work-in-progress Opus
  decoder"*; the README status row is `Opus | - | - | opus | Yes |
  symphonia-codec-opus` (dash = unrated). Its Cargo.toml: version
  workspace (= 0.6.1 train), deps only `log` + `symphonia-core` (pure).
- The published umbrella `symphonia` 0.6.1 (2026-08-13) optional
  dependencies enumerate flac/mp3/aac/adpcm/alac/pcm/vorbis + formats —
  **no opus**. The published `symphonia-codec-wavpack` 0.1.1 (2026-09-06)
  is third-party GPL (`pierreaubert/symphonia-add-ons`, 0.C2-I §2.3) —
  not an upstream-publication precedent.

**Published pure-Rust Opus implementations (the new finding).**
- `ruopus` 0.1.2 (github.com/jmg049/ruopus, MIT; published 2026-06-29,
  last update 2026-07-01; 2 versions, 9 264 downloads): README states —
  from-scratch RFC 6716 decoder **and** encoder, no C, no FFI;
  `unsafe` denied by default except `std::arch` SIMD hot loops, each
  with a `// SAFETY:` note, Miri-checked on SSE2 and AVX2; decoder is
  `no_std` + `alloc` (optional `libm`), builds on wasm32; zero required
  dependencies (one optional FFT crate for speed); MSRV 1.85; *"the
  decoder passes the official Opus conformance vectors"*. Risk profile:
  0.1.x, single maintainer, ~1 month old at last update — **every
  conformance/purity claim is the README's, not ours** (unverified).
- `symphonia-adapter-opus-rs` 0.1.1 (2026-08-02; deps: `ruopus ^0.1`,
  `symphonia-core ^0.6.0`, `log`; no repository URL recorded on
  crates.io): a third-party wiring of ruopus into Symphonia's decoder
  trait. Reference material only — our admitted pattern keeps the glue
  ours (see plan).
- `oporus` 0.2.4 (2026-06-07, mlm-games, *"(Recovered) Rust Opus codec"*,
  2 487 downloads; repo README fetch 404 this session) +
  `symphonia-adapter-oporus` 0.1.3 (enables oporus's `fixed_point`
  feature): the "recovered" provenance could not be verified —
  **secondary candidate only**.
- `symphonia-adapter-libopus` 0.3.0 (updated 2026-05-16): libopus C
  bindings — **forbidden** (doc 04 §1.2), listed to close the loop.

**Landed state in this tree (no code changed by this card).**
- Carriage works: `crates/demux/src/ogg` maps `OggCodec::Opus` →
  descriptor `opus`, extension set includes `.opus`/`.oga`/`.ogv`,
  `Clock::Opus48000` granule arithmetic (960 samples = 20 ms) is
  test-pinned. MKV carries vp9+opus lanes (5.D1/5.D2 records).
- Decode refuses loudly: no Opus decoder is registered in
  `crates/codecs`; the Ogg-family decoder (Vorbis) refuses the `opus`
  descriptor with a typed error, test-pinned (`vorbis.codec_descriptor`,
  `crates/codecs/src/audio/vorbis/decoder.rs` tests).

## Corpus (corpus-v4, 3 Opus lanes, ffprobe 7.1.5 this session)

| Lane | Shape | Notes |
|---|---|---|
| `0047-audio-only.ogg` (42 308 B) | opus, mono, 48 kHz | pre-skip edge lives in its OpusHead |
| `0077-audio-only.ogg` (34 125 B) | opus, **6 ch (5.1)**, 48 kHz | channel-mapping-family-1 table parse is mandatory for admission |
| `0078-vfr-av.mkv` (143 816 B) | mkv, vp9 + opus | the WebM-class combo lane; no Vorbis track to fall back to |

The two Ogg lanes are already demuxed by our Ogg reader; they exercise
OpusHead/OpusComments pages + granule → 20 ms framing on the decode side
once an engine exists.

## Plan (next actions, effort per doc 04 scale)

1. **S — validation spike (wave 6/7, no commitment implied):** vendor
   `ruopus` 0.1.2 into a scratch harness (NOT the workspace graph): run
   (a) the Xiph Opus conformance/test vectors, (b) our differential vs
   ffmpeg 7.1.5 on the 3 corpus lanes above (PCM MD5 at 48 kHz, incl.
   the 5.1 remap), (c) the purity pre-checks: tarball scan for
   `*.c/*.cpp/*.h/*.S/*.asm`/`build.rs` (doc 04 §1.2), license (MIT),
   and an `unsafe` census — the Miri-checked `std::arch` kernels are
   Rust-side unsafe, a doc 11 policy call, not a purity fail by the
   §1.2 rule; record it either way.
2. **M — admission (Phase 6, Lead pre-approval, C5 pattern like the
   other four Symphonia engines):** dep line `ruopus = "=0.1.2"` direct
   (adapter crate not admitted — glue stays ours: OpusHead
   pre-skip/gain/channel-mapping parse, granule arithmetic, error
   taxonomy); + `fuzz_codec_opus` target; + differential suite; +
   record update to `implemented` with evidence.
3. **Monitor:** re-query `https://crates.io/api/v1/crates/symphonia-codec-opus`
   each wave — if upstream ever publishes a real body, prefer it for
   family consistency (swap decision at that point, differential
   re-run).
4. **Fallback (only if 1 fails):** re-open the L–XL self-build gate
   (feasibility §4 ii), possibly as an upstream contribution so the
   reserved `symphonia-codec-opus` name gets its body.
5. Until any of the above lands: `opus = missing/blocked` in the matrix
   (truthful-reporting rule, doc 04 §7 row 3), typed refusal, Vorbis
   fallback noted where a stream has one (0078 does not).

## Doc drift for the Lead (out of this card's scope to edit)

- doc 04 §3.1 Opus row + §7 R-06 row: "no published pure-Rust decoder"
  is stale as of 2026-06-29 (`ruopus`); "contribute to Symphonia in-work
  crate" premise falsified (empty placeholder, re-verified 2026-10-24).
- feasibility `audio-codecs.md` §1 row 10 "None published" and §4's
  "only maintained pure-Rust Opus decoder in existence" self-build
  framing: same correction.
- `feature-matrix.toml` `codec-audio-opus.blocked_reason`: amend from
  "No published pure-Rust decoder" to "no *validated* pure-Rust engine
  (ruopus 0.1.2 published 2026-06-29, unvalidated; upstream
  symphonia-codec-opus an empty placeholder, re-verified 2026-10-24)" —
  and backfill `evidence` to this record.
