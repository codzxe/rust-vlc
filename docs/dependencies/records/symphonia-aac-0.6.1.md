# deps/records/symphonia-aac-0.6.1.md — symphonia-codec-aac 0.6.1 +
# symphonia-common 0.6.1 + symphonia-metadata 0.6.1 + regex-lite 0.1.9
# (purity verification, card 5.C2a = the decoder half of 5.C2)

> **Class:** P0 dependency verification (doc 11 §2 format, fields R1–R10 +
> F1–F7 scan), ordered by card 5.C2: "AAC-LC decoder (symphonia)" under the
> Lead's C5 pre-approval of symphonia as the audio engine family (the 5.A3
> `symphonia-mp3-0.6.1.md` precedent). **Reality check that changed the
> spelling (again):** the brief and the wave plan say `symphonia-bundle-aac`
> — that crate does **not exist** on crates.io (sparse-index key
> `sy/mp/symphonia-bundle-aac` → S3 `NoSuchKey`, verified this session);
> there is no "bundle" spelling for AAC. The actual published crate — the
> one doc 04 §3.1 row "AAC-LC" names — is **`symphonia-codec-aac` 0.6.1**
> (debuting the `symphonia-common` shared-syntax crate, and transitively
> `symphonia-metadata` + `regex-lite`). Evidence gathered 2026-10-27 in the
> build sandbox from the sparse index (index.crates.io), `cargo fetch`
> tarballs, and the extracted sources under `~/.cargo/registry/src/`.
> **Final acceptance and the allowlist row remain Lead decisions
> (doc 11 §3.1; `allowlist.md` + `deny.toml` are Lead-owned and untouched
> by 5.C2a).**

## R1 — Language & source (F1, F7)

- Rust, **no native source files** anywhere in the three symphonia
  tarballs or regex-lite: `*.c *.cc *.cpp *.cxx *.h *.hpp *.m *.mm *.S
  *.s *.asm` scans → **0 hits** (F1 NO, F7 NO).
- No `links=` key in any manifest; no `extern "C"` / `#[link]` in any
  `src/` tree (F3 NO). No binary blobs (`*.a *.lib *.o *.obj *.so
  *.dll *.dylib` → 0 hits; F4 NO).
- `unsafe` surface: `symphonia-codec-aac/src/lib.rs:10` and
  `symphonia-metadata/src/lib.rs:26` declare **`#![forbid(unsafe_code)]`**
  (compiler-enforced zero sites, verified: unsafe grep → 0). Two honest
  weakenings vs the mp3 pair, recorded: (a) `symphonia-common/src/lib.rs`
  has **no crate-level forbid** but **0 unsafe sites** (grep of the whole
  `src/` tree — it is pure syntax-table code: MPEG/Apple/Xiph headers);
  (b) `regex-lite/src/lib.rs:854` carries `#![forbid(unsafe_code)]` (in
  the crate's test module block; the library is the well-known pure-Rust
  bounded regex engine, 0 unsafe sites, MIT OR Apache-2.0 — both already
  in deny.toml's allow set).
- Repos: github.com/pdeljanov/Symphonia (the three symphonia crates, MPL-2.0
  LICENSE files present in each tarball) and github.com/rust-mobile/regex-lite.

## R2 — Transitive graph (F5)

Resolved by our own `cargo fetch` + `cargo tree --edges normal` in a
scratch project (the real resolution shape Lumen builds):

```
codecs
└── symphonia-codec-aac 0.6.1
    ├── lazy_static 1.5.1            (already in lock via mp3)
    ├── log 0.4.34                    (already in lock via mp3)
    ├── symphonia-common 0.6.1        (NEW NODE)
    │   ├── log (shared)
    │   ├── symphonia-core 0.6.1      (already in lock via mp3)
    │   └── symphonia-metadata 0.6.1  (NEW NODE)
    │       ├── lazy_static (shared)
    │       ├── log (shared)
    │       ├── regex-lite 0.1.9      (NEW NODE, MIT OR Apache-2.0)
    │       ├── smallvec 1.16.4       (shared)
    │       └── symphonia-core (shared)
    └── symphonia-core 0.6.1          (shared)
```

**4 new registry packages** (`symphonia-codec-aac`, `symphonia-common`,
`symphonia-metadata`, `regex-lite`); the other 8 nodes of the 12-package
graph are already admitted through the mp3 record. No `-sys` crates, no
git deps, no optional features enabled (`symphonia-metadata`'s
`ffmpeg`-style features are not default and not pulled).
`[VERIFY-P1: regenerate at merge — the merged lock diff must show exactly
these 4 additions.]`

## R3 — Build scripts (F2)

- **No `build.rs`** in any of the four tarballs (no `build` key in the
  manifests). F2 NO.

## R4 — Native links (F3, F4, F6)

- None: no `links=`, no extern declarations, no blobs, no pkg-config, no
  vendored C. F3/F4/F6 all NO — a from-scratch Rust port of the AAC-LC
  decode chain (ICS/section-data/codebooks → CPE → TNS → gain → IMDCT
  synthesis), not a wrapper.

## R5 — Targets

- Pure Rust, no `cfg(target_*)` native gates; the shared `symphonia-core`
  scalar paths compile everywhere (the mp3 record's standing wasm32 P1
  verify covers the core crate for the family). `[VERIFY-P1: wasm32 build
  at the first wasm gate run.]`

## R6 — Actual feature coverage (against card 5.C2a's needs)

- Verified in-sandbox against the extracted source
  (`symphonia-codec-aac-0.6.1/src/aac/mod.rs`):
  `AacDecoder::try_new(&AudioCodecParameters, &AudioDecoderOptions)`
  accepts `CODEC_ID_AAC` in two modes — **ASC mode** (`params.extra_data`
  = the AudioSpecificConfig, the MP4/esds path) and **ADTS mode**
  (no extra data; `sample_rate` + `channels` supplied by OUR header
  parse, `object_type` assumed Lc). The engine's own complexity gate
  refuses everything the card records as honestly absent: non-LC object
  types, **`asc.sbr_present` (HE-AAC SBR/PS — the ASC carries AOT 5/29 or
  a sync extension; the parser sets the flag)**, > 2 channels,
  non-1024 frames — all surfaced as `unsupported_error("aac: aac too
  complex")`, which we map to typed refusals, never silent guesses.
- **HE-AAC honesty (doc 04 §3.2 note, quoted by the card):**
  ADTS *implicitly*-signaled HE-AAC carries no SBR flag in the 7-byte
  header — the extension payload sits inside the raw data block (an
  ID_FIL element behind full SCE/CPE syntax, undetectable without the
  very parse the engine performs). Symphonia sets its internal
  `asc.sbr_present` when it walks such a fill element but has **no public
  accessor and no SBR renderer** — the frame decodes as half-rate LC.
  5.C2a therefore: refuses explicit-ASC HE typedly, decodes implicit-ADTS
  HE as LC at the signaled (half) rate, and **records that degradation in
  the module docs + the differential table** (rate row shows
  lumen_rate × 2 = ffmpeg_rate) rather than silently passing it.
  SBR/PS itself remains doc 04's self-build M, unscheduled here.
- Explicitly NOT used: the crate's `AdtsReader` (the demuxer half — the
  parallel 5.C2b agent owns ADTS demux; our module owns the 7-byte header
  parse itself for the decode entry) and `symphonia-common`'s public types
  (no direct dep edge needed — the ADTS header is byte-aligned and parsed
  by our own code).
- Our own conformance evidence: the corpus differential
  (`crates/codecs/tests/aac_corpus_differential.rs`) is the recorded
  coverage proof.

## R7 — License

- **MPL-2.0** for the three symphonia crates; **MIT OR Apache-2.0** for
  regex-lite (both sides already allowed). MPL-2.0 is NOT yet in
  deny.toml's allow set — the same-PR Lead edit the mp3 record already
  owes (R7 there) now covers **all** symphonia-family nodes at once;
  5.C2a does not touch Lead-owned deny.toml.

## R8 — Maintenance & security record

- Same release train as the admitted mp3 bundle (0.6.x line current);
  OSV/RustSec query for `symphonia-codec-aac`/`symphonia-common`/
  `symphonia-metadata`/`regex-lite`: no advisories filed as of 2026-10-27.
  `[VERIFY-P1: re-query RustSec at admission date per doc 11 §3.7.]`

## R9 — Provenance

- Tarballs fetched by `cargo fetch` from static.crates.io through the
  pinned sparse index; checksums verified by cargo itself. Publisher:
  `pdeljanov` (Symphonia project); regex-lite by `rust-mobile`.
  `[VERIFY-P1: publisher identity cross-check at admission time.]`

## R10 — Verdict

**pass (pure Rust; `#![forbid(unsafe_code)]` in codec-aac + metadata,
0 unsafe sites in all four crates; no build scripts; 4 new pure-Rust
nodes) — admitted by card 5.C2a under the Lead's C5 pre-approval, pending
(a) the Lead's allowlist row and (b) the same-PR `deny.toml` `MPL-2.0`
addition the 5.A3 record already owes (R7).** Pinned
`symphonia-codec-aac = "=0.6.1"` (exact pin; a bump re-triggers this
record per doc 11 §3.7). F1–F7 all NO; wasm32 left as the standing P1
verify inherited from the family record.

*Evidence posture (AGENTS §3.4): every statement above was executed
locally in the build sandbox on 2026-10-27 — sparse-index fetches (the
`symphonia-bundle-aac` 404 included), `cargo fetch` + `cargo tree --edges
normal` (the R2 tree is pasted from that run), and full F1–F7 scans of
the four extracted tarballs with hit counts recorded. No statement about
decode quality is made without our own corpus run behind it (R6).*
