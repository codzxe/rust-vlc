# AUDIT: ui/src/build-info.globals.d.ts (short audit — 7 LOC declaration file, spec §12/§13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines | 7 (2 ambient const declarations) |
| File verdict | REAL (declares exactly the constants vite injects; both sides of the seam verified) |
| Reachable from shipped entry point? | YES — ui/src/build-info.ts:8-9 reads both constants; vite.config.ts:33-34 defines them from package.json at config load (the About panel / build-info surface) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg "__UI_PACKAGE_NAME__\|__UI_PACKAGE_VERSION__" ui/vite.config.ts ui/src/build-info.ts` -> both sides present; read of full file |

## 1. Blunt verdict
Two ambient declarations whose only job is to type the vite `define`
injection — and the injection exists (vite.config.ts:33-34, from
pkg.name/pkg.version) and the consumer exists (build-info.ts:8-9). The
header's anti-drift rationale (no hardcoded literal, no boundary escape)
matches the implementation. The one hazard inherent to this pattern —
running the TS outside vite (e.g., node-based tooling importing
build-info.ts) would hit ReferenceError — is mitigated by nothing in the
tree importing build-info.ts outside the vite graph (build/test lanes both
run through vitest/vite, baseline green). No findings.

## 2. Findings
None. Checked: define/consume pairing (verified), drift between
package.json version and what ships (structurally impossible — same
source), declaration pollution (two globals, both used).
