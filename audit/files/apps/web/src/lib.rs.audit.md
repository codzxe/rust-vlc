# AUDIT: apps/web/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2026-11-21 |
| Lines / functions | 76 / 16 (1 doc-header claim block, 9 `pub mod` declarations, 6 re-export lines covering 14 names) |
| File verdict | UNREACHABLE (real, workspace-tested crate root; the wasm artifact is CI-built for size only; zero shipped consumers) |
| Reachable from shipped entry point? | NO. `lumen-web` is a workspace member (root Cargo.toml `apps/*`), so `cargo test --workspace` compiles and runs it (111 lib tests green, orchrun:3087-3203) and the ci.yml web-budget gate BUILDS the wasm32 artifact (`cargo build -p lumen-web --target wasm32-unknown-unknown --release --locked` via xtask, baseline #16 PASS) — but neither shipped entry point depends on it: apps/cli uses plugin-api/engine crates directly; apps/desktop-tauri's Cargo.toml and Cargo.lock contain NO lumen-web entry (Grep: zero hits). No workflow ships the module; its only browser execution is the manual 2026-11-10 matrix capture (per W12-A08's worker.js audit: "at a foreign commit"). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read lib.rs 1-76 (full) · `ls apps/web/src/` (all 9 declared modules exist: abi.rs, audio/, capabilities.rs, host.rs, io/, open.rs, protocol.rs, render/, source.rs) · `rg -n "lumen-web" apps/desktop-tauri/Cargo.toml apps/desktop/Cargo.toml` + `rg -c "lumen-web" apps/desktop-tauri/Cargo.lock` → 0/absent (no shipped consumer) · `rg -n "AutoplayGate\|WorkletQueue\|WORKLET_QUANTUM" apps/web/src --glob '!audio/*'` (consumer census for the audio re-exports: AutoplayGate via deeper path in host.rs:102,813,849; WorkletQueue/WORKLET_QUANTUM zero non-audio consumers) · `rg -n "serde" crates/commands/Cargo.toml` (serde+serde_json preexist in commands — the "zero new dependencies" claim's basis) · `sed -n '229,320p' apps/web/src/capabilities.rs` (the canvas/AudioWorklet/fetch blocked rows named in the doc — all present, honest reasons) · `rg -n "setTimeout" apps/web/worker.js` (:114 — the doc's re-arm claim) + Read audit/files/apps/web/worker.js.audit.md (W12-A08's verdict cross-ref) · `rg -n "lumen-web\|wasm32\|rebuild" tools/xtask/src/cmd/web_budget.rs docs/web/size-record.toml` + `gate-web_budget___check.log.txt` (the CI-builds-the-module evidence — my v2 correction) · baseline `orchrun-test-workspace-nofailfast.log.txt:3087-3203` (111 tests, incl. abi::tests::the_lane_proof_over_the_abi :3120). Quotes verified with the Read tool. |

## 1. One-paragraph verdict
This is the most honest crate root I have audited in this repository: every load-bearing doc claim checks out against code or runs — the message-port contract and hand-rolled ABI exist and are tested (111 green rlib tests incl. the ABI lane proof), the "zero new dependencies" posture holds (externals are exactly serde/serde_json, both preexisting in `commands`, the wire-family owner), the "cooperative pump slices … re-armed from setTimeout(0)" model is real in both halves (host.rs pump + worker.js:114, W12-A08-verified), the capability snapshot really does carry typed blocked rows for canvas/AudioWorklet/fetch with truthful reasons (capabilities.rs:229-320), the decoder-less wasm posture is implemented and test-pinned, and — the claim I initially disbelieved — "`cargo build --target wasm32-unknown-unknown --locked` proves the real module artifact" is TRUE in CI via the web-budget gate's rebuild (my earlier "nothing builds it" statement was wrong and is corrected in my log; the gate rebuilt the floor at 1,302,846 B in baseline #16). What the honest header also concedes, and my file-by-file audits of this crate confirm: browser-runtime behavior is [VERIFY] — the module is size-checked, never run, by every automated lane; the one real-Chromium execution was a manual capture at a foreign commit; and nothing, anywhere, ships the artifact. The only defects are two S4s: a root re-export surface with zero in-tree consumers (WorkletQueue/WORKLET_QUANTUM) or redundant paths (AutoplayGate re-exported at root but consumed via the deeper module path), and the crate's "wasm32 composition root" framing resting on a build lane that measures bytes, not behavior.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3087-3203`: the crate's rlib compiles and all 111 lib tests pass — the "CI proof is headless-native … boot → loadBlob → session.open → pump slices → events" claim maps to these tests (host::tests::* 14/14, io/seekability/staged_source suites, `abi::tests::the_lane_proof_over_the_abi` :3120 — the "wasm ABI's dispatch core with an in-memory MessagePort double").
— Verified by `gate-web_budget___check.log.txt:4` (baseline run #16, PASS): "rebuild: `floor` — raw 1302846 B, gzip -n 301101 B … (33.2s)" — the ci.yml web-budget gate step really builds `-p lumen-web --target wasm32-unknown-unknown --release --locked` (xtask web_budget.rs:76, size-record.toml:50 `package = "lumen-web"`), so the doc's "cargo build … proves the real module artifact" is a CI fact, not aspiration.
— Verified by Read: all 9 declared modules exist on disk; the canvas/AudioWorklet/fetch blocked rows named in the header exist verbatim in capabilities.rs (canvas-render-surface :293, audioworklet-output :306, http-streaming :281) with reasons that match the audit reality (browser present = [VERIFY]); `threading: Single` is the snapshot's literal field (capabilities.rs:315).

## 3. Findings
### F-001 [S4]
Where: apps/web/src/lib.rs:71 (with the consumer census)
Code:
```
pub use audio::{AutoplayGate, WORKLET_QUANTUM, WorkletQueue};
```
What is wrong: Of the three audio names re-exported at the crate root, `WorkletQueue` and `WORKLET_QUANTUM` have ZERO consumers outside the `audio` module (Grep over apps/web/src excluding audio/: no hits) — their only users are sink.rs's own tests and the JS twin's documentation; `AutoplayGate` IS consumed (host.rs:102,813,849) but through the deeper `crate::audio::autoplay::` path, so the root re-export serves no in-tree caller. For a crate whose only consumers are its own tests (no shipped or external dependent — header Reachability), this is public API surface that exists for an embedder who does not exist.
Why it matters: The root re-export list is the crate's advertised surface; two of its names advertise a model (the worklet queue) that the wasm host itself never uses — the host's audio lane is the null device (host.rs) with the JS processor as the future browser lane. A reader concluding "the wasm module queues audio in WorkletQueue" would be wrong.
What correct looks like: Drop `WORKLET_QUANTUM`/`WorkletQueue` from the root (or gate them behind the same doc-honesty as the clock model), or make the JS-twin constants genuinely derive from them (the mod.rs audit's unpinned-constants finding).
Confidence: HIGH — Verification: read + consumer-census Grep.
### F-002 [S4]
Where: apps/web/src/lib.rs:52-59 (with 55-56)
Code:
```
//! The CI proof is **headless-native**: a `cargo test` lane drives the
//! SAME message paths (boot → `loadBlob` → `session.open` → pump
//! slices → events) through the wasm ABI's dispatch core with an
//! in-memory MessagePort double, and `cargo build --target
//! wasm32-unknown-unknown --locked` proves the real module artifact.
```
What is wrong: Accurate — but the two sentences place a size-measuring build lane ("proves the real module artifact") next to a behavior-proving test lane, and only the latter's limits are then conceded ("[VERIFY] — no browser"). The wasm32 build lane proves the artifact COMPILES and its gzip size is within budget; it executes nothing, lints nothing for the target, and would not catch a wasm32-only logic defect (e.g. in the `#[cfg(target_arch = "wasm32")]` stub paths). A reader can take "proves the real module artifact" as more than compile+size.
Why it matters: Subtle inflation by juxtaposition; this is the crate's own framing of its evidence ceiling, so precision matters. (Everything the header explicitly claims is true; this is about what a skimming reader concludes.)
What correct looks like: "…and the web-budget gate rebuilds the wasm32 artifact (compile + size proof; no target-local test or lint lane exists yet — doc 09 §2.5 items 1-3)."
Confidence: HIGH — Verification: read + web_budget.rs + baseline #16 (the lane's actual scope) + doc 09:84 (the unimplemented test lanes).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module doc block (crate identity, ABI choice, honest posture, evidence labels) | 1-59 | REAL | S4(F-002) | Every load-bearing claim verified (§2); the wasm-bindgen [PROPOSAL]/OQ-1 record matches Cargo.toml's commentary; the [VERIFY] concession is truthful and matches my file-by-file findings (worklet never executed; worker executed once manually). |
| `pub mod abi` | 61 | REAL | — | Exists; `the_lane_proof_over_the_abi` green (orchrun:3120); A08's worker.js audit verified the glue↔ABI invariants. |
| `pub mod audio` | 62 | REAL | — | Exists; autoplay/sink/clock audited by A02 (clock model dormant — their F-001). |
| `pub mod capabilities` | 63 | REAL | — | Exists; 5 tests green; blocked rows verified against the header's claims. |
| `pub mod host` | 64 | REAL | — | Exists; my part4 audit (14 tests green; parts 1-3 with other agents). |
| `pub mod io` | 65 | REAL | — | Exists; fetch_port/mock_fetch/seekability/staged_source suites green (orchrun:3129-3165). |
| `pub mod open` | 66 | REAL | — | Exists; A02's audit: disciplined CLI mirror, seekability-without-reanchor gap. |
| `pub mod protocol` | 67 | REAL | — | Exists; wire names match worker.js literals exactly (W12-A08 verification). |
| `pub mod render` | 68 | REAL | — | Exists; render lane tested in host.rs part4 (loss ladder green). |
| `pub mod source` | 69 | REAL | — | Exists; BlobRegistry backs the open path used by every green test. |
| `pub use audio::{AutoplayGate, WORKLET_QUANTUM, WorkletQueue}` | 71 | REAL (surface) | S4(F-001) | AutoplayGate consumed via deeper path; the other two zero in-tree consumers. |
| `pub use capabilities::{…}` | 72 | REAL | — | HostCapabilities/TargetProfile/capabilities all consumed (host.rs, tests). |
| `pub use host::{SliceBudgets, WebHost}` | 73 | REAL | — | WebHost is the crate's centerpiece; SliceBudgets used by pump tests. |
| `pub use protocol::{HostToWorker, PumpReport, WorkerToHost}` | 74 | REAL | — | The MessagePort contract names; consumed by abi/host/worker glue. |
| `pub use render::{…}` | 75 | REAL | — | RendererCaps/RenderBackend/RendererNegotiation consumed by host render lane + tests. |
| `pub use source::{BlobRegistry, BlobSource}` | 76 | REAL | — | Backs LoadBlob/open; tested green. |

## 5. Test honesty
No tests in this file (facade). The crate's tests — the evidence the header cites — are covered per-module by this wave (host.rs part4 mine; audio/clock/sink A02; io A01/A09; abi A0x; worker.js W12-A08). The lib-level claim "cargo test lane drives the SAME message paths" is backed by orchrun:3087-3203 (111/111).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The wasm32 composition root … the SAME std-only engine core the CLI drives" | lib.rs:6-8 | REAL | Same engine-crate dependency set (Cargo.toml); the rlib tests run the identical code the CLI-shaped tests exercise; only the composition differs. |
| "a typed HostToWorker/WorkerToHost MessagePort contract whose command lane carries the ONE command layer's own envelopes … never re-defined here" | lib.rs:10-13 | REAL | protocol.rs uses commands' envelope types; ARCH-5 single-catalog claim consistent with the commands crate audits. |
| "zero new dependencies … wasm-bindgen is a [PROPOSAL] (OQ-1 undecided)" | lib.rs:14-23 | REAL | apps/web externals = serde + serde_json only, both preexisting in commands (Cargo.toml:39-40); no wasm-bindgen anywhere. |
| "the JS glue re-arms the pump from setTimeout(0) ticks while the PumpReport says busy" | lib.rs:34-36 | REAL | worker.js:114 `setTimeout(() => send('{"type":"pump","ticks":1}'), 0)` gated on `report.busy` (:113); W12-A08 verified the ordering invariant (Pumped pushed last). |
| "the canvas/AudioWorklet/fetch lanes named as typed blocked rows (cards 7.H2/7.H3/7.H4)" | lib.rs:41-43 | REAL | capabilities.rs:281 (http-streaming), :293 (canvas-render-surface), :306 (audioworklet-output) — all present with honest reasons matching audit reality. |
| "the wasm32 module ships the engine core WITHOUT the builtin decoders … Native builds link the real MJPEG + PCM decoders" | lib.rs:43-48 | REAL | Verified in my host.rs part4 audit (decoder seam + capabilities tests; size-report floor row = decoder-less). |
| "The CI proof is headless-native … cargo build --target wasm32-unknown-unknown --locked proves the real module artifact" | lib.rs:52-56 | REAL (with F-002's precision caveat) | Tests: orchrun:3087-3203; build: web-budget gate (ci.yml step → xtask → cargo build, baseline #16 PASS — my v2 correction). |
| "Browser-runtime behavior … is [VERIFY] — no browser exists in this sandbox, and nothing here claims it was exercised" | lib.rs:57-59 | REAL (honest) | Matches: worklet never executed (my audio-worklet.js audit), worker executed once manually at a foreign commit (W12-A08), no CI browser lane. |
| "64-command / 23-event catalog rides inside" | lib.rs:12-13 | REAL-UNPROVEN | The catalog lives in `commands` (envelope types used throughout host/protocol); the exact 64/23 counts not re-verified by me — commands-crate audits own the count. |

## 7. Dead / duplicated / unreachable code
- The crate (and this root) is UNREACHABLE from shipped entry points (header): compiled, tested, and wasm32-size-built in CI, but no product depends on it and no workflow ships the artifact.
- F-001's unconsumed re-exports are the only in-file dead surface.

## 8. Dependencies & flags
- Declared: commands, media-types, plugin-api, codec-api, media-probe, demux, media-clock, playback-engine, audio-engine, audio-output (all internal path deps — the "same engine core"), serde + serde_json (the commands wire family), target-gated codecs (native always; wasm32 opt-in behind builtin-codecs*).
- No dev-deps beyond testkit; `[lints] workspace = true` inherited (apps/web is among the 39/40 inheriting crates per INDEX.md's hygiene note — the lone exception is the workspace-excluded tauri shell); no feature flags in this file.

## 9. Open questions for the next agent
- Wave synthesizer: the apps/web story is now complete across agents — real engine core, real ABI, real tests, CI-built-but-never-executed wasm artifact, worklet never run, nothing shipped. The single highest-leverage gap is doc 09 §2.5 item 3 (wasm-bindgen-test lane) — it would convert three of my file verdicts from UNREACHABLE/REAL-UNPROVEN to REAL in one card.
- Whoever owns product.toml [web.size_budgets] / the web-budget gate: the gate's rebuild row is the ONLY wasm32 compile lane — consider promoting it to also run `cargo check --target wasm32 --all-targets` (doc 09 §2.5 item 1) so the cfg-stub paths get type-checked in CI (today the floor build covers the lib but not tests).
- ui/e2e/web owners: harness.js instantiates the worker but not the worklet (my audio-worklet F-001) — one `new AudioWorkletNode` + two port messages closes the browser-audio evidence gap.
