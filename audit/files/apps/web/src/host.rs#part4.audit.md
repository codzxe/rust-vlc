# AUDIT: apps/web/src/host.rs#part4 (lines 1955-2651, `#[cfg(test)] mod tests`)
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2026-11-21 |
| Chunk boundary | part4 = lines 1955-2651 (the entire `#[cfg(test)] mod tests` block, which runs to EOF; verified `sed -n '1950,1956p'` → `#[cfg(test)]` at 1955, and file ends at 2651 with the module's closing `}`). Parts 1-3 = production lines 1-1954 (natural internal split for the other agents: 1-779 types/consts+WebDecoderProvider, 780-1388 WebHost core+pump, 1389-1954 dispatch/route/session+free fns). Boundary declared in audit/runs/W12-A03.log.md before any host.rs part was audited. |
| Lines / functions | 697 / 19 (1 helper fn + 2 consts + 2 open helpers + 14 test fns; plus 1 cfg-gated import) |
| File verdict | REAL (native lane: all 14 tests executed green in baseline no-fail-fast run; wasm32-target execution of the shipped cdylib: NONE exists anywhere) |
| Reachable from shipped entry point? | Test code — executed by `cargo test`. Ran green in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3115-3128,3200` (all 14 `host::tests::* ... ok`, binary summary `:3203` 111 passed/0 failed). NOTE: the two headline baseline runs (`test-workspace.log.txt`, `test-workspace-allfeatures.log.txt`) ABORTED before lumen-web's test binary ever ran (desktop/first-vertical failures, fail-fast default) — only the orchestrator's `--no-fail-fast` repair run executes this module. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "Running unittests" test-workspace.log.txt` (12 binaries, no lumen_web → abort before web) · `rg -n "lumen_web\|pump_slices\|the_autoplay_gate" orchrun-test-workspace-nofailfast.log.txt` (:3087 binary ran; 14/14 ok) · `rg -n "host::tests::" orchrun-...` (19 hits; 5 are apps/desktop's own host.rs) · Read host.rs 1-210, 459-503, 574-703, 780-860, 920-969, 984-1078, 1120-1389, 1955-2651 (full part + production cross-refs) · `rg -n "WebDecoderProvider"` repo-wide (only host.rs — no other callers/tests) · `rg -n "decoder_for" apps/web/src` (call sites 465/480/488/2272 only) · `cat apps/web/Cargo.toml` (no wasm-bindgen dep; cdylib+rlib; builtin-codecs not default) · `rg -n "lumen-web\|apps/web" .github/workflows/` → NO MATCHES (no workflow names the crate directly; v2 correction from lib.rs audit: the ci.yml "Web size-budget gate" step builds it indirectly — `cargo xtask web-budget --check` runs a real `cargo build -p lumen-web --target wasm32-unknown-unknown --release --locked`, xtask web_budget.rs:76 + size-record.toml:50 `package = "lumen-web"`; baseline #16 gate-web_budget___check.log.txt:4 rebuilt it, raw 1,302,846 B, 33.2 s) · `rg -n "cargo test\|--workspace" .github/workflows/` (ci.yml:234 + ci-full.yml:169 both `--all-features`) · `sed -n '530,560p;640,690p' .github/workflows/ci.yml` (wasm lane = theme-tokens canary only, admits "WHAT IT IS NOT YET") · `sed -n '245,290p' crates/testkit/src/lib.rs` (fixture_bytes reads real path) · `ls tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` (exists) · `rg -n "§3.2\|§4.4\|§5.3" docs/09-web-wasm-target.md` (sections exist). All quotes below verified with the Read tool (sed in this sandbox mangles `#[…]` into `#…]`). |

## 1. One-paragraph verdict
This is a genuinely strong test module: 14 tests drive the REAL open path (blob registry → probe → AVI wiring → engine) over the real first-vertical fixture, assert exact-value outcomes (position≈duration within 5 ms, ONE PresentFrame per pump, freeze-on-pause/park equality, typed refusal codes), and all 14 ran green in the baseline `--no-fail-fast` run — every mutation I reasoned through (no-op pump, unbounded fill_audio, ignored autoplay park, served "unserved" commands, un-lossy render lane) would fail an assertion, not just a hang. The defects are coverage-shaped, not honesty-shaped: the doc comment claiming "the with-codecs contract is pinned by the decoder_for dispatch tests" points at tests that do not exist anywhere in the repo; the one test pinning the SHIPPED decoder-less wasm32 refusal contract is cfg-compiled-out of every CI test lane (all run `--all-features`); the budgets test's flush bound (≤2048) cannot fail on a fixture that produces a few dozen events; and nothing — this module or any other — EXECUTES the wasm32 cdylib that is apps/web's actual product artifact (no wasm-bindgen, no wasm32 test lane; the module IS built for wasm32 in CI via the web-budget gate's rebuild — baseline #16 — but only as a size-measured artifact, never run; see F-004's v2 correction). The module proves the engine core works NATIVELY in the web host's scheduling model; it proves nothing about the browser target.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3087-3203` → `cargo test --workspace --release --locked --no-fail-fast` (full-output repair run, 2026-11-21T08:12:07Z): all 14 `host::tests::*` `ok`, binary summary `test result: ok. 111 passed; 0 failed; 0 ignored` (:3203), including `pump_slices_play_the_fixture_to_its_end` (end-to-end play to `Ended`, both limbs delivering) and `wasm32_profile_decoder_seam_refuses_typed` (default features, so the cfg gate was active).
— Verified by Read of production code each assertion targets: `pump()` (host.rs:1120-1138, cap + per-tick flush), `fill_audio()` (1199-1255, quantum-bounded, parked/paused early-returns), `present_one()` (1145-1166, one frame per pump), `handle()` boot gate (1002-1010), `position_us()` (926-931, clock-derived) — the tests assert on real semantics, not mocks.
— Verified `fixture_bytes("generated/valid/avi_mjpeg_pcm.avi")` reads a real tracked fixture (testkit lib.rs:245-247; file exists; same fixture the baseline `decode-bench` decoded 60/60 frames — INDEX.md AVI contrast row).

## 3. Findings
### F-001 [S3]
Where: apps/web/src/host.rs:2233-2243
Code:
```
    /// pins the decoder-less contract; the with-codecs contract is
    /// pinned by the decoder_for dispatch tests.
    #[cfg(not(feature = "builtin-codecs"))]
    #[test]
    fn wasm32_profile_decoder_seam_refuses_typed() {
```
What is wrong: The comment asserts test coverage that does not exist. `rg "WebDecoderProvider"` over the whole repo (excluding audit/) returns ONLY host.rs — the sole test anywhere constructing the provider or calling `decoder_for` is this same refusal test (host.rs:2272). There are no "decoder_for dispatch tests": no test exercises the with-codecs wasm32 arm (host.rs:478-481) or any of the cfg-gated codec rows (host.rs:613-679) through the dispatch path; capabilities.rs:359 `decoder_claims_match_the_profile` checks only the descriptor CLAIMS, not construction.
Why it matters: A reader auditing coverage for the tree-shake lanes (the entire 7.K1 feature surface) is told it's pinned; it is pinned to nothing. Under `--all-features` (what CI runs) this test is also cfg'd out (F-002), so on the exact lane where the with-codecs arm compiles, zero `decoder_for` coverage exists.
What correct looks like: Either a test that boots `WebDecoderProvider::new(TargetProfile::Wasm32, …)` under `#[cfg(feature = "builtin-codecs")]` and asserts a decoder constructs for mjpeg/pcm, or the comment states plainly: the with-codecs dispatch lane has no test coverage in v1.
Confidence: HIGH — Verification: read + repo-wide rg (all WebDecoderProvider/decoder_for call sites enumerated).

### F-002 [S3]
Where: apps/web/src/host.rs:1964-1965, 2242 (with .github/workflows/ci.yml:234, ci-full.yml:169)
Code:
```
    #[cfg(not(feature = "builtin-codecs"))]
    use testkit::assert_err;
```
(and the same gate on the test at 2242)
What is wrong: The only test pinning the SHIPPED wasm32 module's core behavior — the typed `web.decoders_absent` refusal (the "honest capability snapshot" the whole web story rests on, per docs/web/web-column-pass-20261110.md) — is compiled out whenever `builtin-codecs` is on. Every real CI test lane runs `cargo test --workspace --all-features` (ci.yml:234, ci-full.yml:169), which enables `builtin-codecs` → the test never runs in CI. It ran only in baseline default-feature runs (orchrun…:3128) and would run in the vestigial unpinned template workflow `.github/workflows/rust.yml:26` (`cargo test --verbose`, default features — the one repo workflow still on `actions/checkout@v4`).
Why it matters: A regression in `decoders_absent_error()` (host.rs:418) or the refusal arm (host.rs:482-485) — the exact contract that keeps the web column honest — would pass every gated CI lane. The contract is enforced only by lanes nobody treats as the gate.
What correct looks like: Duplicate the pin without the feature gate (e.g. a `builtin-codecs`-gated twin asserting the with-codecs arm answers, so BOTH halves run under `--all-features`), or add a default-features test job to CI.
Confidence: HIGH — Verification: read + workflow greps (ci.yml:234/ci-full.yml:169 `--all-features`; rust.yml:26 default) + baseline log cite.
### F-003 [S4]
Where: apps/web/src/host.rs:2069-2082
Code:
```
    fn pump_ticks_are_capped_and_flushes_bounded() {
        let mut host = WebHost::new(TargetProfile::Native);
        open_fixture(&mut host);
        let report = host.pump(u32::MAX);
        // events_flushed ≤ max_ticks_per_pump × events_per_flush.
        let budgets = host.budgets();
        let bound = u64::from(budgets.max_ticks_per_pump) * u64::from(budgets.events_per_flush);
```
What is wrong: The bound (default 32×64 = 2048, host.rs:164-173) cannot be violated by this fixture — the ~2 s AVI yields a few dozen wire events total, so even a pump that flushed the entire backlog in one tick passes. The tick-cap half is enforced by hang-not-assert: `pump()`'s loop (host.rs:1123) has no early exit, so removing the `.min(max_ticks_per_pump)` cap at 1121 makes `host.pump(u32::MAX)` iterate ~4.29e9 times — a timeout, not a red assertion. The test name promises "flushes bounded"; the body cannot detect an unbounded flush.
Why it matters: False assurance about a real contract (doc 09 §3.2's "one message can never hog the Worker loop").
What correct looks like: `host.set_budgets(…)` (exists, host.rs:864) with tiny bounds (e.g. events_per_flush=1, max_ticks_per_pump=2), then assert the flush splits across ticks and the tick count is capped — an assertion that fails on mutation.
Confidence: HIGH — Verification: read (pump 1120-1138, flush_events 1339-1353, defaults 164-173).
### F-004 [S4]
Where: apps/web/src/host.rs:2025, 2071, 2088, 2116, 2157, 2212, 2232, 2282, 2304, 2332, 2350, 2364, 2382, 2408, 2451, 2472, 2592 (every `WebHost::new(TargetProfile::Native)`)
Code:
```
    fn pump_slices_play_the_fixture_to_its_end() {
        let mut host = WebHost::new(TargetProfile::Native);
```
What is wrong: All 14 tests run the Native profile; the Wasm32 profile — the one the shipped cdylib actually runs (`build_profile()` cfg, capabilities.rs:138) — is constructed exactly once, in the decoder-seam refusal test. No test boots/drives `WebHost::new(TargetProfile::Wasm32)` through open/pump/present. Mitigations verified: profile is data (decoder wiring + capability snapshot only), and capabilities.rs:344/359 covers both profiles' snapshots — but the wasm32 profile's pump lane has zero as-profiled coverage, and NO test anywhere executes ON the wasm32 target itself (no wasm-bindgen-test dependency, Cargo.toml; doc 09:84's planned `cargo test --target wasm32-unknown-unknown` lane does not exist; CI's only DIRECT wasm cargo build is the theme-tokens canary, ci.yml:577). CORRECTION (v2, from my lib.rs audit): CI DOES build the lumen-web wasm32 module — the ci.yml wasm-build job's "Web size-budget gate" step runs `cargo xtask web-budget --check`, which rebuilds the `floor` row (`package = "lumen-web"`, cargo_features []) via real `cargo build -p lumen-web --target wasm32-unknown-unknown --release --locked` (xtask web_budget.rs:76, size-record.toml:46-51; baseline run #16 gate-web_budget___check.log.txt:4 rebuilt it, raw 1,302,846 B, 33.2 s). What remains absent: any EXECUTION of the module — it is size-measured, never instantiated or run (no wasm-bindgen-test, no browser lane in CI; the 2026-11-10 browser-matrix capture was a manual run).
Why it matters: The module's own doc comments call the fixture run "the lane proof"; the lane proven is the native rlib lane, not the browser lane the product ships — and the wasm32 artifact's only CI presence is a size number, not a behavior.
What correct looks like: A wasm-bindgen-test (or Node wasm_exec) lane executing at least the boot→open→pump slice over the ABI on the real wasm32 module — doc 09 §2.5 item 3, already specced.
Confidence: HIGH — Verification: read + Cargo.toml + workflow greps + doc 09:84 read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `use testkit::assert_err` (cfg import) | 1964-1965 | REAL | — | Correctly gated with its only consumer; comment explains the `-D unused-imports` interaction. |
| `fn request(command) -> WireRequest` | 1967-1972 | REAL | — | Trivial envelope builder; correct. |
| `const WEBGL2_CAPS` | 1976-1980 | REAL | — | Matches RendererCaps shape; cites BCD-verified doc 09 §4.1. |
| `const BOTH_CAPS` | 1984-1988 | REAL | — | webgpu preferred + webgl2 floor. |
| `fn open_fixture_with(host, renderer)` | 1994-2013 | REAL | — | Drives the REAL message path (Boot→LoadBlob→SessionOpen) — not a mock seam; `unreachable!` on non-accept (fails loudly). |
| `fn open_fixture(host)` | 2016-2018 | REAL | — | Delegates; the "legacy/unprobed" lane. |
| `pump_slices_play_the_fixture_to_its_end` | 2023-2063 | REAL | — | Strong: position≈duration <5 ms, both limbs >0, trail crossed Playing, `!busy` at end. Mutation (no-op pump) fails `assert_eq!(last.state, Ended)`. Ran ok (orchrun:3119). |
| `pump_ticks_are_capped_and_flushes_bounded` | 2069-2082 | SHALLOW | S4 | F-003: bound cannot fail on this fixture; cap enforced by hang. Ran ok (orchrun:3122). |
| `audio_fill_is_bounded_per_tick` | 2086-2109 | REAL | — | Discriminating: unbounded fill would advance ~2.97 s > 533 ms bound (ring 131072 frames @44.1 kHz). Multiply-then-divide avoids truncation. Ran ok (orchrun:3115). |
| `unserved_surface_refuses_typed` | 2114-2152 | REAL | — | Typed refusal codes + owner key + NotReady without session; serving the "unserved" command would trip `unreachable!`. Ran ok (orchrun:3127). |
| `snapshots_answer_over_the_open_fixture` | 2155-2206 | REAL | — | duration>0, position∈[0,1], 2 tracks with video+audio rows, engine-info host=="web" + mjpeg present. Ran ok (orchrun:3123). |
| `pause_freezes_the_counter_clock` | 2210-2231 | REAL | — | Exact-equality freeze across 8 pumps; a clock that kept pulling would fail. Ran ok (orchrun:3118). |
| `wasm32_profile_decoder_seam_refuses_typed` | 2242-2275 | REAL | S3 | Correct assertions (stage==Decode, code=="web.decoders_absent"); F-001/F-002: cfg'd out of all CI lanes; its doc comment cites nonexistent dispatch tests. Ran ok in default-feature run only (orchrun:3128). |
| `shutdown_closes_and_acknowledges` | 2280-2298 | REAL | — | Asserts ShutdownComplete queued, session gone, NotReady after. Note: accidentally double-boots (explicit Boot at 2283 + open_fixture's Boot at 1997) — exercises re-boot idempotency without pinning it; harmless. Ran ok (orchrun:3121). |
| `commands_before_boot_refuse_typed` | 2302-2313 | REAL | — | Pins the `web.worker.not_booted` gate (production 1002-1010). Ran ok (orchrun:3116). |
| `frames_to_ns_is_exact_and_saturating` | 2317-2322 | REAL | — | Exact 44100@44.1 kHz = 1e9 ns; u64::MAX saturation pinned. Ran ok (orchrun:3117). |
| `the_boot_snapshot_reports_the_active_backend` | 2330-2397 | REAL | — | Four-scenario ladder (webgpu/webgl2/unavailable(…)/unprobed(…)); a guessed backend string fails the prefix asserts. Ran ok (orchrun:3125). |
| `the_present_path_ships_one_frame_per_pump` | 2406-2462 | REAL | — | Asserts ≤1 PresentFrame per pump batch, dims == container's own, pts present, refused==0; unprobed lane ships 0. Ran ok (orchrun:3200). |
| `surface_loss_falls_then_degrades_to_audio_only` | 2470-2580 | REAL | — | The doc 09 §4.4 ladder end to end: WebGPU→WebGL2 transition, stale loss ignored (0 duplicates), WebGL2 loss → audio-only (0 presents after, position advances, decode counting continues). Strongest test in the module. Ran ok (orchrun:3126). |
| `the_autoplay_gate_parks_freezes_and_resumes` | 2590-2650 | REAL | — | Event exactly once, frozen==0 (park applied before flow — a fill_audio ignoring the park advances the counter and fails), duplicate report no re-fire, resume advances. Ran ok (orchrun:3124). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| pump_slices_play_the_fixture_to_its_end | YES | No-op/short pump leaves state ≠ Ended and position far from duration; both counters must be >0. |
| pump_ticks_are_capped_and_flushes_bounded | NO (for the flush bound) / hang (for the tick cap) | F-003: fixture event count ≪ 2048 bound; cap removal = 4.29e9-iteration hang, not an assertion. |
| audio_fill_is_bounded_per_tick | YES | Unbounded fill drains the 131 072-frame ring ≈2.97 s > 533 ms bound. |
| unserved_surface_refuses_typed | YES | Ok outcome hits `unreachable!`; wrong code/owner fails assert_eq. |
| snapshots_answer_over_the_open_fixture | YES | All four snapshots must Complete with concrete values; degraded answers hit `unreachable!`. |
| pause_freezes_the_counter_clock | YES | Exact-equality position across post-pause pumps. |
| wasm32_profile_decoder_seam_refuses_typed | YES (when compiled) | assert_err + stage/code equality; but compiled out of every --all-features CI lane (F-002). |
| shutdown_closes_and_acknowledges | YES | Missing ShutdownComplete / lingering session / non-NotReady all fail. |
| commands_before_boot_refuse_typed | YES | First outbound must be the typed ControlError with exact code. |
| frames_to_ns_is_exact_and_saturating | YES | Exact values incl. saturation at u64::MAX. |
| the_boot_snapshot_reports_the_active_backend | YES | Four distinct renderer strings asserted by prefix. |
| the_present_path_ships_one_frame_per_pump | YES | >1 present per pump, wrong dims, missing pts, refusal>0, or unprobed-lane presents all fail. |
| surface_loss_falls_then_degrades_to_audio_only | YES | Transition count/shape, duplicate suppression, 0-presents-after-loss, position advance all pinned. |
| the_autoplay_gate_parks_freezes_and_resumes | YES | frozen==0 equality; event count ==1; no re-fire; resume advance. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "rlib: the native lane-proof tests link the same code" | apps/web/Cargo.toml `[lib]` comment | REAL | Tests ran natively green (orchrun:3087-3203); same source compiled into cdylib/rlib. |
| "cargo test --target wasm32-unknown-unknown via wasm-bindgen-test in Node … engine logic, slicing, capability errors" | docs/09-web-wasm-target.md:84 (§2.5 item 3) | PAPER-ONLY | No wasm-bindgen dep in apps/web/Cargo.toml; no such test target; CI's only direct wasm cargo build is the theme-tokens canary (ci.yml:534-546 "WHAT IT IS NOT YET"); the web-budget gate builds lumen-web for wasm32 (baseline #16) but never executes it. |
| "the wasm module SHIPS decoder-less (the honest capability snapshot)" | apps/web/Cargo.toml + docs/web/web-column-pass-20261110.md | REAL-UNPROVEN | Refusal contract implemented (host.rs:482-485, 696-703) and pinned by the seam test (default-features only, F-002); the wasm32 cdylib IS built in CI (web-budget gate rebuild, baseline #16) but never executed by any test or browser lane. |
| Doc 09 §3.2 budgets / §4.4 loss ladder / §5.3 autoplay / §7 boot gate | docs/09 sections (verified present) | REAL | Implemented (pump/fill_audio/present_one/render_lane/autoplay; host.rs:984-1388) and tested green here (orchrun cites above). |
| "The web and desktop builds both play real media through the same command layer" (Phase 7 gate) | docs/09-web-wasm-target.md:30 | UNVERIFIED (web half) | Native half proven by this module; no browser/wasm execution evidence exists anywhere in the baseline. |

## 7. Dead / duplicated / unreachable code
None in this chunk: every helper and test is used and ran. The `WEBGL2_CAPS`/`BOTH_CAPS` consts are used by 5 tests. No `#[ignore]`, no dead tests. (The cfg-gated import at 1964-1965 is the honest kind: gated with its sole consumer.)

## 8. Dependencies & flags
- `testkit` (dev-dep): Must/fixture_bytes/assert_err — used, real (crates/testkit/src/lib.rs:44,100,245).
- `builtin-codecs` feature: NOT default (Cargo.toml `[features]`); gates exactly one import + one test in this chunk (F-002); CI's `--all-features` test lanes enable it → seam test compiled out.
- No new deps introduced by the test module; fixture comes from tracked tests/fixtures (exists on disk).

## 9. Open questions for the next agent
- host.rs parts 1-3 (RESOLVED while writing this audit, recorded for completeness): `builtin_decoder_for`'s FIRST definition carries `#[cfg(any(not(target_arch = "wasm32"), feature = "builtin-codecs"))]` at host.rs:573 — no duplicate-symbol conflict with the wasm32 stub at 696-697; the gating is correct. v2 correction: the wasm32 decoder-less stub IS compiled in CI — the web-budget gate's floor rebuild (default features, wasm32 target, baseline #16) exercises exactly that cfg combination; what no lane does is RUN it or lint it for wasm32.
- abi.rs owner: `the_lane_proof_over_the_abi` ran ok (orchrun:3120) — does it execute the real cdylib exports or only the rlib ABI shims natively?
- Whoever audits docs/09-web-wasm-target.md: §2.5 items 1-3 are PAPER-ONLY as to execution (no workspace-wide wasm clippy/test, no wasm-bindgen-test) — the CI comment (ci.yml:537-539) admits it; the wasm32 BUILD half is real via the web-budget gate (baseline #16). Cross-check with size-report.md's manual one-off builds and the browser-matrix capture (also manual).
