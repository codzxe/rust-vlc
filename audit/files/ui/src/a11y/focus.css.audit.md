# AUDIT: ui/src/a11y/focus.css (short audit — 24 LOC asset-adjacent, spec §12/§13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines | 24 (1 rule: the global :focus-visible floor) |
| File verdict | REAL (global keyboard-focus floor; imported at the app entry; light-mode override verified one layer up) |
| Reachable from shipped entry point? | YES — main.tsx imports focus.css (rg -ln -> main.tsx); it applies to every rendered view |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln focus.css ui/src` -> main.tsx, theme/lumen.css; `rg color-focus-ring ui/src/theme/lumen.css` -> :199-203 override present |

## 1. Blunt verdict
One rule, does exactly what its header says: every keyboard focus stop in
the app gets the 2px accent ring with 2px offset, as a floor beneath the
B1 primitives' own identical declarations. The header's layering claim
verifies: the light-mode re-point exists in theme/lumen.css:203
(`outline-color: var(--color-focus-ring)` under the :root[data-theme=
"light"] :focus-visible selector), and --color-focus-ring is a real
token (tokens.css:50). The WCAG contrast numbers (1.40:1 vs ≥3.26:1) are
design-lane measurements — not re-derived here (UNVERIFIED, measurement
tooling out of scope). No findings: no dead selector, no raw hex (token
discipline kept), no specificity surprise (the (0,3,0) light override
claim is consistent with the selector shape cited).

## 2. Findings
None. Checked: token existence (--color-accent, --color-focus-ring both in
tokens.css), import reachability (main.tsx), duplication with primitives
(documented as intentional floor+own-declaration belt-and-suspenders).
