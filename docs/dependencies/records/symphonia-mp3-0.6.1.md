# deps/records/symphonia-mp3-0.6.1.md — symphonia-bundle-mp3 0.6.1 +
# symphonia-core 0.6.1 (purity verification, card 5.A3)

> **Class:** P0 dependency verification (doc 11 §2 format, fields R1–R10 +
> F1–F7 scan), ordered by doc 13 card 5.A3: "MP3 decoder (symphonia) …
> `[VERIFY purity on the pinned versions]`". The Lead's brief pre-approves
> symphonia as the MP3 decode engine. **Reality check that changed the
> spelling:** the card's assumed crate name `symphonia-codec-mp3` does not
> exist on crates.io (`{"errors":[{"detail":"crate … does not exist"}]}`,
> sparse index `sy/mp/…` confirms); the actual published crates are
> **`symphonia-bundle-mp3` 0.6.1** (MP1/MP2/MP3, features `mp1`/`mp2`/`mp3`
> all default-on — doc 04 §3.1's "symphonia-bundle-mp3 0.6.1 (MP1/MP2/MP3,
> Excellent)" row) and its engine dep **`symphonia-core` 0.6.1**. This is
> the brief's "fewer deps" alternative shape (not the `symphonia` meta
> crate, which would drag every format reader in). Evidence gathered
> 2026-10-26 in the build sandbox from the sparse index
> (index.crates.io), `cargo fetch` tarballs (static.crates.io via cargo),
> and the extracted sources under `~/.cargo/registry/src/`.
> **Final acceptance and the allowlist row remain Lead decisions
> (doc 11 §3.1; `allowlist.md` + `deny.toml` are Lead-owned and untouched
> by 5.A3).**

## R1 — Language & source (F1, F7)

- Rust, **no native source files** in either tarball: scans for
  `*.c *.cc *.cpp *.cxx *.h *.hpp *.m *.mm *.S *.s *.asm` → **0 hits**
  in `symphonia-bundle-mp3-0.6.1/src` (11 `.rs` + `layer1/`, `layer2/`,
  `layer3/` subdirs, `decoder.rs`/`header.rs`/`synthesis.rs` …) and in
  `symphonia-core-0.6.1/src` (F1 NO, F7 NO).
- No `links=` key in either published manifest. No `extern "C"` /
  `extern "system"` / `#[link]` anywhere in either `src/` tree (F3 NO).
- No binary blobs (`*.a *.lib *.o *.obj *.so *.dll *.dylib` → 0 hits;
  F4 NO — nothing vendored; no build script, so nothing downloads).
- `unsafe` surface: **both crates declare `#![forbid(unsafe_code)]` at
  the crate root** (`symphonia-bundle-mp3/src/lib.rs:9`,
  `symphonia-core/src/lib.rs:7`) — stronger than claxon's 5 documented
  blocks: there are zero unsafe sites, compiler-enforced.
- Repo: https://github.com/pdeljanov/Symphonia (single monorepo for all
  symphonia crates). MPL-2.0 LICENSE file present in both tarballs.

## R2 — Transitive graph (F5)

Resolved by our own `cargo fetch` + `Cargo.lock` (the real graph Lumen
builds — not an API walk):

```
codecs
└── symphonia-bundle-mp3 0.6.1        (ck 98ea5ffc8716bff677df…)
    ├── lazy_static 1.5.1              (ck 20870f649af7073d53e3…)
    ├── log 0.4.34                     (ck f9f8bd3e56ce4dfc153c…)
    └── symphonia-core 0.6.1           (ck 01c412864d599d4750d0…)
        ├── bitflags 2.13.2            (ck 3ded4057c258ba199e2d…)
        ├── bytemuck 1.25.2            (ck 95832e849adfb21180cc…)
        ├── lazy_static (shared)
        ├── log (shared)
        ├── num-complex 0.4.6          (ck 73f88a1307638156682b…)
        │   └── num-traits 0.2.19      (ck 071dfc062690e90b734c…)
        └── smallvec 1.16.4            (ck d0d5d4b1981b8bbbb7ae…)
```

9 registry packages total, every one a pure-Rust utility crate already
in common use; no `-sys` crates, no git deps, no optional deps enabled
(`rustfft` behind symphonia-core's non-default feature, not pulled).
The Cargo.lock diff on this branch is exactly these 9 nodes
(`git diff Cargo.lock` → 80 insertions, 1 file).
`[VERIFY-P1: regenerate at merge — the merged lock diff must show
exactly these 9 additions.]`

## R3 — Build scripts (F2)

- **No `build.rs`** in either tarball (manifests have no `build` key).
  F2 NO.

## R4 — Native links (F3, F4, F6)

- None: no `links=`, no extern declarations, no blobs, no pkg-config,
  no vendored C. F3/F4/F6 all NO — from-scratch Rust ports of the MPEG
  audio layer I/II/III decode chain (header → side info → scalefactors →
  alias reduction → IMDCT → synthesis), not wrappers.

## R5 — Targets

- Pure Rust, `#![forbid(unsafe_code)]`, no build script, no
  `cfg(target_*)` native gates. symphonia-core has `opt-simd` features
  (avx/neon/sse) that are **not default** and not enabled by us — the
  scalar paths compile everywhere. `[VERIFY-P1: wasm32 build from our
  own tree at the first wasm gate run.]`

## R6 — Actual feature coverage (against card 5.A3's needs)

- Verifiable in-sandbox: `MpaDecoder::try_new(&AudioCodecParameters,
  &AudioDecoderOptions)` accepts `CODEC_ID_MP1/MP2/MP3`;
  `decode_ref(&PacketRef)` → `GenericAudioBufferRef` (F32 samples,
  `.frames()`, `.spec()`, `copy_to_slice_interleaved::<f32,_>`), the
  engine enforces one-packet-one-frame (`header.frame_size` must equal
  the packet's remaining bytes — our framing layer must slice exact
  frame lengths, which is our own `header.rs`'s job anyway). The
  symphonia 0.6.1 API surface differs from the 0.5.x names in doc 04's
  survey (PacketRef/AudioCodecParameters/…): the module documents the
  exact calls it makes.
- Explicitly NOT used: the crate's own `MpaReader` demuxer and its
  gapless trim (`AudioDecoderOptions::gapless` set to `false`) — the
  card's ID3v2/Xing/VBRI/seek/gapless surface is implemented by
  `crates/codecs/src/audio/mp3/**` ourselves; the engine owns only the
  entropy-decode of one framed packet. A future engine swap stays inside
  that module boundary.
- Our own conformance evidence: ffmpeg-generated fixtures + the corpus
  differential (`crates/codecs/tests/mp3_bitexact.rs`,
  `mp3_corpus_differential.rs`) are the recorded coverage proof — see
  the test file headers for commands and digests.

## R7 — License

- **MPL-2.0** for both crates (LICENSE files + manifest `license` key).
  NOT in deny.toml's current minimal allow set (MIT/Apache-2.0/
  Unicode-3.0/Unlicense). deny.toml's own header comment names exactly
  this case: "admitting a new license class (e.g. MPL-2.0 for a future
  crate) requires a same-PR edit" — the Lead's admission merge must add
  `"MPL-2.0"` to `[licenses] allow` in the same PR that merges this
  record. MPL-2.0 is a weak, file-level copyleft accepted across the
  Rust ecosystem (Firefox-adjacent crates); no GPL/LGPL contamination
  (bans stay absent per doc 11 §3.2).

## R8 — Maintenance & security record

- Symphonia is actively maintained (0.6.x line current; the bundle-mp3
  crate rides the same release train). OSV/RustSec query for
  `symphonia-bundle-mp3`/`symphonia-core`: no advisories filed against
  any version as of 2026-10-26.
  `[VERIFY-P1: re-query RustSec at admission date per doc 11 §3.7.]`

## R9 — Provenance

- Both tarballs fetched by `cargo fetch` from static.crates.io through
  the pinned index; sha256 checksums recorded in R2 were verified by
  cargo itself against the sparse index (a checksum mismatch fails the
  fetch). Publisher: `pdeljanov` (Symphonia project).
  `[VERIFY-P1: publisher identity cross-check at admission time, per
  the cpal record's convention.]`

## R10 — Verdict

**pass (pure Rust, `#![forbid(unsafe_code)]` in both crates) — admitted
by card 5.A3 under the Lead's pre-approval, pending (a) the Lead's
allowlist row and (b) the same-PR `deny.toml` `MPL-2.0` license-class
addition (R7).** Pinned `symphonia-bundle-mp3 = "=0.6.1"` (exact pin;
a bump re-triggers this record per doc 11 §3.7). F1–F7 all NO; R2 graph
is 9 pure-Rust nodes, all recorded; no advisories; wasm32 left as the
standing P1 verify.

*Evidence posture (AGENTS §3.4): every statement above was executed
locally in the build sandbox on 2026-10-26 — sparse-index fetches,
`cargo fetch` + `cargo tree --edges normal` (the R2 tree is pasted from
that run), checksum reads from our own Cargo.lock, and full F1–F7 scans
of both extracted tarballs with hit counts recorded. No statement about
symphonia's decode quality is made without our own fixture run behind
it (see R6).*
