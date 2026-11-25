<!--FIXV1
agent: FX1-R2
title: remedial compile fixes — two E0435 runtime array-repeat lengths (codecs fx1_a08) + one E0308 &str-vs-&[u8] (streaming fx1_a06)
verdict: DONE
changed:
- crates/codecs/tests/fx1_a08_tag_bounds.rs
- crates/streaming/tests/fx1_a06_numeric_guards.rs
tests:
- fx1_a08_truncated_vbri_and_xing_tails_refuse_typed_never_panic
- fx1_a08_ogg_flac_second_header_body_is_exactly_34_bytes
- fx1_a06_dash_template_duration_zero_refuses_typed
- fx1_a06_dash_timeline_d_zero_negative_r_refuses_typed
- fx1_a06_hls_master_oversized_segment_refuses_typed
- fx1_a06_hls_timestamp_map_year_overflow_refuses_none
evidence_read:
- audit/fix/RULES.md
- audit/fix/waves/FX1/FX1-A08.md
- audit/fix/waves/FX1/FX1-A06.md
- audit/fix/waves/FX1/FX1-R1.md
-->

# FX1-R2 — compile-only repair of the two new FX1 test files (remedial batch)

Scope: exactly the three centrally-gated compile errors the assignment
lists. Tests only — no src file, no Cargo.toml, no test logic touched.
All other test targets left untouched.

## Approach

1. Read both cited files in full; for each error, re-derive the cause
   from the source text rather than trusting the message alone:
   - fx1_a08:35 / :47 — both sites are
     `f.extend_from_slice(&[0x41; tail]);` where `tail` is a runtime
     `usize` fn parameter. An array-repeat **length is a const
     context**, so a runtime value there is exactly
     `error[E0435]: attempt to use a non-constant value in a constant`
     (col 33 = the `tail` token). Fix = express the same filler as a
     runtime iterator: `f.extend(std::iter::repeat_n(0x41u8, tail));`
     — the house idiom already committed in this very tests dir
     (fuzz_survival.rs:137 `dqt.extend(std::iter::repeat_n(16u8, 64));`,
     also benches.rs:194, alac_fuzz_survival.rs:179), so no new import,
     no new idiom, no MSRV gamble beyond what the crate already ships.
   - fx1_a06:49:40 — `parse_mpd_bytes(text)` with `text: &str`; real
     signature `pub fn parse_mpd_bytes(bytes: &[u8]) -> Result<MpdParsed,
     MpdError>` (playlists/src/mpd.rs:685, which itself does
     `str::from_utf8(bytes)`). Fix = `parse_mpd_bytes(text.as_bytes())`
     — the crafted MPD payload text still flows through the real
     parser verbatim, just in its byte form (the fn's own contract).
2. Keep every crafted byte, magic, flags word, tail length, and
   assertion byte-for-byte identical; only the two expressions change
   representation, not value or intent.
3. Ground every touched call against the real signatures via rg:
   `parse_vbri(frame: &[u8], frame_offset: u64)` (xing.rs:323),
   `parse_xing(frame: &[u8], frame_offset: u64)` (xing.rs:140),
   `parse_streaminfo(body: &[u8])` (flac/metadata.rs:151),
   `parse_mpd_bytes(bytes: &[u8])` (mpd.rs:685) — all existing call
   sites in fx1_a08 already coerce correctly (`&Vec<u8>`/`&[u8;34]` →
   `&[u8]`; `0` infers `u64`), so only the three cited sites were
   wrong.

## Changes per file

- `crates/codecs/tests/fx1_a08_tag_bounds.rs` — lines 35 and 47:
  `f.extend_from_slice(&[0x41; tail]);` →
  `f.extend(std::iter::repeat_n(0x41u8, tail));` (both occurrences,
  identical text, one edit class). Filler value `0x41`, count
  semantics, and resulting frame bytes are exactly what
  `&[0x41; tail]` produced — including `tail == 0` (empty iterator)
  and `tail == 50` (the 0x0004-flags Xing TOC shape), which is why
  the runtime form was chosen over any fixed literal cap. No import
   added (`std::iter` fully qualified, matching fuzz_survival.rs); no
  other line touched.
- `crates/streaming/tests/fx1_a06_numeric_guards.rs` — line 49:
  `parse_mpd_bytes(text)` → `parse_mpd_bytes(text.as_bytes())`.
  Only call site of the fn in the file; no import or other line
  touched.

## Test design

None changed — that is the point of a remedial compile card. Both
files' test names, crafted payloads, loop windows (0..22 + 22),
expected typed-refusal codes, and controls are verbatim what FX1-A08
and FX1-A06 authored; the edits only make those tests compile. The
tests' own oracles remain the responsibility recorded in those two
agents' reports.

## Verification (self-check, cargo-free per RULES)

- Re-read both files end-to-end after editing:
  - **Every const/array-length is literal-grounded**: fx1_a08's
    remaining repeat arrays are `&[0xFF,0xFB,0x90,0x00]`, `&[0u8; 32]`,
    `[0u8; 34]`, `b"VBRI"`/`b"Xing"` literals; fx1_a06's consts are
    `MANIFEST_URL: &str` (string literal) and `RETAIN/SMALL/OVERSIZED`
    (32/24/64) — all literals; its `vec![..; N]` uses are `vec!`
    (runtime macro), fine with any length. No `const` item anywhere
    derives from a runtime value; no array-repeat with non-const
    length remains (rg `[0x41; tail]` → 0 hits).
  - **Byte vs str**: `parse_mpd_bytes` now receives `&[u8]`
    (`text.as_bytes()`); all other parser calls already match their
    signatures (see grounding above).
  - **No unused imports introduced**: neither edit adds or removes a
    `use`; fx1_a08's four imports keep their users (the fixed lines
    still consume no import).
- rustfmt: the RULES exception could not run — no `rustfmt` binary
  exists in this box (`/home/z/my-project/.cargo/bin` has rustc/cargo/
  clippy but no rustfmt; none system-wide). Both replacement lines are
  single short lines (≤57 cols, 4-space indent) consistent with the
  files' existing formatting, so a rustfmt pass would not reflow them.

## Acceptance vs FIX-PLAN

- Exactly the three assigned errors fixed (2× E0435 in fx1_a08, 1×
  E0308 in fx1_a06); no drift in any cited line. No other file, test
  target, src path, dependency, or Cargo.lock touched. No
  TODO/FIXME/unwrap added. Scratch: none created (nothing outside my
  two files + this report).
- Context: FX1-R1's lint sweep had marked both files "clean", but its
  classes were (unused imports / Debug / unwrap / hallucinated APIs) —
  const-context and call-site-argument types were outside that
  sweep's classes, which is how these three survived to the central
  gate. Nothing in R1's report needs correction (it claimed signature
  existence, not call-site arity/types).

## Risks & followups

- No local compile is permitted (RULES), so this is rg/type-grounded
  repair, not compiler-proven; residual risk concentrates in nothing
  new — the two idioms substituted are the repo's own committed
  idioms (`std::iter::repeat_n` in the same crate's tests dir;
  `.as_bytes()` into the parser's documented `&[u8]` entry point).
- The runtime-behavior oracles of the six listed tests are unchanged
  and remain on FX1-A08/A06 (e.g. if the crafted-MPD text shape and
  the playlists parser's tolerance disagree at test time, that was
  already A06's declared residual risk #5 — my edit does not alter
  the text fed to the parser).
- rustfmt unavailability (no binary in box) — cosmetic only; if the
  central gate formats, the two lines are already idempotent.
