# Subtitle record — SubRip / SRT (parse + render path)

Status: **implemented** (parse + cue→bitmap render path, card 5.M1).
Feature-matrix row `subtitle-srt` was `not_started` ×4 with empty
evidence at wave-9 open — stale vs the landed parser; backfilled by
the 5.Q2-V records audit (this file is that evidence). **This record
is also the record for the sibling row `subtitle-encoding-detection`**
(partial ×3 desktop / not_started web, `docs/feature-matrix.toml`):
that feature is implemented BY this module's `encoding.rs` — the
"Encoding" scope below is its landed subset, and the row's honest
limits (BOM-less UTF-16 not sniffed; no subsdec-class charset
heuristics) are this record's "Honest gaps". Cross-linked by the
8.G1 records sweep (2026-11-10). Front-matter keys per doc 04 §6:

- id = `subtitle-srt` + `subtitle-encoding-detection` (two
  feature-matrix rows, `docs/feature-matrix.toml` — the sibling row
  is carried by this record's Encoding scope, see header)
- status = implemented (parse + render path); platform-independent
  code, evidence Linux-only
- phase = 5 (landed, wave 6)
- fuzz_target = `fuzz_sub_srt` (in-crate `srt` fuzz harness, doc 04
  §8.5 registry name; the `fuzz/fuzz_targets/fuzz_sub_srt.rs` shim +
  `fuzz/Cargo.toml` wiring is Lead-owned, the mjpeg/mp2/flac
  precedent — cargo-fuzz absent from the sandbox, honestly recorded
  by 5.Q1)
- crates = [] — zero external deps
- self_built = [`crates/subtitles/src/srt/**` (encoding/parser/tags/
  damage)], render via the 5.L1 compositing contract
- last_audit = 2026-10-25 (5.Q2-V; engine evidence 5.M1); record
  cross-linked for `subtitle-encoding-detection` by the 8.G1 sweep
  (2026-11-10, no engine claims touched)

## Scope landed (5.M1)

- **Encoding** (`encoding.rs`): BOM matrix UTF-8 / UTF-16LE /
  UTF-16BE; BOM-less VALID UTF-8 wins losslessly; BOM-less INVALID
  UTF-8 falls back to Windows-1252 (issue #17: legacy-SRT 8-bit
  fallback, ffmpeg `sub_charenc` behaviour) with typed damage —
  one fallback event + U+FFFD for the five undefined bytes;
  unpaired-surrogate + odd-trailing-byte degradation; CRLF/CR/LF
  normalization. BOM-less UTF-16 is deliberately NOT sniffed
  (documented honesty: degrades to garbage-block damage, never
  misdetects).
- **Parse** (`parser.rs`): single-pass block walk; index line
  optional/tolerant (synthetic ids); timing tolerances pinned to
  ffmpeg/VLC `sscanf` semantics — `.`-vs-`,` ms, glued `-->`, 1–3
  digit ms FIELD semantics (verified against ffmpeg 7.1.5 output),
  rolled hours (>23 h) + minutes, i64-µs saturation; `X1:`/`Y1:`
  payloads junk-ignored-with-damage; garbage blocks skipped to next
  boundary; run-together blocks RESYNC at next valid timing line
  (iterative — stack bounded, D-2).
- **Tags** (`tags.rs`): `<b>/<i>/<u>/<font color=#rgb|#rrggbb|
  #rrggbbaa>` + named/numeric entities → CueText Plain fast-path /
  Styled runs; unknown tags/entities/unterminated `<` degrade with
  damage (doc 06 §7.8).
- **Damage ledger** (`damage.rs`): capped 256 events, overflow
  COUNTED (`dropped_damage_events`), 18 typed kinds.
- **Render path**: model `Positioning` → compositing `CueLayout` via
  exact rationals; ≤ 4 Mpx bitmap ceiling reuses `place_cue`'s
  pixel-exact math; fonts stay renderer-side (doc 04 §5).

## Corpus / differential evidence (5.M1)

| Sample | Origin | Result |
|---|---|---|
| out1 styled tags | ffmpeg 7.1.5 `-c:s srt` from hand-written WebVTT truth | counts/times(µs)/text/tag-for-tag EQUAL (6/6 asserts) |
| out2 plain + fractional times | ffmpeg 7.1.5 | EQUAL |
| out3 multiline + hour-24 rollover | ffmpeg 7.1.5 | EQUAL |
| UTF-16LE/BE variants | iconv-derived from the above | EQUAL (both endians) |

- Pinned honestly: ffmpeg's VTT→SRT path DROPS `<font color>`
  wrappers (kept text) — asserted as real-world behavior; a
  hand-authored sample pins OUR color mapping.
- Fuzz-lite: 37 committed deterministic mutants (4.7 KB « 200 KB
  budget) + 2000-iteration seeded LCG mutation loop + EVERY-prefix
  truncation sweep of all 4 canonical parents — zero panics, ledger
  cap held, survived-cue intervals valid.
- Gates at card: `cargo test -p subtitles --all-features` 105/0
  (incl. 6 canonical-differential + 4 fuzz-survival); fmt + clippy
  `-D warnings` clean. Current tree (5.N2 completion session,
  re-verified 5.Q2-V): 230/0 with `srt_canonical_differential` 6 +
  `srt_fuzz_survival` 4 green.

## Honest gaps (D-10)

- `X1:`/`Y1:` pixel-coordinate positioning is a recorded damage kind
  (R4 owed) until the cue model grows pixel positioning.
- The plugin-api `SubtitleParser` seam is NOT wired (boundaries.toml
  lists it joining 5.M1; `srt::parse(bytes) -> SrtDocument` is the
  stable entry point).
- No libFuzzer campaign (Lead-owned wiring); no corpus-scale bench
  (parser is one-pass O(n)); BOM-less UTF-16 sniffer intentionally
  absent.
- VLC 3.0.23 oracle PENDING-INFRA (no binary in sandbox; ffmpeg
  7.1.5 is the always-on oracle — the family-wide gap).
- wasm32 untested (web column honestly not_started).

## Next steps

R3 registration seam; Phase-6 sidecar autoload (two-pass load per
oracle evidence) consumes this parser.
