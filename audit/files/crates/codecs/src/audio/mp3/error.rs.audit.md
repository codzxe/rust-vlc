# AUDIT: crates/codecs/src/audio/mp3/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-21 |
| Lines / functions | 187 / 7 items (`Mp3Error` enum, `into_engine_error`, `From`, `Detail` + Display + Error, `unsupported`, `corrupt`) + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — every decode-stage failure of the reachable Mp3Decoder flows through `into_engine_error` (chain: `lumen open x.mp3` → demux es/mp3.rs → pipeline_host.rs:261-265 `"mp3" \| "mp1"` → Mp3Decoder decode/flush → this taxonomy at the contract boundary). Callers of the constructors span mp3/{header,id3,xing,decoder}.rs (verified by `rg -n "mp3::error\|use super::error" crates/codecs/src/audio/mp3`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full Read · `rg -n "free_format" crates/codecs/src/audio/mp3` (the doc's honest-refusal claim is real: header.rs:306 refuses free-format typedly) · `sed -n '30,45p'` + `head -30` of mp2/header.rs / mp2/error.rs (MP2 has its OWN taxonomy — this file's "mp3" FormatInfo name is correctly scoped, no cross-family mislabeling) · baseline test-workspace.log.txt:416-417 (both tests ok) |

## 1. One-paragraph verdict
A small, correct, exactly-tested error taxonomy. The variant→kind→stage mapping table in the docs matches the code arm-for-arm; the constructors are total; the human reason rides in `source` (never `Display`), and both tests assert full rendered strings and are green in the baseline. Zero findings; the verification performed is listed so the zero is auditable.

## 2. What actually works (verified)
- Mapping table (doc lines 28-32) checked arm-for-arm against `into_engine_error` (61-93): `Unsupported` → `ErrorKind::Unsupported` with `FormatInfo{name:"mp3"}` + context code + `Detail` source; `Corrupt` → `ErrorKind::Corrupt{detail}` + code + `Detail` source; `BadState` → `ErrorKind::Decoder{BadState}` with code `mp3.not_configured` — all at `PipelineStage::Decode` ✓.
- Scope check: the module is MP3/MP2-shared in NAME only — mp2 has its own `mp2/error.rs` (its own codes and FormatInfo name), and mp2/header.rs imports only the shared parse types from mp3 — so the hard-coded `name: Some("mp3")` (68) cannot mislabel an MP2 stream through THIS file. (The family parser's `mp3.free_format` code appearing on an mp2 lane is mp2/header.rs's concern, not this taxonomy's.)
- The doc's honesty claims verified: free-format refusal is real (mp3/header.rs:306 `"mp3.free_format"`), constructors total (117-135), no unwrap/expect/panic anywhere in the file, `Detail` keeps the reason out of `Display` (108-112) — matching the rendered-string tests.
- Both tests execute in baseline: test-workspace.log.txt:416-417 (`bad_state_is_a_decoder_kind_not_corrupt`, `taxonomy_maps_each_variant_to_its_public_kind` — ok, inside the codecs lib suite).

## 3. Findings
None. Hostile passes performed, all clean:
- Lost-information check: every field of every variant is consumed by `into_engine_error` (code → ErrorContext, detail → ErrorKind::Corrupt, detail/detail_text → `Detail` source) — nothing silently dropped.
- Cross-family leakage check: covered in §2 (mp2 isolation verified).
- Display-surface check: the asserted renders contain no input-derived bytes (only the static dotted codes and the reason text in `source`) — the no-secrets posture the module doc claims.
- Derive check: `PartialEq/Eq` on the enum requires `CorruptDetail: Eq` — compiles (baseline build CLEAN), and the tests never rely on enum equality, only on rendered output — stronger.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Mp3Error` enum | 33-56 | REAL | — | 3 variants, doc table matches code |
| `Mp3Error::into_engine_error` | 61-93 | REAL | — | arm-for-arm correct mapping; format name "mp3" correctly scoped |
| `From<Mp3Error>` | 96-100 | REAL | — | delegates |
| `Detail` + Display + Error | 105-114 | REAL | — | private source carrier; reason never in Display |
| `unsupported` | 117-122 | REAL | — | total shorthand |
| `corrupt` | 125-135 | REAL | — | total shorthand |
| test `taxonomy_maps_each_variant_to_its_public_kind` | 141-173 | REAL | — | exact rendered Display + stage + source text for all 3 variants — fails on any mapping drift |
| test `bad_state_is_a_decoder_kind_not_corrupt` | 175-185 | REAL | — | kind discrimination (a misclassification would fail) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | string-equality on the full public render + stage + source for all three variants |
| bad_state_is_a_decoder_kind_not_corrupt | YES | `matches!` on the exact kind + negative assertion on Corrupt |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a free-format bitrate stream (…known gap, honestly refused rather than guessed)" | error.rs:8-9 (module doc) | REAL | mp3/header.rs:306 refuses typedly; test header.rs:577 pins the code |
| "Nothing panics; every constructor is total; no `unwrap` on any input path" | error.rs:18-19 | REAL | read: no panic site in the file; constructors 117-135 total |
| variant/kind/stage mapping table | error.rs:28-32 | REAL | verified arm-for-arm (§2) |

## 7. Dead / duplicated / unreachable code
None. (The near-verbatim duplication of this pattern across flac/pcm/mp3/mp2/aac/vorbis/opus error.rs files is a crate-wide convention, deliberately mirrored per each module's docs — consolidating it is a mod.rs-owner decision, not a defect of this file.)

## 8. Dependencies & flags
- Depends only on media_types error types. Compiled under `codec-mp3` (default set; baseline #1 CLEAN). No unsafe.

## 9. Open questions for the next agent
- None for this file. For the mp2 owner: mp2/header.rs:385 asserts the FAMILY parser's `mp3.free_format` code on an mp2 stream — confirm the mp2 error surface intentionally keeps mp3-prefixed codes for family-parser refusals (naming-consistency question, not correctness).
