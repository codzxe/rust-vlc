# fuzz/ — the cargo-fuzz campaign (audit row M14, fix card ED-FIX-5, scaffold
W1-12, full Phase-5 sweep card 5.Q1)

Lumen registers one coverage-guided fuzz target per **landed** parser
(doc 04 §8.5: named exactly as in its tables — `fuzz_demux_*`,
`fuzz_codec_*`, `fuzz_sub_*` — plus the `fuzz_playlist_*` family the
playlist parsers earned in Phase 5). Card 5.Q1 swept the registry to
32 targets covering every Phase-5 parser; card 6.C1c added the DASH
MPD target the wave program's 6.C1 `+FB` criterion names —
**33 targets**; issue #14's netfuzz milestone (M2) added the two
network-facing targets — `fuzz_hls_playlist` (the HLS
interpretation stack) and `fuzz_http_headers` (the transport's
response-head grammar) — **35 targets**; the
registry-vs-implementation diff is pinned empty
by `scripts/fuzz_registry_check.sh` (see "The registry gate" below).

## Status: SCAFFOLDED, NOT WIRED — one Lead decision outstanding

```
LEAD-DECISION required (audit ED-FIX-5 wording:
"cargo-fuzz scaffolding + nightly wiring (Lead dep decision)")

WHAT:   opt in to a nightly Rust toolchain for this repository's fuzz
        campaign. cargo-fuzz passes `-Zsanitizer=address` (and friends)
        to rustc, which is nightly-only; every non-fuzz build, test and
        gate stays on the pinned stable (1.99.1 today).
COST:   `rustup toolchain install nightly` + `cargo +nightly install
        cargo-fuzz` on the dev/CI machine that runs campaigns; the
        workspace's default toolchain, lockfiles and gates are
        untouched (fuzz/ is its own workspace root, see below).
DEP:    libfuzzer-sys = "0.4.13" (crates.io sparse-index verified
        2026-10-22; caret pin >=0.4.13 <0.5; links the stock libFuzzer
        runtime built by its `cc` build script — external dep, hence
        Lead approval under AGENTS.md §3.2/C5). Resolution (not build)
        was verified on stable: see `fuzz/Cargo.lock`.
UNTIL:  the decision lands, nothing in the local gate or CI builds
        this directory *in campaign mode*. What DOES run today on
        stable: `cargo check` in fuzz/ (all 33 targets compile —
        verified 2026-10-24, card 5.Q1; re-verified with all 35
        targets 2026-11-09, issue #14 M2) and the in-crate harness
        sweeps (see "The stable-runnable equivalence").
```

## Targets (the registry — 35, one per landed parser or network
surface)

Every target is a thin `libfuzzer_sys::fuzz_target!` shim; the loop
body, bounds and docs live in the in-crate `fuzz` module named in the
harness column (panic-free by construction; bounds documented per
module). The contract of each harness is **survival**: no panic, no
hang, no unbounded allocation — malformed input must surface as a
typed error, a concealed prefix/frame, or a skipped packet (D-3/D-9).

| `[[bin]]` name         | Harness (the real logic)                       | Phase-5 parser |
| ---------------------- | ---------------------------------------------- | -------------- |
| `fuzz_demux_avi`       | `demux::avi::fuzz::fuzz_demux_avi`             | AVI (vertical 1) |
| `fuzz_demux_wav`       | `demux::wav::fuzz::fuzz_demux_wav`             | WAV/RF64 (5.A2) |
| `fuzz_demux_aiff`      | `demux::aiff::fuzz::fuzz_demux_aiff`           | AIFF/AIFC (5.A2) |
| `fuzz_demux_ogg`       | `demux::ogg::fuzz::fuzz_demux_ogg`             | Ogg (5.A5) |
| `fuzz_demux_mkv`       | `demux::mkv::fuzz::fuzz_demux_mkv`             | Matroska/WebM (5.B) |
| `fuzz_demux_mp4`       | `demux::mp4::fuzz::fuzz_demux_mp4`             | MP4/MOV/3GP (5.C) |
| `fuzz_demux_ts`        | `demux::ts::fuzz::fuzz_demux_ts`               | MPEG-TS (5.I1) |
| `fuzz_demux_ps`        | `demux::ps::fuzz::fuzz_demux_ps`               | MPEG-PS (5.I2) |
| `fuzz_demux_es`        | `demux::es::fuzz::fuzz_demux_es`               | raw ES family (5.I3a) |
| `fuzz_demux_ivf`       | `demux::ivf::fuzz::fuzz_demux_ivf`             | IVF (5.I3b) |
| `fuzz_demux_adts`      | `demux::adts::fuzz::fuzz_demux_adts`           | AAC ADTS (5.J1) |
| `fuzz_demux_mp3stream` | `demux::es::fuzz::fuzz_demux_mp3stream`        | raw MP3 stream (5.I3a) |
| `fuzz_demux_flac`      | `codecs::audio::flac::fuzz::fuzz_demux_flac`   | native FLAC container (5.A1) |
| `fuzz_codec_pcm`       | `codecs::audio::pcm::fuzz::fuzz_codec_pcm`     | PCM (vertical 1) |
| `fuzz_codec_flac`      | `codecs::audio::flac::fuzz::fuzz_codec_flac`   | FLAC decoder (5.A1) |
| `fuzz_codec_mp3`       | `codecs::audio::mp3::fuzz::fuzz_mp3_stream`    | MP3 decoder (5.A3) |
| `fuzz_codec_mp2`       | `codecs::audio::mp2::fuzz::fuzz_mp2_stream`    | MP2 decoder (5.J) |
| `fuzz_codec_aac`       | `codecs::audio::aac::fuzz::fuzz_aac_stream`    | AAC-LC decoder (5.J1) |
| `fuzz_codec_alac`      | `codecs::audio::alac::fuzz::fuzz_codec_alac`   | ALAC decoder (5.D) |
| `fuzz_codec_adpcm`     | `codecs::audio::adpcm::fuzz::fuzz_codec_adpcm` | ADPCM flavor zoo (5.J2) |
| `fuzz_codec_vorbis`    | `codecs::audio::vorbis::fuzz::fuzz_codec_vorbis` | Vorbis decoder (5.A5) |
| `fuzz_codec_mjpeg`     | `codecs::video::mjpeg::fuzz::fuzz_codec_mjpeg` | Motion-JPEG (2.B4) |
| `fuzz_codec_av1`       | `codecs::video::av1::fuzz::fuzz_codec_av1`     | AV1 via rav1d (5.E1) |
| `fuzz_codec_h264`      | `codecs::video::h264::fuzz::fuzz_codec_h264`   | H.264 parser (5.F1/F2) |
| `fuzz_codec_hevc`      | `codecs::video::hevc::fuzz::fuzz_codec_hevc`   | HEVC parser (5.G1) |
| `fuzz_sub_srt`         | `subtitles::srt::fuzz::fuzz_sub_srt`           | SubRip (5.M1) |
| `fuzz_sub_ass`         | `subtitles::ass::fuzz::fuzz_sub_ass`           | ASS/SSA (5.N) |
| `fuzz_sub_vtt`         | `subtitles::vtt::fuzz::fuzz_sub_vtt`           | WebVTT (5.M3) |
| `fuzz_playlist_m3u`    | `playlists::fuzz::fuzz_playlist_m3u`           | M3U/M3U8/HLS (5.O) |
| `fuzz_playlist_pls`    | `playlists::fuzz::fuzz_playlist_pls`           | PLS (5.O) |
| `fuzz_playlist_xspf`   | `playlists::fuzz::fuzz_playlist_xspf`          | XSPF (5.O) |
| `fuzz_playlist_cue`    | `playlists::fuzz::fuzz_playlist_cue`           | CUE sheet (5.O3) |
| `fuzz_demux_mpd`       | `streaming::dash::fuzz::fuzz_demux_mpd`        | DASH MPD (6.C1) |
| `fuzz_hls_playlist`    | `playlists::fuzz::fuzz_playlist_m3u` + `netfuzz::fuzz_hls_playlist` (`#[path]` from `crates/streaming/tests/netfuzz/fuzz.rs`) | HLS interpretation stack (issue #14 M2 / 6.F3) |
| `fuzz_http_headers`    | `streaming::mockhttp::fuzz_response_head` + `netfuzz::fuzz_http_headers` (`#[path]` from `crates/streaming/tests/netfuzz/fuzz.rs`) | HTTP response head (issue #14 M2 / 6.F3) |

Not yet registered (no landed parser — doc 04's tables stay the
contract): ASF, RealMedia, m4v, tx3g, CAF demuxers; Opus/AC-3/DTS/
TrueHD/WMA/WavPack/Speex/MPC/MIDI, VP8/VP9/Theora/MPEG-1-2/MPEG-4
ASP/VC-1/raw-video decoders; SubViewer/MicroDVD/MPL2/SAMI/TTML/PGS/
DVB/VobSub/CC subtitles. Each grows a row here the day its parser
lands (the registry gate below fails the build otherwise).

## Seed corpora (committed — doc 04 §8.5 "seed corpus committed")

`fuzz/corpus/<target>/` holds 1–6 small seeds per target (97 files,
1.4 MB total, **every file < 50 KB**): copied from the committed
CC0 fixtures (`tests/fixtures/CORPUS.md` + the crate test fixtures),
deliberate truncations of over-size parents, cross-format seeds
(e.g. TS bytes into `fuzz_demux_ps` — format confusion is fair fuzz
material), and hand-authored text/cookie/skeleton seeds (CC0, ours —
the "generated malformed files" arm of doc 04 §8.5). Formats with no
committed real fixture yet (PS, HEVC, MP2) ride cross-format seeds
today; their corpora grow from the real-media corpus via the env
fallback below.

Corpus resolution order (never absolute-path-only):
1. `$LUMEN_FUZZ_CORPUS/<target>/` if the env var is set (CI machines
   point it at a shared corpus checkout);
2. `fuzz/corpus/<target>/` relative to the repository root (the
   committed default — cargo-fuzz's own convention).

`fuzz/artifacts/`, `fuzz/coverage/` and `fuzz/target/` remain run
products (ignored); the committed corpora are source.

## The registry gate (registry-vs-implementation diff = ∅)

`scripts/fuzz_registry_check.sh` walks four sets and requires them
**identical, and non-empty corpora present for every target**:

1. the `[[bin]]` names in `fuzz/Cargo.toml`;
2. the target files in `fuzz/fuzz_targets/*.rs`;
3. the registry table rows in this README;
4. the in-crate harness each target's shim calls (`rg`-verified:
   the `fn` exists in the crate the path names).

It is wired into `scripts/full_gate.sh` as step 6 (the CI workflow
mirrors full_gate's steps; the one-line CI addition is the Lead's
`fuzz-registry` step — same command). A new parser without its
target/corpus fails the gate: that is doc 04 §8.5's coverage gate,
feature-audit wired.

## Running a campaign (exact commands)

```sh
# once per machine (the nightly dependency — see LEAD-DECISION above)
rustup toolchain install nightly
cargo +nightly install cargo-fuzz

# from the repository root (cargo-fuzz discovers ./fuzz)
cargo +nightly fuzz run fuzz_demux_mkv    -s none   # no sanitizer
cargo +nightly fuzz run fuzz_codec_av1    -s none
cargo +nightly fuzz run fuzz_codec_flac   -s none   # … all 35

# hardened variants (slower, catches more):
cargo +nightly fuzz run fuzz_codec_flac -s address -O   # ASan + release
cargo +nightly fuzz run fuzz_demux_ts  -s address,memory # ASan+MSan

# the 10-minute clean-run CI shape (per doc 04 §8.5), once wired:
cargo +nightly fuzz run fuzz_demux_mp4 -s address -O -- -max_total_time=600
```

Notes:
- `-s none` drops sanitizers, keeping the run usable on machines
  without the sanitizer runtimes; `-s address` is the cargo-fuzz
  default and what the CI campaign should use.
- `-O` builds release; the scaffold's `[profile.release]` keeps
  `overflow-checks = true` there (integer overflow while walking
  attacker bytes is a finding, not noise).
- `-- -max_total_time=600` is the doc-04 "runs 10 minutes clean in
  CI (release mode)" shape. **No such run has happened yet** — the
  nightly wiring is the outstanding LEAD-DECISION above; what has
  actually run on stable is itemized below.

## Why `fuzz/Cargo.toml` is its own workspace root

The root manifest's member globs (`crates/*`, `apps/*`, …) do not
match `fuzz/`, and the root `Cargo.toml` is Lead-owned (AGENTS.md
§3.7). Without the empty `[workspace]` table in `fuzz/Cargo.toml`,
cargo refuses the package outright ("current package believes it's in
a workspace when it's not" — reproduced on stable 1.99.1 while
scaffolding). The table is cargo's own prescribed fix and keeps this
directory merge-conflict-free; the equivalent Lead-side alternative is
`exclude = ["fuzz"]` in the root workspace (either is fine, not both
needed). Consequence: the fuzz package does not inherit the workspace
lint tables — acceptable, as the binaries are macro shims whose logic
already lives in the lint-clean in-crate harnesses.

## The stable-runnable equivalence (real today, no nightly)

What actually ran on stable 1.99.1 for card 5.Q1 (timestamped
2026-10-24, this worktree):

- `cargo check` in `fuzz/` — **all 33 targets compile** (the
  libfuzzer-sys dep resolves and builds its build script on stable;
  the `-Zsanitizer` flags are cargo-fuzz's, not the manifest's).
  Re-verified with **all 35 targets** on 2026-11-09 (issue #14 M2,
  this worktree).
- `cargo test -p demux -p codecs -p subtitles -p playlists --lib` —
  the in-crate harness sweeps (29 new deterministic hostile-input
  tests: empty/marker/all-0x00/all-0xFF/sync-flood inputs per
  parser) — all pass.
- The pre-existing seeded survival loops (below) keep running in the
  workspace gate.

The deterministic seeded-RNG loop bodies, as ordinary integration
tests:

| Test file                                     | Loops                                              |
| --------------------------------------------- | -------------------------------------------------- |
| `crates/codecs/tests/fuzz_loop_stable.rs`     | `fuzz_codec_pcm` + `fuzz_codec_mjpeg`, N each      |
| `crates/demux/tests/fuzz_loop_stable.rs`      | `fuzz_demux_avi`, N passes                         |

Each loop feeds N seeded pseudo-random inputs (inline xorshift64* —
no new dependencies) through a mix of pure-random bytes, magic-prefixed
random bytes, and seeded mutations/truncations/splices of **real
fixture bytes** (writer ≠ parser author, D-5), with bounded input
sizes and a fixed iteration count. Long-run variants (10×–100× N) are
`#[ignore]`d; run them with:

```sh
cargo test -p codecs --test fuzz_loop_stable -- --ignored --nocapture
cargo test -p demux  --test fuzz_loop_stable -- --ignored --nocapture
```

These loops are the *deterministic* layer; the deliberately
non-deterministic randomized real-media corpus (D-11, `LUMEN_CORPUS`
env trio) is the third layer. The libFuzzer campaign itself is the
fourth — owed the LEAD-DECISION wiring above.
