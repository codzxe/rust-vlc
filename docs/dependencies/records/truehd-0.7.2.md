# deps/records/truehd-0.7.2.md — truehd 0.7.2 (+ bitstream-io 4.10.0
# transitively): purity verification AND compile-admission verdict
# (card 5.K1, wave 7)

> **Class:** P0 dependency verification (doc 11 §2 format, fields R1–R10 +
> F1–F7 scan), ordered by card 5.K1 "TrueHD hardening (truehdd crate)"
> under the Lead's C5 pre-approval. **Naming reality check (the AAC
> record's pattern):** the brief, the wave plan and doc 04 §8.6 item 5 all
> say "the `truehdd` crate" — on crates.io `truehdd` 0.6.2 is the **CLI
> toolbox**; the **library** the decoder would ride is **`truehd` 0.7.2**
> (doc 04 §3.1 row "TrueHD (MLP)" spells this correctly). This record
> covers the library. Evidence gathered 2026-10-24 in the build sandbox
> from the crates.io API, `cargo fetch` tarballs and the extracted
> sources under `~/.cargo/registry/src/`. **Verdict up front: purity
> PASS, admission BLOCKED — the pinned tarball does not compile under
> the workspace toolchain** (rustc 1.99.1 is OOM-killed at ~3.4–3.6 GB
> anon-RSS in the 4 GB sandbox, even for `cargo check`); root cause
> isolated to a macro expansion in the transitive `bitstream-io`
> dependency (R11 below). Per the mandate ("unadmittable → honest blocked
> record, no half-work"): no `Cargo.toml` edit, no module, no tests
> shipped with this card. Final acceptance remains a Lead decision
> (doc 11 §3.1).**

## R1 — Language & source (F1, F7)

- Rust, **no native source files** in the tarball: `*.c *.cc *.cpp
  *.cxx *.h *.hpp *.m *.mm *.S *.s *.asm` scans → **0 hits** (F1 NO,
  F7 NO). No binary blobs (`*.a *.lib *.o *.obj *.so *.dll *.dylib` →
  0 hits; F4 NO). 28 `.rs` files, 12 779 lines.
- No `links=` key, no `extern "C"` / `#[link]` anywhere (F3 NO). The
  manifest is explicit: `build = false`, `autolib = false`.
- `unsafe` surface: **2 sites**, both in `src/structs/oamd.rs` — line
  562 (`RAMP_DURATION_LIST.get_unchecked(..)` inside the Atmos
  ramp-duration table lookup) and line 1462
  (`transmute::<u8, SpeakerLabels>` for the speaker-label enum). Both
  live in the **Dolby Atmos OAMD (object-audio metadata) path** —
  outside the FBA decode path a v1 integration would exercise
  (presentations 0–2, per the feasibility §3 posture). No
  crate-level `#![forbid(unsafe_code)]`; the count is the whole-tree
  grep.
- Repo: github.com/truehdd/truehdd, Apache-2.0 `LICENSE` present in
  the tarball. Publisher `saindriches` (crates.io audit action
  "publish", verified via the API this session).

## R2 — Transitive graph (F5)

Resolved by our own `cargo fetch` + `cargo tree --edges normal` in a
scratch project:

```
codecs
└── truehd 0.7.2
    ├── anyhow 1.0.104                 (NEW NODE)
    ├── bitstream-io 4.10.0            (NEW NODE)
    │   └── no_std_io2 0.9.4           (NEW NODE)
    │       └── memchr 2.8.3           (already in workspace lock)
    ├── log 0.4.34                     (already in lock)
    └── thiserror 2.0.21               (already in lock)
        └── thiserror-impl 2.0.21 (proc-macro; syn/quote/
            proc-macro2/unicode-ident — all already in lock)
```

**4 new registry packages** would enter the lock (`truehd`, `anyhow`,
`bitstream-io`, `no_std_io2`). The `evo-protection` feature (optional
`hmac` + `sha2`) is **not enabled** — Evolution-frame HMAC verification
is out of v1 scope, keeping the graph crypto-free. No `-sys` crates, no
git deps, no default-off features pulled.

## R3 — Build scripts (F2)

- **No `build.rs`** (`build = false` in the manifest). F2 NO.

## R4 — Native links (F3, F4, F6)

- None: no `links=`, no extern declarations, no blobs, no pkg-config,
  no vendored C. F3/F4/F6 all NO — a from-scratch Rust implementation
  of the MLP/TrueHD decode chain, not a wrapper.

## R5 — Targets

- Pure Rust, no `cfg(target_*)` native gates in the crate itself.
  **Moot for admission**: the crate does not reach codegen in this
  environment (see R11). MSRV: manifest `rust-version = "1.88.0"` (the
  feasibility §3 quote of README "MSRV 1.95.0" is stale vs the
  manifest — recorded; our toolchain 1.99.1 satisfies either).
  `[VERIFY-P1: wasm32 build at the first wasm gate, post-unblock.]`

## R6 — Actual feature coverage (against card 5.K1's needs)

API verified in-sandbox against the extracted source
(`src/process/{extract,parse,decode}.rs`):

- Pipeline: `Extractor` (byte-push, sync-search, `Iterator<Item =
  Result<Frame>>`) → `Parser::parse(&Frame) → AccessUnit` →
  `Decoder::decode_presentation(&AccessUnit, usize) →
  DecodedAccessUnit`.
- `DecodedAccessUnit` carries `pcm_data: [[i32; 16]; 160]`
  (sample-major, **24-bit-magnitude** signed samples),
  `sample_length`, `channel_count`, `sampling_frequency`,
  `channel_labels: Vec<ChannelLabel>` (observed: `L R C LFE Ls Rs`
  for the 5.1(side) stream), `is_duplicate`, `substream_info_changed`.
- Recovery vocabulary maps onto our typed model: extract errors
  (`ExtractError`) per skipped frame, `error_count()`,
  `reset_for_next_major_sync()` on both Parser and Decoder for
  post-fatal resync — the damaged-stream behavior the feasibility §3
  recorded.
- Presentations: multi-presentation decode
  (`decode_presentations([bool; 4])`); presentation 0 of a
  multichannel stream is the **author-embedded stereo downmix**
  (measured: the ffmpeg-authored 5.1 fixture's presentation 0 decodes
  to silence, the native 6 channels live at presentation 1).
- **Decode quality — measured, in scratch (see R11 for why this is
  not a shipped test):** with the compile workaround applied to a
  /tmp copy of the tarball, the engine decodes four ffmpeg 7.1.5
  authored streams **bit-exactly** (lossless ⇒ exact match after the
  documented `value << 8` left-justify into ffmpeg's s32 convention):

  | fixture (ffmpeg 7.1.5 `-c:a truehd`/`mlp`) | pres | samples (×ch) | mismatches vs `ffmpeg -f s32le` |
  |---|---|---|---|
  | stereo 44.1 kHz sine 5 s (`.thd`) | 0 | 220 500 × 2 | **0 of 441 000** i32 |
  | 5.1(side) 48 kHz pink 5 s (`.thd`) | 1 (native) | 240 000 × 6 | **0 of 1 440 000** |
  | 5.0(side) 96 kHz white 4 s (`.thd`) | 1 (native) | 384 000 × 5 | **0 of 1 920 000** |
  | MLP (DVD-Audio subset) stereo 44.1 kHz 3 s (`.mlp`) | 0 | 132 320 × 2 | **0 of 264 640** |

  MD5 equality also holds at both widths for the stereo lane: our
  trimmed output shifted `<<8` reproduces ffmpeg's s32le MD5
  `72aa2a86cbd8813f02da3612d9531e3e` exactly, and the low-3-bytes
  form reproduces the s24le MD5 `3d092c4c77896b975338d1a08137dfba`
  exactly. One honest convention gap, recorded for the unblock card:
  at TrueHD (not MLP) stream end the engine emits **20 extra
  inter-channel samples of trailing zeros** (all measured zero; a
  partially-padded final access unit) beyond ffmpeg's 220 500 — a
  trailing-trim rule our adapter must own. Fixture regeneration is
  deterministic (`anoisesrc` default seed; commands recorded in the
  5.K1 worklog).
- Scope ceilings measured: ffmpeg 7.1.5's `truehd` **encoder**
  supports at most 5.1(side) — 8/16-channel presentations and the
  Atmos/DAMF presentation are **untested** (Dolby reference streams
  are not redistributable; C-V6 acquisition item, corpus-plan.md:394).
  The legacy **MLP subset** (FBB/DVD-Audio) decodes presentation 0
  bit-exactly — the engine's own FBB support, recorded as-measured.

## R7 — License

- **Apache-2.0** (crate metadata + `LICENSE` file in tarball).
  Already in deny.toml's allow set (claxon precedent). No new license
  work for the Lead.

## R8 — Maintenance & security record

- Young research crate: created 2025-07-23, 12 versions, latest
  0.7.2 published 2026-09-15; ~3.4 k downloads (1.6 k recent).
  Self-described "research implementation … not intended for
  production" (README banner, feasibility §3). Single-maintainer risk
  already carried by R-43.
- OSV queries (api.osv.dev, this session): **no advisories** for
  `truehd` or `bitstream-io`.
  `[VERIFY-P1: re-query RustSec at any future admission date per
  doc 11 §3.7.]`

## R9 — Provenance

- Tarball fetched by `cargo fetch` from static.crates.io through the
  pinned sparse index; checksum verified by cargo. Publisher:
  `saindriches` (truehdd project). Older-version audit: 0.6.0 and
  0.7.1 tarballs fetched and inspected — both already use the same
  `define_huffman_tree!` macro (no version dodge exists).
  `[VERIFY-P1: publisher identity cross-check at admission time.]`

## R10 — Verdict

**blocked (purity PASS — F1–F7 all NO, no build scripts, 2 `unsafe`
sites both in the out-of-scope Atmos OAMD path — but the pinned
tarball cannot be compiled by the workspace toolchain; see R11).**
No dependency edge, no module, no tests ship with card 5.K1 (mandate:
"honest blocked record, no half-work"). Unblock triggers, in the Lead's
court: (a) an upstream `bitstream-io` release fixing the `ToBits`
expansion (R11) — then re-run this record against the same `truehd`
0.7.2 pin; (b) a Lead-approved `[patch.crates-io]` vendor-fork of
`bitstream-io` replacing the write-tree macro — the scratch
equivalent is ~60 lines, compiles clean, and is the build behind the
R6 bit-exact table (R-43's vendor-fork fallback, stays pure Rust);
(c) `[VERIFY: retest on a ≥8 GB host]` — the OOM fires at ~3.4 GB
anon-RSS on this 4 GB/no-swap sandbox; a bigger CI runner may pass,
but the minimal repro (R11) is the one-command re-check on any host.
The decode-quality question 5.K1 was chartered to answer is **already
answered YES in scratch** (R6); what remains blocked is admission.

## R11 — The blocker, isolated (compile-time OOM; evidence)

- **Symptom:** `cargo check`, `cargo build` (debug and release),
  incremental on and off — `rustc` 1.99.1 is **SIGKILLed by the global
  OOM killer** while compiling `truehd 0.7.2` (frontend: even
  `--emit=dep-info,metadata` dies). dmesg (captured this session):
  `Out of memory: Killed process … (rustc) total-vm:20.6 GB,
  anon-rss:3.58 GB` (host: 4 GB RAM, no swap). A workspace that
  carried this dep would fail fmt/clippy/test gates at compile.
- **Bisection trail (all reproduced this session):** full crate OOM →
  `process`+`structs` removed, still OOM → **`utils` only, still
  OOM** → `utils/bitstream_io.rs` only, still OOM → the three
  `define_huffman_tree!` invocations removed: **compiles clean**.
- **Minimal repro (~20 lines, verbatim):** depends on
  `bitstream-io = "=4.10.0"` and invokes
  `define_huffman_tree!(HuffTree1 : i32 = [ [[[ (-7)… depth-8 tree
  …]]] ]);` (the exact tree is `truehd`'s
  `src/utils/bitstream_io.rs:17–25`) — `cargo check` OOMs identically.
- **Root cause:** the macro's **`ToBits` (write-tree) expansion**
  (`compile_write_tree_nodes!`, bitstream-io 4.10.0
  `src/huffman.rs:129+`) — a hand-expanded **read-only** (`FromBits`)
  tree with identical nesting compiles fine, and the **full `truehd`
  0.7.2 crate compiles cleanly** when the three trees are replaced by
  hand-written `FromBits` impls (the decoder never uses `ToBits`).
- **No dodge:** bitstream-io 4.10.0 is the newest release
  (2026-04-14, crates.io API this session); `truehd` 0.6.0/0.7.1
  carry the same macro. Not tested (out of budget): older rustc
  (crate MSRV 1.88.0) — the sandbox lacks the disk to stage a second
  toolchain; recorded as
  `[VERIFY: whether rustc ≤1.9x regressed this expansion]`.

## Downmix policy (card 5.K1 item 3, recorded here for the unblock)

**No silent downmix — ever.** TrueHD's own multi-presentation
structure (presentation 0 is typically an authored Lt/Rt stereo
downmix of the native multichannel presentation) is **author
content**, not our conversion: a Lumen TrueHD decoder selects the
presentation matching the negotiated channel configuration and decodes
native channels; if the audio-engine needs fewer channels, downmixing
is the audio-engine's job (its mixer), never the codec's. The adapter
must surface `channel_labels` and `substream_info_changed` so the
engine can re-negotiate honestly. Atmos presentations stay
out-of-scope v1 (feasibility §3 posture; also where the crate's only
2 `unsafe` sites live).

*Evidence posture (AGENTS §3.4): every statement above was executed
locally in the build sandbox on 2026-10-24 — crates.io/OSV API
queries, `cargo fetch` + `cargo tree --edges normal` (the R2 tree is
pasted from that run), full F1–F7 scans of the extracted tarballs
with hit counts, the OOM bisection (each step re-compiled), the
minimal repro, the scratch-patched full-crate compile, and the
four-stream ffmpeg differential behind the R6 table (MD5s quoted).
No decode-quality claim rides on anything but our own measured runs;
nothing was shipped into the workspace.*
