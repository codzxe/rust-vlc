# Video record — MPEG-4 Part 2 / ASP (DivX/Xvid) decode

Decision record (card 5.H2-I, doc 04 §4.1 row "MPEG-4 Part 2 / ASP (DivX/Xvid)"
+ §8 row 8; matrix row `codec-video-mpeg4-asp`). No code changed. Front-matter
keys per doc 04 §6:

- id = `codec-video-mpeg4-asp` (matches `docs/feature-matrix.toml:526`, which
  carries `status = not_started` ×4 with an empty `evidence` — this record is
  that evidence)
- status = **blocked for v1 decode** (typed refusal; matrix `not_started` ×4
  unchanged by this card)
- phase = 6 (admission path below; doc 04 had `later`, §8 row 8)
- fuzz_target = `fuzz_codec_mpeg4asp` (planned name, doc 04 §4.1 row — does
  not exist until a decoder is admitted)
- crates = [] — **none admitted**; `oxideav-mpeg4video` 0.1.8 is the recorded
  candidate (purity + license pre-checks PASSED this session, below)
- self_built = []
- last_audit = 2026-10-24 (this card's evidence pass; prior: doc 04 §4.1,
  2026-10-19)

## Verdict (the card's question: pure-Rust availability / self-build split / patent)

**BLOCKED-with-record for v1 — reason AMENDED; primary unblock =
Phase-6 validate-then-admit `oxideav-mpeg4video`; self-build L–XL DEMOTED to
last-resort fallback (split proposal recorded below per the card's ask).**

1. **Pure-Rust availability:** doc 04's "none" (2026-10-19) is upheld at the
   exact-name level — but is **stale as a global claim**: a credible MIT
   clean-room crate has been published since 2026-04-17 (`oxideav-mpeg4video`;
   0.1.6 live since 2026-07-09) and was missed by the 2026-10-19 survey. Same
   class of miss as `ruopus` (5.K2-I). The blocked reason becomes "no
   *validated* pure-Rust engine", not "no engine".
2. **Self-build L–XL: demoted.** Duplicating a spec-cited, conformance-pinned
   MIT clean-room decoder is the Opus precedent (audio-opus.md verdict item 2)
   — needs a validation failure first. Split proposal kept as the fallback
   (§ Plan item 4).
3. **Not defer-Phase-7.** Unlike Opus (README-claimed conformance),
   oxideav-mpeg4video ships 24 committed black-box fixtures with SHA-256
   provenance, root-caused deviation write-ups, cargo-fuzz targets, and
   Miri-by-default CI — and unlike H.264/HEVC there is **no credible hardware
   path** (matrix note: in-box Media Foundation on Windows only; no VA-API
   driver class exposes ASP — feature-matrix.toml:536, Lead-seeded, not
   re-verified here). Software decode is the only route to parity, and corpus
   demand is real: **18 of 143 corpus-v4 lanes (12.6 %)**.

## Evidence (all fetched this session, 2026-10-24, unless dated)

**crates.io exact names** (API): `mpeg4`, `mpeg4video`, `mpeg-4`, `divx`,
`xvid`, `libxvid` → all NOT_FOUND; search `mpeg4asp` → 0 results. No Xvid/
DivX C-binding crate exists even to forbid. Confirms doc 04 §4.1 row.

**The candidate — `oxideav-mpeg4video` 0.1.8** (github.com/OxideAV/
oxideav-mpeg4video; MIT, "Copyright (c) 2026 Karpelès Lab Inc."; created
2026-04-17; 13 versions, **9 yanked** (0.0.3–0.1.5); 2 201 downloads / 1 231
recent; updated 2026-09-12; sole dependency `oxideav-core ^0.1`):

- **Purity pre-checks PASSED (doc 04 §1.2 method, done this card):** tarball
  82 files — 71 Rust sources + manifests/docs/CI only; **zero** `.c/.cpp/.h/
  .S/.asm`; **no `build.rs`**; `unsafe` census: **0 occurrences in 67 377
  LOC** (src/). Transitive `oxideav-core` 0.1.41 (updated 2026-10-22, 190 k
  downloads): 33 `.rs`, no C, no build.rs, MIT, **11 `unsafe` occurrences in
  13 978 LOC** (Rust-side unsafe — doc 11 policy call, recorded). MSRV 1.95 /
  edition 2024 — our 1.99.1 pin satisfies it.
- **Claimed decode surface (README's claims, NOT ours — the spike's job):**
  end-to-end elementary-stream decode; I/P/B/S(GMC) VOP walks; QPel; GMC
  1–3 warp points; OBMC; interlaced tools (field DCT/MC, §7.7.2.2 interlaced
  B + direct); data partitioning; RVLC both directions; short header
  (H.263-compatible) both ways; SA-DCT + ΔDC-SA-DCT; `vop_coded == 0` NVOP
  copies (the packed-bitstream prerequisite); pixel-count DoS cap; packet
  decoder with extradata priming, `reset()` (seek), FourCCs XVID/DIVX/DX50/
  FMP4/MP4V/M4S2 + MP4 OTI 0x20. Encoder included (out of our scope).
- **Conformance evidence (their record):** black-box oracle = **ffmpeg 8.1
  binary used opaquely** ("no external implementation source was consulted" —
  fixtures provenance + SHA-256 in tests/fixtures/NOTES.md, offline-produced);
  24 fixtures: 12 bit-exact, 4 near-tie ±1, 3 bounded + root-caused; an
  opt-in `ecosystem-compat` mode reproducing 3 documented spec-vs-ecosystem
  divergences (§7.7.2.2 interlaced-direct, §7.4.4.5 mismatch, §7.8.7.3 GMC
  AMV rounding). cargo-fuzz targets (short_header, stream_decode, …) with
  240/480 s clean runs claimed; CI = thin shim over the OxideAV org workflow,
  **Miri by default**.
- **Their own gap list:** S(GMC)+data-partitioned decodes but the deployed
  reference desyncs (combination absent in the wild); interlaced+DP/RVLC is
  *not codable* (Annex G Table G.2 note e — typed rejection, a spec ruling);
  I-VOP error-concealment policy open (P-VOP wired); **no integer IDCT** —
  the f64 Annex A.1 transform only (perf risk for our real-time budget:
  spike item (c)).
- **Risk profile:** 0.1.x, yank-heavy, single-org maintainer, ~2 k downloads,
  all conformance claims self-asserted. Adoption bar (feasibility §1.3
  young-crate precedent): **verify-then-adopt, never adopt-and-trust**.

**Landed state in this tree (no code changed by this card):** MKV maps
`V_MPEG4/ISO/ASP` → descriptor `"mpeg4"` (test-pinned,
crates/demux/src/mkv/header.rs:754); MP4 maps `mp4v` → `"mpeg4video"`
(crates/demux/src/mp4/mod.rs:442) — **descriptor vocabulary mismatch** to
unify at admission; AVI maps only mjpeg+pcm (StreamFormat,
crates/demux/src/avi/header.rs:217) — the ASP fourccs are **not yet in the
AVI vocabulary** (admission prerequisite). No mpeg4 decoder is registered in
`crates/codecs` → typed refusal today (stream-granularity: sibling audio
tracks keep playing).

## Corpus (corpus-v4, 143 files; ffprobe 7.1.5 this session)

**18 MPEG-4 Part 2 lanes** — 11 AVI (10 `FMP4` + 1 `xvid`), 4 MKV
(`V_MPEG4/ISO/ASP`), 3 MP4 (`mp4v`): 7 Advanced Simple Profile with
B-frames (incl. `0117-packed-bs.avi` — the xvid packed-bitstream lane,
160×120) + 4 Simple Profile in AVI; dims span 1×1 (`0107-odd-dims.avi`) to
800×600; `0101-maxtracks-avi.avi` carries SP among its track bomb. QPel/GMC
presence in the corpus lanes: not probed this session (spike item (b)
generates them explicitly).

## Patent note (doc 11 §8.3 already registers the family — cited, not re-drafted)

- Source license ≠ patent license (doc 11 §8.3): adopting an MIT clean-room
  crate changes **nothing** vs self-build — distributing an implementing
  decoder is the trigger either way.
- Doc 11 §8.3 row "MPEG-2 / MPEG-4 ASP": "MPEG-2 pool largely expired
  (2019+) `[VERIFY]`; **ASP pool status varies**"; exposure "future
  self-build phases (6+)"; mitigation "re-verify at build time". Every pool
  line there carries the `[VERIFY: counsel]` banner — this record adds no
  legal claim beyond it. No royalty-free pledge exists (unlike Opus/VP8/
  VP9/AV1 rows).
- **Exposure amendment this verdict forces:** the validate-then-admit path
  would ship ASP decode in a Phase-6 binary — the **ADR-010 shipping-matrix
  gate becomes binding at admission time** (feature-gated build, regional
  variance, source-build path — the same options as the AAC row's "flag
  before first public binary release").

## Plan (next actions, effort per doc 04 scale)

1. **S — validation spike (Phase 6 opener, scratch harness, NOT the
   workspace graph):** (a) differential decode vs ffmpeg 7.1.5 on the 18
   corpus lanes (YUV hashes; spec mode + ecosystem-compat mode); (b)
   generated feature-matrix lanes — qpel / GMC / interlaced / packed-BS via
   the ffmpeg `mpeg4` encoder; (c) **perf smoke of the f64-IDCT path**
   against the real-time budget; (d) purity pre-checks re-run on the
   then-current version (this card's pass recorded above).
2. **M — admission (Lead pre-approval, C5 pattern):** dep
   `oxideav-mpeg4video = "=0.1.x"` (+ `oxideav-core` transitive); glue OURS:
   descriptor unification (`"mpeg4"` vs `"mpeg4video"`), AVI fourcc →
   descriptor + BITMAPINFOHEADER extradata priming (packed-BS NVOP handling),
   ecosystem-compat default decision (it matches the ffmpeg/VLC oracle our
   differential uses — propose ON, recorded as a choice), seek/reset, §12 DoS
   cap audit; `fuzz_codec_mpeg4asp` re-pointed from the self-build plan to
   admission fuzzing; **ADR-010 patent gate before the first public binary**;
   record update to `implemented` with evidence.
3. **Monitor:** re-query crates.io each wave (0.1.x churn; the 9-yank history
   argues for a pin). Sibling `oxideav-mpeg12video` 0.0.13 already noted on
   5.H1's matrix row — same spike can carry both.
4. **FALLBACK — L–XL clean-room self-build, split proposal (only if 1
   fails), per the card's requested shape:**
   - `6.?a` (M): bitstream parser — start-code scan, §6.2 VOL/VOS/VOP
     headers, video packets/resync, short-header detect; probe/profile
     surface (the always-valuable piece, mirrors the H.264 parser posture).
   - `6.?b` (L): texture decode — VLC/RVLC tables, inverse quant methods
     1/2, IDCT (integer, Annex A accuracy), SA-DCT.
   - `6.?c` (L): motion compensation — half/qpel interpolation, field MC,
     OBMC, GMC warp, B-VOP direct mode.
   - `6.?d` (M): reference chain, display reorder, TRB/TRD time model,
     error-concealment policy, optional post-filter (deblock/dering — not
     spec-mandated for ASP).
5. Until any of the above lands: `mpeg4 = not_started/blocked` in the matrix
   (truthful-reporting rule), typed refusal naming the missing decoder.

## Matrix-row proposal (for the Lead; feature-matrix.toml untouched by this
investigation card)

- `codec-video-mpeg4-asp`: status `not_started` ×4 unchanged; `evidence` ←
  this record; notes amend to: candidate `oxideav-mpeg4video` 0.1.8
  purity-verified 2026-10-24 (MIT, 0 unsafe / 67 k LOC, no build.rs, no C;
  transitive `oxideav-core` 11 unsafe / 14 k LOC) — validate-then-admit
  Phase 6, spike items (a)–(d); hw posture unchanged (MF Windows-only, no
  VA-API class — Lead-seeded note); ADR-010 patent gate before first public
  binary; self-build L–XL split proposal recorded as fallback.

## Doc drift for the Lead (out of this card's scope to edit)

- doc 04 §4.1 ASP row "none" → stale since 2026-07-09; §4.5 "video codec
  hole" summary and §8 row 8 ("L–XL self-builds queued Phase 6+") → now
  validate-then-admit primary, self-build fallback.
- doc 11 §8.3 MPEG-2/ASP row exposure: "future self-build phases (6+)" →
  "Phase-6 admission path (oxideav-mpeg4video) or self-build; ADR-010 gate
  before first public binary".
- feasibility `video-codecs.md` carries the same stale "none".
