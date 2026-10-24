# deps/records/claxon-0.4.3.md — claxon 0.4.3 (purity verification, card 5.A1)

> **Class:** P0 dependency verification (doc 11 §2 format, fields R1–R10 +
> F1–F7 scan), ordered by doc 13 card 5.A1: "FLAC decoder (symphonia-claxon
> or claxon `[VERIFY purity]`)" — the Lead's wave-1 brief pre-approves ONE of
> the two as the FLAC decode engine, **but only after purity verification on
> the exact version pinned**. Chosen candidate: `claxon` (the card's stated
> preference: tiny, zero transitive deps). Evidence gathered 2026-10-25 in
> the build sandbox: crates.io API (`/crates/claxon`, `/crates/claxon/0.4.3`,
> `/crates/claxon/0.4.3/dependencies`), OSV API, and tarball inspection of
> the static.crates.io download (sha256 below). **Final acceptance and the
> allowlist row remain Lead decisions (doc 11 §3.1; this file's sibling
> `allowlist.md` is Lead-owned and untouched by 5.A1).**

## R1 — Language & source (F1, F7)

- Rust, **no native source files** in the claxon 0.4.3 tarball (sha256
  `4bfbf56724aa9eca8afa4fcfadeb479e722935bb2a0900c2d37e0cc477af0688`,
  86,626 bytes, 7 `.rs` files / 3,699 lines in `src/`): scans for
  `*.c *.cc *.cpp *.cxx *.h *.hpp *.m *.mm *.S *.s *.asm` → **0 hits**
  (F1 NO, F7 NO).
- No `links=` key in the published manifest. No `extern "C"` /
  `extern "system"` / `#[link]` declarations anywhere in `src/` (F3 NO).
- No binary blobs (`*.a *.lib *.o *.obj *.so *.dll *.dylib` → 0 hits;
  F4 NO — nothing vendored, and with no build script nothing can be
  downloaded at build time).
- `unsafe` surface (recorded for the Lead's review; our `unsafe_code`
  lint binds our own crates, not dependencies): **5 `unsafe` blocks**,
  each with an in-source invariant comment —
  `src/input.rs:148` (`get_unchecked` after an explicit bounds check in
  the same function), `src/frame.rs:569` (stereo `get_unchecked` guarded
  by a constructor-time assertion that the buffer ≥ 2× block duration),
  `src/metadata.rs:437/474/545` (`Vec::set_len` after reading exactly the
  declared length into the allocation). No raw pointers, no FFI, no
  transmute.
- Repo: https://github.com/ruuda/claxon (crates.io `repository` field).
  Description: "A FLAC decoding library". Published by `ruuda`
  (crates.io `published_by`).

## R2 — Transitive graph (F5)

Per the crates.io dependencies API (fetched 2026-10-25): claxon 0.4.3 has
**zero normal dependencies**. Its five declared deps are all
`kind: dev` — `hound ^3.0`, `mp4parse ^0.8`, `ogg ^0.5.1`, `time ^0.1`,
`walkdir ^1.0` — used only by claxon's own tests/benches/examples, which
our build never compiles. The effective graph Lumen builds is therefore:

```
claxon 0.4.3   (no outgoing edges)
└── (root) codecs
```

No `-sys` crates, no build edges, nothing new for the tree-diff beyond the
single `claxon` node. `[VERIFY-P1: regenerate from our own Cargo.lock at
admission/merge time — the record walked the API, and the merged Cargo.lock
diff must show exactly this single addition.]`

## R3 — Build scripts (F2)

- **No `build.rs`** in the tarball (find → 0 hits; manifest has no `build`
  key). F2 NO — there is no script to invoke any toolchain.

## R4 — Native links (F3, F4)

- None: no `links=`, no extern declarations, no blobs, no pkg-config, no
  vendored C. F3/F4/F6 all NO — this is a from-scratch pure-Rust FLAC
  decoder (metadata + frame + subframe modules), not a wrapper.

## R5 — Targets

- Pure Rust with no target-specific code paths, no build script, no
  `cfg(target_*)` native gates: desktop triples and wasm32 compile the
  same 7 files. `docs.rs` build status for claxon 0.4.3 shows the crate
  building on the standard target matrix `[VERIFY-P1: wasm32 build from
  our own tree at the first wasm gate run]`.

## R6 — Actual feature coverage (against card 5.A1's needs)

- Verifiable in-sandbox: decodes native FLAC (STREAMINFO metadata parse,
  `FlacReader::new`, `FlacReader::samples()` iterator, `FrameReader`
  with `max_block_size` sizing, per-frame block/duration access), exposes
  seek points via `metadata::SeekTable` (`SeekTable::from_bytes`, opaque
  `SeekPoint` with sample/frame-number fields) — the surface the card's
  SEEKTABLE seeking needs. Bit depth up to 32 (`i32` samples), arbitrary
  channel counts.
- Known upstream posture (doc 04 §2.1/§2.2, verified 2026-10-19):
  claxon is frozen since 2020 (last release 2020-08-09) but stable and
  widely used (4.7M downloads on 0.4.3). Ogg-FLAC is NOT covered (dev-dep
  only in claxon's own tests) — out of card scope: 5.A1 is the native-FLAC
  codec card; Ogg pairing belongs to the Ogg demuxer card (5.B1).
- Our own conformance evidence: the card's ffmpeg-generated fixtures and
  bit-exact differential tests (`crates/codecs/tests/flac_bitexact.rs`)
  are the recorded coverage proof — see the test file header for commands
  and MD5s.

## R7 — License

- Apache-2.0 (crates.io version endpoint AND `license` file in the
  tarball) — inside deny.toml's allow set. No license-set change needed.

## R8 — Maintenance & security record

- Last release 0.4.3 on 2020-08-09; unmaintained-since-2020 is a recorded
  doc 04 gap, accepted by the card's design: the module wraps it behind
  our own error taxonomy so a future swap to `symphonia-bundle-flac`
  stays inside `crates/codecs/src/audio/flac/**`.
- RustSec/OSV: **2 advisories, both historical and NOT affecting 0.4.3** —
  RUSTSEC-2018-0004 / GHSA-8c6g-4xc5-w96c ("uninitialized memory
  exposure"), affected ranges `< 0.3.2` and `< 0.4.1`; 0.4.3 is outside
  both ranges (OSV API fetch 2026-10-25). No open advisories for the
  pinned version. `[VERIFY-P1: re-query RustSec at admission date per
  doc 11 §3.7 triggers.]`

## R9 — Provenance

- Tarball fetched from `https://static.crates.io/crates/claxon/claxon-0.4.3.crate`;
  sha256 `4bfbf56724aa9eca8afa4fcfadeb479e722935bb2a0900c2d37e0cc477af0688`
  matches the crates.io version endpoint `checksum` field exactly (both
  recorded above). Publisher: `ruuda`.
  `[VERIFY-P1: publisher identity + index-hash cross-check at admission
  time, per the cpal record's convention.]`

## R10 — Verdict

**pass (pure Rust) — admitted by card 5.A1 under the Lead's pre-approval,
pending the Lead's allowlist row.** Pinned `claxon = "=0.4.3"` (exact pin —
no caret drift; a bump re-triggers this record per doc 11 §3.7). F1–F7 all
NO; R2 graph is a single edge-free node; license allowlisted; the only
historical advisory does not touch 0.4.3. The Lead's merge of
`crates/codecs/Cargo.toml` + `Cargo.lock` + this record is the actual
admission (AGENTS §1: dependency approval is Lead-only); allowlist.md is
deliberately NOT edited by 5.A1 (outside R1 ownership).

*Evidence posture (AGENTS §3.4): every statement above was executed locally
in the build sandbox on 2026-10-25 — API fetches (curl, exit 0), OSV query,
sha256 of the downloaded tarball matching the index checksum, full tarball
extraction and F1–F7 scans with hit counts recorded. No statement about
claxon's decode quality is made without our own fixture run behind it.*
