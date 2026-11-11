# Security sweep — parsers (issue #52, card W23-F sweep 1 of 4)

Task ID `7.w23f` (agent `w23-f`, branch `w23/f`). Scope: every
parser that consumes untrusted bytes — `crates/demux/**` (ADTS, AIFF,
ASF, AVI, ES/AnnexB/MP3, IVF, MKV, MP4, Ogg, PS, TS, WAV),
`crates/codecs/**` (aac, adpcm, alac, flac, mp2, mp3, pcm, voris,
av1/rav1d, dnx, h264, hevc, mjpeg, mpeg12, vp9),
`crates/playlists/**` (m3u, pls, xspf, cue, hls, mpd),
`crates/subtitles/**` (ass, cc, dvb, pgs, srt, vobsub, vtt),
`crates/media-probe/**`, `crates/input/**`, `crates/streaming/**`
(mpd/dash, hls, http header grammar).

Threat model: a malicious media file (or hostile HTTP response, for
the streaming half) must never abort the process (panic), hang, or
allocate without a ceiling. Typed `EngineError`s, damage ledgers and
bounded recovery are the *expected* outcomes on garbage.

## 1. Static audit — panic-call inventory

Method: `rg 'unreachable!|\.unwrap\(\)|\.expect\(|panic!\('` over all
seven parser crates' `src/`, classifying every hit as test-module
(each crate's `#[cfg(test)] mod tests`) or live code. Cross-checked
against the workspace lint policy (`unwrap_used`/`expect_used`/
`panic`/`unreachable`-shaped lints `deny` in the root `Cargo.toml`
`[workspace.lints]`, inherited via `[lints] workspace = true` —
CI-enforced by `cargo clippy -D warnings`).

**Result: zero panic-capable calls in non-test parser code.** All
remaining hits sit inside `#[cfg(test)]` modules (verified: `input`'s
live-looking hits at `buffered.rs:461` etc. are below the file's
`#[cfg(test)]` at line 419; `contract.rs` hits are under its
line-537 gate).

### 1.1 Fixes landed this sweep (the prior-session carry, verified + completed)

The inherited uncommitted work removed **57 panic-capable sites**
(`unwrap_or_else(|| unreachable!(..))` and `match … None =>
unreachable!(..)` shapes) across 14 files; each None/Err arm now
returns a typed `EngineError` or skips the dead branch. Breakdown
(site counts from `git diff` of the fix commit):

| File | Sites | Residue class |
|---|---|---|
| `crates/demux/src/ogg/mod.rs` | 21 | insert-only stream-map lookups (`streams.get(..).unwrap()` → `if let Some`, skipped not panicked); Speex header slice; `mint_track` → `Result` |
| `crates/demux/src/es/mp3.rs` | 8 | `sticky`/`pending`/`track`/`head` state-machine arms → `not_started` / EOS |
| `crates/demux/src/ogg/page.rs` | 5 | `try_into().unwrap()` on length-checked slices → typed `bad_capture` / verify-fail |
| `crates/codecs/src/video/av1/decoder.rs` | 6 | engine-handle absence → typed `BadState` (`engine_state_error`); `Rav1dHandle::get` → `Option` |
| `crates/demux/src/ivf/mod.rs` | 5 | track-absent arms → `not_started`; const `TimeBase` literals (`1/1000`) |
| `crates/demux/src/adts/mod.rs` | 5 | `descriptor()` → `Result`; const `TimeBase` literals |
| `crates/demux/src/es/annexb.rs` | 4 | `descriptor()`/`mint_track` → `Result`; track arm → `not_started` |
| `crates/demux/src/ogg/error.rs` | +1 ctor | new `bad_descriptor()` typed error (+ registry test row) |
| `crates/demux/src/adts/error.rs` | +1 ctor | new `bad_descriptor()` typed error (+ registry test row) |
| `crates/demux/src/mkv/cluster.rs` | 1 | lacing match `_` arm → typed `bad_structure` (2-bit field: unreachable by construction) |
| `crates/demux/src/ts/mod.rs` | 1 | const `TimeBase { 1, 90_000 }` literal (DVB-builder precedent) |
| `crates/codecs/src/audio/alac/decoder.rs` | 1 | fresh-slot `get_mut` None → typed `BadState` |

Public-API changes (compile-enforced, call sites updated):
`adts::AdtsHeader::descriptor` / `adts::AdifInfo::descriptor` /
`es::mp3::descriptor` now return `Result<CodecDescriptor,
EngineError>` — the static-literal `CodecDescriptor::new` unwrap is
gone. Test call sites updated (`adts/tests.rs`, `es/tests.rs`), and
both new error constructors gained registry-table test rows.

**Residue-class honesty:** these arms are unreachable *by
construction* (documented in code at each site: insert-only maps,
length-checked slices, 2-bit saturating fields, `configure()`-opened
engines). No fuzz corpus or unit test can drive them; the regression
evidence is therefore (a) the full conformance suites on the changed
code paths, (b) the API-signature tests above, and (c) the dynamic
campaign in §2 — not per-arm trigger tests, which are impossible
without breaking the invariants themselves.

### 1.2 Indexing and arithmetic posture (audit, no fixes required)

`clippy::indexing_slicing` (warning-level pass, lib targets):
demux 461, codecs 540, subtitles 378, streaming 66, media-probe 40,
playlists 18, input 6 — ~1 509 sites, all in the house style
`if data.len() < N { return Err(..) }` before `data[i]`. Spot-audit
of the arithmetic-indexing patterns (e.g. `mkv/cluster.rs`'s inline
vint-width walk) found the guards sound — the `parse_vint_id` pre-pass
rejects `first == 0` and widths > 4 before the inline walk can slice
past `data.len()`. Full per-site indexing audit + release-profile
overflow posture (`overflow-checks` is on in dev/test, **wraps
silently in release**) remain open items → recorded for **8.A1**
(deep review) in §4.

## 2. Dynamic audit — targeted corpus + mutation runs

`cargo-fuzz` the binary is **not installed** and the campaign's
nightly wiring is an outstanding Lead decision (`fuzz/README.md`).
What was possible and run instead (recorded honestly):

- The `fuzz/` workspace **builds on stable** (verified:
  `cargo build` in `fuzz/` — `libfuzzer-sys` 0.4.x compiled with the
  stable 1.99.1 toolchain; no `-Zsanitizer` flags needed for a plain
  non-ASAN build). Its 35 target binaries were built in the shared
  dev-profile target dir — **overflow checks ON**, so any arithmetic
  overflow panics (the sweep's contract).
- Each of the 35 targets ran its committed seed corpus
  (`fuzz/corpus/<target>/`, registry-gated non-empty by
  `scripts/fuzz_registry_check.sh`) plus an 11-second random-mutation
  loop (`-max_total_time=10`).

**ACTUAL results — 35/35 targets, exit 0, zero panics/hangs,
~165.7 M total executions, peak RSS 27–38 Mb per target (bounded
memory, MP §12).** Per-target run counts are in the sweep's worklog
entry; headline rows: `fuzz_codec_hevc` 9.95 M, `fuzz_playlist_pls`
9.78 M, `fuzz_codec_h264` 9.07 M, `fuzz_demux_flac` 8.50 M,
`fuzz_sub_vtt` 8.33 M.

Existing suite evidence re-verified on the fixed tree (this sweep,
actual): demux 454+8 doc-tests, codecs 461, playlists 134,
subtitles 335, media-probe 92, input 46, streaming 242 — all green,
including each crate's committed mutated-corpus survival tests and
deterministic bit-flip/truncation loops (the D-11 dirty-world
mandate).

## 3. Verdict

No panic-on-malicious-input finding survived the sweep: static
inventory empty (lint-enforced), 57 legacy residue sites converted to
typed errors, 165.7 M mutation executions without an abort, memory
bounded. Parsers: **clean at first-pass depth**.

## 4. Remaining for 8.A1 (deep review)

1. Coverage-guided fuzzing (needs the Lead's nightly/`libFuzzer`
   instrumentation decision — this sweep ran uninstrumented
   random mutation, which is shallower).
2. Per-site review of the ~1 509 length-guarded indexing sites
   (off-by-one hunt) — style is uniform, spot-audits clean.
3. Release-profile arithmetic-overflow posture: `[PROPOSAL]` enable
   `overflow-checks = true` in the release profile (cost: small
   runtime penalty) or audit hot arithmetic — Lead decision.
4. Wire a stable-mode corpus/mutation smoke (the §2 method) into CI
   as a cheap complement to the registry gate — `[PROPOSAL]`
   (touches `fuzz/` + CI, both Lead-owned).
