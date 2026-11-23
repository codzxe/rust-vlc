# AUDIT: Cargo.lock
| Field | Value |
|---|---|
| Auditor | W01-A05 |
| Date | 2026-11-23 |
| Lines / functions | 1441 / 158 package entries (39 workspace, 119 registry) |
| File verdict | REAL (generated lockfile, in sync — audited as data) |
| Reachable from shipped entry point? | YES — consumed by `cargo build --release --locked -p lumen-cli -p desktop` (baseline exit 0, audit/runs/baseline/cargo-build-release-lumen-cli.txt:1 "cmd: cargo build --release --locked -p lumen-cli" + ":Finished `release` profile") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | python3 parse of all 158 entries (dupes/roots/reachability); grep banned crates -> 0 hits; sed verbatim-quote checks (all pass); manifest sweep of 39 Cargo.tomls (testkit placement) — exit 0 |

## 1. One-paragraph verdict
This is a real, Cargo-generated lockfile (v4 format) that is provably in sync
with the workspace manifests: the orchestrator baseline built the release lane
with `--locked` and exited 0, and cargo test per-package passed 4744 (baseline
cargo-test-workspace.txt). As data it is clean: exactly 158 entries, only 3
duplicate-version families, zero denied/banned crates, zero git or alternate-
registry sources, and every external codec engine matches its exact-pin
admission. The defects are structural, not integrity: 26 of 158 entries are
unreachable from the shipped binaries (the whole wasmi/plugin-host stack and
all platform-* crates are compiled by `--workspace` but not shipped), and the
crate named `renderer-wgpu` contains no wgpu at all.

## 2. What actually works (verified)
— python3 full parse: 158 entries / 155 unique names; local (non-registry)
set == exactly the 39 workspace members derived from members list in
Cargo.toml:41 + exclude (apps/desktop-tauri): `crates/*` (31 incl.
_template->`template`), `apps/*` (cli->lumen-cli, control-server, desktop,
web->lumen-web), tools/xtask, tools/mem-audit->lumen-mem-audit,
tools/sweep->lumen-sweep, tests/integration/first-vertical. No orphan local
package, no missing member.
— exact-pin cross-check (codecs/Cargo.toml vs lock): claxon 0.4.3 (L103),
symphonia-{bundle-mp3,codec-aac,codec-alac,codec-vorbis,core} 0.6.1
(L1004-1071), rav1d 1.1.0 (L674), ruopus 0.1.2 (L763), rusty_vp9 0.1.3
(L816), filmcraft-dnx 0.9.0 (L217), rust_h264 0.6.1 (L769). All present at
the pinned versions.
— `grep -cE 'name = "(ffmpeg|gstreamer|libvlc|vlc|openh264|x264|libvpx|libde265|libass|freetype|minimp3|opus|audiopus|vorbis|dav1d|bindgen|libloading)"' Cargo.lock` -> 0. No deny.toml-banned crate is locked.
— every source line is `registry+https://github.com/rust-lang/crates.io-index` (no git deps) — consistent with deny.toml:404-407 `unknown-git = "deny"`.
— manifest sweep: `testkit` appears in [dev-dependencies] only across all 39 member manifests (lockfile cannot distinguish; manifests prove the MP §3.7 dev-only contract holds).
— baseline: `cargo build --release --locked -p lumen-cli` finished 2m28s exit 0; `-p desktop` exit 0.

## 3. Findings
F-001 [S4]
Where: Cargo.lock:248-267 (also 270-282, 1087-1106)
Code:
```
[[package]]
name = "getrandom"
version = "0.2.17"
...
[[package]]
name = "getrandom"
version = "0.4.3"
```
What is wrong: three duplicate-version families ship in one graph: getrandom
0.2.17 (via ring/rustls) + 0.4.3 (via jobserver/cc), hashbrown 0.15.5 (via
string-interner/wasmi) + 0.17.1 (via indexmap/toml), syn 2.0.119 (via
atomig-macro/strum_macros/zerocopy-derive) + 3.0.9 (via serde/schemars/
thiserror macro stacks). Why it matters: duplicate rand/hashmap/proc-macro
code inflates the binary and the build; deny.toml:153 sets
`multiple-versions = "warn"` with a [PROPOSAL] to flip to deny that has not
happened. What correct looks like: dedup pass or the promised deny flip.
Confidence: HIGH — Verification: ran (python3 parse) + read.

F-002 [S3]
Where: Cargo.lock:623-637 (plugin-host-wasm), 561-586 (platform-*), 988-995 (subtitles), 1188-1199 (transcode), 190-196 (device-discovery)
Code:
```
[[package]]
name = "plugin-host-wasm"
version = "0.1.0"
dependencies = [
 "codec-api",
 ...
 "wasmi",
]
```
What is wrong: 26 of 158 entries are unreachable from the release-lane roots
(lumen-cli, desktop), verified by transitive-closure parse. Nothing in the
workspace depends on plugin-host-wasm, platform-linux, platform-macos,
platform-windows, subtitles, transcode, or device-discovery (reverse-dep
count 0), so the entire wasmi 2.0.0 stack (wasmi, wasmi_collections,
wasmi_core, wasmi_ir, wasmparser, spin, string-interner, libm — 8 registry
crates) exists in this lockfile only to be built by `--workspace` lanes,
never shipped. Why it matters: the plugin host and per-platform audio edges
are unwired; `cargo build --release -p lumen-cli -p desktop` (the shipped
lane per AGENT-SPEC §7) links none of it; the wasmi tree is pure compile
cost. What correct looks like: wire the consumers or document the members as
deliberately-unshipped incubation crates in the registry doc.
Confidence: HIGH — Verification: ran (closure from lumen-cli/desktop).

F-003 [S3]
Where: Cargo.lock:738-746
Code:
```
[[package]]
name = "renderer-wgpu"
version = "0.1.0"
dependencies = [
 "media-clock",
 "media-types",
 "testkit",
 "video-engine",
]
```
What is wrong: the crate named `renderer-wgpu` has zero wgpu dependency —
its dep list is four in-house crates. The wgpu-30.0.1 admission record
(docs/dependencies/records/wgpu-30.0.1.md) says wgpu was conditionally
deferred (option 2, "backend seam", NOT admitted), so the name promises a
GPU renderer the graph does not contain. Why it matters: checklist 6.5
"names that lie"; anyone auditing "the wgpu renderer" from the name will
find a software-path stub seam. What correct looks like: rename
(renderer-soft / renderer-backend) or land wgpu.
Confidence: HIGH — Verification: read + ran (dep-list parse).

F-004 [S4]
Where: Cargo.lock:1406-1408 (also root Cargo.toml:37-40)
Code:
```
[[package]]
name = "xtask"
version = "0.1.0"
```
What is wrong: the xtask tool that implements feature-audit, parity-report,
config-validator, sbom and check-architecture has ZERO dependencies — no
TOML parser, no serde, nothing. The root manifest documents the consequence
(Cargo.toml:37-40): the TOML reader is hand-rolled and "does not support
multi-line arrays", forcing the workspace `members` array to stay
single-line. Why it matters: the gate tooling parses structured config with
a bespoke parser; a legal TOML reshaping of the root manifest silently
breaks the arch gate (the comment says it already happened once in "the
salvage"). What correct looks like: a toml dep for xtask (it is a tool, not
a shipped crate; the std-only C5 policy does not apply to tools — mem-audit/
sweep also carry no deps, but they don't parse TOML).
Confidence: HIGH — Verification: read (lock + manifest comment).

F-005 [S4]
Where: crates/codecs/Cargo.toml:146 (cross-reference; lock entry at Cargo.lock:217-220)
Code:
```
# 6.M-dnx (Lead admission, docs/dependencies/records/filmcraft-dnx-0.1.0.md):
```
What is wrong: the manifest cites record `filmcraft-dnx-0.1.0.md`, but the
file on disk is `docs/dependencies/records/filmcraft-dnx-0.9.0.md` and the
locked/pinned version is 0.9.0. Stale doc pointer. Why it matters: the
admission-record chain is the audit trail (doc 11 §3.7); a wrong filename
breaks provenance lookup. What correct looks like: fix the comment to
filmcraft-dnx-0.9.0.md. (File owner: codecs manifest agent.)
Confidence: HIGH — Verification: ran (`ls docs/dependencies/records/`) + read.

## 4. Package-class inventory (per-function equivalent — sampled 100% of entries programmatically)
| Class | Count | Verdict | Notes |
|---|---|---|---|
| Workspace members | 39 | REAL | All match members list + exclude; names verified per-dir |
| Codec engines (exact-pinned) | 10 | REAL | All at admitted pins (§2) |
| wasmi plugin stack | 8 | UNREACHABLE (release lane) | Only consumer plugin-host-wasm itself unwired |
| TLS/HTTP (ureq/rustls/ring/webpki) | 11 | REAL | Via lumen-cli->streaming->ureq; ring pulls getrandom 0.2 |
| Symphonia common tree | 6 | REAL | shared by mp3/aac/alac/vorbis features (default-on) |
| Proc-macro stacks | syn2/3 + friends | REAL | causes F-001 syn dupe |
| Windows target shims | 10 | REAL | windows-sys 0.52 (ring) + windows-link 0.2 (parking_lot_core) — two families, normal |
| rav1d build graph | cc/nasm-rs/jobserver/raw-cpuid/strum/paste/atomig/zerocopy | REAL | Note: cc+nasm-rs remain locked even though admission pins asm OFF (default-features=false); build.rs is present-but-inert per record R3 — lockfile cannot prove feature state (UNVERIFIED: features not encoded in lock) |
| template | 1 | REAL | Intentional lint-canary member, publish=false, no deps (L1108-1110) |
| xtask / tools | 3 | REAL | xtask dep-free (F-004); mem-audit/sweep mirror engine crates |

## 5. Test honesty
N/A — generated data file, no tests. Baseline evidence instead: --locked
release build exit 0 proves manifest<->lock consistency; 2 failing
first-vertical tests (baseline) are unrelated to the lock.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "crates.io only: no git dependencies" | deny.toml:400-407 | REAL | every source line is the crates.io registry (§2) |
| "license minimal set for current lockfile (22 registry crates)" | deny.toml:73 | STALE-COMMENT (S4-grade, file not in my list) | lock now has 119 registry entries; comment self-describes as historical, later cards appended classes |
| rav1d admission "default-features=false + bitdepth_8/16" | codecs/Cargo.toml:125 | REAL-UNPROVEN | pin text read; lock cannot encode feature resolution (UNVERIFIED: no cargo tree run — load discipline) |
| wgpu "conditional (E2), NOT admitted" | records/wgpu-30.0.1.md | REAL | no wgpu in lock; renderer-wgpu has no wgpu dep (F-003) |
| truehd record "blocked — no dependency edge, no module ships" | records/truehd-0.7.2.md R10 | REAL | no truehd/bitstream-io in lock |
| cpal "not admitted and not a workspace dependency" | records/audio-output-cpal.md | REAL | no cpal in lock |
| engine-info "decoders: mjpeg, pcm_s16le" vs error text "landed: ... av1, h264" | baseline mp4-smoke.md | CONTRADICTION for the registry agent | lock shows av1 (rav1d) + h264 (rust_h264) + 8 more engines all compiled into `codecs` with default features ON (codecs/Cargo.toml default = full set) — the binaries CAN contain them; why engine-info registers only 2 is a runtime-registry question outside this file |

## 7. Dead / duplicated / unreachable code
See F-002 (26 unreachable entries) and F-001 (3 dupe families). No unused
registry orphans: every non-member entry is reachable from some workspace
member (verified by reverse-dep closure over all 39 members).

## 8. Dependencies & flags
Lock format v4 consistent with pinned toolchain rust-toolchain.toml:9
`channel = "1.99.1"`. lumen-cli 0.1.3 / desktop 0.1.3 (only members not at
0.1.0 — version bumps, harmless). ureq/rustls stack reachable from shipped
CLI via streaming (Cargo.lock:943-954): the shipped binary carries a full
TLS stack for network streaming. getrandom 0.2.17 (via ring) is a 2020-era
major kept alive only by ring 0.17 — the usual rustls tax.

## 9. Open questions for the next agent
- Who should depend on plugin-host-wasm / platform-*/subtitles/transcode/
  device-discovery, and does any doc promise them in the shipped product?
- Trace the decoder-registry lie: engine-info lists mjpeg+pcm only while
  codecs compiles 11+ engines by default (§6 last row) — registry agent.
- codecs/Cargo.toml:146 stale record filename (F-005) — codecs manifest agent.
- Cargo.lock:1411-1418 zerocopy 0.7.35 (via rav1d) is an old line; check
  advisories posture against deny.toml ignore ledger (only RUSTSEC-2024-0436
  paste is listed; zerocopy 0.7 has had advisories upstream — UNVERIFIED
  offline, flag for the cargo-deny run owner).
