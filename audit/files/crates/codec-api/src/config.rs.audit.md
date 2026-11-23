# AUDIT: crates/codec-api/src/config.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 277 (187 source + 90 tests) / 7 non-test fns + 3 tests |
| File verdict | REAL (clean self-validating config types) — with a hardwired thread budget (F-001) |
| Reachable from shipped entry point? | YES — DecoderConfig is constructed and passed to `Decoder::configure` on every shipped decode: apps/cli pipeline_host.rs (uses codec_api::DecoderConfig, seeded_config at :202-203) and playback-engine/src/pipeline.rs:1301/1344/1691/1701; apps/web/src/host.rs:2265 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read of full file; `Grep` consumer maps (`.threads()`, `DecoderConfig::new`, `ThreadBudget::`); `rg thread product-config/cli` (no user setting); baseline citation audit/runs/baseline/cargo-test-workspace.txt:252-254 |

## 1. One-paragraph verdict
Small, honest config-carrier module: `ThreadBudget` is a self-validating newtype (1..=256, typed `codec.threads.out_of_range` refusal), `DecoderConfig`/`EncoderConfig` are plain assembled structs whose "cannot fail" claim holds because every field is a pre-validated type, and the Debug impls deliberately print the accelerator's `backend_id` instead of a pointer (correct observation that trait objects can't derive Debug and diagnostics should say what was negotiated). The three tests assert exact error payloads and debug output and passed in the baseline. Two defects, both hygiene: the entire thread-budget negotiation surface is decorative in shipped builds (every host passes `ThreadBudget::default()`; no user setting exists anywhere; the only decoder that even reads the value gets 1), and `ThreadBudget::new` stamps `PipelineStage::Decode` even when the budget is for an `EncoderConfig`.

## 2. What actually works (verified)
— `Grep DecoderConfig::new` -> shipped construction sites in playback-engine/src/pipeline.rs (1301/1344/1691/1701), apps/web/src/host.rs:2265, and the CLI's reconfigure path (pipeline_host.rs:202-203, 430-432) — the type is on every decode's hot path, not dead vocabulary.
— Baseline: `config::tests` 3/3 ok (audit/runs/baseline/cargo-test-workspace.txt:252-254) with exact kind/code/stage assertions on the rejection path.
— av1/decoder.rs:792 consumes the budget (`i32::try_from(cfg.threads.threads()).unwrap_or(1)`) — at least one decoder reads it.

## 3. Findings
F-001 [S4]
Where: config.rs:20-37 vs production call sites
Code: (verbatim, config.rs:34-37)
```
    /// Largest budget a configuration may carry `[PROPOSAL]` — 256
    /// helper threads exceeds any real codec's useful parallelism while
    /// keeping a misconfiguration visible at configure time.
    pub const MAX_THREADS: u32 = 256;
```
What is wrong: The negotiation surface this type exists for is unused in shipped builds: every production construction passes `ThreadBudget::default()` (= 1) — playback-engine/src/pipeline.rs:1304/1347/1694/1704, apps/web/src/host.rs:2268 — and `rg thread` over product-config's model and the CLI finds no user-facing setting. The validation path (`new` rejecting 0 and >256) is exercised only by this file's unit test; the av1 consumer always receives 1. The `[PROPOSAL]` marker on MAX_THREADS is honest, but the module doc's framing ("the registry hands a Decoder ... thread budget") describes plumbing that today can only ever carry 1. Not fake — the invariant holds — but the configurability is future tense.
Confidence: HIGH — Verification: Grep consumer map.
F-002 [S4]
Where: config.rs:49-53
Code: (verbatim)
```
            Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("codec.threads.out_of_range"),
            ))
```
What is wrong: Two taxonomy nits: (a) the stage is hardcoded `Decode`, but the same `ThreadBudget` is carried by `EncoderConfig` — an encoder-side misconfiguration would be reported as a Decode-stage error; (b) `Unsupported` (with `format: None`) is semantically for capability refusals, not for an out-of-range integer in a config — an Invalid/Config-style kind would carry the meaning. Blast radius: diagnostics only.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ThreadBudget::new | 45-55 | REAL | S4 | validation correct; F-002 stage/kind |
| ThreadBudget::threads | 58-60 | REAL | — | accessor |
| ThreadBudget::Default | 63-69 | REAL | — | 1 |
| DecoderConfig::new | 100-114 | REAL | — | infallible assembly of validated parts ✓ |
| DecoderConfig::Debug | 117-134 | REAL | — | backend_id not pointer ✓ |
| EncoderConfig::new | 158-170 | REAL | — | symmetric |
| EncoderConfig::Debug | 173-186 | REAL | — | same discipline |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| thread_budget_bounds_are_enforced | YES | exact ok/bad boundaries incl. u32::MAX, error kind/code/stage asserted |
| decoder_config_assembles_and_debugs_its_negotiation | YES | field equality + Debug rendering contains "software"/"hw.mock" |
| encoder_config_mirrors_decoder_config | YES | same for the encoder twin |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every field is a self-validating type, so a DecoderConfig that exists is a config the codec may trust" | config.rs:8-10 | REAL | read (CodecDescriptor/GenerationId/ThreadBudget all validated at construction) |
| "the negotiation itself ... is the registry's job and lives elsewhere (MP §6)" | config.rs:6-8 | SHALLOW | the negotiation's user-facing knob (threads) does not exist yet (F-001) |
| "H.264/HEVC receive one [accelerator] per ADR-001 §1 when the platform provides an audited adapter" | config.rs:86-88 | REAL-UNPROVEN | the accelerator plumbed in the shipped software-only lane is always None (see sps.rs audit F-005: hw lane unwired) |

## 7. Dead / duplicated / unreachable code
None dead in this file. The out-of-range error path is test-only today (F-001) but is the module's contract.

## 8. Dependencies & flags
crate::hw::HwAccelerator, media_types (EngineError/GenerationId/SharedBytes/CodecDescriptor). No features. Consumed by every decoder crate + playback-engine + web host + tools.

## 9. Open questions for the next agent
playback-engine/pipeline.rs owns the four production `DecoderConfig::new` sites — whether any future settings key is *supposed* to feed `threads` would settle F-001's "[PROPOSAL]" framing.
