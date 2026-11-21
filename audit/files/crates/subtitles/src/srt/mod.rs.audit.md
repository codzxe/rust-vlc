# AUDIT: crates/subtitles/src/srt/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-21 |
| Lines / functions | 67 / 6 module decls + 4 re-export lines + 1 doctest (no functions) |
| File verdict | UNREACHABLE (real module hub, doctest executed in baseline; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `subtitles::srt::parse` consumed only by in-crate tests (`tests/srt_canonical_differential.rs:22`, `tests/srt_fuzz_survival.rs:25`) and the nightly-only `fuzz/fuzz_targets/fuzz_sub_srt.rs:19`; no app/engine crate depends on `subtitles` (proof in ass/tags.rs audit §2) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n 'pub fn parse|pub struct SrtDocument' srt/parser.rs` (re-export targets exist) · `rg -n 'pub struct SrtRenderError|TextBoxLayout|TextMetrics|cue_layout|place_plain_text|plain_text' srt/render.rs` (all 6 exist; SrtRenderError is a `pub enum` at render.rs:84) · `rg -n 'pub enum SourceEncoding' srt/encoding.rs` · `rg -n 'crate::font|font::' srt/` (EMPTY — no font-stack pull) · `rg -n 'dropped_damage_events|into_parts' srt/parser.rs` (wiring for the damage doc) · baseline doctest citation (below) |

## 1. One-paragraph verdict
A clean 67-line module hub: six submodule declarations that all match real
files, twelve re-export lines whose every target exists (verified
one-by-one), and a module doc whose contract claims are each machine-
checkable — and were checked. The doctest at lines 35-48 EXECUTED and PASSED
in the baseline no-fail-fast run, pinning both the clean-parse path (1 cue,
`start_us() == 1_000_000`) and the dirty-world promise (mid-file garbage →
1 cue kept, `is_clean() == false`). No findings. The file is unreachable
from shipped binaries with the rest of the crate — the hub of a library
nothing ships.

## 2. What actually works (verified)
- Doctest executed: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:6531`
  — `test crates/subtitles/src/srt/mod.rs - srt (line 35) ... ok`, inside
  `Doc-tests subtitles … test result: ok. 2 passed` (the other is lib.rs's
  vtt example). The primary fail-fast baseline log never reached the crate's
  tests (aborted at apps/desktop) — see ass/tags.rs audit §2.
- Re-export completeness (all 12 names):
  - `damage::{MAX_DAMAGE_EVENTS, SrtDamage, SrtDamageKind}` — damage.rs:17,121,25.
  - `encoding::SourceEncoding` — encoding.rs:35.
  - `parser::{SrtDocument, parse, parse_str}` — parser.rs:50,76,93.
  - `render::{SrtRenderError, TextBoxLayout, TextMetrics, cue_layout, place_plain_text, plain_text}` — render.rs:84,153,56,172,282,143.
- Contract claim "never returns an error": `pub fn parse(bytes: &[u8]) -> SrtDocument`
  (parser.rs:76) — no `Result` in the public parse signature; verified.
- Contract claim "never aborts the whole file on mid-file garbage": pinned
  by the executed doctest (dirty example, lines 43-47).
- Contract claim "never panics": the crate's fuzz lane passed in the same
  baseline run (`srt_fuzz_survival` ok; `fuzz_sub_srt` harness is the
  in-crate `srt::fuzz` module this hub declares).
- "Never: Font stacks, rasterization, GPU paths … geometry only":
  `rg 'crate::font|font::' srt/` returns NOTHING — the srt tree does not
  touch the crate's font module; render.rs exposes geometry-only items.
- "Std-only (C5); no unsafe": crates/subtitles/Cargo.toml has exactly one
  runtime dep (`media-types`, path); workspace lints `unsafe_code = "deny"`
  (root Cargo.toml) — no unsafe can compile in this module tree.

## 3. Findings
None. Checked and verified clean, with the method for each:
- Module decls ↔ files (encoding/parser/tags/render/damage/fuzz all present
  in src/srt/ — directory listing).
- Re-export target existence (12/12, rg'd — see §2).
- Doctest truth (executed in baseline, cited).
- Public-API shape vs doc claims (parse signature, SrtDocument fields
  incl. `dropped_damage_events` wired at parser.rs:81/85, 97/101).
- Module boundary claims (no font pull, no unsafe, std-only).
- Ownership list in the doc matches the six declared modules exactly.
A zero-finding file is the honest result here: the hub has no logic to get
wrong, and every claim it makes is either compiled, executed, or grepped.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc (incl. doctest) | 1-53 | REAL | — | Every claim audited in §2; doctest executed in baseline. |
| `pub mod damage` | 55 | REAL | — | File exists; audited (this wave, W09-A02). |
| `pub mod encoding` | 56 | REAL | — | File exists; audited by W09-A01. |
| `pub mod fuzz` | 57 | REAL | — | In-crate fuzz harness pattern (pub module, called by the nightly cargo-fuzz shim fuzz_sub_srt.rs:19); compiled into every build by design — deliberate, documented in fuzz/Cargo.toml. |
| `pub mod parser` | 58 | REAL | — | File exists (567 lines); another agent's assignment. |
| `pub mod render` | 59 | REAL | — | File exists; geometry-only (verified). |
| `pub mod tags` | 60 | REAL | — | File exists; audited (this wave, W09-A02). |
| re-exports (4 lines) | 62-67 | REAL | — | 12/12 targets verified present. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| doctest `srt` (mod.rs:35-48) | YES | Asserts cue count, exact `start_us()`, `is_clean()` true→false across clean and dirty inputs; executed in baseline (log line cited in §2). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "parse takes raw bytes … never returns an error, never aborts the whole file … never panics" | mod.rs:26-31 | REAL | Signature parser.rs:76; doctest (executed) pins dirty-file tolerance; fuzz lane passed in baseline. |
| "BOM detection (UTF-8 / UTF-16LE / UTF-16BE), UTF-8 fallback with lossy degradation, line-ending normalization" (encoding ownership) | mod.rs:8-10 | REAL (in-crate) | encoding.rs exists with `SourceEncoding`; its tests (bom_detection_matrix, utf16_le_and_be_decode_identically, …) all ok in the no-failfast baseline. |
| "real-world tolerances: `.`-vs-`,` milliseconds, glued `-->`, rolled hours, 1–3-digit ms" | mod.rs:12-14 | REAL (in-crate) | parser.rs `timing_tolerances` test ok in baseline (assertions live in the parser file — that agent's audit). |
| "Never: Font stacks, rasterization, GPU paths — geometry only" | mod.rs:50-53 | REAL (module-scoped) | `rg font` over srt/ empty; render.rs exports geometry items only. NOTE: the crate itself DOES contain a `font/` module (used outside srt) — the "Never" is honestly scoped to this module, but the crate-level rendering-boundary story belongs to lib.rs/font owners. |
| matrix row `subtitle-srt` status implemented ×3, "230/0 current tree" | feature-matrix.toml:900-901 | REAL (code+tests) / UNREACHABLE (product) | Crate real and green in-crate; no shipped binary links `subtitles` (ass/tags.rs audit §2 proof). The row nowhere discloses non-integration. |

## 7. Dead / duplicated / unreachable code
- Unreachable from shipped entry points (crate-level proof, ass/tags.rs
  audit §2) — the hub re-exports an API only tests and the non-gated
  nightly fuzz workspace consume.
- No dead re-exports (all 12 targets exist and are used by tests).

## 8. Dependencies & flags
- No features, no cfg gates; `pub mod fuzz` unconditionally compiled (the
  in-crate harness pattern — deliberate per fuzz/Cargo.toml comments).
- std-only + media-types at the crate level.

## 9. Open questions for the next agent
- lib.rs owner: the crate-level "Never: rendering" boundary vs the
  `font/` module (`font/mod.rs`, `font/pipeline.rs` — W09-A03's files):
  which layer owns font metrics, and does any format path besides ass pull
  it? (srt verifiably does not.)
- matrix owner: `subtitle-srt` row says implemented on windows/linux/macos
  — with zero shipped consumers, decide whether the row needs the same
  non-integration disclosure requested for the ASS row (ass/tags.rs audit
  §7).
