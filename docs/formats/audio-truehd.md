# Audio record — TrueHD / MLP (decode)

Records-consolidation entry (card 5.REC-2; evidence from card 5.K1 —
the PENDING-UNBLOCK hardening investigation; dependency record
`docs/dependencies/records/truehd-0.7.2.md`). Front-matter keys per
doc 04 §6:

- id = `codec-audio-truehd-mlp` (matrix row carries `not_started` +
  the 5.K1 hardening note — consistent: nothing shipped)
- status = **blocked (pending-unblock)** — decode stays
  typed-unsupported in v1 exactly as before; the blocker is
  root-caused with a proven unblock path (below)
- phase = 6 (unblock decision at the R-10/R-43 gates)
- fuzz_target = `fuzz_codec_truehd` (planned name — nothing to fuzz
  until admission)
- crates = [] — **none admitted** (truehd 0.7.2 purity PASSes but is
  NOT in the workspace graph; see blocker)
- self_built = []
- last_audit = 2026-10-24 (card 5.K1 evidence; this consolidation
  2026-10-26)

## The blocker (root-caused, measured)

- The library crate is `truehd` 0.7.2 (`truehdd` 0.6.2 is the CLI).
  Purity: PASS — 28 `.rs`, build=false, zero native/blobs, Apache-2.0,
  4 lock nodes (truehd/anyhow/bitstream-io/no_std_io2; optional
  hmac/sha2 not enabled; 2 unsafe sites, both in the out-of-scope
  Atmos OAMD path).
- **rustc 1.99.1 is OOM-killed compiling it** (global OOM, dmesg
  captured: anon-rss ~3.4–3.6 GB on the 4 GB/no-swap sandbox) across
  4 full-crate configurations (check/build, debug/release,
  incremental on/off). Bisection: `utils/bitstream_io.rs` alone OOMs;
  removing the three `define_huffman_tree!` invocations compiles
  clean; a ~20-line minimal repro (one macro invocation) OOMs
  identically; hand-expanded read-only trees compile fine ⇒ root
  cause = bitstream-io 4.10.0's `ToBits`/write-tree macro expansion.
  No dodge available: 4.10.0 is the newest bitstream-io; truehd
  0.6.0/0.7.1 carry the same macro.
- Per the mandate: NO Cargo.toml edit, no module, no tests shipped.

## The proof (scratch differential — decode quality is NOT the blocker)

4 ffmpeg 7.1.5-authored streams through the /tmp-patched tarball
(the 3 trees replaced by ~60 hand-written FromBits impls; the decoder
never uses ToBits), full-sample compare vs `ffmpeg -map 0:a -f s32le`:

| fixture | pres | samples×ch | verdict |
|---|---|---|---|
| truehd stereo 44.1k sine 5s | 0 | 220 500×2 | bit-exact (s32le `72aa2a86…` + s24le `3d092c4c…`) |
| truehd 5.1(side) 48k pink 5s | 1 (native 6ch) | 240 000×6 | bit-exact (pres 0 = author-embedded SILENT stereo downmix) |
| truehd 5.0(side) 96k white 4s | 1 | 384 000×5 | bit-exact |
| MLP (DVD-Audio legacy) stereo 44.1k | 0 | 132 320×2 | bit-exact |

4 065 640 i32 samples compared, 0 mismatches. PCM conventions pinned:
engine emits 24-bit-magnitude i32 (sample-major); ffmpeg s32le =
value<<8 — pure normalization, byte-equal after the shift. Honest
tail gap: at TrueHD (not MLP) EOS the engine emits 20 extra
inter-channel samples of trailing zeros that ffmpeg trims — the
future adapter owns trailing trim. Fixtures are deterministic
(ffmpeg commands + sha256 in the 5.K1 worklog entry).

## Downmix policy (recorded for the admission card)

**NO silent downmix, ever.** Presentations are author content (pres 0
typically the authored Lt/Rt downmix; measured silent on the 5.1
fixture while the native 6ch lives at pres 1). The decoder picks the
presentation matching the negotiated channel config and decodes
native channels; any downmix is the audio-engine mixer's job;
`channel_labels` + `substream_info_changed` must surface for honest
renegotiation; Atmos stays out of v1.

## Honest gaps

- 8/16-channel + Atmos presentations untested: ffmpeg's truehd
  encoder caps at 5.1(side) and Dolby reference streams are
  non-redistributable (C-V6, corpus-plan.md:394) — generated+ffmpeg
  IS the honest reference set for this card.
- Older-rustc behavior (no disk for a second toolchain) and ≥8 GB
  host retest: `[VERIFY]` flagged.
- Carriage today: MKV/PS map truehd descriptors; decode refuses
  typed (stream-granularity policy).

## Next step (the unblock decision — Lead, R-10/R-43)

Upstream bitstream-io fix, OR the Lead-approved ~60-line vendor patch
(already bit-exact in scratch) — then admission (C5 pattern, glue
ours, fuzz_codec_truehd, trailing-trim adapter duty) and this record
flips to `implemented` with the table above as its baseline.
