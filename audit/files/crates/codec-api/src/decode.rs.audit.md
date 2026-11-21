# AUDIT: crates/codec-api/src/decode.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-21 |
| Lines / functions | 279 / 5 production items (trait `Decoder`, `DecodeOutcome`, `RecoverReason`, trait `FrameSink`, `drop_stale_packet`) + 2 tests + 1 compile_fail doctest |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the decode contract itself: `pipeline_host.rs` builds `Box<dyn Decoder>` (e.g. :243 mjpeg arm), engine calls `limb.decoder.decode(packet, &mut sink)` (`playback-engine/src/pipeline.rs:2362`); `drop_stale_packet` is the first statement of all 16 decoders (grep census below). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "drop_stale_packet" --type rust crates/ apps/` (16 decoder call sites + engine comment); `rg -n "HwFallback"` (constructed only in tests); `rg -n "catch_unwind" --type rust crates/ apps/` (only testkit); `sed -n '2350,2385p' playback-engine/src/pipeline.rs` (bare decode call); `rg -n "profile\|panic" Cargo.toml` (no panic=abort); `sed -n '237,256p'` + `tail -30` of test-workspace.log.txt (unit tests pass; run aborts before doc-tests) |

## 1. One-paragraph verdict
The decode-side contract trait, outcome enum, recovery vocabulary, sink, and the stale-packet guard — the exact seam every shipped decoder implements. The surface is disciplined: `#[must_use]` on outcomes backed by a workspace `unused_must_use = "deny"` lint, a compile_fail doctest pinning the `Send` supertrait, and the stale-packet convention is genuinely honored by every one of the 16 in-tree decoders (grep-proven). Two defects: the binding claim that "the engine treats a panicking decoder as an `ErrorKind::Decoder` fault and discards the instance" is PAPER-ONLY — there is no `catch_unwind` in the engine or any host, the decode call at pipeline.rs:2362 is bare, and the host code itself anticipates poll-thread panics via mutex poisoning, i.e. a panic kills the session rather than being caught; and `RecoverReason::HwFallback` documents an ADR-001 hw→sw fallback that no code implements (the hw decoder is hardware-only by its own design). The compile_fail doctest — the only executable enforcement of the `Send` claim — is not evidenced in the baseline log (the workspace test run aborted at `zero_alloc_transport` before any Doc-tests section appears).

## 2. What actually works (verified)
— Both unit tests `ok` in baseline (test-workspace.log.txt:251-252).
— Stale-guard adoption census: `rg -n "drop_stale_packet" --type rust crates/ apps/` → first statement of `decode` in aac, mp3, mp2, opus, adpcm, pcm, alac, flac, vorbis, av1, h264/sw, h264/decode, mjpeg, dnx, vp9, hevc — all 16 decoders. The "dropped on sight" contract is real in-tree.
— `#[must_use]` enforcement: workspace `Cargo.toml:53` `unused_must_use = "deny"` — ignoring a `DecodeOutcome` cannot compile.
— `FrameSink` error-propagation contract implemented on the engine side: pipeline.rs:2362's `Err` arm handles typed errors, per-frame damage accounting (2372-2380) — the sink/outcome vocabulary is genuinely consumed.

## 3. Findings
### F-001 [S3]
Where: crates/codec-api/src/decode.rs:71-75
Code:
```
/// * **Panics are contract violations.** A panicking decoder is a
///   decoder defect (the engine treats it as an
///   [`ErrorKind::Decoder`](media_types::ErrorKind::Decoder) fault and
///   discards the instance); corrupt media is
```
What is wrong: The claimed engine mechanism does not exist. `rg -n "catch_unwind" --type rust crates/ apps/` → only `crates/testkit` (test helpers). The engine calls `limb.decoder.decode(packet, &mut sink)` bare (`playback-engine/src/pipeline.rs:2362`) on the synchronous poll thread ("v0 is synchronous and thread-free", pipeline.rs:32). The CLI host even anticipates the real consequence: `apps/cli/src/pipeline_host.rs:1190` "lock means a poll-thread panic already happened" (mutex poisoning). A panicking decoder therefore unwinds through the engine and kills the host session — it is never "treated as an ErrorKind::Decoder fault", the instance is never "discarded" by anyone.
Why it matters: The contract's containment story is docs-only; a plugin decoder (MP §6 registry, future) or any latent panic in an in-house decoder escalates to session death with a poisoned-lock error shape the contract explicitly says cannot happen. Mitigated today by the workspace `unwrap_used = "deny"` lint across in-house decoders — but the contract text describes engine behavior, not lint policy.
What correct looks like: Either the engine wraps the decode call in `catch_unwind` and maps to a typed `ErrorKind::Decoder` fault + instance discard (as documented), or the doc honestly says panics propagate and are a hard session fault in v0.
Confidence: HIGH — Verification: read + grep census (engine, both hosts, no handler).
### F-002 [S4]
Where: crates/codec-api/src/decode.rs:162-164
Code:
```
    /// The hardware path failed and the call fell back to (or re-tried
    /// in) software (ADR-001 hw-first fallback, decode- and encode-side).
    HwFallback,
```
What is wrong: No production code constructs `RecoverReason::HwFallback` (rg census: only `encode.rs:172,176` — inside `#[cfg(test)] mod tests` — and `decode.rs`'s own enum test). The hw `H264Decoder` is hardware-only by design (`h264/decode.rs:2-14`: "no software fallback anywhere in this file"), and the shipped policy is `software-only` (product.toml), so the hw→sw fallback the variant names does not exist anywhere.
Why it matters: Dead vocabulary that implies a resilience mechanism (ADR-001 hw-first fallback) the codebase does not have; a reader auditing hw failure handling will chase a ghost.
What correct looks like: Construct it when a lane actually falls back, or mark the variant `#[allow(dead_code)]` with an honest "reserved for 7.B" note.
Confidence: HIGH — Verification: read + grep census.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `trait Decoder` (+docs) | 86-113 | REAL | F-001 (doc) | 3 methods; `Send` supertrait; compile_fail doctest (24-54) pins !Send rejection — the only executable Send enforcement; NOT evidenced in baseline (workspace test run aborted at zero_alloc_transport before doc-tests; no "Doc-tests" section anywhere in the log — UNVERIFIED). |
| `Decoder::configure` | 92 | REAL | — | Contract: typed rejection; honored (every decoder's configure refuses typed — verified for mjpeg/h264 in my other files). |
| `Decoder::decode` | 101-105 | REAL | — | Bounded-work + sink-propagation contract; engine side consumes outcomes and Err arms correctly (pipeline.rs:2362-2385). |
| `Decoder::flush` | 112 | REAL | — | Drained-then-stays-Drained contract. |
| `enum DecodeOutcome` | 122-144 | REAL | — | `#[must_use]` + Copy/Eq/Hash; `Emitted{frames>=1}` invariant checked by `contract::decode_outcome_agrees_with_sink` (contract.rs; its test `agreement_rejects_lies_and_accepts_truth` ok in baseline :247). |
| `enum RecoverReason` | 153-168 | REAL | F-002 | CorruptInput/ConcealedFrame/Discontinuity constructed by real decoders (e.g. mjpeg concealment → ConcealedFrame; damage accounting in engine :2374-2379); HwFallback dead (F-002). |
| `trait FrameSink` | 186-193 | REAL | — | Deliberately !Send (doc 173-177) — thread-affine sink design coherent with worker-thread model. |
| `drop_stale_packet` | 203-209 | REAL | — | Pure guard; adopted by all 16 decoders (§2); behavior pinned by its test. |
| test `stale_guard_fires_only_for_past_generations` | 230-251 | REAL | — | Past/current/future generation triple; future-generation rule delegated to `Packet::is_stale` (packet.rs, media-types wave) — honest scope note. |
| test `outcome_is_small_and_comparable` | 254-278 | REAL | — | Weak-but-honest derive check (Copy/Eq/distinctness); not a tautology — it pins the derives the engine's match arms rely on. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `stale_guard_fires_only_for_past_generations` | YES | Three-way behavioral assert, not a derive echo. |
| `outcome_is_small_and_comparable` | PARTLY | Pins derives/distinctness only — would not catch a semantic change to Emitted/Drained meaning (that's contract.rs's job, and its suite passed). |
| compile_fail doctest (Send) | YES when run | Compile-fail doctests fail if the code compiles; NOT EVIDENCED in the baseline log (run aborted pre-doc-tests) — UNVERIFIED. |
| Run status | 2/2 unit tests `ok` (baseline :251-252); 16/16 codec-api suite ok (:239-255). | |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the engine treats [a panic] as an ErrorKind::Decoder fault and discards the instance" | decode.rs:71-75 | FALSE (today) | No catch_unwind in engine/hosts; bare call pipeline.rs:2362; host anticipates poisoning (pipeline_host.rs:1190). F-001. |
| "Implementations should run drop_stale_packet as their first statement" | decode.rs:69-70 | REAL | All 16 decoders do (grep census). |
| "Emitted { frames: n } means exactly n accept calls" | decode.rs:118-121 | REAL | `contract::decode_outcome_agrees_with_sink` + suite ok in baseline (:247). |
| "It is *not* an error: the decoder stayed within its bounded work" (Recovered) | decode.rs:148-152 | REAL | Engine treats Recovered as non-fatal damage accounting (pipeline.rs:2374-2379). |
| HwFallback = "hardware path failed and fell back to software" | decode.rs:162-164 | PAPER-ONLY | Zero constructors outside tests (F-002). |

## 7. Dead / duplicated / unreachable code
- `RecoverReason::HwFallback`: dead variant (F-002).
- No other dead code; the file is the contract hub — maximal fan-in (16 decoders, engine, contract suite, encode mirror).

## 8. Dependencies & flags
Deps: `crate::config::DecoderConfig`, `media_types` (EngineError/Frame/GenerationId/Packet). No unsafe, no feature gates, std-only. Lint leverage: workspace `unused_must_use`/`unwrap_used` = deny give the contract teeth in-tree.

## 9. Open questions for the next agent
- W06 contract.rs owners (A04/A05): does the contract suite test `flush`-after-`Err` ("must answer, not panic", decode.rs:76-78)? If untested, that clause is another paper promise.
- Engine wave: confirm whether ANY panic path from a decoder reaches the host today (in-house decoders are lint-blocked from unwrap; the unsafe hw lanes cannot ship panics either since they're unreachable — F-001 may stay latent until the plugin registry lands).
- Whoever owns `media_types::Packet::is_stale`: the guard's future-generation behavior is delegated there — is "future is not stale" pinned by a packet.rs test?
