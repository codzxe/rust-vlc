# AUDIT: crates/codecs/src/audio/flac/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-21 |
| Lines / functions | 210 / 8 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (support module of the reachable FLAC lane: `lumen open` on an ogg/mkv/mp4/wav FLAC track → demuxers mint "flac" — ogg/mod.rs:155, mkv/header.rs:809, mp4/mod.rs:454, wav/format.rs:209 → apps/cli/src/pipeline_host.rs:253 `\"flac\" => self.seeded(track, config, FlacDecoder::new)` → flac/decoder.rs:61 `use super::error::{FlacError, corrupt, unsupported}` — every constructor here is on that path). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg '"flac"' crates/demux/src apps/cli/src (descriptor minters — 4 containers); rg "FlacError\|error::" flac/decoder.rs:61 (consumer); rg "flac" audit/runs/baseline/test-workspace.log.txt:358-369 (error tests ok: taxonomy, structured class, bad-state kind); sed -n full-file read; hand-check of all three variant mappings against the doc table. |

## 1. One-paragraph verdict
A small error-taxonomy module done to the house pattern: three internal variants (`Unsupported`/`Corrupt`/`BadState`) mapped one-to-one onto `ErrorKind::Unsupported`/`Corrupt`/`Decoder{BadState}` at stage `Decode`, with a `From` impl, a source-chain `Detail` carrier whose `Display` is the human reason, and two total shorthand constructors. Every claim in the module docs checks out against the code, the mapping is pinned by tests that assert the exact rendered `Display` strings and the structured `CorruptDetail` payload, and the tests pass in the committed baseline. I found nothing wrong; the checklist below lists everything examined and how.

## 2. What actually works (verified)
— verified by audit/runs/baseline/test-workspace.log.txt:362-365 → `flac::error::tests::{taxonomy_maps_each_variant_to_its_public_kind, corrupt_variant_carries_the_structured_class, bad_state_is_a_decoder_kind_not_corrupt}` all ok.
— verified by read: variant→kind→stage mapping in `into_engine_error` (64-96) is exactly the doc table (29-31); `Corrupt` carries the structured `CorruptDetail` through to `ErrorKind::Corrupt { detail }` (82-87) without loss; `BadState` produces no `source` (test pins "" via `unwrap_or_default`).

## 3. Findings
None. Zero findings — exact coverage of what was checked, per the instructions:
- **Fakeness:** no `todo!`/`unimplemented!`/`unreachable!` outside tests (rg over the file; tests' `unreachable!` at 195 is the fail-arm of a match that must succeed); no constant/`Ok(())` bodies; no ignored parameters (both constructors consume all args); no stub seams.
- **Correctness:** no panics/unwrap/expect/indexing/slicing in production lines (only `extra`-free fixed field moves); no arithmetic at all; no endianness concerns; `code: &'static str` guarantees stable context codes; the `From` impl (99-103) delegates to the same mapper (no divergent second mapping).
- **Performance:** nothing on a hot path (error construction only on failure).
- **Tests:** all three tests are mutation-sensitive — changing any variant's kind, stage, context-code plumbing, `Display` format, or source-chain wiring flips at least one assertion (the taxonomy test pins exact rendered strings, which also cross-pins media_types' `EngineError` Display against drift).
- **Architecture/hygiene:** duplication with the ADPCM/PCM/MJPEG `Detail` patterns is the deliberate house pattern (doc 2.B4 reference); `Corrupt`'s `detail` + `detail_text` pair is intentional (structured class for observability, string for the source chain) and both are consumed; no dead code (`FlacError` variants all constructed by flac/decoder.rs per rg; constructors used at decoder.rs:155-208 call sites).
- **Docs:** the two-bullet damage taxonomy (7-15) matches where the decoder actually raises each class (decoder.rs:155-208 constructs `InvalidStructure`/`Truncated`/`ChecksumMismatch` corrupt errors and `unsupported(...)` for geometry); "Nothing panics; every constructor is total" verified by inspection.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-18 | REAL | — | claims verified (see §3 checklist) |
| `enum FlacError` (3 variants) | 32-59 | REAL | — | all variants constructed by decoder.rs; field docs accurate |
| `FlacError::into_engine_error` | 64-96 | REAL | — | mapping matches doc table exactly; no panics |
| `From<FlacError> for EngineError` | 99-103 | REAL | — | delegates to the same mapper (single source of truth) |
| `Detail` + Display + Error | 108-117 | REAL | — | source-chain carrier; Display is the raw reason |
| `unsupported` ctor | 120-125 | REAL | — | total |
| `corrupt` ctor | 128-138 | REAL | — | total; keeps structured + text detail |
| tests (3) | 144-209 | REAL | — | mutation-sensitive; pass in baseline |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | pins exact Display strings, stage, and source text for all three variants |
| corrupt_variant_carries_the_structured_class | YES | destructures `ErrorKind::Corrupt{Truncated{34,10}}` — dropping the structured detail or swapping the class fails |
| bad_state_is_a_decoder_kind_not_corrupt | YES | kind-shape assertions |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Evidence |
|---|---|---|---|
| variant→kind→stage table | error.rs:27-31 | REAL | code 64-96 identical |
| "Configuration (Unsupported) … engine treats these as terminal" | error.rs:7-10 | REAL-UNPROVEN (engine-side handling) | terminality is the engine's use of `ErrorKind::Unsupported` — outside this file; consistent with the pcm/adpcm `unsupported` precedents |
| "Nothing panics; every constructor is total; no unwrap on any input path" | error.rs:17-18 | REAL | inspection (no panic-capable expression in production lines) |

## 7. Dead / duplicated / unreachable code
None. (The `Detail` type is textually duplicated across codec modules — the documented house pattern, each copy independently pinned.)

## 8. Dependencies & flags
- `media_types` only; no external deps, no unsafe, no features. File is `pub(crate)`-scoped internals of a reachable decoder.

## 9. Open questions for the next agent
- flac/decoder.rs owner (if unassigned in this wave): the taxonomy's "unparseable metadata chain → Unsupported (terminal)" choice means a file with a damaged metadata chain reports as unsupported-format rather than corruption — verify the CLI's exit/error rendering makes that distinction visible (doc 03 §4.5(d) conformance).
