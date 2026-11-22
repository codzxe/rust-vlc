# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 392 / 1 test fn + 6 row/patch builders |
| File verdict | REAL for the AVI matrix; the MP4 matrix rows assert a fictional host |
| Reachable from shipped entry point? | N/A (test) — plain `#[test]` + `serialize_tests`; part of W00 baseline (execution UNVERIFIED, no cargo). |
| Findings | S0:0 S1:0 S2:1 S3:0 S4:1 |
| Commands run | Read; cross-check of the "same typed answer the real host gives" claim against apps/cli/src/demux_host.rs:444-458 (probe → containers::open) and apps/cli/src/containers/mod.rs:54/67 (the registry serves both "avi" and "mp4"); sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The corrupt-media matrix (doc 10 §6.4 row 9): 13 committed malformed AVI fixtures + 3 byte-level temp-copy corruptions, each pinned to a specific observed flavor (typed refusal / bounded containment with exact damage counts and frame floors-and-ceilings / first-read fatal), plus universal invariants (bounded pumps, exactly one end-of-media or exactly one fatal, clean close per session) and a double-run memory-neutrality proof through the counting allocator with the one-time warm-up delta explained inside the assertion message itself. The AVI half is excellent, flavor-pinned evidence. The MP4 half is fiction: all 10 mp4 rows pin `probe.unsupported` — "the same typed answer the real host gives an mp4", says the comment — but the production host's container registry **serves mp4** (`containers/mod.rs:67: "mp4" => mp4::open(stream, opts)`); the real host probes mp4 successfully and dispatches into the mp4 demuxer. The suite's local sniff (host.rs:899-928) is a stale mirror of an AVI-only era, so the mp4 rows prove the harness refuses what the product attempts.

## 2. What actually works (verified)

- Flavor pinning (38-52, 273-302): three-way enum match with an explicit `unreachable!` on flavor drift — a demuxer that starts recovering where it used to refuse fails the row instead of hiding behind the union contract. The doc header (7-11) states this doctrine.
- Refusal arm invariants (225-241): state `Idle`, no session, zero pipeline events — the refusal leaves nothing behind.
- Contained arm (243-257, 278-294): exact damage-event count, frame floor AND ceiling (≤121), exactly one `playback.end-of-media`.
- Fatal arm (259-262): exactly one fatal event.
- Memory neutrality (327-391): clean-parent warm-up, then the matrix run twice; the assertion demands live-bytes EXACT equality across runs, with the first run's one-time delta over baseline named in the failure message rather than hidden. Any per-session leak in the corrupt paths fails this.
- Bounded everywhere: `run_until_terminal(512)` with pumps-spent in messages; `serialize_tests` for the allocator.
- The committed-fixture row names (`generated/malformed/avi_mjpeg_pcm__*.avi`) resolve through testkit's fixture store (`.must()` — missing fixture fails loudly; corpus existence is W16's audit).
- The per-row comments (92-135) cross-reference the demux-level contract tests by name (`trunc2_at_first_movi_chunk_starts_but_errors_at_the_boundary`, `flip1_advisory_field_flip_demuxes_identically`, etc.) — a real traceability trail between engine-level flavors and demux-level contracts.

## 3. Findings

### F-001 [S2] The MP4 matrix pins a refusal the production host does not give
Where: crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:139-153 (and the local sniff at host.rs:895-928); production: apps/cli/src/demux_host.rs:444-458 + apps/cli/src/containers/mod.rs:67
Code (scenario_09:139-141):
```rust
/// The committed class-B MP4 matrix: every row refuses at the probe
/// stage (this build's first vertical serves AVI only, D-018 — the
/// same typed answer the real host gives an mp4).
```
Code (scenario_09:148-151):
```rust
    .map(|op| Row {
        fixture: format!("generated/malformed/mp4_mjpeg_pcm__{op}.mp4"),
        expect: Pinned::Refused("probe", "unsupported"),
```
Code (apps/cli/src/containers/mod.rs:67):
```rust
        "mp4" => mp4::open(stream, opts),
```
What is wrong: The claim "the same typed answer the real host gives an mp4" is FALSE. The production open path probes the stream (media-probe registry reports format "mp4"), then dispatches through the container registry, which **serves mp4** — `mp4::open` constructs the demux-mp4 stage. The production host never produces `probe.unsupported` for a valid `ftyp` signature. The 10 MP4 rows therefore assert the behavior of the suite's own stale local sniff, not of any shipped binary; as acceptance evidence for doc 10 row 9's MP4 half they are void.
Why it matters: The suite is cited (docs/scenarios/scenarios-b-report.md, per the header) as the class-B corrupt-media evidence. Half its matrix is a green test against a fictional host — the exact parallel-implementation hazard filed as host.rs F-001, now with a proven divergence instance. A user feeding a corrupt mp4 to the shipped CLI gets demux-stage behavior (typed demux errors, possibly partial decode), not the probe refusal this suite pins.
What correct looks like: Either drop the mp4 rows (the suite cannot serve mp4 by construction) or mark them explicitly "local-host-only flavor" and route real mp4 corrupt-media acceptance to a suite that drives the production host (the same [PROPOSAL] as host.rs F-001).
Confidence: HIGH — Verification: read (both dispatch tables).
### F-002 [S4] Pinned frame ceilings/floors are magic numbers tied to one fixture generation
Where: crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:99-135, 291
Code (291):
```rust
            assert!(
                *frames <= 121,
                "{name}: bounded recovery, not a runaway ({frames} frames)"
```
What is wrong: The ceiling 121 (parent = 120 frames) and the per-row floors (59/60/100/118) are hard-coded against the current committed fixture bytes. Regenerating the corpus changes the arithmetic and turns the pins into failures that read like regressions. The doctrine (flavor pinning, "drift is a failure, not a re-pinning" — 271-272) makes this a feature, but nothing in the file states the numbers' provenance (fixture frame count) except inference.
Why it matters: Maintainability/honesty of the pins: a reader cannot tell whether 121 is a policy bound or an accident of the fixture.
What correct looks like: Name the parent frame count once (`const PARENT_FRAMES: u64 = 120;`) and derive the ceiling, with the floors annotated per row (they already carry per-row comments — add the arithmetic).
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `Pinned`/`Observed`/`Row`/`Patch` types | 38-163 | REAL | The pinning vocabulary; owned `String` fixture names with the leak-rationale comment (56-60). |
| `avi_rows` | 73-137 | REAL | 13 rows, per-row provenance comments citing demux contracts. |
| `mp4_rows` | 142-153 | FICTION | F-001: asserts a local-host-only refusal. |
| `patches` | 174-212 | REAL | 3 byte-level corruptions with documented offsets and rationale (advisory-field philosophy). |
| `open_and_drive` | 216-268 | REAL | Bounded drive-to-terminal with per-flavor invariants. |
| `assert_row` | 273-302 | REAL | Flavor-drift `unreachable!`; exact counts; floor+ceiling. |
| corrupt_media_is_typed_bounded_and_memory_neutral | 307-392 | REAL | Warm-up + double-run memory proof; S09_ONLY debug knob does not gate the default run. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| corrupt_media_is_typed_bounded_and_memory_neutral | YES (AVI half) | Flavor drift → unreachable!/assert fails; leak → second-run live-bytes assert; hang → 512 bound breach; runaway decode → ceiling 121. NO for the MP4 rows — they test the local host's own sniff, which cannot regress independently of the suite. |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every one driven through the command layer" (2-3) | module doc | SHALLOW | Through the LOCAL command layer (host.rs F-001); the mp4 rows' command layer answers a refusal the production layer does not. |
| "the matrix is the evidence in `docs/scenarios/scenarios-b-report.md`" (14-15) | module doc | UNVERIFIED | The report doc is W19's claim-ledger territory; the mp4 rows' evidence value is void regardless (F-001). |
| "the same typed answer the real host gives an mp4" (140-141) | comment | FALSE | containers/mod.rs:67 serves mp4; F-001. |
| "any drift in recovery behavior fails loudly instead of hiding behind the union" (9-11) | module doc | REAL | assert_row's drift arm + exact pins. |
| "memory-neutrality is the SECOND run returning exactly to the first run's level" (332-334) | comment | REAL | 381-391 implements exactly that. |

## 7. Dead / duplicated / unreachable code

`mp4_rows` (F-001) — ten rows of pinned fiction. The `S09_ONLY` knob is a debug affordance, not dead code.

## 8. Dependencies & flags

Harness + commands + testkit (+ committed class-B fixture corpus). `S09_ONLY` env filter is opt-in; default runs everything. Counting allocator discipline via main.rs.

## 9. Open questions for the next agent

- W05 (demux-mp4) + W13: what DOES the shipped CLI answer for each `mp4_mjpeg_pcm__{op}.mp4` fixture? That is the real MP4 corrupt-media matrix nobody has pinned — this suite's mp4 rows must not be cited as it.
- W19: docs/scenarios/scenarios-b-report.md — does it present the mp4 rows as product behavior? If yes, that report inherits F-001's FALSE claim.
