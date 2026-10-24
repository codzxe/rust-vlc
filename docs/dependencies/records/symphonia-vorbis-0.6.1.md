# deps/records/symphonia-vorbis-0.6.1.md — symphonia-codec-vorbis 0.6.1
# (+ symphonia-common / symphonia-metadata / regex-lite, purity
# verification, card 5.B2)

> **Class:** P0 dependency verification (doc 11 §2 format, fields R1–R10 +
> F1–F7 scan), ordered by card 5.B2: "Vorbis decoder (symphonia or lewton)
> `[VERIFY]`". The Lead's wave brief prefers the
> `symphonia-bundle-vorbis` spelling to match 5.A3's
> `symphonia-bundle-mp3` pattern. **Reality check that changed the
> spelling:** `symphonia-bundle-vorbis` does not exist on crates.io
> (sparse index `sy/mp/symphonia-bundle-vorbis` → `NoSuchKey`, checked
> 2026-10-26); the published Vorbis engine crate of the same 0.6.1 train
> is **`symphonia-codec-vorbis` 0.6.1** — the mirror-image of 5.A3's
> finding (there the *assumed* `-codec-` name didn't exist; here the
> *actual* one is `-codec-`, the bundle naming is MP3-only). The
> alternative the card offered, `lewton` 0.10.2, was NOT taken: it pulls
> the `ogg` + `byteorder` + `ogg-crc` crates (its own container layer —
> a duplicate of our landed `demux::ogg`), and its last release predates
> the 0.6.1 symphonia train we already pin. Evidence gathered 2026-10-26
> in the build sandbox from the sparse index (index.crates.io), a
> scratch `cargo fetch` + `cargo tree --edges normal` (pasted in R2),
> and the extracted sources under `~/.cargo/registry/src/`.
> **Final acceptance and the allowlist row remain Lead decisions (doc 11
> §3.1; `boundaries.toml` + `deny.toml` are Lead-owned and untouched by
> 5.B2 — the same posture as the claxon/symphonia-mp3 admissions, whose
> `boundaries.toml` row is still owed by the registry card R3).**

## R1 — Language & source (F1, F7)

- Rust, **no native source files** in any of the four new tarballs:
  scans for `*.c *.cc *.cpp *.cxx *.h *.hpp *.m *.mm *.S *.s *.asm` →
  **0 hits** in `symphonia-codec-vorbis-0.6.1/src` (`lib.rs`,
  `codebook.rs`, `common.rs`, `dsp.rs`, `floor.rs`, `residue.rs`,
  `window.rs`), `symphonia-common-0.6.1/src` (`apple/`, `mpeg/`, `xiph/`),
  `symphonia-metadata-0.6.1/src`, `regex-lite-0.1.9/src` (F1 NO, F7 NO).
- No `links=` key in any published manifest (F3 NO). No `extern "C"` /
  `extern "system"` / `#[link]` anywhere in the four `src/` trees.
- No binary blobs (`*.a *.lib *.o *.obj *.so *.dll *.dylib` → 0 hits;
  F4 NO — nothing vendored; no build script, so nothing downloads).
- `unsafe` surface: `symphonia-codec-vorbis/src/lib.rs:9` and
  `symphonia-metadata/src/lib.rs:26` declare `#![forbid(unsafe_code)]`
  (compiler-enforced zero unsafe sites, the 5.A3 posture);
  `symphonia-common/src` and `regex-lite/src/lib.rs:854` —
  `regex-lite` also `#![forbid(unsafe_code)]`; `symphonia-common`
  carries **no `unsafe` token at all** in its `src/` tree (grep → 0)
  though it does not carry the forbid attribute itself (its 4 clippy
  allows only). The workspace's own `unsafe_code = deny` lint covers
  the whole compiled graph regardless.
- Repo: https://github.com/pdeljanov/Symphonia (monorepo;
  `regex-lite` is rust-lang/regex's compact sibling, MIT OR Apache-2.0,
  `#![forbid(unsafe_code)]` at `src/lib.rs:854`). MPL-2.0 LICENSE
  files present in the three symphonia tarballs.

## R2 — Transitive graph (F5)

Resolved by a scratch `cargo fetch` + `cargo tree --edges normal`
(the real graph shape Lumen's codecs crate builds):

```
codecs
└── symphonia-codec-vorbis 0.6.1        (ck 73d90b4fcf796137cc…)
    ├── log 0.4.34                       (already in graph)
    ├── symphonia-common 0.6.1           (ck 2acc3fcc18ec9b8cd…)  [NEW]
    │   ├── log 0.4.34                   (already in graph)
    │   ├── symphonia-core 0.6.1         (already in graph, mp3's pin)
    │   └── symphonia-metadata 0.6.1     (ck 83713a97705d77bde…)  [NEW]
    │       ├── lazy_static 1.5.1        (already in graph)
    │       ├── log 0.4.34               (already in graph)
    │       ├── regex-lite 0.1.9         (ck 5153198d…, MIT OR     [NEW]
    │       │                              Apache-2.0, no deps)
    │       ├── smallvec 1.16.4          (already in graph)
    │       └── symphonia-core 0.6.1     (already in graph)
    └── symphonia-core 0.6.1             (already in graph)
```

**4 new registry packages** vs the pre-5.B2 lock
(`symphonia-codec-vorbis`, `symphonia-common`, `symphonia-metadata`,
`regex-lite`); every other node was already admitted by the 5.A3 mp3
record. No `-sys` crates, no git deps, no optional features enabled
(`symphonia-metadata`'s `flac` feature rides its symphonia-common
dependency edge — pure Rust metadata parsing, no native code; the
vorbis path never calls it, the engine only uses
`symphonia-common`'s Xiph header constants/helpers).
`[VERIFY-P1: regenerate at merge — the merged lock diff must show
exactly these 4 additions.]`

## R3 — Build scripts (F2)

- **No `build.rs`** in any of the four tarballs (manifests carry no
  `build` key). F2 NO.

## R4 — Native links (F3, F4, F6)

- None: no `links=`, no extern declarations, no blobs, no pkg-config,
  no vendored C. F3/F4/F6 all NO — a from-scratch Rust port of the
  Vorbis I decode chain (codebook VQ / floor curves / residue / MDCT
  overlap-add synthesis), not a libvorbis wrapper.

## R5 — Targets

- Pure Rust, no `cfg(target_*)` native gates in the four new crates;
  symphonia-core's `opt-simd` features stay off (5.A3 posture — the
  scalar paths compile everywhere, wasm32 included).
  `[VERIFY-P1: wasm32 build from our own tree at the first wasm gate
  run.]`

## R6 — Actual feature coverage (against card 5.B2's needs)

- Verifiable in-sandbox: `VorbisDecoder::try_new(&AudioCodecParameters,
  &AudioDecoderOptions)` requires `codec == CODEC_ID_VORBIS` and
  `extra_data`; `read_extradata` accepts either the Matroska Xiph-laced
  form (`0x02` leading byte — `symphonia-common`'s
  `unpack_xiph_laced_extradata`) or **ident‖setup concatenated** (the
  non-laced branch — the form our ogg seam feeds it).
  `decode_ref(&PacketRef)` → `GenericAudioBufferRef` (F32, one
  overlap-add block per packet, `(prev_n + n) / 4` frames);
  `AudioDecoder::reset()` clears the DSP overlap state (the
  post-seek discontinuity path).
- Explicitly NOT used: symphonia's own `OggReader` demuxer, its
  metadata revision (`symphonia-metadata`) and gapless trimming
  (`AudioDecoderOptions::gapless = false` — the ogg granule clock, not
  the engine, owns Vorbis positioning). The card's header parsing
  (identification geometry, Vorbis comment exposure), granule
  arithmetic, error taxonomy and the codec-api contract surface are
  implemented by `crates/codecs/src/audio/vorbis/**` ourselves; the
  engine owns only the entropy+DSP decode of one framed audio packet.
- Our own conformance evidence: ffmpeg-authored fixtures + the corpus
  differential (`crates/codecs/tests/vorbis_bitexact.rs`) are the
  recorded coverage proof — see the test file headers for commands and
  digests.

## R7 — License

- **MPL-2.0** for the three symphonia crates (LICENSE files +
  manifest `license` keys); **MIT OR Apache-2.0** for `regex-lite`
  (already an allowed class). The MPL-2.0 class admission note of the
  5.A3 record (R7) carries over verbatim: the Lead's merge must add
  `"MPL-2.0"` to `[licenses] allow` in `deny.toml` in the same PR
  that merges this record (if not already landed with 5.A3's merge).

## R8 — Maintenance & security record

- Same 0.6.1 release train as the admitted mp3 bundle
  (pubtime 2026-08-13, publisher `pdeljanov`). OSV/RustSec query for
  `symphonia-codec-vorbis`/`symphonia-common`/`symphonia-metadata`/
  `regex-lite`: no advisories filed against any version as of
  2026-10-26. `[VERIFY-P1: re-query RustSec at admission date per
  doc 11 §3.7.]`

## R9 — Provenance

- All four tarballs fetched by `cargo fetch` from static.crates.io
  through the pinned sparse index; the R2 checksums were verified by
  cargo itself against the index (a mismatch fails the fetch) — the
  `symphonia-codec-vorbis-0.6.1.crate` sha256 recomputed in-sandbox
  matches the index `cksum` byte-for-byte.
  `[VERIFY-P1: publisher identity cross-check at admission time, per
  the cpal record's convention.]`

## R10 — Verdict

**pass (pure Rust; `#![forbid(unsafe_code)]` in the codec + metadata
crates, zero unsafe tokens in symphonia-common, regex-lite forbidden) —
admitted by card 5.B2 under the Lead's wave-4 brief, pending (a) the
Lead's allowlist row and (b) the `deny.toml` MPL-2.0 license-class
addition if 5.A3's merge has not already carried it (R7).** Pinned
`symphonia-codec-vorbis = "=0.6.1"` (exact pin; a bump re-triggers this
record per doc 11 §3.7). F1–F7 all NO; R2 adds exactly 4 pure-Rust
nodes, all recorded; no advisories; wasm32 left as the standing P1
verify (shared with 5.A3).

*Evidence posture (AGENTS §3.4): every statement above was executed
locally in the build sandbox on 2026-10-26 — sparse-index queries,
scratch `cargo fetch` + `cargo tree --edges normal` (the R2 tree is
pasted from that run), sha256 recomputation of the crate tarball, and
full F1–F7 scans of all four extracted tarballs with hit counts
recorded. No statement about symphonia's decode quality is made without
our own fixture run behind it (see R6).*
