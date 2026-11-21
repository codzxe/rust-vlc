# AUDIT: crates/demux/src/es/aac.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-21 |
| Lines / functions | 34 / 4 (EXTENSIONS, RawAacDemuxer re-export, probe, open) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO — the shipped route for raw `.aac`/`.adts`/`.adif` is: media-probe `AdtsSignature` (phase5.rs:134-148) → `FormatId::adts()` → `apps/cli/src/containers/mod.rs:52` `"adts" => adts::open(...)` → `containers/adts.rs:40,66` `use demux::adts::AdtsDemuxer` — the real implementation directly. The only consumer of THIS module is `crates/demux/tests/es_corpus.rs:235` (`demux::es::aac::open`), an env-gated corpus test whose baseline pass is vacuous (see below). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full read; `Grep "RawAacDemuxer|es::aac"` workspace-wide (one test caller only); `rg -n "EXTENSIONS" crates/demux/src/adts/mod.rs` (superset claim); `sed -n '130,150p' crates/media-probe/src/signature/phase5.rs` (probe id routing); `rg -n "use demux|AdtsDemuxer" apps/cli/src/containers/adts.rs` (the real path); baseline `sed -n '2019,2028p'` (es_corpus result) |

## 1. One-paragraph verdict
A 34-line registry-surface alias: re-exports `crate::adts::AdtsDemuxer` as
`RawAacDemuxer`, delegates `probe` and `open`, and declares `EXTENSIONS = ["aac"]`
while documenting that the landed implementation claims the superset
`["aac","adts","adif"]` (verified, adts/mod.rs:135). The delegation is honest and the
doc correctly points every behavior note at `crate::adts` — but no shipped entry
point reaches it: the probe id for raw AAC files is `adts`, and the CLI's container
arm imports the implementation crate module directly. Its sole caller is an env-gated
corpus test whose baseline "ok" is a vacuous skip-pass (0.00s, per-file
`SKIP es corpus` continue). Harmless dead-weight indirection with a live drift risk
(the extension lists already disagree — deliberately, but nothing enforces the
deliberateness).

## 2. What actually works (verified)
— verified by read + rg: `probe`/`open` are exact delegates (`crate::adts::probe`,
`RawAacDemuxer::new`); no behavior exists here to break.
— verified by baseline sed 2019-2028: `corpus_aac_lanes_match_ffprobe_packet_counts ... ok` — VACUOUS (see F-001); the demuxer it delegates to carries its own real evidence (24 adts tests, baseline 1494-1517).

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/es/aac.rs:13-16 (with the routing facts)
Code:
```rust
pub const EXTENSIONS: &[&str] = &["aac"];

/// The raw-AAC demuxer: the landed [`crate::adts::AdtsDemuxer`].
pub use crate::adts::AdtsDemuxer as RawAacDemuxer;
```
What is wrong: The module is a parallel registry surface that the shipped binary
never consults: media-probe's `AdtsSignature::extensions()` is
`&["aac", "adts", "adif"]` and its hits carry `FormatId::adts()`, dispatched to
`containers::adts` — which imports `demux::adts::AdtsDemuxer` directly. The only
code that calls `es::aac::open` is `tests/es_corpus.rs:235`, and that suite
per-file `continue`s when the corpus dir is absent — the baseline block (log lines
2019-2028, `finished in 0.00s`, 0 ignored) shows it vacuous-passed. So this
module's `probe`, `open` and `EXTENSIONS` have zero non-vacuous execution evidence
and zero production reach. The extension lists already diverge from the
implementation's (["aac"] vs ["aac","adts","adif"]) — documented as intentional,
but a second registry truth that can silently drift is a standing trap.
Why it matters: Two sources of truth for the same format's claims; an auditor or
future maintainer grepping `EXTENSIONS` finds two different answers, and a change
to the adts registry (new extension, changed probe) does not propagate here.
What correct looks like: Delete the module and point `es_corpus` at
`demux::adts::open` (one-line change), or wire the ES registry genuinely (if a
separate `.aac` registry row is wanted, derive `EXTENSIONS` from
`crate::adts::EXTENSIONS` instead of re-declaring a subset).
Confidence: HIGH — Verification: read + Grep (caller census: exactly one test caller; CLI path verified at containers/adts.rs:40,66; probe id verified at phase5.rs:134-148)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `EXTENSIONS` | 13 | REAL (data) | F-001 | subset of adts::EXTENSIONS; divergence documented |
| `RawAacDemuxer` (re-export) | 16 | REAL | — | pure alias |
| `probe` | 20-22 | REAL | — | exact delegate |
| `open` | 30-34 | REAL | — | exact delegate; doc's Errors section accurately mirrors AdtsDemuxer::new's refusal set (empty/unseekable/unknown-length/no-syncword) |

## 5. Test honesty
No tests in this file. Its only consumer (`es_corpus.rs`) is env-gated and
vacuous-passed in the baseline (per-file `SKIP es corpus` continue at line 232-234;
0.00s runtime). Nothing anywhere pins this module's own surface (e.g. that
`es::aac::probe` stays in sync with `adts::probe` — today guaranteed only by the
delegate's one-line body).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the `.aac` extension is the landed ADTS/ADIF demuxer — consumed, not duplicated" | module doc 1-2 | REAL | the re-export/delegates verified |
| "this module is the registry surface" | module doc 5-6 | FALSE in effect | the real registry surface is media-probe's AdtsSignature + the CLI dispatch; nothing shipped consults this module |
| "the landed implementation claims the superset `aac`/`adts`/`adif`" | doc 12 | REAL | adts/mod.rs:135 verified |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable-from-production aliasing (F-001) — kept out of
S3 because it is documented as a deliberate registry surface and does delegate
honestly (no fabricated behavior), and deleting it is a one-line consumer change.

## 8. Dependencies & flags
`crate::adts` only (plus `plugin_api::SourceStream` in the signature). No features,
no unsafe, no tests of its own.

## 9. Open questions for the next agent
- Who consumes `demux::es::EXTENSIONS`-style registry constants, if anyone? If the
  answer is "nobody but docs", the whole per-format EXTENSIONS pattern deserves a
  wave-level look (each demuxer module declares extensions that the actual probe
  layer re-declares independently — the aiff one has the same dual-declaration
  shape, `aiff::EXTENSIONS` vs phase5's AIFF signature).
