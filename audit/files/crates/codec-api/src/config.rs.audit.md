# AUDIT: crates/codec-api/src/config.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-21 |
| Lines / functions | 277 / 9 (ThreadBudget + 4 assoc items/impls, 2 config structs + new/Debug ×2, MockAccelerator, 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → playback-engine/src/pipeline.rs:1702-1708 `DecoderConfig::new(track.codec.clone(), generation, ThreadBudget::default(), None, None)` → `provider.decoder_for(track, &config)` → the decoders' `configure`. EXECUTED: every baseline open/decode run (AVI MJPEG 60/60, MP4 420 h264 play) constructed a DecoderConfig. ThreadBudget's non-default range is consumed only by the av1 decoder — codecs/src/video/av1/decoder.rs:792 `i32::try_from(cfg.threads.threads())` — reachable via IVF/MP4 av1 tracks, though the engine always passes default(1).) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read tool full file; `rg "cfg.threads\|config.threads"` over codecs → single consumer (av1/decoder.rs:792); `rg '"av1"\|"av01"'` over demux/ivf + containers → IVF mints av1 tracks (apps/cli/src/containers/ivf.rs:260-282, mp4.rs:239); Read pipeline.rs:1695-1710 (engine always passes `ThreadBudget::default()`); Read media-types/src/packet.rs:246-255 (SharedBytes Debug caps at 8 bytes — no log firehose via `extra_data` Debug); baseline log :243-245 (3/3 config tests ok) |

## 1. One-paragraph verdict
A small, disciplined config surface: ThreadBudget enforces its 1..=256 invariant at construction with a typed error, both config structs are assembled from self-validating types only (the "cannot fail" claim is true — every field is a validated newtype), and the Debug impls deliberately print the negotiated accelerator's backend id instead of a pointer, with extra_data rendered through SharedBytes' 8-byte-capped Debug (verified — no payload firehose in diagnostics). Tests are tight (bounds matrix including u32::MAX, Debug-content assertions on both software and accelerated configs). The one defect is a stage mislabel in ThreadBudget's error: it always reports `PipelineStage::Decode` even when the budget is being constructed for an `EncoderConfig`. Everything else checks out.

## 2. What actually works (verified)
— 3/3 in-file tests ok in baseline (test-workspace.log.txt:243-245).
— `ThreadBudget::new` bounds: `(1..=MAX_THREADS).contains` — both edges (0 and 257 and u32::MAX rejected, 1 and 256 accepted) test-pinned; `Default` = 1 (trivially inside the invariant).
— Reachability executed: pipeline.rs:1702-1708 constructs `DecoderConfig::new(...)` on every decoder obtain — ran in all baseline smoke sessions.
— Debug honesty: accelerator → `backend_id()` string (117-133); `extra_data` → SharedBytes Debug (media-types/src/packet.rs:246-255) which caps at 8 hex bytes of the view — verified, so a config Debug dump cannot spill a large avcC/hvcC blob into logs.

## 3. Findings

### F-001 [S4]
Where: crates/codec-api/src/config.rs:45-54
Code:
```
    pub fn new(threads: u32) -> Result<ThreadBudget, EngineError> {
        if (1..=Self::MAX_THREADS).contains(&threads) {
            Ok(ThreadBudget { threads })
        } else {
            Err(EngineError::new(
                PipelineStage::Decode,
```
What is wrong: the typed error's stage is hard-coded `PipelineStage::Decode`, but `ThreadBudget` is shared by `EncoderConfig` (config.rs:150 — "Helper-thread budget") — a rejected encoder-side budget reports the failure as a Decode-stage error. No encoder lane constructs a budget today (the encoder surface is Phase-7 `[PROPOSAL]`), so the mislabel is latent.
Why it matters: error-stage taxonomy is load-bearing in this repo (the engine's `per_frame_damage` classifier and session error routing key off kind/stage); a wrong stage would misroute an encoder misconfiguration in diagnostics.
What correct looks like: either a `stage` parameter on `new` (callers know their lane), or a neutral stage (`PipelineStage::Decode`→ `PipelineStage::Codec` if the enum has one, else parameterize).
Confidence: HIGH — Verification: read; PipelineStage enum variants not enumerated (media-types — outside my files), so the "no neutral variant exists" possibility is UNVERIFIED.
Confidence note: the mislabel itself is read-proven; only the best fix shape depends on the enum.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ThreadBudget` struct + invariant docs | 28-31 | REAL | — | 1..=256 documented at :26-27 |
| `ThreadBudget::MAX_THREADS` | 37 | REAL | — | 256, `[PROPOSAL]` marked |
| `ThreadBudget::new` | 45-55 | REAL | S4 | F-001 stage mislabel; bounds correct |
| `ThreadBudget::threads` | 58-60 | REAL | — | getter |
| `impl Default` | 63-69 | REAL | — | 1 — inside invariant |
| `DecoderConfig` struct | 78-95 | REAL | — | pub fields, all self-validating types; extra_data honestly `[PROPOSAL]`-documented (:90-93) |
| `DecoderConfig::new` | 100-114 | REAL | — | infallible by construction — claim verified |
| `DecoderConfig::Debug` | 117-134 | REAL | — | backend_id not pointer; extra_data via capped SharedBytes Debug (verified) |
| `EncoderConfig` + `new` + `Debug` | 143-186 | REAL | — | mirror without extra_data, rationale documented (:139-142) |
| `MockAccelerator` (test) | 192-202 | REAL | — | minimal 2-method impl |
| `thread_budget_bounds_are_enforced` | 204-225 | REAL | — | strong: both edges + kind/code/stage asserts |
| `decoder_config_assembles_and_debugs_its_negotiation` | 227-259 | REAL | — | asserts both Debug variants' content; `rendered.contains("4")` (:258) is a weak pin (any stray '4' passes — e.g. a generation counter of 4) but harmless |
| `encoder_config_mirrors_decoder_config` | 261-276 | REAL | — | structure + Debug content |

## 5. Test honesty
3/3 ok in baseline (:243-245).
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| thread_budget_bounds_are_enforced | YES | accepts/rejects matrices incl. u32::MAX; asserts kind + code + stage |
| decoder_config_assembles_and_debugs_its_negotiation | YES (mostly) | field equality + Debug contains "software"/"h264"/"hw.mock"; the "4" assertion is weak (see table note) but the rest are specific |
| encoder_config_mirrors_decoder_config | YES | codec/generation/threads equality + Debug shape |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every field is a self-validating type, so a `DecoderConfig` that exists is a config the codec may trust" | config.rs:8-10 | REAL | all five fields are validated newtypes/validated-constructing types (CodecDescriptor, GenerationId, ThreadBudget, Arc<dyn HwAccelerator>, SharedBytes) |
| "Invariant: `1..=MAX_THREADS` — never zero … never unbounded" | config.rs:25-27 | REAL | :46 range check; test-pinned at both edges |
| "assemble a negotiated decoder config … this cannot fail" | :98-99 | REAL | all inputs already-validated types; constructor is pure field moves |
| "H.264/HEVC receive one [accelerator] per ADR-001 §1 when the platform provides an audited adapter" | :86-88 | REAL-UNPROVEN | no audited adapter exists in-tree (product.toml hw-decode=false:168; my hevc audit); the field is plumbed but always None in shipped engine paths (pipeline.rs:1706) |
| "no encoder lane today" (implicit in EncoderConfig `[PROPOSAL]` docs) | :139-142 | REAL | no Encoder consumer in the tree (encode.rs surface is Phase-7) |

## 7. Dead / duplicated / unreachable code
`EncoderConfig` + its `new`/`Debug` have zero production consumers (no encoder implementations exist in the tree) — Phase-7 `[PROPOSAL]` surface, honestly documented, exercised only by its own test. Not dead by deception; noted for the wave's dead-code census. The `ThreadBudget` non-default range is unreachable in shipped builds (engine always passes `default()`; only av1 reads it — with value 1).

## 8. Dependencies & flags
`crate::hw::HwAccelerator`, media-types (ErrorContext/CodecDescriptor/SharedBytes/EngineError/GenerationId/PipelineStage). No external deps, no unsafe, no feature gates. The `Arc<dyn HwAccelerator>` field is the ADR-001 seam — product.toml `hw-decode = false` keeps it None in shipped builds, matching.

## 9. Open questions for the next agent
- hw.rs owner (mine, next file): does any shipped path ever construct an `HwAccelerator`? (Expected NO per product.toml — will answer there.)
- The av1 decoder is the only `cfg.threads` consumer (av1/decoder.rs:792); if the engine ever grows a threads setting, that lane becomes the only one honoring it — the dnx module docs already state other decoders ignore the budget by design (single-threaded admission records).
