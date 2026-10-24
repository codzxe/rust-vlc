# deps/records/symphonia-alac-0.6.1.md — symphonia-codec-alac 0.6.1
# (purity verification, card 5.J3a = the ALAC half of 5.J3)

> **Class:** P0 dependency verification (doc 11 §2 format, fields R1–R10 +
> F1–F7 scan), ordered by card 5.J3: "ALAC decoder (symphonia)" under the
> Lead's C5 pre-approval of symphonia as the audio engine family (the 5.A3
> `symphonia-mp3-0.6.1.md`, 5.B2 `symphonia-vorbis-0.6.1.md` and 5.C2a
> `symphonia-aac-0.6.1.md` precedents). **Name check:** the card says
> "verify exact name/version" — the crate is **`symphonia-codec-alac`**
> (no "bundle" spelling exists for ALAC; sparse-index key
> `sy/mp/symphonia-codec-alac`), and the 0.6.1 train matches the pinned
> core/common. Evidence gathered 2026-10-28 in the build sandbox from the
> sparse index (index.crates.io), the `static.crates.io` tarball
> (sha256 `920a78f96f3cf62932d0c497959c0cc2100ef36b03c7ca1417b260f432a482d8`,
> matching the index checksum), and the extracted source.
> **Final acceptance and the allowlist row remain Lead decisions
> (doc 11 §3.1; `allowlist.md` + `deny.toml` are Lead-owned and untouched
> by 5.J3a).**

## R1 — Language & source (F1, F7)

- Rust, **no native source files** in the tarball: `*.c *.cc *.cpp *.cxx
  *.h *.hpp *.m *.mm *.S *.s *.asm` scans over the extracted tree →
  **0 hits** (F1 NO, F7 NO). Single source file
  (`src/lib.rs`, 671 lines — the whole ALAC decode chain).
- Manifest: `build = false`, no `links=` key, no `extern "C"` / `#[link]`
  (F2/F3 NO). No binary blobs (`*.a *.lib *.o *.obj *.so *.dll *.dylib` →
  0 hits; F4 NO).
- `unsafe` surface: `src/lib.rs:10` declares **`#![forbid(unsafe_code)]`**
  — compiler-enforced zero sites (grep → 0).
- License **MPL-2.0** (LICENSE in the tarball; the Symphonia family
  standard — same as the mp3/vorbis/aac records).
- Repo: github.com/pdeljanov/Symphonia (crate `symphonia-codec-alac`).

## R2 — Transitive graph (F5)

Resolved from the sparse index + our existing lock (the real resolution
shape Lumen builds — all siblings already admitted):

```
codecs
└── symphonia-codec-alac 0.6.1     (NEW NODE — the only addition)
    ├── log 0.4                    (already in lock via mp3)
    ├── symphonia-common 0.6.1     (already in lock via aac; the ALAC
    │                              │  MagicCookie parser lives here)
    └── symphonia-core 0.6.1       (already in lock via mp3)
```

**1 new registry package.** No `-sys` crates, no git deps, no optional
features (the crate exposes none). `[VERIFY-P1: regenerate at merge — the
merged lock diff must show exactly this 1 addition.]`

## R3 — Build scripts (F2)

- **No `build.rs`** (no `build` key). F2 NO.

## R4 — Native links (F3, F4, F6)

- None: no `links=`, no extern declarations, no blobs, no pkg-config, no
  vendored C. F3/F4/F6 all NO — a from-scratch Rust port of the ALAC
  decode chain (rice residuals → adaptive LPC → mid-side decorrelation →
  extra-bits splice), not a wrapper.

## R5 — Targets

- Pure Rust, no `cfg(target_*)` native gates; rides `symphonia-core`
  (the mp3 record's standing wasm32 P1 verify covers the core crate for
  the family). `[VERIFY-P1: wasm32 build at the first wasm gate run.]`

## R6 — Actual feature coverage (against card 5.J3a's needs)

Verified in-sandbox against the extracted source
(`symphonia-codec-alac-0.6.1/src/lib.rs`):
`AlacDecoder::try_new(&AudioCodecParameters, &AudioDecoderOptions)`
requires `CODEC_ID_ALAC` + `params.extra_data` = the ALAC "magic cookie"
(parsed by `symphonia-common::apple::audio::alac::MagicCookie::read`:
24-byte ALACSpecificConfig, optionally prefixed by a 12-byte `frma`/`alac`
atom, optionally extended by a 24-byte `chan` layout block — exactly the
MP4 sample-entry / esds and MKV CodecPrivate passthrough shapes).
Per packet: SCE/CPE/LFE element walk + DSE/FIL skips, partial-frame
(32-bit sample count) support, uncompressed frames, rice escape coding,
zero-run blocks, adaptive-LPC orders 0–31 (order 31 = the double-pass
first-order special case), mid-side decorrelation, extra-bits (shift>0)
splice. Channel layouts mono→7.1 via the 8 known `chan` tags or implicit
per-count layout. Frame-length DoS gate at 4096×16.
The ALACSpecificConfig **parse itself is ours** (typed
`alac.config.*` errors, module docs); the cookie bytes are handed to the
engine verbatim (passthrough, no re-serialization).

## R7 — Withdrawn / dangerous versions (F6)

- 0.6.1 is the latest published (0.6.0-alpha.1/2 and 0.5.x precede;
  nothing yanked in the series; no RUSTSEC advisories against the
  symphonia family as of this session).

## R8 — License compatibility

- MPL-2.0 (permissive-with-file-scope-copyleft); consistent with the
  already-admitted symphonia crates. Source-linked, not statically
  modified by Lumen — the wrapper-side workaround in 5.J3a is documented
  in the module and does not patch engine code.

## R9 — Provenance

- Tarball sha256 `920a78f9…482d8` matches the sparse-index checksum;
  `cargo fetch` resolves the same bytes; the `Cargo.toml` carries the
  crate's own `[lints]` (clippy `unwrap_used = warn`) — engine-internal
  only, not inherited by Lumen.

## R10 — Size / attack surface

- 671 lines of decode logic + the shared `symphonia-common` cookie
  parser (already admitted). No `unsafe`, no build steps, no I/O beyond
  the packet slice handed in (`BitReaderLtr::new(packet.data)` — bounded
  by the packet).

## F1–F7 summary

| check | verdict |
|---|---|
| F1 native source | NO |
| F2 build script | NO |
| F3 native linking (`links=`) | NO |
| F4 binary blobs | NO |
| F5 transitive surprises | NO (1 new node, all siblings shared) |
| F6 native target gates | NO |
| F7 asm | NO |
