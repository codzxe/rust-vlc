# AUDIT: crates/demux/src/lib.rs

| Field | Value |
|---|---|
| Auditor | W05-O (orchestrator) |
| Date | 2026-11-16 |
| Lines / functions | 56 / 0 (module declarations + crate doc only) |
| File verdict | REAL (code) — crate doc STALE |
| Reachable from shipped entry point? | YES — `lumen open` → apps/cli host → demux_host → containers dispatcher → this crate's modules (chain derived and cited by W05-A01: demux_host.rs:458 → containers/mod.rs:67 → mp4::open) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | read; rg for module wiring; workspace test suite (demux suites passed, 2026-11-16) |

## 1. One-paragraph verdict
This file is 56 lines of module declarations plus a crate doc that describes a crate that no longer exists: "v0 carries the AVI 1.0 demuxer of the first vertical" — while the declarations right below it list fourteen demuxers (adts, aiff, asf, avi, es, ivf, mkv, mp4, ogg, ps, ts, wav…). The code is real and reachable; the doc's self-description froze at v0 and was never updated as the MP4/MKV/TS demuxers landed.

## 2. What actually works (verified)
- All declared modules exist and compile (`cargo build --release --locked` PASS 2026-11-16); the mp4 module demuxes a real H.264+AAC MP4 (executed open trace, `audit/runs/baseline/mp4-smoke-open.txt`).
- The "house rules" claims (std-only, no unsafe, no unwrap/expect/panic in non-test code) are testable: `cargo test --workspace` includes demux suites — passed; W05-A01 rg-verified no unwrap/expect/unsafe in mp4/mod.rs.

## 3. Findings

### F-001 [S3] Crate doc describes a v0-only crate, contradicting its own module list
- **Where:** `crates/demux/src/lib.rs:4-5` vs `:43-55`
- **Code:**
  ```rust
  //! v0 carries the AVI 1.0 demuxer of the first vertical (Lead decision
  //! D-018, variant A: AVI + MJPEG video + PCM audio) in [`avi`].
  ```
  …followed 39 lines later by:
  ```rust
  pub mod adts;
  ...
  pub mod mp4;
  pub mod mkv;
  ```
- **What is wrong:** The crate self-describes as "the AVI demuxer" while shipping 14 demuxers. The module-map paragraph (lines 12-33) documents avi/wav/ivf/aiff/ts/asf and stops — no mention of mp4, mkv, ogg, ps, adts, es even though they are declared at 43-55.
- **Why it matters:** Crate docs are the first thing a reviewer reads; this one structurally understates the crate by a factor of ~14 and thereby hides the MP4 demuxer from doc-level discovery.
- **What correct looks like:** Update the crate doc to enumerate all 14 modules, or drop the narrative.
- **Confidence:** HIGH — **Verification:** read (both quotes verified in-file).

### F-002 [S4] `todo!`/panic-free house rule stated here; violations live elsewhere
- **Where:** `crates/demux/src/lib.rs:37-41`
- The "bounded memory (MP §12): every read is chunk-bounded and declared sizes are ceiling-checked" claim is the exact rule W05-A01 found violated at `mp4/mod.rs:1253-1266` (stts/ctts loops unclamped against declared body length). The rule-as-doc is honest; enforcement is not total.
- **Confidence:** HIGH — cross-verified (W05-A01 F-003; see that file).

## 4. Per-function table
No functions — module declarations only (14 modules, all real, all compiled in the shipped build).

## 5. Test honesty
N/A (no tests in this file; demux suites ran in the workspace run — passed).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "demux = the AVI demuxer of the first vertical" | this file's own doc | FALSE-as-current-description | module list at :43-55 |
| House rule "no unwrap/expect/panic in non-test code" | this file :37-38 | REAL for mp4/mod.rs (rg by W05-A01); not re-verified for all 14 modules — UNVERIFIED beyond mp4 |

## 7. Dead / duplicated / unreachable code
None (declarations only).

## 8. Dependencies & flags
No feature gates; std-only per C5 — consistent with Cargo.toml (no [dependencies] section verified by W05-A02's sibling audits).

## 9. Open questions for the next agent
- Do mkv/ts/ps/ogg demuxers uphold the panic-free/bounded rule as mp4 (mostly) does? Unassigned at depth this wave — recorded as deferred coverage.
