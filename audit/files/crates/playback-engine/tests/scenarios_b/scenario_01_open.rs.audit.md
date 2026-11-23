# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_01_open.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-23 |
| Lines / functions | 144 / 1 (one #[test]) |
| File verdict | REAL for variants 1+3, SHALLOW for variant 2 (the MP4 refusal pins the harness's own hardcoded probe, not the shipped product's behavior) |
| Reachable from shipped entry point? | N/A (test file) — drives the real engine via scenarios_b/host.rs `dispatch`; variant 2's refusal is produced by the HARNESS's local sniff (host.rs:897–926), which the crate cannot avoid (no dev-dep on media-probe/apps) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `./target/debug/deps/scenarios_b-3501eaa5a90abc4a open_answers` → `1 passed; 0 failed` in 0.01s (prebuilt; cargo blocked by full disk); fixture existence verified (`tests/fixtures/generated/valid/{avi_mjpeg_pcm.avi,mp4_mjpeg_pcm.mp4}` present) |

## 1. One-paragraph verdict
Variant 1 (valid AVI parent) is a strong, honest test: real fixture (425,516 B, real MJPEG+PCM interleave per its structure.json), accept with session+generation, settle to `Playing`, the wire track list read back and asserted (2 tracks, video+audio, both selected), duration exactly the declared 2,000,000 µs with exactly one `timing.duration-known`, clean close. Variant 3 (absent path) pins the typed Input/Inaccessible refusal with the basename surviving redaction and no session surviving. Variant 2, however, asserts "an mp4 must refuse in this build" — true only of THIS HARNESS's local sniff (host.rs:911–926 hardcodes "first vertical: AVI, D-018"), and false of the shipped product: the real CLI wires an mp4 container (apps/cli/src/containers/mod.rs:67 `"mp4" => mp4::open(stream, opts)`), and I executed `lumen open sample.mp4` walking to `Playing` (exit 0) in the session.rs audit. The scenario enshrines a stale D-018 claim as if it were product behavior.

## 2. What actually works (verified)
- `./target/debug/deps/scenarios_b-3501eaa5a90abc4a open_answers` → `test scenario_01_open::open_answers_honest_facts_or_typed_refusals ... ok` (executed).
- Fixtures real and present: `avi_mjpeg_pcm.avi` (425,516 B — its structure.json maps 120 real movi chunks) and `mp4_mjpeg_pcm.mp4` under tests/fixtures/generated/valid/ (verified by ls; `fixture_path` would fail the test otherwise).
- Mutation reasoning (variants 1+3): a wrong track list, a duration off by a tick, a missing/duplicated `duration-known`, a surviving session after refusal, or an unredacted error shape each fail an exact assert.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/scenarios_b/scenario_01_open.rs:103–125 (with the harness's refusal at host.rs:911–926)
Code:
```
    // 2. The MP4 parent: probe-stage refusal, typed — this build's
    //    first vertical serves AVI only (D-018).
```
and the assertion:
```
        Ok(_) => unreachable!("an mp4 must refuse in this build"),
```
What is wrong: the refusal under test is the HARNESS's own `probe_refusal` (host.rs:911–926: `format \`mp4\` has no demuxer in this build (first vertical: AVI, D-018)`), a locally-mirrored sniff the crate needs because it cannot dev-depend on media-probe — but the comment asserts it as "this build"'s behavior, and the shipped build contradicts it: apps/cli/src/containers/mod.rs:67 wires `"mp4" => mp4::open(stream, opts)` (plus wav/aiff/adts/ogg/ivf/mkv/ts/ps/es), and the executed mp4 smoke (`lumen open sample.mp4` → `Playing`, exit 0) proves MP4 opens in the release binary. Why it matters: doc 10 §6.4 row 1's "refusal variants" evidence, read at face value, claims MP4 is unsupported by the product; it is unsupported only by this test harness's class-B AVI scope. What correct looks like: the comment should say "this HARNESS's first-vertical scope serves AVI only (D-018-era scope); the shipped host wires mp4 and its refusal behavior is tested in apps/cli" — or the variant should be dropped/moved to the CLI's own suite. Confidence: HIGH — Verification: read + executed (both this test and the shipped binary's mp4 open). Blast radius: evidence quality (misleading row-1 evidence), not runtime behavior.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open_answers_honest_facts_or_typed_refusals` | 22–144 | REAL (v1, v3) / SHALLOW (v2) | S3 | v1: accept + settle + wire track list + exact duration + single duration-known + close; v2: F-001; v3: typed Input/Inaccessible, basename survives, no session, Idle |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| v1 (valid parent) | YES | exact track kinds/selection, exact µs duration, count of duration-known == 1, Playing settle |
| v2 (mp4 refusal) | NO — for the PRODUCT | the refused path is harness-local code; the shipped product opens MP4s; only a change to the harness's own sniff/probe_refusal would flip it (F-001) |
| v3 (absent path) | YES | stage/code/message-content/no-session/Idle all pinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the MP4 parent (same essence, unserved container) refuses at the PROBE stage, typed — never a garbage success" | scenario_01_open.rs:9–11 | TRUE in the harness, FALSE as a product claim | harness refusal at host.rs:925; product opens mp4 (containers/mod.rs:67; executed smoke) — F-001 |
| "this build's first vertical serves AVI only (D-018)" | scenario_01_open.rs:103–104 | FALSE for the shipped build | 10 container families wired in apps/cli/src/containers/mod.rs |
| "the valid parent answers Accepted with the honest track list … a duration-known of exactly the declared 2.000 s" | scenario_01_open.rs:12–16 | REAL | executed pass with exact asserts |

## 7. Dead / duplicated / unreachable code
- None in this file (the duplication is the host's mirrored sniff — host.rs's disclosure, see F-001).

## 8. Dependencies & flags
- Test-only; sibling host/main via `crate::`; commands, testkit. No #[ignore] (23 tests in the binary, this one runs). Executed via prebuilt binary because cargo is disk-blocked (see log).

## 9. Open questions for the next agent
- Whoever owns apps/cli's container suite: is there a CLI-side test asserting the mp4 OPEN path (the product truth) to counterbalance this harness-scoped refusal? If not, row-1 evidence is one-sided.
- scenario_10_missing.rs (the absent-path family) is the deeper variant-3 suite — not my assignment.
