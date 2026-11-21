# AUDIT: crates/codecs/src/audio/vorbis/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-21 |
| Lines / functions | 214 / 10 (5 prod items + 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.ogg|.mkv` → ogg/mod.rs:152 / mkv/header.rs:811 mint codec `"vorbis"` → apps/cli/src/pipeline_host.rs:271-275 `self.seeded(track, config, VorbisDecoder::new)` → decoder.rs headers/decode paths construct these errors at 30+ sites: headers.rs ×10, decoder.rs ×12, comments.rs ×10 — every vorbis decode failure in a shipped build flows through `into_engine_error` here) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg `unsupported\(|corrupt\(` in vorbis/*.rs (30+ production construction sites enumerated); rg `"vorbis\.[a-z_.]*"` (full code inventory — 25 distinct codes); rg `"vorbis"` crates/demux/src (ogg:152, mkv:811 mint the descriptor); rg `vorbis.packet.framing` (test-only phantom code); sed -n decoder.rs:225-307 (overlap-scoped isolation claim implementation: engine.reset() at decoder.rs:280); grep baseline log (3/3 error tests ok, :505-507) |

## 1. One-paragraph verdict
A small, clean error taxonomy that does exactly what its docs claim: three internal variants (`Unsupported`, `Corrupt`, `BadState`) mapped one-to-one to `ErrorKind::Unsupported/Corrupt/Decoder` at stage `Decode`, with the human reason carried in the `source` chain rather than `Display` (the stated no-secrets pattern). The constructors are used at 30+ real production sites with 25 distinct stable codes, the damage-isolation claim documented here is genuinely implemented in decoder.rs (`engine.reset()` on packet rejection, decoder.rs:277-280), and all 3 tests executed in the baseline run. The only defects are two test-level nits: one test example uses a context code that no production site ever emits, and the BadState source assertion cannot distinguish "no source" from "empty source".

## 2. What actually works (verified)
— Variant→kind mapping table (error.rs:31-35) matches the code (error.rs:71-99) exactly; stage pinned `Decode` in all three arms (test asserts it).
— `Corrupt` carries the structured `CorruptDetail` into `ErrorKind::Corrupt { detail }` without loss — pinned by `corrupt_variant_carries_the_structured_class` matching `Truncated { expected: 30, found: 7 }` after conversion (baseline test-workspace.log.txt:506 ok).
— Display-surface separation: reason strings ride in `source` (`Detail` impl, error.rs:112-121), `Display` shows only kind+code — pinned by exact rendered-string assertions (baseline :505 ok).
— Production wiring: constructor sites enumerated at headers.rs:92-238, decoder.rs:232-1039, comments.rs:113-251; codes follow a consistent `vorbis.<header>.<rule>` grammar (25 distinct codes inventoried).
— The module doc's overlap-scoped isolation claim is implemented where it belongs: decoder.rs:276-285 resets the engine on packet rejection with the same doc citation.

## 3. Findings
### F-001 [S4] Taxonomy test exercises a phantom context code
Where: crates/codecs/src/audio/vorbis/error.rs:157-163
Code:
```rust
                corrupt(
                    "vorbis.packet.framing",
                    CorruptDetail::ChecksumMismatch,
                    "comment framing bit not set",
                ),
```
What is wrong: `"vorbis.packet.framing"` is emitted by no production site — the real framing codes are `vorbis.comment.framing` (comments.rs) and `vorbis.packet.decode` (decoder.rs). rg over the whole tree finds the string only in this test. The mapping machinery is still exercised (the constructor is code-agnostic), but the test presents a taxonomy site that does not exist.
Why it matters: A reader auditing error-code coverage will hunt for a `vorbis.packet.framing` site that isn't there; the test's example misrepresents the taxonomy's real surface.
What correct looks like: Use a real production code (e.g. `vorbis.comment.framing`) as the example.
Confidence: HIGH — Verification: read (rg inventory of all `vorbis.*` codes).

### F-002 [S4] BadState source assertion cannot fail on an empty-string source
Where: crates/codecs/src/audio/vorbis/error.rs:175-178
Code:
```rust
            let source = std::error::Error::source(&engine)
                .map(|s| s.to_string())
                .unwrap_or_default();
            assert_eq!(source, source_text);
```
What is wrong: For the `BadState` case `source_text` is `""`, and `unwrap_or_default()` maps a missing source to `""` — so the assertion passes both when the error has NO source (correct) and when someone later attaches an empty `Detail("")` source (incorrect). The assertion does not pin "no source chain exists".
Why it matters: Weak pin on the one variant documented as source-less; a regression that attaches a source would slip through.
What correct looks like: `assert!(std::error::Error::source(&engine).is_none())` for the BadState case.
Confidence: HIGH — Verification: read (Option path analysis).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum VorbisError` | 36-63 | REAL | — | 3 variants; doc table matches mapping exactly |
| `VorbisError::into_engine_error` | 68-101 | REAL | — | stage Decode all arms; Unsupported hard-codes FormatInfo name "vorbis" (correct — module-scoped); Corrupt passes `detail` by value (structured, lossless); BadState has no source (pinned weakly — F-002); two String allocations per call (error path, cold — acceptable) |
| `From<VorbisError> for EngineError` | 103-107 | REAL | — | delegates ✓ |
| `Detail` + Display + Error impls | 112-121 | REAL | — | chain-root carrier, no panics |
| `unsupported` constructor | 124-129 | REAL | — | total, `impl Into<String>` |
| `corrupt` constructor | 132-142 | REAL | — | total, structured detail + text |
| test `taxonomy_maps_each_variant_to_its_public_kind` | 148-180 | REAL | — | exact Display strings + stage + source text; F-001/F-002 nits |
| test `corrupt_variant_carries_the_structured_class` | 182-201 | REAL | — | destructures the kind; mutation of detail plumbing fails it |
| test `bad_state_is_a_decoder_kind_not_corrupt` | 203-213 | REAL | — | pins the kind discriminant |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | exact rendered strings (any kind/stage/code change breaks); F-002 weakens only the source-chain half of the BadState case |
| corrupt_variant_carries_the_structured_class | YES | matches the destructured CorruptDetail payload |
| bad_state_is_a_decoder_kind_not_corrupt | YES | kind discriminant match |

All 3 executed ok in baseline (test-workspace.log.txt:505-507).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "typed, stage-tagged internal errors that map one-to-one onto EngineError kinds" | error.rs:1-3 | REAL | 3 variants → 3 kinds, stage Decode (code + test) |
| "a damaged packet poisons at most itself and the next packet's overlap half — the decoder resets cleanly" | error.rs:14-19 | REAL | decoder.rs:276-285 `engine.reset()` on rejection with the same doc citation (implementation is decoder.rs's, cross-verified here) |
| "Nothing panics; every constructor is total; no unwrap on any input path" | error.rs:21-22 | REAL | no unwrap/expect/panic in the file (read); constructors total |
| "the 2.B4 FeatureDetail pattern verbatim" (provenance) | error.rs:109-111 | UNVERIFIED | mjpeg module's error file not re-read this session (other agent's file); the structural pattern (source-chain Detail) is present and consistent |

## 7. Dead / duplicated / unreachable code
- None. Every item is used: constructors at 30+ production sites, `From` used by `?` at those sites, `Detail` used by both arms.
- This is the FLAC/MP3-error-pattern twin (module doc claims the 5.A1/5.A3 pattern; structure is consistent with mp3/error.rs's `corrupt`/`unsupported` seen in my header.rs audit — same shape, deliberate crate-local duplication across codec families).

## 8. Dependencies & flags
- `media_types` only (EngineError/ErrorKind/PipelineStage/CorruptDetail/ErrorContext/FormatInfo); no external deps; no `unsafe`; rides the `codec-vorbis` feature (in default set — compiled in shipped builds per baseline build #1).

## 9. Open questions for the next agent
- vorbis/decoder.rs owner: `decode_block_into` returns `vorbis.output.slot_size` as **Corrupt** (decoder.rs:296-301) when the caller-provided slot is too small — that is a host/contract bug (our side), not media damage; classifying it `Corrupt` blames the file. Consider `ErrorKind::Decoder` for that site (cross-file, severity call belongs to that file's audit).
- vorbis/headers.rs + comments.rs owners: the 25 production codes were inventoried for existence, not per-site correctness — that audit belongs to your files.
