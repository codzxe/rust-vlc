# AUDIT: crates/media-probe/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-21 |
| Lines / functions | 49 / 10 config keys (package×6, lints, features×1, dependencies×2, dev-dependencies×2) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. Call chain: apps/cli/Cargo.toml:56 `media-probe = { path = "../../crates/media-probe" }` → lumen CLI open path. Executed: baseline mp4-smoke-open-session.txt (committed) shows the open path reaching "event: tracks 2 ready" after probing; matrix notes cite live probe errors ("probe.no-demuxer", docs/feature-matrix.toml:217) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Grep `media-probe` in **/Cargo.toml (dependents: apps/cli:56, apps/web:63, tools/bench-vs-vlc:53, tools/mem-audit:51, tools/sweep:48) · grep `plugin_api|media_types` in src/ non-comment lines (lib.rs:171, report.rs:16, signature/mod.rs:28, config.rs:11 — both prod deps used) · grep `contract` consumers repo-wide (zero targeted feature enables; only CI `--all-features`) · grep `all-features|cargo build` in .github/workflows (release.yml builds `-p lumen-cli -p desktop` with NO features; ci.yml:233-234 + ci-full.yml:107/136/169 test with --all-features) · sed -n boundaries.toml [crates.media-probe] block (exact match) · grep `pub struct ProbeReport` (report.rs:270) |

## 1. One-paragraph verdict
The cleanest manifest audited so far: two production deps, both verifiably
used in non-test code; two dev-deps, both used; zero external deps (C5 true);
boundaries.toml agreement exact; reachable and live in the shipped `lumen`
binary via the open path. The one blemish is the `contract` feature's
justification comment: it names "the engine wiring tests" and "any downstream
test harness" as opt-in consumers, and neither exists — the feature is
enabled today only by CI's blanket `--all-features`, which makes the 10
bounded-read conformance tests in tests/probe_io.rs silently skippable if
that flag is ever dropped from a lane.

## 2. What actually works (verified)
— Dep usage: `media-types` (config.rs:11 `use media_types::EngineError;`,
contract.rs:38-39), `plugin-api` (lib.rs:171
`use plugin_api::{SeekOrigin, SourceStream};`, report.rs:16,
signature/mod.rs:28) — production code, not just docs.
— Dev-deps used: `testkit` + `input` in tests/common/mod.rs and 4 test files.
— Boundaries agreement: boundaries.toml `[crates.media-probe]` →
internal_deps = ["media-types", "plugin-api"], dev_deps = ["testkit",
"input"] — identical to this manifest (sed -n verified).
— C5 claim TRUE: zero external crates; all path deps.
— Description claim TRUE: `pub struct ProbeReport` at report.rs:270;
`max_lookahead` bound implemented in lib.rs:203
(`fill_window(stream, config.max_lookahead)?`).
— Crate builds + tests in baseline #1/#3 (INDEX.md) and the probe executes
in the MP4 smoke session (committed mp4-smoke-open-session.txt:
"event: tracks 2 ready").

## 3. Findings
### F-001 [S3] — `contract` feature comment claims consumers that do not exist; only blanket `--all-features` ever enables it
Where: crates/media-probe/Cargo.toml:29-36
Code:
```toml
# it via cfg(test); this feature is how integration tests, the engine
# wiring tests, the future doc 06 §7 conformance kit, and any downstream
# test harness opt in. Release binaries (no feature) carry zero of its
# panicky conformance vocabulary.
contract = []
```
What is wrong: Repo-wide grep for any Cargo.toml enabling media-probe's
`contract` feature (via `features = [...]`): ZERO results. The "engine
wiring tests" (playback-engine) never reference `media_probe::contract`
(grep: only self-hits). No "downstream test harness" exists —
tools/bench-vs-vlc/Cargo.toml:52 enables `input`'s contract feature, NOT
media-probe's. The feature is enabled exclusively by CI's global
`--all-features` (ci.yml:233-234, ci-full.yml:107/136/169) and by the
crate's own tests/probe_io.rs:18 gate.
Why it matters: The comment is the design justification for a feature flag;
it misstates who depends on it. A maintainer removing the (apparently
unconsumed) feature, or trimming `--all-features` from a test lane, loses
the 10 bounded-read conformance tests with zero compile or CI error —
the exact "workflows that pass while testing nothing" failure mode.
What correct looks like: Either a real opt-in consumer (dev-dep with
`features = ["contract"]` where the wiring tests live) or an honest comment:
"enabled only by CI --all-features; probe_io.rs is its sole consumer".
Confidence: HIGH — Verification: repo-wide Cargo.toml + src grep (zero
targeted enables).

### F-002 [S4] — feature-gated test file compiles to zero tests when the flag is off, silently
Where: crates/media-probe/Cargo.toml:36 (feature) → tests/probe_io.rs:18 `#![cfg(feature = "contract")]`
Code:
```toml
contract = []
```
What is wrong: With the feature off, `cargo test -p media-probe` compiles
probe_io.rs as an empty binary — 0 tests run, no warning. Baseline #3
(no all-features) passed 1681 workspace tests; baseline #4 (--all-features)
passed 1751 — the 70-test delta includes these 10 gated tests. A plain
`cargo test` run by any developer exercises none of the bounded-read
guarantees the manifest advertises ("the fill loop reads at most
`max_lookahead` bytes").
Why it matters: The strongest property claims of this crate (bounded reads)
are verified only in a global-flag lane, not in the default test path.
What correct looks like: `required-features = ["contract"]` on a `[[test]]`
entry makes cargo skip-with-a-listing rather than silently run nothing, or
the assertions move into unit tests (cfg(test)) that always run.
Confidence: HIGH — Verification: read probe_io.rs:18 + baseline #3 vs #4
counts (INDEX.md rows 3-4).

## 4. Per-key table
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `package.name = "media-probe"` | 20 | REAL | — | Matches boundaries.toml [crates.media-probe] |
| `package.version = "0.1.0"` | 21 | REAL | — | Repo convention |
| `package.edition = "2024"` | 22 | REAL | — | Workspace-consistent |
| `package.description` | 23 | REAL | — | ProbeReport (report.rs:270), signature scoring (src/signature/), confidence/track hints — all real |
| `package.publish = false` | 24 | REAL | — | — |
| `[lints] workspace = true` | 26-27 | REAL | — | Inherits deny tables |
| `[features] contract = []` | 29-36 | REAL | S3/S4 | Test-only by design; release builds exclude it (verified: release.yml:193 builds with no --features); but no targeted consumer exists (F-001) and off-state silently drops 10 tests (F-002) |
| `media-types` dep | 39 | REAL | — | Used (config.rs:11, contract.rs:38); matches boundaries |
| `plugin-api` dep | 40 | REAL | — | Used in production code (lib.rs:171, report.rs:16, signature/mod.rs:28); matches boundaries |
| dev-dep `testkit` | 45 | REAL | — | Used in tests (common/mod.rs + suites) |
| dev-dep `input` | 49 | REAL | — | Used (composition proof through real FileAccess/FileStream — grep tests/) |

## 5. Test honesty
Manifest has no tests of its own. Relevant: tests/probe_io.rs (10 tests,
all gated on `contract`) — would they fail if bounded-read were broken?
Designed to (CountingStream + assert_bounded_read, probe_io.rs:8-11), but
they only execute under --all-features (baseline #4 passed 1751 — the
baseline .log evidence itself is missing, see my .gitignore audit F-001;
the count delta 1681→1751 is the only verification). The other three test
files run unconditionally in baseline #3.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this feature is how integration tests, the engine wiring tests, … and any downstream test harness opt in" | Cargo.toml:32-34 | FALSE (for engine wiring tests + downstream harness) | Zero targeted feature enables repo-wide; playback-engine never references media_probe::contract — F-001 |
| "Release binaries (no feature) carry zero of its panicky conformance vocabulary" | Cargo.toml:34-35 | REAL | release.yml:193 `cargo build --release --locked -p lumen-cli -p desktop` — no --features/--all-features (grep verified) |
| "probing pulls its bounded window through plugin_api::SourceStream" | Cargo.toml:6-7 | REAL | lib.rs:171 `use plugin_api::{SeekOrigin, SourceStream};` |
| "the fill loop reads at most max_lookahead bytes" | Cargo.toml:15-16 | REAL-UNPROVEN (src auditor to prove at runtime) | lib.rs:203 uses config.max_lookahead; runtime bound asserted only by gated probe_io tests (F-002) |
| "zero external dependencies" | Cargo.toml:13 | REAL | [dependencies]/[dev-dependencies] all path deps |

## 7. Dead / duplicated / unreachable code
None in this manifest. The `contract` feature is the borderline case: live
in CI only via --all-features (F-001/F-002). No duplicated deps.

## 8. Dependencies & flags
2 internal prod deps + 2 internal dev deps; zero external; 1 feature
(`contract`, empty — pure cfg gate, no optional deps). Matches
boundaries.toml exactly. Release build excludes the feature by design
(verified in release.yml).

## 9. Open questions for the next agent
- The media-probe src auditor (config.rs/report.rs/signature/*) should prove
  the "no code path probes more than max_lookahead" claim at the fill_window
  level — the manifest asserts it, the gated tests are its only witness.
- If doc 06 §7's conformance kit lands, wire it as a real consumer of the
  `contract` feature so F-001's stale comment becomes true.
- apps/web/Cargo.toml:63 also depends on media-probe (unreachable today?
  apps/web build status is not in the baseline lanes) — web auditor's call.
