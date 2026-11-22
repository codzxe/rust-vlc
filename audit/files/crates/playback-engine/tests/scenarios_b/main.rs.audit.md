# AUDIT: crates/playback-engine/tests/scenarios_b/main.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 72 / 1 function (`alloc_state`) — test-binary root: docs + 19 `mod` declarations + global allocator |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test) — `main.rs` under tests/scenarios_b/ is a cargo test binary; part of W00 baseline (execution UNVERIFIED, no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 (zero-findings disclosure in §2) |
| Commands run | ls of tests/scenarios_b/ (all 19 declared modules exist) and tests/common/; cross-check of the ARCH-5/ARCH-8 honesty note against host.rs (my file #17). No cargo (adaptation). |

## 1. One-paragraph verdict

The root of the doc 10 §6.4 scenario suite: a 16-row scenario map in the header (module → scenario → pass criteria), a process-global counting allocator so the D-2 bounded-memory assertions are measured in-process, one accessor function, and 19 module declarations. Every declared module exists, the shared `common` module is path-included from ../common (cargo's auto-discovery limitation, handled correctly), and the header contains an honest note that this suite drives the production stack through a local copy of the dispatch shape because dev-depending on `lumen-cli` would be a boundary cycle. The suite's real quality lives in host.rs and the scenario files — host.rs and three scenarios are mine (files #17, #19, #20, #21); the rest belong to other agents.

## 2. What actually works (verified — zero-findings disclosure per §13)

Checked, with method:
- **All 19 declared modules exist** with content (ls: host.rs, probe.rs, scenario_01…scenario_16, plus the path-included ../common/mod.rs).
- **Counting allocator**: `#[global_allocator] static ALLOC: testkit::CountingGlobalAlloc` (44-45) + the `alloc_state()` accessor (48-50) — the D-2 measurement plumbing the scenario docs describe; same testkit discipline as zero_alloc_resample.rs (my file #10).
- **The 16-row table** (7-24) maps 1:1 onto the 16 scenario module names — no phantom rows, no missing modules.
- **The ARCH-5/ARCH-8 honesty note** (26-33): verified while auditing host.rs — the suite does use a local `lumen_dispatch` shape rather than the production `lumen-cli` entry; the note discloses exactly this and the reason (boundaries.toml cycle). Whether the local shape has drifted from the production dispatcher is the host.rs audit's question (it is — see host.rs F-001, cross-file).

## 3. Findings

None in this file. The root is declarations plus accurate documentation; the audit weight lands on the modules. The one systemic observation (local dispatch copy vs production entry) is filed where the code lives — host.rs.

## 4. Per-function table

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| Module docs (scenario map + allocator rationale) | 1-42 | REAL | Table matches the tree; note verified against host.rs. |
| `#[global_allocator] ALLOC` | 44-45 | REAL | testkit counting allocator. |
| `alloc_state` | 48-50 | REAL | Accessor; used by scenario files' D-2 asserts. |
| `mod common` (path include) | 52-53 | REAL | ../common/mod.rs exists. |
| `mod host` / `mod probe` | 55-56 | REAL | Both exist; host.rs is my file #17. |
| `mod scenario_01…16` | 57-72 | REAL | All 16 exist; 08/09/15 are mine (files #19-#21). |

## 5. Test honesty

No tests in this file. The serializer discipline claim (40-42, "every test takes the whole-binary serializer first") is a contract the scenario files must uphold — checked in my three; the other thirteen are other agents' files.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The sixteen scenario modules each own one §6.4 row" (5-6) | module doc | REAL | 16 scenario files, 16 table rows, names match. |
| "the AVI demuxer and the MJPEG/PCM decoders are the real production stack; the named fakes are the scripted access source, the scripted demux stage, and the observing/vanishing sink" (30-33) | module doc | REAL for my files | Verified in host.rs (real demux/decoder crates used; fakes at the access/demux seams). The sink observation holds for my three scenarios; the other thirteen are others' audits. |
| "allocator live-bytes deltas stand in for RSS, which is not measurable in-proc" (39-40) | module doc | REAL | Accurate technical statement; the guard/serializer mechanics are testkit's (file #10 verified the same machinery). |

## 7. Dead / duplicated / unreachable code

None at this level.

## 8. Dependencies & flags

std + playback-engine + testkit (+ the production demux/codec crates via host.rs). No feature gates at the root. One test binary for all scenarios — the whole-binary serializer makes that sound.

## 9. Open questions for the next agent

- Scenario auditors (01-07, 10-14, 16): does each file actually take `serialize_tests()` before any measured window, as this root's doc promises (40-42)?
- W13: quantify the drift risk flagged in host.rs F-001 — the production `lumen_dispatch` (apps/cli/src/dispatch or lumen_dispatch module) vs this suite's local copy: if they diverge, this suite greens a dispatch path users do not run.
