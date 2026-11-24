# AUDIT: ui/src/components/icon/IconWrapper.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 58 / 1 (IconWrapper) |
| File verdict | REAL-UNPROVEN (presentational; `sm`/`md` presets currently unused in production — self-disclosed) |
| Reachable from shipped entry point? | YES — AppShell.tsx:64,669,683 (rail stops + add-media foot, always `lg`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full read; grep production usage of size presets |

## 1. One-paragraph verdict
A three-class span wrapper (hit-area presets sm 24 / md 30×32 / lg 44 + active accent). Correct, tiny, presentational. The header honestly discloses that production mounts only `lg` (verified: the only non-test usages are AppShell.tsx:669,683, both `size="lg"`); `sm`/`md` are additive API surface pinned by tests — dead-ish presets, but labeled as such (catalog-reserved, w26.k04).

## 2. What actually works (verified)
— verified by grep: `lg` used in the shell rail; className composition filter-join; active accent class; children passthrough.

## 3. Findings
F-001 [S4]
Where: ui/src/components/icon/IconWrapper.tsx:19-21
Code:
```
 * w26.k04 (j04-12): sm/md are catalog-reserved hit-area presets — production
 * currently mounts only lg (AppShell rail); sm/md stay as frozen-compatible
 * additive API (pinned by tests) for dense-row hosts.
```
What is wrong: two of three size presets ship unused (dead API surface). Self-disclosed and test-pinned, so hygiene-grade. Confidence: HIGH — Verification: grep (only lg in non-test code).

## 4. Per-function table
n.a. — <100 LOC single component.

## 5. Test honesty
Presets pinned by tests per header (icon lane tests, UNVERIFIED depth).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "production currently mounts only lg (AppShell rail)" | L19-21 | REAL | grep: only AppShell.tsx:669,683 |

## 7. Dead / duplicated / unreachable code
sm/md presets (F-001) — reserved, not fake.

## 8. Dependencies & flags
icon.css. No feature flags.

## 9. Open questions for the next agent
None.
