# fuzz/ — the cargo-fuzz campaign (audit row M14, fix card ED-FIX-5, delivered by W1-12)

Lumen registers three coverage-guided fuzz targets (doc 04 §8.5: one
per parser, named exactly as in its tables). Until this scaffold, the
target files existed but no `fuzz/Cargo.toml` did — they were never
compiled, never run (audit `docs/audits/2026-10-22-directives-audit.md`
row M14, "all 3 cargo-fuzz targets are dead files").

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
        this directory. The commands below are the exact wiring.
```

## Targets

| `[[bin]]` name         | Harness (the real logic)                 | Exercises                                                                 |
| ---------------------- | ---------------------------------------- | ------------------------------------------------------------------------- |
| `fuzz_codec_pcm`       | `codecs::audio::pcm::fuzz::fuzz_codec_pcm`  | PCM decode both surfaces: free `decode_pcm` + codec-api Decoder cycle, over a config spread |
| `fuzz_demux_avi`       | `demux::avi::fuzz::fuzz_demux_avi`          | AVI construct → `start` → full event drain (budgeted) → 4 data-derived seek probes + drains |
| `fuzz_codec_mjpeg`     | `codecs::video::mjpeg::fuzz::fuzz_codec_mjpeg` | MJPEG decode both surfaces: free `decode_image` + codec-api Decoder cycle |

The target files are thin `libfuzzer_sys::fuzz_target!` shims; every
loop body, bound and doc lives in the in-crate `fuzz` modules listed
above (panic-free by construction; bounds documented per module). The
contract of each harness is **survival**: no panic, no hang, no
unbounded allocation — malformed input must surface as a typed error,
a concealed prefix/frame, or a skipped packet (D-3/D-9).

## Running a campaign (exact commands)

```sh
# once per machine (the nightly dependency — see LEAD-DECISION above)
rustup toolchain install nightly
cargo +nightly install cargo-fuzz

# from the repository root (cargo-fuzz discovers ./fuzz)
cargo +nightly fuzz run fuzz_codec_pcm   -s none   # no sanitizer
cargo +nightly fuzz run fuzz_demux_avi    -s none
cargo +nightly fuzz run fuzz_codec_mjpeg  -s none

# hardened variants (slower, catches more):
cargo +nightly fuzz run fuzz_codec_pcm  -s address -O    # ASan + release
cargo +nightly fuzz run fuzz_codec_pcm  -s address,memory        # ASan+MSan
```

Notes:
- `-s none` drops sanitizers, keeping the run usable on machines
  without the sanitizer runtimes; `-s address` is the cargo-fuzz
  default and is what a CI campaign should use.
- `-O` builds the fuzz targets in release mode; the scaffold's
  `[profile.release]` keeps `overflow-checks = true` there (integer
  overflow while walking attacker bytes is a finding, not noise).
- Seeds: drop files into `fuzz/corpus/<target>/` (cargo-fuzz's
  convention; directory is created on first run). Good first seeds,
  per the targets' header comments: the committed PCM chunks and JPEG
  frames from the class-A fixtures (CC0, `tests/fixtures/CORPUS.md`)
  plus systematic truncations, and the class-B derivative containers.
- Artifacts (`fuzz/artifacts/`, `fuzz/corpus/`, `fuzz/coverage/`,
  `fuzz/target/`) are run products, not source; add them to ignores if
  the campaign becomes routine.

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

The same three harness loop bodies run under a deterministic seeded
RNG on the pinned stable toolchain, as ordinary integration tests:

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
non-deterministic randomized real-media corpus (D-11,
`tools/corpus/`) remains the project's other adversarial leg.
