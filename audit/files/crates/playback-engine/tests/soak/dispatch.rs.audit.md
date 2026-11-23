# AUDIT: crates/playback-engine/tests/soak/dispatch.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-23 |
| Lines / functions | 1058 / 42 (incl. 2 unit tests) |
| File verdict | REAL (test-harness dispatch layer; exercises the real engine over the real AVI/MJPEG/PCM stack) |
| Reachable from shipped entry point? | N/A (test harness) — reachable from the `soak` test target: tests/soak/main.rs `#[test] soak_loops_stay_memory_bounded` → scenario.rs:403/432/466/546 `lumen_soak_dispatch` → this file |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p playback-engine --test soak unserved` → 1 passed; `cargo test -p playback-engine --test soak seek_without_session` → 1 passed |

## 1. One-paragraph verdict
This is the soak's ARCH-5 dispatch mirror: it exists because a playback-engine test cannot link `lumen-cli`'s `lumen_dispatch` (dependency direction), and the module doc records that lineage honestly. It drives the REAL engine (`Engine::open`/`poll`) over the REAL first-vertical stack (`input::FileAccess` → `AviDemuxer` → real `MjpegDecoder`/`PcmDecoder`), with a frozen virtual clock for determinism and a wall-clock memory sampler for the leak measurement. Bounded awaits everywhere (`AWAIT_POLLS = 2048` — a hang is a failed test, not a timeout mystery), typed refusals for every unserved command, and a load-bearing one-drain rule documented with the harness bug it fixed. Both unit tests pass (executed). The weaknesses are latent fixture-coupling: the provider dispatches on track KIND, not codec, with a silent PCM fallback.

## 2. What actually works (verified)
- `cargo test -p playback-engine --test soak unserved` → `test dispatch::tests::unserved_commands_refuse_typed ... ok`; `cargo test -p playback-engine --test soak seek_without_session` → `test dispatch::tests::seek_without_session_refuses ... ok` (both executed, exit 0).
- The full-stack path is real: `open_source` (590–694) constructs `input::FileAccess::new()` → `AviDemuxer::new(stream)` → harvests `strf` PCM facts → `SessionConfig` with frozen clock → `engine.open(config)` → bounded `pump_until` settle — no mocks of the thing tested anywhere in the file.
- Unit sanity: wire `Timestamp` is µs (commands/src/catalog/types.rs:8 "integer **microseconds**"), and `dispatch_seek` converts `time.0.saturating_mul(1000)` (µs→ns) and `delta_ms.saturating_mul(1_000_000)` (ms→ns) — units correct.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/soak/dispatch.rs:211–239
Code:
```
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        match track.kind {
            MediaKind::Video => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
```
and 224–229:
```
                let (channels, rate) = self
                    .pcm_facts
                    .iter()
                    .find(|(handle, _)| *handle == track.id.handle())
                    .map(|(_, facts)| *facts)
                    .unwrap_or((2, 48_000));
```
What is wrong: the provider picks decoders by track KIND (any Video → MJPEG, any Audio → PCM) with a silent `(2, 48_000)` fallback when facts are missing. For the current AVI/MJPEG+PCM fixtures this is correct, but a future fixture with any other codec silently wires a wrong decoder and the soak would measure memory on garbage decode, and a facts-harvest regression (missing `strf`) would be masked by the fallback (frames still decode, wrong geometry, no refusal). Why it matters: the soak is the bounded-memory evidence surface; a mis-wired limb changes the allocation behavior it measures. What correct looks like: match on `track.codec` and refuse typed on unknown codecs / missing facts (the `apps/cli` provider shape). Confidence: HIGH — Verification: read. Blast radius: test harness only (fixtures constrain it today).
F-002 [S4]
Where: crates/playback-engine/tests/soak/dispatch.rs:116–130 (CountingSink docs) and 152–170 (`accept`)
Code:
```
/// Frames per track handle (fixtures carry ≤ 4 tracks; handle ≥ 8
/// folds into the last slot — counted, never dropped silently).
```
What is wrong: `let index = (track.handle() as usize).min(7);` folds any track handle ≥ 8 into slot 7 — documented, but `frames_total()` then reports conflated counts for >8-track fixtures. Fixtures carry ≤4 tracks today, so this is a documented constraint, not a defect. Confidence: HIGH — Verification: read.
F-003 [S4 — recorded as a cross-file fact, not a defect of this file]
Where: crates/playback-engine/tests/soak/dispatch.rs:10–14 vs apps/cli/src/dispatch.rs:199–200
Code (this file):
```
//!   `tracks.set-audio-track` (the CLI row still answers "track
//!   switching is card 3.A3", which has landed engine-side; the soak
//!   needs the switch op);
```
What is wrong: nothing in this file — the claim is TRUE and I verified it: the shipped CLI refuses `tracks.set-*` with "track switching is card 3.A3" (apps/cli/src/dispatch.rs:199–200) even though the engine's switch flow is fully implemented (session.rs:2132–2218, executed path). The SHIPPED product cannot do track switching that the engine supports and this soak exercises. Flagging for the apps/cli owner: stale capability refusal at the CLI dispatch layer. Confidence: HIGH — Verification: read both sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FrozenNow::now_ns` | 77–81 | REAL | — | constant 0, disclosed ("never-advancing") — determinism instrument, not a fake clock masquerading as time |
| `Sampling::sample_if_due` | 121–126 | REAL | — | cadence-gated wall sample |
| `CountingSink::frames_total/bytes_total` | 135–143 | REAL | — | counter sums |
| `CountingSink::accept` | 152–176 | REAL | — | counts + drops (no banking); F-002 fold note |
| `SoakProvider::new` | 196–199 | REAL | — | |
| `SoakProvider::decoder_for` | 202–239 | REAL w/ latent hazard | S4 | kind-based dispatch + silent PCM fallback (F-001) |
| `soak_queues` | 250–265 | REAL | — | small bounded caps (64 pkt/1–8 MB/400 ms) |
| `soak_budget` | 268–275 | REAL | — | demux 256 / packets 8 / flush 16 per tick |
| `SoakHost::new` | 318–335 | REAL | — | fresh Engine |
| `SoakHost::install_sampling` | 338–345 | REAL | — | fn-pointer live-bytes reader |
| `SoakHost::end_warmup/note_cycle_complete/force_sample` | 348–374 | REAL | — | window bookkeeping |
| `SoakHost::sampling_elapsed_s/take_sampling` | 379–395 | REAL | — | |
| `SoakHost::state/stats/sink` | 397–420 | REAL | — | accessors over engine truth |
| `SoakHost::next_request` | 423–425 | REAL | — | |
| `SoakHost::drain_outputs` | 437–502 | REAL | — | the one-drain rule: every output processed exactly once; seek/switch terminals recorded; the same-poll terminal bug documented as fixed |
| `SoakHost::pump` | 505–510 | REAL | — | poll + drain + sampler tick |
| `SoakHost::pump_until` | 513–521 | REAL | — | bounded predicate pump |
| `SoakHost::await_receipt` | 523–543 | REAL | — | bounded, typed timeout error |
| `SoakHost::await_seek_terminal` | 546–567 | REAL | — | flag cleared at submit ⇒ same-poll landing seen |
| `SoakHost::await_switch_terminal` | 570–588 | REAL | — | same shape |
| `SoakHost::submit_and_await` | 590–610 | REAL | — | submit → await → truthful-UI outcome |
| `SoakHost::open_source` | 618–694 | REAL | — | real access→demux→facts→open→settle; divergence (no media-probe) recorded; settle-at-first-sighting rationale with measured 1597/1597 note |
| `lumen_soak_dispatch` | 700–712 | REAL | — | the single entry; response carries live session/gen |
| `route` | 718–760 | REAL | — | served subset + typed refusals; close composed by the engine itself |
| `dispatch_seek` | 767–810 | REAL | — | µs/ms→ns conversions correct; Chapter → typed NotFound |
| `dispatch_audio_switch` | 817–849 | REAL | — | Id/None only; Auto → typed refusal (product rule) |
| `wire_error` | 857–864 | REAL | — | stage-prefix strip + published mappings |
| `session_error_wire` | 867–894 | REAL | — | all five SessionError shapes typed |
| `receipt_outcome` | 897–912 | REAL | — | Accepted/Completed/Rejected split |
| `rejection_wire` | 915–981 | REAL | — | all ten RejectionReasons mapped |
| `unserved` | 988–998 | REAL | — | typed UnsupportedForHost naming the harness |
| `file_uri` | 1005–1041 | REAL | — | canonicalize + is_file + UTF-8 + shared constructor |
| tests `unserved_commands_refuse_typed` / `seek_without_session_refuses` | 1046–1058 | REAL | — | executed: both pass; assert error code + stage, would fail on regression |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `unserved_commands_refuse_typed` | YES | asserts UnsupportedForHost + Control stage on set-rate |
| `seek_without_session_refuses` | YES | asserts NotReady without a session |
(The heavyweight honesty lives in main.rs's soak driver + scenario.rs — not this file; this file is the dispatch layer they drive. The soak main test runs by default for 2.0 minutes (`parse_minutes` default at main.rs:97), so it IS part of the baseline 4744-pass run, not an ignored ritual.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the CLI row still answers 'track switching is card 3.A3', which has landed engine-side" | dispatch.rs:10–13 | REAL | apps/cli/src/dispatch.rs:199–200 verbatim "track switching is card 3.A3"; engine-side flow session.rs:2132–2218 (audited, previous file) |
| "a playback-engine test cannot link it (dependency cycle)" | dispatch.rs:7–9 | REAL | lumen-cli depends on playback-engine (apps/cli/Cargo.toml), reverse edge would be cyclic |
| "the one-drain rule is load-bearing (found the hard way)" | dispatch.rs:437–449 | REAL | the seek terminal legitimately lands in the receipt's own poll (session.rs poll applies command then ticks in the same call) — verified in session.rs audit |
| "the memory measurement is REAL wall clock" | dispatch.rs:42–45 | REAL | Sampling uses CadenceSampler wall ticks inside pump; FrozenNow only feeds the engine clock/throttle |

## 7. Dead / duplicated / unreachable code
- None dead. Duplication is acknowledged and justified (the CLI dispatch mirror); the mirrored `wire_error`/`session_error_wire`/`receipt_outcome`/`rejection_wire` shapes duplicate apps/cli/src/host.rs logic by necessity — a drift risk both files' docs acknowledge.

## 8. Dependencies & flags
- Test-only file (`tests/soak/`); dev-deps used: demux, input, codecs, media-clock, packet-queue, commands, testkit — all real crate paths, no feature gates, no #[ignore] anywhere in the soak tree (verified by rg).

## 9. Open questions for the next agent
- scenario.rs / main.rs / memory.rs / report.rs (the actual soak driver, sampler, and slope fit) are NOT in my assignment — the bounded-memory claim (CEILING_MB_PER_H = 50.0, MAX_SKIP_RATE = 0.25 "above this the scenario is a ritual — D-6") rests there.
- Who owns apps/cli/src/dispatch.rs should answer why `tracks.set-*` is still refused in the shipped CLI (F-003).
