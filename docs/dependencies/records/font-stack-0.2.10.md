# deps/records/font-stack-0.2.10.md — the subtitle font stack (swash + skrifa), **PROPOSAL — Lead approval pending**

> **Class:** P0 dependency survey for issue #17 gap **G-5** ("font
> stack: text cues render as placeholder blocks",
> `docs/gates/phase5-records-audit.md` §1.5/§3, evidence
> `crates/subtitles/src/ass/render.rs:31` at the time of filing).
> **Nothing in this record is admitted**: no `Cargo.toml` row, no
> allowlist entry, no code path imports any crate below. The in-tree
> deliverable is the SEAM (`crates/subtitles/src/font/**`:
> `FontStack`/`FontProvider` source resolution + the `GlyphPipeline`
> trait with the `MonospaceCellPipeline` placeholder), designed so the
> Lead's admission is a behind-the-seam swap. Evidence gathered
> 2026-11-09 in the build sandbox: crates.io API (`/crates/<name>`,
> version + dependencies endpoints), OSV API, and static tarball
> inspection of the static.crates.io downloads (sha256s below), per
> `docs/dependencies/audit-method.md` M1–M6. F1–F7 = doc 11 intake
> flags, R1–R10 = record fields.

## R0 — Recommendation (one paragraph)

Admit **`swash = "=0.2.10"`** (default features: `std`, `scale`,
`render`) as the single font stack for the subtitle glyph pipeline:
font introspection + complex-text shaping + glyph rendering in one
pure-Rust crate, actively maintained (0.2.10 released 2026-07-17),
Apache-2.0 OR MIT, zero native code / build scripts / links. Its
parser dependency is **`skrifa`** (Google Fonts'
[fontations](https://github.com/googlefonts/fontations) project,
resolved `<=0.44` → 0.44.0, 2026-07-04) — the actively maintained
successor lineage to the RazrFalcon crates. The minimal alternative
(`ttf-parser` + `ab_glyph` + `rustybuzz`) is **rejected on
maintenance**: RUSTSEC-2026-0192 (`ttf-parser` unmaintained) and
RUSTSEC-2026-0206 (`rustybuzz` unmaintained) sit directly in its
graph (details R8).

## R1 — Language & source (F1, F3, F4, F7)

- Rust, **no native source files** in any surveyed tarball's `src/`
  (scans `*.c *.cc *.cpp *.cxx *.h *.hpp *.m *.mm *.S *.s *.asm` → 0
  hits; the only C++ anywhere is `ttf-parser`'s `testing-tools/
  font-view/` dev tool, never compiled by the library — 12 files,
  recorded for completeness). F1 NO, F7 NO.
- No `links=` key in any manifest; no `extern "C"`/`#[link]` in any
  `src/` (F3 NO). No binary blobs — `*.a *.lib *.o *.obj *.so *.dll
  *.dylib` → 0 hits in all five inspected tarballs (F4 NO).
- No `build.rs` in any surveyed crate (F2 NO, R3).
- `unsafe` surface (dependency-internal; our workspace `unsafe_code =
  deny` binds only our crates): swash 0.2.10 — 54 sites; ttf-parser
  0.25.1 — `#![forbid(unsafe_code)]` (0); ab_glyph 0.2.32 — 0;
  owned_ttf_parser 0.25.1 — 5 (all `core::hint`/`Pin` interior
  shortcuts in `src/owned.rs`); rustybuzz 0.20.1 — 140 (mostly
  `unsafe impl bytemuck::Pod/Zeroable` for zero-copy buffers; the
  optional `wasm-shaper` feature that needs `wasmi` is OFF by
  default); skrifa 0.49.0 — 1 (the `forbid` line itself).
- Repos (crates.io `repository` field): swash/zeno/yazi →
  `github.com/dfrg/swash`, `github.com/dfrg/zeno`,
  `github.com/dfrg/yazi`; skrifa/read-fonts →
  `github.com/googlefonts/fontations`; ttf-parser/rustybuzz →
  `github.com/harfbuzz/*`; ab_glyph/owned_ttf_parser →
  `github.com/alexheretic/*`; fontdb → `github.com/RazrFalcon/fontdb`.

## R2 — Transitive graph (F5) — what Lumen would actually build

`swash 0.2.10` (default features) resolves (crates.io dependencies
API, fetched 2026-11-09):

```
swash 0.2.10                     (Apache-2.0 OR MIT, 2026-07-17)
├── skrifa >=0.31.1, <=0.44  → 0.44.0   (MIT OR Apache-2.0, 2026-07-04)
│   ├── read-fonts ^0.41     → 0.41.x   (fontations sibling)
│   ├── bytemuck ^1.13
│   └── (dev/optional: core_maths for no_std)
├── zeno ^0.3.3               (Apache-2.0 OR MIT, 2025-05-08) [render path]
└── yazi ^0.2.1               (Apache-2.0 OR MIT, 2026-11-01) [scale path]
```

Five new nodes (swash, skrifa, read-fonts, zeno, yazi) plus the
already-allowlisted `bytemuck`. No `-sys` crates, no build edges.
`[VERIFY-P1: regenerate from our Cargo.lock at admission/merge — the
ranges above resolve today, a fresh resolve may pick newer patch
versions.]`

Rejected-trio graph (for the record): `ab_glyph 0.2.32` →
`owned_ttf_parser 0.25.1` → `ttf-parser 0.25.1` (+ `ab_glyph_
rasterizer 0.1.8`, optional `libm`); `rustybuzz 0.20.1` → ttf-parser
+ bitflags, bytemuck, core_maths, log, smallvec, unicode-bidi-
mirroring, unicode-ccc, unicode-properties, unicode-script (optional
`wasmi` for `wasm-shaper`, off).

## R3 — Build scripts (F2)

None in any surveyed tarball (find → 0 hits; no `build` manifest key).

## R4 — Native links (F3, F4, F6)

None. swash/skrifa/zeno/yazi/read-fonts are pure Rust, no pkg-config,
no vendored C. F6 NO.

## R5 — Targets

Pure Rust, no target gates, no `cfg(target_*)` native paths in the
five tarballs' `src/`; desktop triples and wasm32 compile the same
files (swash documents no_std support via `core_maths`/`libm`
features). `[VERIFY-P1: wasm32 build from our own tree at the first
wasm gate run.]`

## R6 — Feature coverage against the seam's needs

`crates/subtitles/src/font/pipeline.rs` needs: face parsing
(family/style/name tables), per-size glyph metrics, complex-text
shaping (ASS fan subs are the whole point of G-2/G-5 — Arabic/Indic
joining, mark positioning), and anti-aliased rasterization to an
alpha mask. swash covers all four in one crate (introspection /
`shape` / `render` with its zeno rasterizer); skrifa covers parsing +
glyph scaling with variable-font support. The seam's
`GlyphPipeline::rasterize → GlyphMask` maps 1:1 onto swash's render
output (coverage → our straight-alpha mask). The rejected trio splits
the same needs across three crates (ttf-parser parse-only, ab_glyph
raster-only, rustybuzz shape-only) with no single maintainer.

## R7 — License

swash 0.2.10: **Apache-2.0 OR MIT** (version endpoint + tarball
LICENSE files). skrifa/read-fonts: MIT OR Apache-2.0. zeno/yazi:
Apache-2.0 OR MIT. All inside deny.toml's allow set — no license-set
change needed. (RazrFalcon lineage: ttf-parser MIT OR Apache-2.0,
ab_glyph/owned_ttf_parser Apache-2.0, rustybuzz MIT, fontdb MIT —
also permissive; irrelevant post-rejection.)

## R8 — Maintenance & security record (the deciding field)

- **swash 0.2.10**: released 2026-07-17; repo active under dfrg with
  2026-era releases of its whole dep family (yazi 0.2.2 released
  2026-11-01). OSV: **0 advisories** (queried 2026-11-09).
- **skrifa 0.44.0** (the version swash resolves): 2026-07-04,
  fontations publishes continuously (0.49.0 on 2026-10-21). OSV: 0.
- **ttf-parser 0.25.1**: OSV **RUSTSEC-2026-0192 "ttf-parser is
  unmaintained"** (all versions). Last release 2024-11-29.
- **rustybuzz 0.20.1**: OSV **RUSTSEC-2026-0206 "rustybuzz is
  unmaintained"** (all versions). Last release 2024-11-12.
- **ab_glyph 0.2.32 / owned_ttf_parser 0.25.1 / ab_glyph_rasterizer
  0.1.8**: no direct advisories, but the first two's own dependency
  graph terminates at the unmaintained ttf-parser (0.1.8's last
  release is 2023-01-13). Published by `alexheretic` (ab_glyph/
  owned) and `alerque` (ttf-parser/rustybuzz) respectively;
  swash/skrifa by `nicoburns`/`cmyr`/`dfrg` (fontations
  maintainers).
- **fontdb 0.24.0** (2026-07-29, RazrFalcon/fontdb, published by
  nicoburns): surveyed and NOT needed for v1 — system-font
  enumeration is out of scope; our sources are bundled + container
  attachments (`MemoryFontProvider`). Revisit if a system-fonts card
  ever opens.
- **cosmic-text 0.19.0** (2026-04-22, pop-os): full layout engine —
  paragraph/layout/bidi — far beyond the subtitle need and a much
  larger graph; rejected as over-scope (the compositor already owns
  layout).
- `[VERIFY-P1: re-query OSV at the admission date per doc 11 §3.7.]`

## R9 — Provenance

Tarballs fetched from static.crates.io (sha256 recorded; matching the
crates.io version-endpoint checksums at fetch time, 2026-11-09):

| Crate | sha256 |
|---|---|
| swash 0.2.10 | `6c2499c2d826531388872b2268718aed907a39bd785ab0dcfe57fab26283f92e` |
| ttf-parser 0.25.1 | `d2df906b07856748fa3f6e0ad0cbaa047052d4a7dd609e231c4f72cee8c36f31` |
| ab_glyph 0.2.32 | `01c0457472c38ea5bd1c3b5ada5e368271cb550be7a4ca4a0b4634e9913f6cc2` |
| owned_ttf_parser 0.25.1 | `36820e9051aca1014ddc75770aab4d68bc1e9e632f0f5627c4086bc216fb583b` |
| rustybuzz 0.20.1 | `fd3c7c96f8a08ee34eff8857b11b49b07d71d1c3f4e88f8a88d4c9e9f90b1702` |
| skrifa 0.49.0 | `a7459fb60888ad9a5f367dd293fa2fb9df142cb451aa6bfd6cd66a7d6a9b739d` |

(skrifa pinned at the newest 0.49.0 for the survey; the swash
constraint would resolve 0.44.x at admission — regenerate the row
then.) `[VERIFY-P1: publisher identity + index-hash cross-check at
admission time, per the cpal record's convention.]`

## R10 — Verdict

**conditional pass → [PROPOSAL — Lead approval pending].** Purity is
clean across the board (F1–F7 NO for the recommended set); licenses
allowlisted; the decision field is maintenance, where the swash/
fontations lineage is the only surveyed stack with 2026 releases and
zero advisories. If admitted: pin `swash = "=0.2.10"`, implement
`GlyphPipeline` for it inside `crates/subtitles/src/font/` (the seam
this record's sibling code lands), and re-run R2/R9 from the merged
Cargo.lock. Two non-admission blockers live OUTSIDE the crates: (a)
no default TTF/OTF face exists in-repo (the only font file is
`ui/src/assets/fonts/nunito-latin-var.woff2`, OFL 1.1 — a WOFF2
container needing Brotli decompression, i.e. another dependency; the
engine default face choice — e.g. an OFL face shipped as raw TTF —
is the Lead's, with licensing recorded here when made); (b) MKV
attachment→`MemoryFontProvider` wiring belongs to the demux card
(the seam is the deliverable of issue #17).

*Evidence posture (AGENTS §3.4): every API fetch, OSV query, tarball
download and scan above was executed in the build sandbox on
2026-11-09; command transcripts and raw JSON live outside the repo
(scratch `/tmp/font-audit/`, gitignored territory). No statement
about the crates' runtime behavior is made — nothing was built or
executed; capability claims cite crate documentation fields only.*
