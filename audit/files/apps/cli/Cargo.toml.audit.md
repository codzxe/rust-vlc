AUDIT: apps/cli/Cargo.toml
===========================

| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-17 |
| Lines / functions | 71 / manifest (0 functions) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — this manifest DEFINES the shipped entry point: bin target `lumen` (lines 43-45) + lib `lumen_cli` (39-41). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |

Commands run (static):
- Read file complete
- Per-crate usage count: `for c in <15 crates>; rg -t rust "${c}::|use ${c}" src tests | wc -l` → every internal dependency has ≥3 real usages (commands 131, media_types 104, demux 54, playback_engine 33, media_clock 37, plugin_api 30, codec_api 21, codecs 19, streaming 10, settings_schema 9, video_engine 7, renderer_wgpu 6, audio_engine 5, input 5, audio_output 3, media_probe 3) — no dead internal deps.
- `rg -n "lumen-cli" -A2 Cargo.lock` → locked at 0.1.3 with the full dep set; workspace lockfile present (W00 baseline built `--release --locked` PASS).
- `[features]` section: ABSENT (verified by read) — no feature gates exist in this crate.

## 1. One-paragraph verdict

An honest manifest. Package/bin split (`lumen-cli` vs `lumen`) is deliberate and documented (collision-avoidance for `cargo run -p`); `publish = false`; edition 2024; workspace lints inherited. All 15 internal path dependencies are used (verified by usage counts, none zero); the 2 external deps (`serde_json`, `toml`) are exactly the two the header justifies with layering arguments (the command layer's wire format; the settings validator's own value type) and both are used in code (`serde_json` in run.rs machine mode; `toml` at the settings seam). Critically for the wave's core question: there is NO `[features]` section — every decoder arm in pipeline_host.rs (including h264 at :346 and aac at :288) compiles unconditionally into the shipped `lumen` binary; nothing is flag-gated off. Two hygiene items only: the externals are caret-pinned in the manifest (mitigated by the locked workspace), and the header's dependency story is duplicated prose that can drift from the table below it.

## 2. What actually works (verified)

— verified by usage counts (command above): zero unused internal dependencies; no unreferenced crates.
— verified by read: no `[features]` ⇒ the h264/aac/vp9/av1 decoder dispatch (pipeline_host.rs:288, 304-346) is unconditionally compiled — the "no decoder in this build" bench message (bench.rs:334) cannot be attributed to a disabled feature.
— verified by Cargo.lock read: `lumen-cli 0.1.3` locks serde_json/toml transitively; W00 baseline release build used `--locked` (baseline build-release.log).

## 3. Findings

### F-001 [S4] — external deps caret-pinned in-manifest; lockfile is the only pin
Where: apps/cli/Cargo.toml:64-65
Code:
```toml
serde_json = "1.0"
toml = "1"
```
What is wrong: bare caret requirements; `toml = "1"` even omits the patch segment. Safety rests entirely on the committed Cargo.lock and release builds using `--locked`.
Why it matters: any non-locked build (`cargo install --path`, a consumer pulling the lib) floats within the major; hygiene per §3.5 "unpinned".
What correct looks like: `serde_json = "=1.0.x"`-style pins or a deny/audit policy entry (cargo-deny config is another wave's file).
Confidence: HIGH — Verification: read + Cargo.lock cross-check.

### F-002 [S4] — 31-line prose header duplicates the dependency table
Where: apps/cli/Cargo.toml:1-31 vs 47-65
Code:
```toml
# External deps (2, both already Lead-admitted workspace externals via
# the crates that own them — see boundaries.toml):
```
What is wrong: the header narrates the dependency set in prose ("codecs + codec-api (MJPEG/PCM decoders behind the Decoder contract)") that is already stale relative to the code it documents — the decoders now include h264/aac/vp9/av1/etc. (pipeline_host.rs:288-346), and `streaming` (line 62) appears nowhere in the prose block.
Why it matters: the prose says "MJPEG/PCM decoders" in 2026-11 while the binary dispatches 14 codec descriptors; a reader of the header underestimates the crate exactly the way the bench message does.
What correct looks like: drop the inventory prose; keep the layering rationale only.
Confidence: HIGH — Verification: read (prose vs the pipeline_host dispatch table).

## 4. Per-function table

Not applicable (manifest). Sections audited:
| Section | Lines | Verdict | Notes |
|---|---|---|---|
| header prose | 1-31 | SHALLOW | S4 (F-002): rationale good, inventory stale. |
| `[package]` | 32-37 | REAL | 0.1.3 matches shipped releases; publish=false. |
| `[lib]`/`[[bin]]` | 39-45 | REAL | lib+bin split documented and correct. |
| `[dependencies]` | 47-65 | REAL | 15 internal (all used — verified) + 2 justified externals (both used). S4 pin style. |
| `[dev-dependencies]` | 67-68 | REAL | testkit only. |
| `[lints]` | 70-71 | REAL | workspace = true (central lint policy). |

## 5. Test honesty

Not applicable.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "External deps (2, both already Lead-admitted…)" | Cargo.toml:20-21 | REAL | exactly 2 externals; boundaries.toml existence UNVERIFIED (another wave's file). |
| "codecs + codec-api (MJPEG/PCM decoders behind the Decoder contract)" | Cargo.toml:11-12 | FALSE (stale) | pipeline_host.rs:288-346 dispatches 14+ descriptors (h264 landed 2026-11-15 per its comment). |
| "no audio hardware in this sandbox, ADR-005 era" / "no GPU … 2.D1's recorded decision" | Cargo.toml:13-16 | REAL | audio-output null backend; renderer-wgpu software compositor (W00 baseline corroborates: "no decoder in this build (first vertical: mjpeg + pcm)" bench + null-device features in engine-info). |

## 7. Dead / duplicated / unreachable code

None: no unused dependencies (verified by counts), no features to be never-enabled, no dev-dep leakage into production.

## 8. Dependencies & flags

This IS the dependencies file. The absence of `[features]` is load-bearing evidence for the wave's core question — see §2. Workspace membership: apps/cli is a workspace member (root Cargo.toml members; membership comments at root lines 4-37 are another agent's file).

## 9. Open questions for the next agent

1. Root workspace Cargo.toml + Cargo.lock policy: who enforces `--locked` for release builds (release workflow is Wave 17's file) — F-001's mitigation depends on it.
2. boundaries.toml (dependency admission record): verify serde_json/toml admissions and whether the h264-bringing `rust_h264` 0.6.1 admission (referenced at pipeline_host.rs:332-336) is recorded there.
