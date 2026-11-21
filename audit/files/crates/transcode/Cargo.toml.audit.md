# AUDIT: crates/transcode/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-21 |
| Lines / functions | 66 / 14 config keys (package×6, lints, dependencies×4, dev-dependencies×3) |
| File verdict | REAL (crate itself: UNREACHABLE from shipped builds — see F-002) |
| Reachable from shipped entry point? | NO. Proof: Grep `transcode` across every `**/Cargo.toml` in the repo → matches only crates/transcode/Cargo.toml itself (zero dependents); Grep `transcode` in apps/ and tests/integration/ → zero hits. Declared off: product.toml:165 `transcode = false # Phase 7`, product.toml:164 `recording = false # Phase 7`. Matrix rows `playback-record` / `ui-dialog-convert` (docs/feature-matrix.toml:1636, 2919) honestly say `not_started` with empty evidence |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | Grep `transcode` in **/Cargo.toml (no dependents) · grep dep usage in src/ and tests/ (all 4 prod + 3 dev deps used in real code) · `grep -n "mod \|pub use" crates/transcode/src/lib.rs` (mp4mux exported at 159, 172) · sed -n on matrix rows 1630-1640, 2913-2923, 3015-3025 · sed -n 360-400p boundaries.toml (deps match exactly) · sed -n 160-170p product.toml · grep todo!/unwrap/expect/panic/unsafe in src (only doc-comment "no unsafe" claims; zero real) |

## 1. One-paragraph verdict
A clean, honest manifest whose dependency edges exactly match the
architecture declaration (boundaries.toml `[crates.transcode]`: same 4
internal prod deps + same 3 dev deps, zero external, verified line-by-line)
and whose C5 claim ("std-only, zero external dependencies") is true. Every
declared dependency is genuinely used in real code, none duplicated, no
features declared. Two defects: the header's "honest capability" block is
stale — it says NO other muxer ships while `src/mp4mux.rs` (card 7.E2) is
implemented, exported, and tested; and the whole 6,527-line crate is parked
dead — no shipped binary can reach it (declared off in product.toml, so the
deadness is at least an honest, recorded decision).

## 2. What actually works (verified)
— Dependency honesty: all four `[dependencies]` used in non-comment code:
`codec_api` (graph.rs:55 `use codec_api::Decoder;`), `demux`
(graph.rs:56/convert.rs:29 `use demux::wav::format::PcmSample;`),
`media_types` (convert.rs:30, error.rs:13, job.rs:51, limits.rs:30, …),
`plugin_api` (graph.rs:60, record.rs:123). All three dev-deps used in
tests/ (codecs, input, testkit — grep per test file).
— C5 std-only claim TRUE: zero external crates in [dependencies]/[dev-dependencies];
all path deps (sed -n '55,66p').
— Lint claims TRUE: grep for todo!/unimplemented!/unwrap()/expect(/panic! in
src/ outside comments → zero real hits; "unsafe" appears only in doc
assertions (lib.rs:145, mp4mux.rs:5) with no unsafe blocks.
— Manifest compiles in the workspace: baseline run #1 `cargo build --release
--locked --workspace` exit 0 (INDEX.md #1); its tests run in baseline #3
(1681 passed workspace-wide).

## 3. Findings
### F-001 [S3] — "honest capability" header comment and package description are stale: the MP4 muxer DID ship in-tree
Where: crates/transcode/Cargo.toml:9-11 and :49
Code:
```toml
# capability: NO other encoder and NO other muxer ships in v1 (MP4
# remux is the later 7.E2 card); every unsupported conversion is a
# typed `ErrorKind::Unsupported`.
```
What is wrong: `src/mp4mux.rs` exists — header "The minimal in-tree MP4
muxer (card 7.E2, issue #44)", exported publicly at lib.rs:159 (`pub mod
mp4mux;`) and lib.rs:172 (`pub use mp4mux::{…}`), with a dedicated
tests/mp4_remux.rs suite. The package `description` (line 49) likewise
lists only "WAV PCM encode + stream-copy sinks (card 7.E1 …)". The same
stale sentence is duplicated in boundaries.toml:377-378 ("MP4 remux is the
later 7.E2 card").
Why it matters: The manifest's capability block is the first thing a
reviewer/auditor reads to know what this crate does; it now understates the
crate and contradicts its own code. Stale "honesty" statements are how
future over/under-claims slip through unreviewed.
What correct looks like: Update header + description to name the MP4
stream-copy muxer (7.E2) and the ES sink, and fix the mirrored sentence in
boundaries.toml.
Confidence: HIGH — Verification: read (lib.rs:159,172; mp4mux.rs:1-30;
tests/mp4_remux.rs present).

### F-002 [S3] — 6,527-line crate is dead code from every shipped entry point's perspective
Where: crates/transcode/Cargo.toml:55-59 (the dependency edges nothing consumes)
Code:
```toml
[dependencies]
codec-api = { path = "../codec-api" }
demux = { path = "../demux" }
media-types = { path = "../media-types" }
plugin-api = { path = "../plugin-api" }
```
What is wrong: No manifest in the repository depends on `transcode` (Grep
`transcode` in **/Cargo.toml → only self-matches; Grep in apps/ and
tests/integration/ → zero hits). The `lumen` CLI and desktop-tauri cannot
reach it. Mitigation: product.toml:165 declares `transcode = false # Phase
7` and matrix rows honestly say `not_started` — the parking is a recorded
decision, not an accident.
Why it matters: Every workspace CI build/test lane (baseline #1/#3/#4) pays
compile+test cost for code no user can execute; and the audit must record
that despite "real RIFF/WAVE writing" and a real MP4 muxer, zero transcode
capability is customer-visible. Any future doc claiming record/convert
works would be FALSE on reachability alone.
What correct looks like: Either the Phase-7 wiring (CLI convert/record
command) or an explicit `[workspace] default-members` note that transcode
is test-only until Phase 7; at minimum keep product.toml/matrix honest
(they currently are).
Confidence: HIGH — Verification: repo-wide grep for dependents (zero);
product.toml/matrix read.

## 4. Per-key table
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `package.name = "transcode"` | 46 | REAL | — | Matches boundaries.toml [crates.transcode] |
| `package.version = "0.1.0"` | 47 | REAL | — | Repo-wide convention (no [workspace.package] inheritance exists) |
| `package.edition = "2024"` | 48 | REAL | — | Consistent with workspace |
| `package.description` | 49 | REAL | S3 | Stale: omits the shipped 7.E2 MP4 muxer (F-001) |
| `package.publish = false` | 50 | REAL | — | Never published |
| `[lints] workspace = true` | 52-53 | REAL | — | Inherits deny tables (canary-verified mechanism, see my build.rs audit) |
| `codec-api` dep | 56 | REAL | — | Used (graph.rs:55); matches boundaries declaration |
| `demux` dep | 57 | REAL | — | Used (convert.rs:29, graph.rs:56); matches boundaries |
| `media-types` dep | 58 | REAL | — | Used pervasively; matches boundaries |
| `plugin-api` dep | 59 | REAL | — | Used (graph.rs:60, record.rs:123); matches boundaries |
| dev-dep `codecs` | 63 | REAL | — | Used in tests/record.rs, wav_roundtrip.rs, job_lifecycle.rs; declared dev edge in boundaries |
| dev-dep `input` | 64 | REAL | — | Used in tests incl. mp4_remux.rs; declared dev edge |
| dev-dep `testkit` | 65 | REAL | — | Used in 3 of 4 test files; declared dev edge |
| (absent `[features]`) | — | REAL | — | No features declared → no never-enabled-feature exposure (hostile question answered for this crate) |

## 5. Test honesty
The manifest itself has no tests; the crate's four test suites run in
baseline #3 (workspace test run, 1681 passed). Their honesty is the
transcode-src auditor's scope (wave assignments), not this file's.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NO other encoder and NO other muxer ships in v1 (MP4 remux is the later 7.E2 card)" | Cargo.toml:9-11 | FALSE (stale) | src/mp4mux.rs (card 7.E2) implemented + exported (lib.rs:159,172) + tested — F-001 |
| "std-only, zero external dependencies" | Cargo.toml:40 | REAL | [dependencies] all path deps (sed -n 55,66p) |
| "No unsafe, no unwrap/expect/panic/todo" | Cargo.toml:41-42 | REAL | grep src/: zero real hits |
| "codec-api: the Decoder/FrameSink contracts … decode stage runs any host-provided Decoder" | Cargo.toml:18-21 | REAL | graph.rs:55 `use codec_api::Decoder;` |
| Matrix `playback-record` / `ui-dialog-convert` not_started | docs/feature-matrix.toml:1636/2919 | REAL | Statuses read directly; consistent with unreachability (F-002) |

## 7. Dead / duplicated / unreachable code
The crate as a whole: UNREACHABLE from shipped binaries (F-002) — but
deliberate (product.toml Phase 7 gate). No unused or duplicated
dependencies in this manifest (every edge consumed; boundaries.toml agrees
exactly).

## 8. Dependencies & flags
4 internal prod deps + 3 internal dev deps; zero external; zero features;
`[lints] workspace = true`. Matches boundaries.toml [crates.transcode]
internal_deps/dev_deps verbatim (sed -n 387,392p). No version pins needed
(path deps).

## 9. Open questions for the next agent
- boundaries.toml:377-378 carries the same stale "MP4 remux is the later
  7.E2 card" sentence — whoever audits boundaries.toml should file the
  mirror of F-001.
- If Phase 7 wiring lands, re-check that no feature flag gets invented here
  that release.yml then never enables (the W01-A03 hostile question).
- mp4_remux.rs test file does not use `testkit` — confirm no missing
  dev-dep edge when tests grow.
