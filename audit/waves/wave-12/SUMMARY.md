# WAVE 12 — SUMMARY (plugins / wasm / web)

Date: 2026-11-21 (session 2) · Agents W12-A01…A10 · Domain: plugin-api,
plugin-host-wasm (wasmi host), plugins/ extensions, apps/web (WASM engine,
worker, worklet), fsjail, staged_source.
Gate: **PASS — 63/63 assignments audited, 0 missing, 0 thin, citation
spot-checks clean.**

## Verdicts (file level)
UNREACHABLE 40 · REAL 22 · REAL-UNPROVEN 2

## Findings
**S0:0 S1:1 S2:14 S3:77 S4:118 — total 210**

## The wave's hostile questions
- **Is there a real plugin loaded anywhere? NO.** plugin-host-wasm is a leaf
  crate with zero dependents; no shipped entry point loads any plugin. The
  wasmi host executes the committed example extensions **only under
  `cargo test`** (baseline :4572-4574) — never in any shipped binary.
- **apps/web builds a real wasm32 cdylib** (Chromium-proven, 300 KB gzip
  floor, web-budget CI gate actually builds it) that **nothing ships**: no
  dependent, desktop-only packaging; the audio worklet has never executed;
  the shipped web engine is blob-only and decoder-less.
- **Doc lies found:** "the conformance kit checks this honesty" (access.rs:38)
  — the ConformanceRunner exists only in design docs, zero code (A04);
  doc 06 §5.4 row 7 contradicted by the handle-table lifetime cap (A03).

## Worst findings
1. [S1] instance.rs `read_guest_memory` allocates a host buffer from a
   guest-reported length with no cap before bounds-check — 4 GiB
   alloc/abort path breaching "bounded memory everywhere" (A09).
2. [S2] host.rs:1307/1546/1567 `last_fatal` never cleared on session
   teardown — a late decode fatal poisons the next `session.open`; wrong-state
   teardown orphans a live engine session, permanently bricking the worker
   against reopening media (A05).
3. [S2] web host `submit_and_await` (host.rs:1598-1600) swallows `InboxBusy`
   and answers every refused command "session closed" — false terminal
   diagnosis; mirrored in the CLI host (A06).
4. [S2] seekability.rs:131 — wire-end truncation checks the promised span's
   end, not the delivered body: a short last-chunk 206 is silently
   `Ok(Ranged)` — the exact "silent EOF" the module exists to prevent (A01).
5. [S2] manifest.rs:597-607 — `sample_rates`/`bit_depths` escape kind-key-set
   validation because the membership check is an empty-bodied `for` loop (A07).
6. [S2] staged_source.rs `insert_window` evicts the window ahead of the read
   cursor → readiness-gate deadlock the module's own doc claims "cannot
   happen"; plus a second bug silently accepting span-short 206 bodies (A10).
7. [S2] wasmi host OOB *write* probes never reach the escalation ledger —
   reads trip the disable threshold at 3, writes hammer forever uncounted (A02).

## Blocked / UNVERIFIED
- Browser-lane execution never ran in baseline (rc-smoke steps 7-8 skipped,
  web-matrix lane absent) — REQUEST-ORCH-RUN filed.
- wasm32 build + plugin crates' test suites: zero runs in any primary
  baseline log (fail-fast abort) — greens only in the no-fail-fast repair
  run; REQUEST-ORCH-RUN filed (`cargo test -p plugin-host-wasm --locked`,
  `-p lumen-web --locked`).
- Browser evidence exists only at foreign SHA 3755dc3 (2 REQUEST-ORCH-RUN).
