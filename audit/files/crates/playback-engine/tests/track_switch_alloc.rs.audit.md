# AUDIT: crates/playback-engine/tests/track_switch_alloc.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-23 |
| Lines / functions | 593 / 11 helpers/fakes + 1 test |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test binary) — `tests/track_switch_alloc.rs` compiles as its own integration test (own `#[global_allocator]`); runs under `cargo test -p playback-engine` (baseline: workspace 4744 passed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/track_switch_alloc-1e8c33ec32cd5d16` -> 1 passed, 0 failed, exit 0 (binary 18:07 newer than source 17:35) |

## 1. One-paragraph verdict
A genuine counting-allocator proof of the decoder-cache claim (card 3.A3: track switches must not re-allocate decoder working memory), and it would fail loudly if the cache broke. The fake decoder allocates exactly one 256 KiB block at construction and nothing ever after; the soak performs 41 alternating audio switches and asserts three INDEPENDENT oracles: the provider's exact build counts (one build per track, ever — the engine cannot fake this), the engine's own `decoders_built`/`decoders_reused` accounting (cross-checked against the provider's), and the global allocator's per-switch churn ceiling of 32 KiB — 8× below one context, so a teardown-and-rebuild cache blows the bound by construction. Band (16 KiB) and creep (1 KiB) bounds catch retention/leak shapes, an instrument-liveness tripwire (`calls > 0`) catches a dead allocator, and every switch asserts real frame delivery plus terminal observation (bounded 20-poll budget, `unreachable!` on miss). Executed: passes. The only gap is coverage, not honesty: the soak alternates AUDIO tracks only — video-track switches and the class-S subtitle seam have no allocation proof.

## 2. What actually works (verified)
- `./target/debug/deps/track_switch_alloc-1e8c33ec32cd5d16` -> `test result: ok. 1 passed; 0 failed` (exit 0).
- Mutation reasoning: cache broken (teardown+rebuild) → build counts 2+ per track (asserts 506-520 fail), `decoders_reused < 40` (524-527 fails), per-switch churn bytes += 256 KiB > 32 KiB bound (570-575 fails). Retention leak → creep > 1 KiB (543-550 fails). Dead instrument → `calls > 0` fails (588-592). No-op soak → frame-growth assert per switch (494-498) fails.
- The measured-window discipline (warm-up 0..=4 excluded, bounds pinned from measurement with stated margins, "re-measure and re-pin honestly rather than weakening the assertion silently" at 54-58) is the correct D-6 practice.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/track_switch_alloc.rs:459-465 (test doc) + 475-489 (the soak loop)
Code:
```
/// The decoder-cache recycling proof: 41 alternating audio switches
/// with (a) exact build counts — one decoder per track, ever; (b) the
/// engine's own `decoders_reused` accounting; (c) bounded memory
```
What is wrong: coverage gap, not a false claim — the doc says "switching tracks must not re-allocate decoder working memory" (module header, 2-4) but the soak drives only `set_audio_track` between audio tracks 2↔3. A video-limb switch path (`set_video_track`) or a switch that recycles the VIDEO decoder is never soaked; if the cache recycling logic has a video-specific rebuild path, this proof is blind to it. (The video decoder is built once at wire and never switched away from in this suite.) What correct looks like: one more soak arm alternating video tracks, or a doc line scoping the claim to audio switches. Confidence: HIGH — Verification: read + ran.
Why it matters: the module-level claim ("switching tracks") is broader than what is proven ("switching audio tracks").

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedNow + impl MonotonicSource | 127-133 | REAL | — | frozen clock, deterministic |
| TallyConsumer + impl FrameConsumer | 137-146 | REAL | — | non-retaining sink (comment: retention would be the test's own growth) |
| MarkerDemuxer + impl Demuxer | 150-189 | REAL | — | round-robin static packets; reanchor resets counters; seek refuses (not the reanchor path) |
| ContextDecoder + impl Decoder | 196-247 | REAL | — | one 256 KiB context at construction, zero after; static-payload frames; stale-generation packets dropped |
| ContextProvider + impl DecoderProvider | 251-289 | REAL | — | counts builds per track id — the independent oracle |
| base / at_millis / track / marker_packet | 295-331 | REAL | — | builders, static payloads |
| soak_queues / soak_budget | 335-353 | REAL | — | 16-packet caps, 18/4/8 tick budget |
| Harness::new | 362-409 | REAL | — | 3 tracks (1 video + 2 audio), injected frozen clock + throttle |
| pump_to_playing | 411-423 | REAL | — | bounded 80 polls, assert Playing |
| settle_switch | 428-453 | REAL | — | bounded 20 polls, drains into caller's reused buffer (D-1), `unreachable!` on no terminal |
| decoder_contexts_survive_switches_without_reallocation | 464-593 | REAL | S4 | the proof itself (ran, passes); audio-only coverage (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| decoder_contexts_survive_switches_without_reallocation | YES | three independent oracles (provider counts, engine stats, allocator churn/band/creep) + per-switch frame delivery + instrument tripwire (ran) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "switching tracks must not re-allocate decoder working memory" (2-4) | module doc | REAL for audio switches / UNPROVEN for video | audio soak passes (ran); video switch path untested here (F-001) |
| "Both oracles are asserted" (35-36) | module doc | REAL | provider build counts (506-520) + allocator churn (565-575) |
| "a broken cache cannot hide under it" (37, 8× margin) | module doc | REAL | 32 KiB bound vs 256 KiB context: any rebuild exceeds 8× |
| measured constants "recorded beside each constant" (54-58) | module doc | REAL | BOUND_BAND/CREEP/CHURN docs carry measured values (4096/0/214 calls/~6 KiB) |

## 7. Dead / duplicated / unreachable code
`#[allow(dead_code)] context: Vec<u8>` (199-200) is intentional (memory stand-in, never read) — honest allow with comment. No other dead code. Duplication: the fake family mirrors replacement_alloc.rs / bounded_alloc.rs discipline (stated, cross-file by design).

## 8. Dependencies & flags
testkit::alloc_count::CountingGlobalAlloc (process-global, hence dedicated binary + serialize_tests), testkit::Must, playback_engine public API, packet_queue, codec_api, plugin_api, media_clock. No env knobs, no #[ignore], no feature gates.

## 9. Open questions for the next agent
- Does `set_video_track` reuse the same cache path as audio? Owner: tracks.rs / track_switch.rs auditors (W03 domain). If video switches realloc, F-001 upgrades to S2.
- testkit::alloc_count instrument itself (CountingGlobalAlloc correctness) is another agent's file — the tripwire here (`calls > 0`) is a liveness check, not a calibration proof.
