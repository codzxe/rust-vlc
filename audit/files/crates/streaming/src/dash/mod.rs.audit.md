# AUDIT: crates/streaming/src/dash/mod.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Lines / functions | 39 / 0 (module facade: 4 `pub mod`, 3 `pub use` groups) |
| File verdict | UNREACHABLE (facade over real submodules; no shipped entry point reaches the DASH lane, and the lane has no session/opener at all) |
| Reachable from shipped entry point? | NO — rg `DashScheduler|plan_representation|select_representation_index` across crates/ + apps/: hits only inside `streaming/src/dash/*` itself. No `open_*` session exists for DASH (contrast HLS's `open_vod`/`open_live`/`open_master`), no CLI/desktop/web wiring, and no socket e2e test exists (crates/streaming/tests/ contains hls_*/ranged_wav/auth e2e files — none for dash). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |

## 1. One-paragraph verdict
A clean facade whose doc comment accurately describes its four submodules — but the lane it fronts is a bigger hole than the HLS lane: DASH has parsing (playlists::mpd), URL/segment planning (resolve), representation selection (abr), and scheduling arithmetic (sched), and NO session that fetches segments and serves bytes. There is no DASH equivalent of `open_vod`/`open_live`; nothing a host could play through even if it wanted to wire it. The matrix row (network-dash, "partial") discloses "mockhttp socket e2e + DASH-IF vectors pending" — honest about the missing e2e, silent about the missing session layer. The facade also compiles a fuzz harness into the production library.

## 2. What actually works (verified)
— Module compiles and re-exports resolve (the submodule's own lane); the exported names exist (rg confirms all re-exported symbols are defined in their submodules). The submodules' internal tests run under the crate baseline (streaming lib tests, cited from audit/runs baseline: 4744 passed workspace-wide). Nothing in THIS file executes logic — it is declarations only.

## 3. Findings
F-001 [S3]
Where: crates/streaming/src/dash/mod.rs:25-39 (lane-level)
Code:
```
pub mod abr;
pub mod fuzz;
pub mod resolve;
pub mod sched;
```
What is wrong: the DASH lane stops at arithmetic. `resolve` plans URLs, `abr` picks a representation, `sched` computes availability/live-edge — but there is no fetch-loop session producing a `SourceStream` (no `open_dash`, no segment downloader, no buffer model). HLS got three sessions; DASH got zero. Even fully wired into the CLI, DASH playback would be impossible with the current tree. The matrix's "partial" status with evidence citing only parse/plan/fuzz/abr/sched is technically honest but a reader could reasonably infer a playable lane. What correct looks like: either the session card (with its own honest "pending" note) or a matrix note saying "no session — not playable in any configuration".
Confidence: HIGH — Verification: rg for session/opener symbols in dash/**; tests/ directory listing (no dash e2e).
F-002 [S4]
Where: crates/streaming/src/dash/mod.rs:26 (with crates/streaming/src/dash/fuzz.rs:1-40)
Code:
```
pub mod fuzz;
```
What is wrong: the fuzz harness (`fuzz_demux_mpd` + its deterministic sweep) is a public module of the PRODUCTION library, not a test-only target — it compiles into every release build of the streaming crate (rlib weight; the final binary may linker-strip the unreferenced code, but the source-level separation is wrong for a "test/conformance-only" concern; compare input's `contract` module which is correctly `#[cfg(any(test, feature = "contract"))]`). The harness's own doc says its contract is "absence-of-panic/hang/unbounded-allocation, nothing else" — a test concern. What correct looks like: `#[cfg(any(test, feature = "fuzz"))] pub mod fuzz;` mirroring the input crate's pattern.
Confidence: HIGH — Verification: read mod.rs:26 (no cfg) + fuzz.rs (no cfg on the module items; only :87 `#[cfg(test)]` for its tests).
F-003 [S4]
Where: crates/streaming/src/dash/mod.rs:36-39 (re-exports)
Code:
```
pub use sched::{
    DashSchedError, DashScheduler, FetchStep, NextStep, SAFE_BEHIND_LIVE_EDGE_S,
    live_edge_presentation_seconds, parse_iso8601_epoch_seconds, segment_availability_epoch,
};
```
What is wrong: `parse_iso8601_epoch_seconds` is a SECOND ISO-8601 parser in the streaming crate, sibling to `hls::timestamp_map::parse_iso8601_utc` — in which I EXECUTED an integer-overflow panic/wrap on unbounded years (see timestamp_map.rs audit F-001). Checked in this audit: sched.rs:96-118 parses FIXED-POSITION fields with a 4-digit year (`num(0..4)`), so the 9-digit-year crash class CANNOT occur there — the sibling bug is NOT shared (verified by read). The finding that stands is the duplication itself: two same-job ISO parsers in one crate with divergent grammars and validation depth (timestamp_map validates month lengths; sched accepts day 1..=31 for every month — that residue belongs to sched's own lane).
Confidence: HIGH on the duplication (read); the sibling-overflow question is RESOLVED — sched does not share it (fixed-width year, verified by read).

## 4. Per-function table
Not applicable — 0 functions; declarations only. Submodule audits are other agents' lanes (resolve/abr/sched/fuzz are separate assignment rows).

## 5. Test honesty
No tests in this file. The lane's evidence lives in submodule tests + the netfuzz target; the absence of any socket-level DASH e2e (while HLS has four e2e files) is itself the honesty gap noted in F-001.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "resolve … a pure function of the parsed model plus the manifest URL, bounded by typed plan budgets" | mod.rs:10-11 | REAL-UNPROVEN (submodule lane) | resolve.rs audited elsewhere; bounds consts cited in fuzz.rs docs |
| "fuzz … the in-crate fuzz harness … panic-free by construction" | mod.rs:12-15 | REAL-UNPROVEN | harness exists; "by construction" is a claim about the parser, not executed here |
| matrix network-dash "Partial per 6.C2 flip: … static+dynamic scheduling with live availability join landed; … mockhttp socket e2e + DASH-IF vectors pending" | docs/feature-matrix.toml:1454 | REAL but incomplete | e2e absence confirmed (no dash e2e file); session-layer absence NOT disclosed — F-001 |

## 7. Dead / duplicated / unreachable code
- Whole lane unreachable from shipped entry (F-001).
- `pub mod fuzz` in production (F-002).
- ISO-8601 parser duplication with timestamp_map (F-003) — two same-job parsers in one crate.

## 8. Dependencies & flags
No direct deps in this file (facade). Submodules depend on `playlists::mpd`. No feature gates — everything compiles in the default release lane, reinforcing F-002.

## 9. Open questions for the next agent
- sched.rs owner: does `parse_iso8601_epoch_seconds` bound the year? (Executed proof of the sibling bug: audit/runs/w10a06-verify/verbatim_probe.rs pattern — same probe technique applies.)
- Was a DASH session card ever scheduled (6.C3+?), or does the wave program consider DASH complete at planning+ABR?
