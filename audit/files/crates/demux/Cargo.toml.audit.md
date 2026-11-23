# AUDIT: crates/demux/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Lines / functions | 39 / 7 config keys (package:5, lints:1, deps:2, dev-deps:2) |
| File verdict | REAL (with two stale header/description claims) |
| Reachable from shipped entry point? | YES — the crate is a dependency of lumen-cli (apps/cli/Cargo.toml) and the web app; baseline release lane builds clean (audit/runs/baseline/cargo-build-release-lumen-cli.txt) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | full read; rg dependency usage (media-types/plugin-api/testkit/input all used); rg unsafe/ResourceLimit verification; sed quote verification; baseline clippy cite |

## 1. One-paragraph verdict
The manifest matches the architecture claims it makes that matter: exactly the two
declared layer-L2 dependencies (media-types, plugin-api — zero external crates, C5
holds), dev-only testkit + input, workspace lints inherited, publish=false. The
"no unsafe / no unwrap/expect/panic/todo" claim is enforced by the workspace lint set
(root Cargo.toml:53-66) and the baseline clippy run is CLEAN. Three stale/partial
header claims: "every read is chunk-bounded" is false since the 7-c prefix-read
redesign, "refused above a typed ResourceLimit ceiling" is true only for some demuxers
(AVI damage-skips instead), and the package description still describes the v0 AVI-only
crate though twelve format modules ship.

## 2. What actually works (verified)
— Dependency minimality verified: `[dependencies]` = media-types + plugin-api only;
  both used throughout (rg). No unused crates.
— Dev-deps used: testkit (Must/fixture_path — every integration suite), input
  (open_via_input, tests/common/mod.rs, exercised by fixtures_class_b.rs:27).
— unsafe_code=deny holds: rg "unsafe" over src/ hits only doc comments asserting its
  absence (es/mod.rs:22, asf/mod.rs:34, asf/data.rs:47).
— Baseline: audit/runs/baseline/cargo-clippy.txt ends "Finished `dev` profile ...
  in 59.06s" (clean run; clippy -D warnings CLEAN per AGENT-SPEC §13).

## 3. Findings
F-001 [S4]
Where: crates/demux/Cargo.toml:15
Code:
```
# C5 discipline: std-only, zero external dependencies. No unsafe, no
# unwrap/expect/panic/todo (workspace lints). Bounded memory (MP §12):
# every read is chunk-bounded and payload sizes are refused above a
# typed ResourceLimit ceiling rather than trusted.
```
What is wrong: "every read is chunk-bounded" is FALSE since the 7-c/V-1 redesign:
`SourceReader::read_prefix_into` (riff.rs:357-368) — the payload path — makes one
full-size `read_into` ask by design (documented there). Only the Vec/exact paths are
READ_STEP_MAX-bounded. Same stale wording as riff.rs:304 (my F-001 there).
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/demux/Cargo.toml:15-16 (same quote)
What is wrong: "payload sizes are refused above a typed ResourceLimit ceiling" — true
for adts (adts/error.rs:127), es/annexb (annexb.rs:50) and ogg (ogg/error.rs:104,117 —
packet_overrun is ResourceLimit), but AVI over-ceiling chunks (mod.rs:517-536) are
damage-noted and SKIPPED, not refused with a typed error. The blanket claim
overstates the AVI lane's behavior. Confidence: HIGH — Verification: read + rg.
F-003 [S4]
Where: crates/demux/Cargo.toml:22
Code:
```
description = "Lumen container demultiplexers: AVI 1.0 RIFF walk, hdrl/strl track discovery, idx1 seek table, MJPEG+PCM packet extraction (card 2.B2, doc 03 §4.3/§7)."
```
What is wrong: The description describes the v0 crate. src/ ships twelve format
modules (adts, aiff, asf, avi, es, ivf, mkv, mp4, ogg, ps, ts, wav — verified by ls);
the description names only AVI. Stale metadata that undersells/misdescribes the crate
to anyone reading `cargo metadata`. Confidence: HIGH — Verification: read + ls.

## 4. Per-function table
N/A (<100 LOC; config file). Key inventory: package name/version/edition(2024)/
description/publish(false) — all sane; `[lints] workspace = true` — inherits the deny
set; dependencies 2/2 used; dev-dependencies 2/2 used.

## 5. Test honesty
N/A (no tests in a manifest). The manifest's enforcement claims are backed by the
workspace lint config and the clean baseline clippy run.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "std-only, zero external dependencies" | :13 | REAL | [dependencies] = 2 path deps only |
| "No unsafe, no unwrap/expect/panic/todo (workspace lints)" | :13-14 | REAL | root lints deny all five; clippy baseline CLEAN; rg no unsafe blocks |
| "every read is chunk-bounded" | :15 | FALSE | read_prefix_into full-ask (F-001) |
| "payload sizes refused above typed ResourceLimit" | :15-16 | PARTIAL | true adts/es/ogg; AVI damage-skips (F-002) |
| "registry row: layer L2, deps media-types + plugin-api" | :2 | REAL | matches [dependencies] |

## 7. Dead / duplicated / unreachable code
None: no unused dependencies, no declared-but-unread config keys.

## 8. Dependencies & flags
media-types, plugin-api (path); testkit, input (dev). No features, no default-feature
gaming, no optional deps. Edition 2024.

## 9. Open questions for the next agent
- If the Cargo.toml header is treated as architecture documentation, the same stale
  "chunk-bounded" sentence likely lives in other crate headers — worth a repo-wide
  grep by the docs-domain agent.
