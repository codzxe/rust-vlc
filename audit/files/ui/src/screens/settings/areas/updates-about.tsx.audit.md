# AUDIT: ui/src/screens/settings/areas/updates-about.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 73 / 2 (AboutPanel, AboutArea) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (areas/index.tsx:14,141 `{advancedOpen ? <AboutArea /> : null}` — behind the global screen's Advanced disclosure) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | cat src/build-info.ts; rg vite.config.ts define (33-34); rg update command in generated protocol (absent); rg AboutArea consumer |

## 1. One-paragraph verdict
Small, honest surface. Version facts come from build-time injection
(vite.config.ts:33-34 `define`s __UI_PACKAGE_NAME__/__UI_PACKAGE_VERSION__ from
package.json; build-info.ts re-exports) — no drift-prone literals. No
"check now" button exists because no update command exists in the protocol
(rg: absent) — the module explicitly refuses to invent one (MP §5). The
engine version is NOT shown here (deferred to engine-info surfaces). Zero
findings.

## 2. What actually works (verified)
— build-info.ts + vite.config.ts:33-34 form the injection chain — claim
  matches code.
— rg "update" in generated/commands.ts: no update command — the no-button
  decision is truthful.
— Reachability: only as the About fold behind the global Advanced disclosure
  (documented frame deviation, h08 #2).

## 3. Findings
None. Checked: hardcoded literals (none — all values injected/typed), dead
buttons (none — the update row/button was deliberately NOT rendered), i18n
keys (catalog-consumed pattern), testids present for the two rows.

## 4. Per-function table
Not applicable (<100 LOC; 2 presentational components, both REAL).

## 5. Test honesty
No tests in file; about rows covered by settings suites (other agents).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "injected at build time ... via vite.config.ts `define`" | :10-12 | REAL | vite.config.ts:33-34 |
| "No 'check now' action exists in the command catalog" | :21-22 | REAL | rg update in commands.ts -> none |
| "Runtime engine/protocol versions come from system.get-engine-info" | :14-15 | REAL-UNPROVEN | command exists in protocol; the consuming surface is another agent's file |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
build-info (build-injected globals), i18n. No flags.

## 9. Open questions for the next agent
None.
