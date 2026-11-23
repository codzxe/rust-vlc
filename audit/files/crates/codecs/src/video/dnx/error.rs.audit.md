# AUDIT: crates/codecs/src/video/dnx/error.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 271 / 6 production fns + 2 tests |
| File verdict | REAL (typed taxonomy, total mapping, tests pass live) |
| Reachable from shipped entry point? | WIRED BUT COLD — `DnxDecoder` is shipped (feature `codec-dnx` in codecs `default`; apps/cli/src/pipeline_host.rs `"dnxhd"` arm exists) but **no container lane mints the `dnxhd` descriptor** (the CLI comment itself says so), so no shipped `lumen open` path can construct the decoder today |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
A small, disciplined error taxonomy: three internal variants with a one-to-one mapping to EngineError kinds, shorthand constructors, a total mapping of every `filmcraft_dnx::Error` arm (including the decode-unreachable `Input` arm, mapped defensively), and Display/source-chain tests that pass live. Two quality defects: the truncation arm fabricates `expected: 0` as a sentinel for "engine did not signal the required size" — a false number riding in a public structured detail that a consumer may render literally — and the whole dnx lane is shipped-cold (no container produces its descriptor), so this taxonomy is unreachable from any real file-open path in the current build.

## 2. What actually works (verified)
- `timeout 60 ./target/debug/deps/codecs-bae0f1756cc0a809 dnx --test-threads=2` → `13 passed; 0 failed` (this file's 2 tests included: exact rendered strings + source-chain text; every engine variant mapped to its stable `codecs.dnx.*` code).
- Totality: the `engine_error` match covers all five engine arms; `UnknownCid` is correctly classified `Unsupported` (stream feature) not `Corrupt`.

## 3. Findings

F-001 [S4 — `expected: 0` sentinel is a false structured number]
Where: dnx/error.rs:151-164
Code:
```
        filmcraft_dnx::Error::Truncated => corrupt(
            "codecs.dnx.frame.truncated",
            CorruptDetail::Truncated {
                // The engine reports truncation without the required
                // byte count; 0 = "not signalled by the engine" and
```
What is wrong: `CorruptDetail::Truncated { expected: 0, found: N }` publicly asserts "0 bytes were expected", which is false — the truth is "unknown". Any consumer rendering the detail (UI, logs, tests comparing structured fields) gets a fabricated number; the honest form is an `Option`-shaped detail or a distinct variant. The comment admits the sentinel; the type system does not.
Confidence: HIGH — Verification: read + ran the test that pins the sentinel (`expected: 0, found: 10` asserted at :260-269 — the test locks the lie in place).

F-002 [S4 — the entire dnx lane is shipped-cold]
Where: dnx/error.rs (module), apps/cli/src/pipeline_host.rs `"dnxhd"` arm
Code:
```
            // DNxHD/DNxHR (card 6.M-dnx, filmcraft-dnx =0.9.0
            // default-features OFF — the record's rayon-free pin):
            // raw VC-3 ES decodes statelessly. No container lane
```
What is wrong: no demuxer in the tree mints the `dnxhd` CodecDescriptor (rg: only tools/vta uses dnxhd), so the CLI arm — and with it this error taxonomy — cannot fire from any shipped file-open. Not a defect of this file (the wiring comment is honest), but the lane's entire evidence story lives in tools/vta + codecs tests, not the product.
Confidence: HIGH — Verification: rg over demux/apps.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DnxError::into_engine_error | 69-101 | REAL | — | stage Decode, kinds per table, source chain |
| From<DnxError> | 104-108 | REAL | — | |
| Detail (Display/Error) | 112-121 | REAL | — | chain-root carrier |
| unsupported / corrupt ctors | 124-142 | REAL | — | |
| engine_error | 149-185 | REAL | S4 | total; F-001 sentinel |
| tests ×2 | 191-270 | REAL | — | pass live; lock exact renderings |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | exact Display strings, stage, source text per variant |
| engine_error_maps_every_engine_variant | YES | all 5 arms → stable codes; kind classification asserted |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Nothing panics; every constructor is total; no unwrap on any input path" | error.rs:25-26 | TRUE (this file) | read; constructors are total |
| "the engine is typed and panic-free by construction (unsafe_code = forbid ...)" | error.rs:145-147 | UNVERIFIED | filmcraft-dnx crate lint config not checked here (registry cache exists); the admission record asserts it |
| "zero panics on truncated/garbage/zeroed inputs" | error.rs:21-23 | REAL-UNPROVEN | the hostile battery is tools/vta/dnx + codecs tests (13/13 pass incl. this file); the specific zero-panics battery not re-run here |

## 7. Dead / duplicated / unreachable code
Shipped-cold lane (F-002): compiled, wired, no production trigger. The `Input` engine arm is decode-unreachable (mapped defensively, disclosed).

## 8. Dependencies & flags
filmcraft-dnx =0.9.0 (optional, feature `codec-dnx` in default), media-types error vocabulary.

## 9. Open questions for the next agent
- Matrix auditors: does any row claim DNxHD "playback" as landed? Truthful phrasing is "decoder landed; no container lane mints the descriptor" — playback of a real .dnxhd-in-container file is impossible in the shipped build.
