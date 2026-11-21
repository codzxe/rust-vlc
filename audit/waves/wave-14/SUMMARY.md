# WAVE 14 — SUMMARY (desktop-tauri shell)

Date: 2026-11-21 (session 2) · Agents W14-A01…A09 (wave 14 is a 9-agent wave
per assignments.tsv; A10 verified zero orphans) · Domain: apps/desktop-tauri.
Gate: **PASS — 9/9 assignments audited, 0 missing, 0 thin, citation
spot-checks clean.** The headline S0 was independently re-verified by the
orchestrator (rg: one call site, zero definitions).

## Verdicts (file level)
REAL 2 · REAL-UNPROVEN 4 · UNREACHABLE 1 · FALSE 1 · (lockfile REAL)

## Findings
**S0:3 S1:3 S2:5 S3:16 S4:16 — total 43** (S0 counts include the same
cross-file E0425 owned by A01 and referenced by A02/A03/A05/A08)

## THE headline finding — the GUI does not exist
**`app.rs:301` calls `spawn_session_pump`; the only definition in the repo is
`spawn_event_pump` (app.rs:190). That is a guaranteed E0425 — the
desktop-tauri crate CANNOT COMPILE at HEAD e83e5bd3.** No `lumen-tauri`
binary can be produced from this tree by any lane. INFRA-BLOCKED.md:5's "the
crate compiles in its dedicated CI lane" is FALSE for this commit (the
comment at app.rs:298 even cites CI run 43384519598 for *earlier* E0308s —
the fix that introduced this call apparently never compiled).
Consequence for the owner's complaint: **there is no GUI to open an MP4
with** — the question "where does the MP4 path fall out in the GUI" resolves
to "at the compiler."

## Other findings
1. [S1] UI never sends `session.close` → the stale-`last_fatal` brick
   (W13-A08) is inherited by any future GUI: open a bad file once, and every
   later open fails until restart (A01).
2. [S1] Capability ACL unreachable while the crate doesn't compile; tauri
   2.12.5 ACL default-set coverage unverifiable offline (A08).
3. [S2] Dual-lockfile drift: the tauri graph compiles the same 21 first-party
   crates against 11 diverged deps (toml 1.1.8→1.2.1, foldhash 0.1.5→0.2.0)
   with zero executed test/deny evidence (A06).
4. [S2] tauri.conf.json `height: 800` vs product.toml `height = 720` —
   visible resize jump on every launch, no drift-tie test (A07).
5. [S2] Real freeze vectors (once a GUI exists): FIFO/stalled-mount opens
   under the one global lock (A01).
6. [S3] Both shipped "icons" are byte-identical single-pixel #4f8cff
   placeholder squares (verified by regeneration+cmp); macOS ships iconless
   (A09).
7. [S3] tauri build.rs has never executed in any committed evidence
   (baseline died at gdk-3.0.pc before the build-script stage) (A04).

## Blocked / UNVERIFIED
- Standing UNVERIFIED-BUILD (webkit/gtk absent in sandbox).
- REQUEST-ORCH-RUN filed: `cargo check --manifest-path
  apps/desktop-tauri/Cargo.toml --locked` on a webkit host (expected E0425),
  and the tauri-graph cargo-deny.
