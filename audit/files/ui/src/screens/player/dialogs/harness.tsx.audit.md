# AUDIT: ui/src/screens/player/dialogs/harness.tsx (short audit — 17 LOC, dev-only harness, spec §12/§13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 17 / 1 (ToolsDialogHarness) |
| File verdict | REAL (dev-only harness; the "never imported by routes" claim verifies) |
| Reachable from shipped entry point? | NO (by design) — `rg -ln ToolsDialogHarness ui/src` -> only the sibling harness.test.tsx; no route/app import exists |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumer census (above); read of full file + harness.test.tsx imports/:45-47 |

## 1. Blunt verdict
A two-line mount of the REAL ToolsDialog (open, tab parametrized, close
handler a no-op — correct for a static vision harness) behind a div with a
data-harness-tab marker. The isolation claim checks out: the only importer
is its own test file, which gates the standalone-HTML export behind the
W26C5_HARNESS_OUT env var (:45-47, dev-only tooling — never set in CI
runs per the vitest baseline). Because it mounts the production dialog
component itself (not a copy), it cannot drift from the real thing — the
correct harness pattern.

## 2. What actually works (verified)
— Consumer census (rg): test-only import; no route file references it.
- The no-op onClose is honest for a screenshot harness (clicking close
  does nothing; nothing pretends otherwise).

## 3. Findings
None. Checked: production leakage (none), drift risk (none — mounts the
real component), env-var hygiene (gated, documented).
