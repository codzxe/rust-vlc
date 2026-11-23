# AUDIT: crates/codecs/src/audio/mp2/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-16 |
| Lines / functions | 187 / 4 (enum + into_engine_error + From + 2 ctors) + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:266-269 `"mp2" => { let mut decoder = Mp2Decoder::new(); ... }` → mp2/decoder.rs:36 `use super::error::{Mp2Error, corrupt, unsupported}` (also header.rs:35); apps/web/src/host.rs:632 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `diff <(sed-normalized mp2/error.rs) <(sed-normalized mp3/error.rs)` (exit 0, ~49 changed lines of 187 — structural clone); `rg -n "mp2::error" audit/runs/baseline/cargo-test-workspace.txt` (exit 0, 2/2 ok lines 486-487); sed quote verification |

## 1. One-paragraph verdict
A correct, small error taxonomy: three variants (Unsupported/Corrupt/BadState) mapping one-to-one onto public `ErrorKind`s at stage `Decode`, with stable dotted codes, structured `CorruptDetail`, human-readable reasons riding in `source` (not `Display`), total constructors, and zero panics. Both tests assert the exact rendered strings, the stage tag, and the source-chain text — they would fail on any mapping regression. The only complaint is the clone: this is the MP3 taxonomy file with ~49/187 lines changed (names + doc prose), the fourth sibling of the same skeleton (mp2/mp3/opus/vorbis = 771 lines carrying one shape).

## 2. What actually works (verified)
— verified by `rg -n "mp2::error" audit/runs/baseline/cargo-test-workspace.txt` -> `taxonomy_maps_each_variant_to_its_public_kind ... ok`, `bad_state_is_a_decoder_kind_not_corrupt ... ok` (lines 486-487).
— verified by reading the mapping (error.rs:62-94): Unsupported → `ErrorKind::Unsupported` + `FormatInfo{name:"mp2"}`; Corrupt → `ErrorKind::Corrupt{detail}`; BadState → `ErrorKind::Decoder{BadState}` with code `mp2.not_configured` — matches the doc table at lines 29-33 exactly.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/mp2/error.rs:1-187 (whole file) vs crates/codecs/src/audio/mp3/error.rs (and opus/error.rs, vorbis/error.rs)
Code (error.rs:34-38, verbatim — the shape shared by all four):
```
pub(crate) enum Mp2Error {
    /// A configuration or stream feature outside the supported subset.
    /// `code` names the exact rule; `detail` rides in `source`.
    Unsupported {
```
What is wrong: four per-codec copies of one taxonomy skeleton (187+186+184+214 lines); the sed-normalized diff against mp3/error.rs shows only ~49 differing lines — names, doc prose, and code strings; the type shape, mapping body, `Detail` carrier, and ctor pair are identical. Any fix to the mapping semantics (e.g. a new `CorruptDetail` propagation rule) must be applied four times; three of the four will drift (the classic copy-paste tax this audit tree keeps finding). The file's own header calls it "the 5.A3 MP3 pattern verbatim" — an admitted clone. What correct looks like: a generic `CodecError`-style taxonomy parameterized by codec name (the per-codec stable *codes* stay string-literal at the call sites), or accept the duplication with a cross-clone test that pins the four mappings to one table. Confidence: HIGH — Verification: ran diff. Severity S4: deliberate, documented, and low-risk — but it is 771 lines of one idea.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Mp2Error` (enum) | 35-57 | REAL | — | 3 variants, documented mapping table |
| `into_engine_error` | 62-94 | REAL | — | exact kind/stage/code mapping, test-pinned |
| `From<Mp2Error>` | 97-101 | REAL | — | delegation |
| `Detail` + Display/Error | 106-115 | REAL | — | source-chain carrier, no secrets in Display |
| `unsupported` ctor | 118-123 | REAL | — | total |
| `corrupt` ctor | 126-136 | REAL | — | total, structured detail |
| test `taxonomy_maps...` | 143-174 | REAL | — | exact render + stage + source text, 3 cases |
| test `bad_state_is_a_decoder_kind_not_corrupt` | 177-186 | REAL | — | kind discrimination pinned |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | asserts full Display string, stage, and source text per variant — any mapping/code drift fails |
| bad_state_is_a_decoder_kind_not_corrupt | YES | matches! on kind |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Nothing panics; every constructor is total; no `unwrap` on any input path" | error.rs:19-20 | REAL | read: no unwrap/expect/panic in the file; clippy baseline clean |
| variant→kind→stage table | error.rs:29-33 | REAL | test asserts all three rows |
| "per-frame isolation ... a damaged packet never poisons the next" | error.rs:14-17 | claim about decoder.rs, not this file | UNVERIFIED here — decoder.rs is another agent's row; the taxonomy merely does not contradict it |

## 7. Dead / duplicated / unreachable code
Duplication: F-001 (four-clone family). No dead code within the file: all variants constructed by decoder.rs/header.rs (rg census: decoder.rs:36, header.rs:35 imports).

## 8. Dependencies & flags
`media_types` error types only. Rides the `codec-mp2` default feature (Cargo.toml: `codec-mp2 = ["codec-mp3"]`). No unsafe.

## 9. Open questions for the next agent
- mp3/error.rs owner (same wave, likely): the free-format-bitrate refusal lives in your taxonomy's doc — confirm the decoder actually refuses free-format streams rather than mis-decoding them (doc 04 §3.2 known gap; I only saw the mp2 side).
