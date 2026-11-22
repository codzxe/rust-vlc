# CLAIM AUDIT (BATCH): seven Cargo.toml manifests — wave-3 engine crates + lumen-cli

| Field | Value |
|---|---|
| Auditor | W03-A06 (rescue batch for files #12–#18) |
| Date | 2026-11-22 |
| Files in batch | apps/cli/Cargo.toml (71 LOC), crates/playback-engine/Cargo.toml (56), crates/input/Cargo.toml (38), crates/audio-engine/Cargo.toml (38), crates/video-engine/Cargo.toml (32), crates/packet-queue/Cargo.toml (31), crates/media-clock/Cargo.toml (26) |
| Claims extracted | 24 |
| REAL | 22 | REAL-UNPROVEN | 0 | SHALLOW | 0 | STUB | 0 | PAPER-ONLY | 0 | FALSE | 1 | UNREACHABLE | 0 (one feature-gated module: UNREACHABLE-by-design in release, tested in CI) |
| Internal contradictions found | 1 (playback-engine header vs its own lib.rs) |
| Cross-refs | Per-file audits already exist (apps/cli — W13-A01; playback-engine + packet-queue — W03-A05; video-engine — W03-A03; input/audio-engine/media-clock — wave-3 agents). This batch independently re-verifies each manifest's claims and adds the release-build feature-surface proof. No contradictions with the prior audits. |

## 1. The assigned question: feature flags vs the shipped release build

**The release build** (`.github/workflows/release.yml:193`):

```
run: cargo build --release --locked -p lumen-cli -p desktop
```

No `--features` → **default features only**. Verified facts:

1. **Exactly one non-default feature exists across all seven manifests (+ apps/desktop, checked): `input`'s `contract = []`** (crates/input/Cargo.toml:24-30). The other six crates declare NO `[features]` section at all (grep across all eight manifests: one hit).
2. Therefore the shipped release binaries compile **everything** in playback-engine, packet-queue, media-clock, audio-engine, video-engine, apps/cli and apps/desktop **unconditionally** — reachability of any code in these crates is a call-graph question, not a feature-flag question. (Consistent with W13-A01's finding that the h264/aac decode arms compile into the shipped CLI — apps/cli has no `[features]`.)
3. `input`'s `contract` module (`src/contract.rs`, "panicky conformance vocabulary") is **intentionally absent from release binaries** — the manifest comment says so and the build confirms it structurally. Its tests run only in CI's all-features lane: `cargo build --workspace --all-features --locked` / `cargo test --workspace --all-features` (ci.yml:234-235, verified). Consequence for the audit's evidence base: **the W00 baseline (plain `cargo test --workspace`) did not run the contract-gated tests** — cross-ref W03-A03's finding on input/tests/truncation.rs (6 acceptance tests absent from the 1751).
4. The two external dependencies in the whole set (apps/cli's `serde_json = "1.0"`, `toml = "1"`) are locked (Cargo.lock: serde_json 1.0.154, toml 1.1.8+spec-1.1.0) and the release build is `--locked` — reproducible. Every other dependency in all seven manifests is a workspace path dependency.
5. All seven manifests set `[lints] workspace = true` (no local lint suppressions) and `publish = false`.

## 2. Per-file claim ledger

| # | File | Manifest shape | Header claims vs reality | Verdict | Findings |
|---|---|---|---|---|---|
| 12 | apps/cli/Cargo.toml | 16 path deps + 2 external (serde_json, toml), 1 dev-dep (testkit), no `[features]`, bin `lumen` + lib `lumen_cli`, v0.1.3 | "External deps (2, both already Lead-admitted…)" — verified exactly 2; each justification checked against usage: serde_json = the command-layer wire format, toml = `settings_schema::runtime::validate_value`'s `toml::Value` seam (both grep-verified in apps/cli/src). The 5 least-obvious internal deps all used: `streaming` (demux_host.rs:383 HttpFetch, :462 RangedResource), `settings_schema` (help.rs:34,157), `media_probe` (demux_host.rs:444), `renderer_wgpu` (pipeline_host.rs:79-80), `audio_output` (pipeline_host.rs:52-53 null device). No unused dependency found. | REAL | 0 |
| 13 | crates/playback-engine/Cargo.toml | 6 path deps (codec-api, commands, media-clock, media-types, packet-queue, plugin-api), zero external; dev-deps testkit + demux + codecs + input (documented test-only composition edge with Lead flag) | "Zero external dependencies, no unsafe" — verified (no external entries; C5). "plugin-api is the sixth [dep]" — verified present with the layer-legality argument. **"Seek/sync are deliberately EXCLUDED (Phase 3…)" is FALSE**: `src/seek.rs` exists and `lib.rs:103` declares `pub mod seek;` — the Phase-3 cards landed, the header never updated (W03-A05's S4, independently re-confirmed here). | REAL (manifest) with one FALSE header claim | S4 ×1 (F-001) |
| 14 | crates/input/Cargo.toml | 2 path deps (media-types, plugin-api), zero external; the set's only `[features]`: `contract = []` (non-default) | "C5 discipline: std-only, zero external dependencies" — verified. "Release binaries (no feature) carry zero of its panicky conformance vocabulary" — verified structurally (release.yml:193 passes no features) and the intent is honest. The cost: contract-gated acceptance tests are outside the default test run (see §1.3). | REAL | S4 ×1 (F-002, consequence documentation) |
| 15 | crates/audio-engine/Cargo.toml | 2 path deps (media-types, media-clock), zero external | "media-clock joins with card 3.B1's drift micro-resampler… No clock objects cross this edge" — the dep exists and the comment pre-notes it as boundaries.toml's own recorded edge; per W03-A03's audits the resampler/governor machinery exists but production only ever constructs the degenerate case (ZERO correction) — the manifest is honest about the dep, the dormant usage is a code-level fact already on record. | REAL | 0 |
| 16 | crates/video-engine/Cargo.toml | 2 path deps (media-types, media-clock), zero external | "needs media-types + media-clock only; plugin-api joins with the video-filter chain (a later card)" — the two deps verified; the "later card" never landed (W03-A03: video-engine/src/adjust.rs is UNREACHABLE, no filter chain exists), so the note remains a promise. The manifest itself declares nothing false. | REAL | 0 |
| 17 | crates/packet-queue/Cargo.toml | 1 path dep (media-types), zero external, no features | "`media-types` is the ONLY dependency" — verified (single `[dependencies]` entry). Concurrency = std Mutex+Condvar — consistent with the queue source (queue.rs uses `std::sync`). The cleanest manifest in the set (agrees with W03-A05). | REAL | 0 |
| 18 | crates/media-clock/Cargo.toml | 1 path dep (media-types), zero external | "deps: media-types only… never touches I/O, device APIs or UI" — verified (single dep; the crate observes audio via the injected `SampleCounterSource` trait, per the clock source read during my scheduling.rs audit). | REAL | 0 |

## 3. Findings

### F-001 [S4] playback-engine's header still says seek/sync are excluded while the crate ships them

Where: crates/playback-engine/Cargo.toml:5-8 vs crates/playback-engine/src/lib.rs:103
Code (manifest):
```toml
# The minimal state machine (2.C2 scope): the 8 wire-visible playback
# states, two-level session epochs, and the trait-level pipeline wiring
# demux → packet queues → decoders → sinks. Seek/sync are deliberately
# EXCLUDED (Phase 3: cards 3.A1-3.A3, 3.B1).
```
Code (lib.rs:103): `pub mod seek;`
What is wrong: the Phase-3 cards landed (seek.rs exists and is a public module; the seek_flow/track_switch suites my wave audited prove the behavior end-to-end), but the manifest's scope statement was never updated. Anyone reading the manifest to learn the crate's scope is told seek does not exist.
Why it matters: the manifest header is the scope-of-record for the crate; it understates the shipped surface (the inverse of the usual inflation — but still wrong).
What correct looks like: "Seek landed with cards 3.A1-3.A3 (seek.rs); sync primitives with 3.B1."
Confidence: HIGH — Verification: read (manifest + `ls src/seek.rs` + lib.rs:103).
(Cross-ref: W03-A05's per-file audit found the same; re-derived independently, verdict unchanged.)

### F-002 [S4] input's `contract` feature: honest gate, but its evidence consequence is invisible in the manifest

Where: crates/input/Cargo.toml:24-30
Code:
```toml
[features]
# The executable-contract module (`src/contract.rs`) is test/conformance
# surface only (audit F-2, error-surface-20261023 §3): unit tests compile
# it via cfg(test); this feature is how integration tests, the future doc 06
# §7.2 conformance kit, and any downstream test harness opt in. Release
# binaries (no feature) carry zero of its panicky conformance vocabulary.
contract = []
```
What is wrong: nothing in the gate itself — it is documented, intentional, and the release-build consequence stated in the comment is true. The unrecorded consequence: every `--features contract`-gated test is outside the default `cargo test --workspace` run, so the project's headline test count (W00's 1751) and any evidence string citing "input tests pass" excludes them unless the CI all-features lane (ci.yml:234-235) is named. W03-A03 already flagged the concrete instance (truncation.rs's 6 acceptance tests).
Why it matters: feature-gated tests are the silent-coverage class §3.4 hunts; a manifest that gates tests should say "these do not run by default" as loudly as it says "release carries none of it".
What correct looks like: add one line to the comment: "default `cargo test` does NOT run these; CI's --all-features lane does (ci.yml step 3)."
Confidence: HIGH — Verification: read + ci.yml:228-240 + release.yml:193 + the W00 baseline scope (plain `cargo test --workspace`).

## 4. Contradictions

| This manifest says | Other source says | Which is true (proof) |
|---|---|---|
| playback-engine: "Seek/sync are deliberately EXCLUDED" (:6-8) | lib.rs:103 `pub mod seek;` + tests/seek_flow.rs (21 end-to-end AC tests, W03-A05's audit: REAL) | the code — seek is shipped and tested; the header is stale (F-001) |

## 5. Fabrication / inflation indicators

None found: dependency counts, externals counts, and C5 (std-only) claims all verify exactly across the seven files; the single feature is self-declared as test-only and behaves as declared in both build configurations checked.

## 6. What was checked and how (zero-findings rows, §13 requirement)

- **apps/cli**: every `[dependencies]` entry's usage grep-verified in apps/cli/src (16/16 used — the five least obvious quoted in the table); `[features]` absent (grep); lock pins for both externals; bin/lib naming claims vs `[[bin]]`/`[lib]`.
- **audio-engine**: dep list vs usage (media-clock consumed by the resampler/drift modules per W03-A03's source audits); zero externals; no features.
- **video-engine**: two deps verified; no features; the deferred plugin-api note checked against reality (never landed — informational, not a manifest defect).
- **packet-queue**: single dep verified against the source's imports; no features.
- **media-clock**: single dep verified; the "observes, never drives" claim consistent with the `SampleCounterSource` injection read during my scheduling.rs/common audit.

## 7. Matrix rows (if feature-matrix)

N/A — but the claim ledger should carry: "the shipped release build enables NO optional features anywhere in the engine-crate set; the only gated code (input::contract) is conformance surface, absent from release by design." Any matrix row claiming a feature-flag-gated capability as shipped must name a flag the release build actually passes — none exists.
