# AUDIT: apps/cli/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-21 |
| Lines / functions | 71 / config keys: package, lib, bin, dependencies×17, dev-dependencies×1, lints |
| File verdict | REAL (shipped entry-point manifest; one open architecture violation) |
| Reachable from shipped entry point? | IS the shipped entry point: `[[bin]] name = "lumen"` (line 43-45) — the binary baseline MP4 smoke ran on (`lumen 0.1.3`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `rg -l '<crate>::' apps/cli/src` per dep (all 17 deps used, counts below); `rg -l testkit apps/cli/tests` (9 test files); lock greps (toml ×1 v1.1.8+spec-1.1.0, serde_json ×1 v1.0.154, lumen-cli ×1 v0.1.3); boundaries.toml:512-517 diff; baseline INDEX #11 (check-architecture exit 1), #1 (build), MP4 smoke (bin self-report 0.1.3) |

## 1. One-paragraph verdict
The manifest of the shipped `lumen` binary: 15 internal deps + `streaming` + 2 externals, a `[lib]` target (consumed by apps/desktop, the composition root — boundaries.toml:45 `desktop->lumen-cli`, apps/desktop/Cargo.toml:40), and a `[[bin]]` named `lumen`. Every declared dependency is genuinely referenced by `apps/cli/src` (verified per-crate), the lock resolves each to exactly one version, and the version `0.1.3` matches the binary's self-report in the baseline smoke run. The defect: `streaming = { path = ... }` (line 62) is an edge the architecture allowlist never ratified — boundaries.toml:514 omits it and this file's own header (lines 8-18) omits it — leaving `cargo xtask check-architecture` failing (baseline #11) and the shipped entry point silently pulling the entire network stack (ureq, rustls, ring) that the header's "first vertical pipeline" story never mentions.

## 2. What actually works (verified)
— All 17 deps used: audio_engine 1 file, audio_output 2, codec_api 12, codecs 4, commands 21, demux 13, input 2, media_clock 15, media_probe 1, media_types 16, playback_engine 14, plugin_api 14, renderer_wgpu 1, settings_schema 2, streaming 1, video_engine 1, serde_json 2, toml 2 (rg per crate). testkit: 9 dev test files. No dead deps.
— Lock integrity for this manifest: exactly one `toml` (1.1.8+spec-1.1.0), one `serde_json` (1.0.154), one `lumen-cli` (0.1.3) in Cargo.lock.
— `[[bin]] name = "lumen"` + version 0.1.3 == baseline MP4 smoke binary self-report (`lumen 0.1.3`, INDEX MP4 section).
— Header's decoder claim ("MJPEG/PCM decoders") matches baseline `lumen engine-info` → `decoders: mjpeg, pcm_s16le`.

## 3. Findings
### F-001 [S2]
Where: apps/cli/Cargo.toml:62 (+ header omission at :8-18)
Code:
```
streaming = { path = "../../crates/streaming" }
```
What is wrong: The dependency edge lumen-cli→streaming is declared here but registered nowhere: (a) boundaries.toml:514 `[crates.lumen-cli] internal_deps = ["audio-engine", ..., "video-engine"]` — 15 entries, no `streaming`; (b) this file's header (lines 8-18) enumerates and justifies every other internal dep and omits `streaming` entirely. Baseline run #11 proves the consequence: `cargo xtask check-architecture` exits 1 with exactly this edge violation — still unfixed at HEAD (e83e5bd3 + audit commits).
Why it matters: This is drift between the declared architecture and the shipped entry point, and the gate that should enforce it is red. Worse for trust: the manifest's header presents the CLI as the "first vertical's real pipeline" while quietly linking the full network stack (ureq→rustls→ring C/asm) into the shipped binary; the one dep without a written justification in this file is the one that drags in TLS. An unratified edge at the composition root is precisely what the allowlist exists to catch.
What correct looks like: boundaries.toml `[crates.lumen-cli] internal_deps` gains `"streaming"` (Lead-owned registration), and this header gains a streaming bullet (what it serves: `http(s)` URL opens via demux_host.rs:461-492) — or the dep is removed if URL open is not a shipped 0.1.3 feature.
Confidence: HIGH — Verification: read (boundaries diff + sed quotes) + baseline #11 cited.

### F-002 [S4]
Where: apps/cli/Cargo.toml:64-65
Code:
```
serde_json = "1.0"
toml = "1"
```
What is wrong: Loose version specs. The house admission style documented in streaming's manifest ("Exact pin (house admission style): a bump re-triggers the record per doc 11 §3.7") is not followed for the CLI's two direct externals; any semver-compatible bump lands silently via `cargo update`.
Why it matters: Inconsistent pin discipline at the shipped entry point; the doc-11 §3.7 re-review trigger the repo advertises does not fire for these two.
What correct looks like: `serde_json = "=1.0.154"`, `toml = "=1.1.8"` (or a documented exception for the "already-admitted via owner crates" family).
Confidence: HIGH — Verification: read (lock versions cited); severity LOW because Cargo.lock pins builds regardless.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package] name="lumen-cli"` | 32-37 | REAL | — | name/bin split documented :4-6; version 0.1.3 == shipped binary self-report (baseline smoke). |
| `[lib] name="lumen_cli"` | 39-41 | REAL | — | Consumed by apps/desktop/Cargo.toml:40 (the composition root) — the lib target is not dead. |
| `[[bin]] name="lumen"` | 43-45 | REAL | — | The shipped entry point; baseline ran it (MP4 smoke, engine-info). |
| 15 internal deps (audio-engine … video-engine, excl. streaming) | 48-61,63 | REAL | — | Each used in src (rg counts §2); each registered in boundaries.toml:514. |
| `streaming` dep | 62 | REAL / UNREGISTERED | S2 | Used (demux_host.rs) but unregistered edge → F-001. |
| `serde_json = "1.0"` | 64 | REAL | S4 (F-002) | Used in 2 src files; single lock version 1.0.154. |
| `toml = "1"` | 65 | REAL | S4 (F-002) | Used in 2 src files (settings validation seam); lock 1.1.8+spec-1.1.0. |
| `[dev-dependencies] testkit` | 68 | REAL | — | 9 test files use it; testkit is dev-only-by-contract (boundaries dev_only). |
| `[lints] workspace = true` | 70-71 | REAL | — | Inheritance verified (file-1 audit loop-grep). |

## 5. Test honesty
No tests in the manifest. apps/cli/tests contains 9 integration test files (bench_decode, command_surface, container_conformance, native_seek, pipeline_accounting, session_script, sync_scenarios, ttf_seek, unsupported_path) — honesty of those belongs to the cli-source agents; baseline #3 (1681 passed) covers them.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "External deps (2, both already Lead-admitted workspace externals)" | apps/cli/Cargo.toml:20-21 | REAL-UNPROVEN | serde_json/toml each ×1 in lock; "admitted via owner crates" not re-verified here (records owned by other crates' audits). |
| Header dep enumeration = the shipped pipeline | apps/cli/Cargo.toml:8-18 | FALSE (by omission) | 16th dep `streaming` at :62 absent from the enumeration — F-001. |
| "audio path over the null device … no audio hardware in this sandbox" | apps/cli/Cargo.toml:13-14 | REAL | Baseline engine-info: `features: … audio.null-device …` — the shipped binary indeed has no real audio output. |

## 7. Dead / duplicated / unreachable code
None: every dep used; lib target has an external consumer (apps/desktop); no duplicated deps.

## 8. Dependencies & flags
No `[features]` table — decoder/feature limitations (mjpeg/pcm only) are runtime, not cargo features. No build-deps. 16 internal + 2 external deps all resolved to single lock versions.

## 9. Open questions for the next agent
- Who owns the lumen-cli→streaming registration fix (Lead "at wave close" per streaming/Cargo.toml:39-41)? Until then every check-architecture run stays red — does CI treat it as blocking or is it in an allow-fail lane? (W01-A01 will check .github/workflows when tracing xtask gates; matrix/doc agents should check whether http(s) open is advertised in docs/feature-matrix.toml — if a row claims network open works, it needs this edge legal AND a baseline run, which does not exist.)
- The `--control-stdio` machine mode and `--json` events claim (:23-25) — verified by apps/cli/tests/command_surface.rs? Belongs to cli-source agents.
