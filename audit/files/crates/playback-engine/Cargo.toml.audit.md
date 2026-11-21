# AUDIT: crates/playback-engine/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-21 |
| Lines / functions | 56 / config keys: package, lints, dependencies×6, dev-dependencies×4 |
| File verdict | REAL (crate executed in baseline smoke; header comment frozen at v0 scope) |
| Reachable from shipped entry point? | YES (call chain: shipped `lumen` bin → `apps/cli/Cargo.toml:58` playback-engine dep → 14 files in apps/cli/src use `playback_engine::…`; the baseline MP4 open-session transcript's `state-changed Idle → Opening → Buffering → Error → Idle` events are this crate's state machine executing) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | boundaries.toml:294-305 diff vs manifest (exact match); `rg -l '<crate>::' crates/playback-engine/{src,tests}` per dep (all used; demux/codecs/input = tests-only, 0 hits in src); `grep -n 'pub mod' crates/playback-engine/src/lib.rs` (15 modules); dependents grep (plugin-host-wasm, transcode, apps/cli, apps/control-server, apps/web); baseline INDEX MP4 smoke + #3 |

## 1. One-paragraph verdict
Disciplined manifest: six internal path deps, four dev-only path deps, zero externals — matching boundaries.toml `[crates.playback-engine]` (294-305) field-for-field, with the C5 std-only claim actually true here (unlike streaming's stale copy). Dev-edge hygiene is real: `demux`/`codecs`/`input` appear in 0 src files, tests-only as documented. The crate is executed by the shipped binary (baseline MP4 smoke state transitions). One defect: the header still asserts "Seek/sync are deliberately EXCLUDED" in present tense while `src/seek.rs`, `frame_step.rs`, and 13 other sibling modules are exported (lib.rs:93-107) — the same frozen-v0-header drift found in streaming's manifest.

## 2. What actually works (verified)
— Boundaries match: manifest deps {codec-api, commands, media-clock, media-types, packet-queue, plugin-api} == boundaries.toml:296; dev {testkit, demux, codecs, input} == boundaries.toml:304; external_allowlist [] == no externals in manifest.
— Dev-only discipline: `rg -l 'demux::|codecs::|input::' crates/playback-engine/src` → 0 files; hits only under tests/ (3 files each). The "library stays trait-level" claim (:50-51) holds.
— Every dep used: codec_api 19 files, commands 54, media_clock 24, media_types 37, packet_queue 17, plugin_api 26, testkit 57.
— Execution: baseline MP4 smoke (`audit/runs/baseline/` mp4-smoke-open-session.txt) shows engine state transitions and `TimingDurationKnown` — this crate ran in the shipped binary; AVI contrast run (#20 rc-smoke) shows full Idle→Playing→Stopped→Idle cycle.

## 3. Findings
### F-001 [S4]
Where: crates/playback-engine/Cargo.toml:5-8
Code:
```
# The minimal state machine (2.C2 scope): the 8 wire-visible playback
# states, two-level session epochs, and the trait-level pipeline wiring
# demux → packet queues → decoders → sinks. Seek/sync are deliberately
# EXCLUDED (Phase 3: cards 3.A1-3.A3, 3.B1).
```
What is wrong: "Seek/sync are deliberately EXCLUDED" is present-tense and false at HEAD. The crate exports `pub mod seek` (lib.rs:103), `pub mod frame_step` (:98), plus chapters, bookmarks, hotplug, recovery, snapshot — 15 modules total, far beyond the "minimal state machine" the header freezes at card 2.C2. Phase 3 landed; the header never updated.
Why it matters: A reader of the manifest (the crate's front door) is told the crate has no seek — while apps/cli/tests contains native_seek.rs, ttf_seek.rs, seek_flow.rs and the shipped binary exposes seek. Any architecture or scope decision made from this header is made on stale facts.
What correct looks like: Header rewritten to current scope (state machine + epochs + seek + frame-step + chapters/bookmarks + hotplug + recovery + snapshot), with the 2.C2 origin kept as history.
Confidence: HIGH — Verification: read (lib.rs module list + sed quote).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package]` | 24-29 | REAL | — | description accurately scopes to state machine + epochs + wiring (again narrower than the src reality). |
| `[lints] workspace = true` | 31-32 | REAL | — | Verified inherited (file-1 loop-grep). |
| `codec-api` | 35 | REAL | — | Used (19 files). Layer 1 → L4 legal downward edge. |
| `commands` | 36 | REAL | — | Used (54 files). |
| `media-clock` | 37 | REAL | — | Used (24 files). |
| `media-types` | 38 | REAL | — | Used (37 files). |
| `packet-queue` | 39 | REAL | — | Used (17 files). |
| `plugin-api` | 40 | REAL | — | Used (26 files); the documented Lead-flag sixth dep (:13-18) — boundary registered. |
| `[dev-deps] testkit/demux/codecs/input` | 45,54-56 | REAL | — | All tests-only (0 src hits); all four in boundaries.toml:304 dev_deps. |

## 5. Test honesty
No tests in manifest. The dev-dep justification (:46-53) names tests/track_switch_fixture.rs — it exists (ls tests/), along with 27 sibling test files; per-test honesty belongs to the playback-engine source agents. Baseline #3 covers the suite (1681 passed).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Seek/sync are deliberately EXCLUDED" | Cargo.toml:7-8 | FALSE | lib.rs:103 `pub mod seek`; src/seek.rs exists; apps/cli/tests/native_seek.rs exists. |
| "C5 (std-only): zero external dependencies, no unsafe" | Cargo.toml:20-20 | REAL | [dependencies] = 6 internal path deps, no registry deps; external_allowlist [] (boundaries.toml:305). |
| "the library stays trait-level: no concrete demux/codec crate appears in [dependencies]" | Cargo.toml:50-51 | REAL | Verified: demux/codecs only in [dev-dependencies]; 0 src references. |
| "plugin-api is the sixth" Lead-flag dep | Cargo.toml:13-18 | REAL | Present at :40 and in boundaries.toml:296. |

## 7. Dead / duplicated / unreachable code
None in the manifest: all deps used, no externals, no duplication. (15 src modules vs the header's minimal scope is covered as F-001, manifest-side.)

## 8. Dependencies & flags
No `[features]`, no build-deps, no externals. Layer 4 crate per boundaries.toml:295 — all six internal edges point downward to L0/L1 (media-types L0; codec-api/plugin-api/packet-queue/commands/media-clock lower layers). Consistent.

## 9. Open questions for the next agent
- The crate has grown to 15 modules (hotplug, recovery, snapshot, model_check…) — the playback-engine source agents should check whether all are reachable from the shipped CLI or some are dead weight linked in anyway (e.g. `hotplug` — device hotplug in a headless player?).
- apps/web and apps/control-server also depend on playback-engine (grep §2) — is apps/web actually shipped anywhere (parity-report says web=0/252)? Cross-check with the control-server manifest audit (next file) and the web app agents.
