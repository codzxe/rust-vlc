# Video record — VP8 / VP9 decode (card 5.P1-I)

Decision record, doc 04 §8.6 item 4's VP8/VP9 half ("video codec hole",
§4.5). No code changed. Front-matter keys per doc 04 §6:

- id = `codec-video-vp9` + `codec-video-vp8` (two feature-matrix rows,
  `docs/feature-matrix.toml` 473–496: both `not_started` on all four
  platforms with empty `evidence` — this record is that evidence)
- status = **vp9: blocked (candidate named, validation pending)**;
  **vp8: blocked (no validated engine)**
- phase = 6 for the VP9 software-decode admission (matrix `phase_target
  = 5` recorded the ambition; validation spike is the entry gate); VP8
  recheck at Phase 7–8 (matrix `phase_target = 8`)
- fuzz_target = `fuzz_codec_vp9` / `fuzz_codec_vp8` (doc 04 §4.1 rows —
  do not exist until a decoder is admitted)
- crates = [] — none admitted; demux-side carriage already landed (below)
- self_built = []
- last_audit = 2026-10-24 (this card's evidence pass; prior: doc 04
  §4.1 table, verified 2026-10-19)

## Verdict

**Two-part verdict, because the two codecs split on evidence.**

1. **VP9 (P0): validate-then-admit `rusty_vp9` — NOT self-build.** A
   pure-Rust VP9 decoder **is published on crates.io** and doc 04
   §4.1's "none (crates.io `vp9` NOT_FOUND)" is stale (see evidence):
   the exact-name crate is indeed absent, but the survey stopped there
   and missed `rusty_vp9` (created 2026-07-29, i.e. *before* the
   2026-10-19 survey date). Primary plan: **S validation spike**
   (conformance + differential + purity census), then **M admission**
   (direct dep, glue ours, `fuzz_codec_vp9`) in Phase 6. Until the
   spike passes, v1 ships the typed blocked gap — the *recorded reason
   amends* from "no pure-Rust decoder exists" to "no **validated**
   pure-Rust engine" (same amendment shape as 5.K2-I/Opus).
2. **VP8 (P1): blocked — upheld, reason unchanged.** No credible
   full-video VP8 decoder exists today (candidates are a self-described
   scaffold and a 17-day-old crate — evidence below). Typed
   decode-pending semantics; recheck triggers named.

**The card's three listed options, each recorded:**

- **Blocked record** — SELECTED as the v1 shipping state for both
  codecs (nothing is admitted by this card; no codec code). For VP9 it
  is the interim state only, with the unblock plan attached.
- **WebCodecs-analog (web column)** — NOT selected as primary. Doc 09
  §10 makes WebCodecs the web analog of ADR-001 OS decoders, but its
  own status is `partial [PROPOSAL — pending WebCodecs ADR]` (doc 09
  §12 item 6 keeps H.264/HEVC blocked on web until that ADR passes;
  VP8/VP9 would ride the same ADR), and it covers **zero** of the
  desktop/native product that this card's priority serves. Recorded as
  the *web-column complement* once the ADR opens: VP8/VP9 are in the
  broadly supported WebCodecs set (`vp8`/`vp09`, doc 09 §8.1 support
  matrix, verified 2026-10-19) and report
  `decoders[].hardware = true, backend = "WebCodecs"` truthfully
  (matrix row `platform-hwdecode-vp9` already anticipates this).
- **Self-build (XL per doc 04 §4.1/§4.5 — the card brief said "L";
  the doc's XL is the number of record)** — DEMOTED to last resort for
  VP9. VP9 is dav1d-class complexity (doc 04 §4.5 frames the whole
  VP8/VP9 pair as the "video codec hole", XL each, feeding the §8
  Phase 5/6 self-build program). With `rusty_vp9` published and
  bit-exact-claiming, an XL duplicate needs the validation spike to
  fail first. For VP8 self-build stays `later` per doc 04 §4.1
  (lower P1, no engine, no forcing corpus pressure).

## Evidence (all fetched this session, 2026-10-24, crates.io API)

**Doc 04 §4.5/§4.1 re-verification.** Exact-name queries today:
`vp8` → `{"errors":[{"detail":"crate \`vp8\` does not exist"}]}`;
`vp9` → same. The literal 2026-10-19 query result holds. The
*survey's conclusion* does not (below). `rav1e` is AV1-encode-only
(doc 04 §4.4 already notes it for the future transcode path) — no
VPx decoder there. Forbidden C bindings confirmed present and
excluded: `shiguredo_libvpx` 2026.2.0-canary.3, `vpx-rs` 0.2.1,
`env-libvpx-sys` 5.1.3, `media-codec-vpx` 0.8.0 ("based on libvpx"),
`libvpx-native` 0.4.5, `vpx` 0.3.1 (2015).

**VP9 — pure-Rust decoders exist (the material finding):**

- `rusty_vp9` 0.1.3 (Apache-2.0, edition 2021, lib-only, **zero
  dependencies** — no `cc`, no FFI surface possible without one;
  created 2026-07-29, 4 releases 0.1.0→0.1.3, latest 2026-10-17, 3 617
  downloads). Description verbatim: *"Pure-Rust VP9 video decoder +
  encoder, no C/FFI. Decoder bit-exact against all 315 official libvpx
  conformance vectors."* Repo: `Remade-With-Rust/remade_ffmpeg_rs`
  (README not reachable at guessed paths this session — claim is the
  crate description's, **unproven by us; that is the spike**).
  Ecosystem signal: consumed by `rff-codec-vp9` 0.4.3 (updated
  2026-10-24, deps = `rusty_vp9 ^0.1.3` + rff-core, pure), and forked
  by `vp9-mt` 0.1.1 (2026-10-08, "tile-column threading"). Risks:
  0.1.x, single-org maintainer, no independent conformance report,
  `build.rs`/`unsafe` census not yet done `[VALIDATE]`.
- Runners-up recorded: `vp9dec` 0.1.1 (MIT, 2026-07-24, zero deps,
  "from-scratch, clean-room" — no conformance claim); `rvp9-decoder`
  0.2.0-alpha.4 + `rvp9-core` (BSD-3-Clause, 2026-10-14, "spec-
  conformant; validated against the official VP9 test vectors" —
  alpha, core advertises "per-arch SIMD kernels" ⇒ rav1d-style
  asm/unsafe policy check needed before any use);
  `zvidlib-vp9-decoder` 0.5.0 (2026-10-08, young). `oxideav-vp9`
  0.0.13 is a self-described *"orphan-rebuild scaffold pending
  clean-room re-implementation"* — not a decoder.
- `vp9-parser` 0.3.2 (17 505 dl) is bitstream-parse only — useful
  oracle/accel reference, not decode.

**VP8 — no credible engine:**

- `zvidlib-vp8` 0.5.0 (MIT): "Pure-Rust VP8 decoder and encoder" —
  created **2026-10-07** (17 days old), 109 downloads, `zvidlib-core`
  dep-clean, `simd`/`hardware` optional features; below the young-crate
  adoption bar (feasibility §1.3 precedent, cf. 5.K3-I/wmvkit).
- `oxideav-vp8` 0.2.7 (MIT, 11 676 dl): description itself says
  *"orphan-rebuild scaffold pending clean-room re-implementation"*.
- `videoson-codec-vp8` 0.7.0 is a wrapper "via oxideav-vp8" — inherits
  the scaffold. WebP-family crates (`oxideav-webp`, `vaam-image-webp`,
  `image-webp`) decode VP8 *intra-only still images* — not VP8 video
  (no inter prediction); excluded.

**Landed carriage (unchanged by this card, cited for the record).**
MKV/WebM maps `V_VP8`/`V_VP9` → descriptors `vp8`/`vp9`
(`crates/demux/src/mkv/header.rs:772-773`, test-pinned :866-867);
MP4 maps `vp08`/`vp09` fourccs (`mp4/mod.rs:444-445`); IVF maps
`VP90` → `vp9` (`ivf/mod.rs:212`) with a generated
`vp9_64x48_25fps.ivf` fixture lane. Decode refusal is typed and
honest: the ADR-001 `unsupported-software-decode-pending` vocabulary
(`product-config/src/model.rs:158`, `plugin-api/src/codec.rs:52`).

**Corpus (ffprobe 7.1.5, this session — corrects the card brief).**
Corpus-v4 has exactly **one `.webm` lane: `0126-webm-video.webm` =
VP8-only**, 480×360 yuv420p, 1.64 s, **no audio track** (the brief's
"1 webm (vp9+opus)" is imprecise — that lane is `0078-vfr-av.mkv`,
VP9+Opus, 160×120, 3.61 s, `.mkv` extension). So: one VP8 lane
(webm), one VP9 lane (mkv) — both Matroska/WebM family, both already
demuxed per 5.D1. The validation spike must generate additional
profile coverage (vp9 profile 0/2, 4:2:0/4:2:2, odd dims,
superframes) — C-V-family acquisition or generated lanes.

## Plan (next actions)

1. **S spike (Phase 6 entry): `rusty_vp9` validation.** (a) purity:
   tarball scan, `build.rs` presence, `unsafe` census, license
   (Apache-2.0) — doc 11 policy gates; (b) conformance: libvpx VP9
   test vectors (the 315 the crate claims — reproduce independently);
   (c) differential vs ffmpeg 7.x on the 0078 lane + generated
   profile matrix; (d) wasm32 build check (doc 09 web column).
2. **M admission** (spike green): direct `rusty_vp9 = "=0.1.3"` pin,
   glue ours behind `codec-api` (C5 pattern), `fuzz_codec_vp9`
   target, feature gate `codec-video-vp9` (doc 09 §5 tree-shake).
3. **VP8 recheck (Phase 7–8):** `zvidlib-vp8` maturity + any
   oxideav-vp8 clean-room completion; hw paths recorded in the matrix
   row (WebCodecs `vp8` on web; Linux VA-API; macOS VT exposure
   `[VERIFY]` per matrix note).
4. **WebCodecs ADR** (Lead, doc 09 §12 item 6): when opened, add
   VP8/VP9 rows to its codec list so the web column gains hw VP9
   ahead of (or instead of, on Firefox-Android) the software path.
5. Doc drift for the Lead (NOT edited here, out of scope): doc 04
   §4.1 VP8/VP9 candidate cells + §4.5 summary; feature-matrix
   `evidence` backfill for both rows.

## Corrupt-input behavior / Limits / Results

Not applicable — no decoder is admitted by this record. See the
admission card's record for those sections once `fuzz_codec_vp9`
exists.
