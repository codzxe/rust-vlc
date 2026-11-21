# AUDIT: ui/src/screens/discovery/discovery.css
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 121 / 12 rule blocks |
| File verdict | REAL (clean tokens-only sheet; renders in committed discovery-*.png baselines that no run ever diffed) |
| Reachable from shipped entry point? | YES as web-UI source: imported by DiscoveryScreen.tsx:71, route /discovery (App.tsx:49-52). Real shell: no GUI binary (W14). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `grep -n -A6 "lumen-section-header__title" src/components/surface/surface.css` (:310 uses `--font-size-label`); `grep -n "font-size-label\|font-size-section" src/tokens/generated/tokens.css` (BOTH 15px at HEAD — the :15-17 comment's 16px claim is stale, override is a no-op); producer cross-check from my DiscoveryScreen.tsx audit (all 12 classes emitted); Δ-math recompute: light panel #c6c6c6 vs surface #c2c2c2 = Δ4 ✓, dark #191919 vs #090909 = Δ16 ✓ (comment accurate); baseline #22/#23 |

## 1. One-paragraph verdict
A small, disciplined sheet: every class is produced by DiscoveryScreen.tsx, raw px appears only as token arithmetic, untrusted OS volume ids get a hard 40% max-width + anywhere-wrap (V7 layout safety, a genuinely thoughtful touch), and the two cross-cutting token debts it works around are documented with the owning lane named. One of those two workarounds is stale at HEAD: the SectionHeader "16px drift" pin overrides `--font-size-label` with `--font-size-section` — but both tokens are 15px now, so the rule is a no-op wearing a false justification. The light-mode panel-step workaround is real (Δ4 vs Δ16 verified) and correctly fronted by a fallback chain.

## 2. What actually works (verified)
— verified by producer cross-check (my DiscoveryScreen.tsx audit's class census) → all 12 selectors emitted (`__head/__title/__subtitle/__section/__count(-num)/__panel(--prose)/__volume-list/__volume-id/__network-status`).
— verified by Δ recompute → the :113-115 comment's numbers are exact (light Δ4, dark Δ16); `--color-surface-panel-raised` exists in the light block (#dedede), so the fallback at :119 is a safety net, not dead weight.
— verified by baseline #22/#23 → compiles; DiscoveryScreen tests pass; discovery route baselines committed (never diffed — wave-wide gap, list.css F-001).

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/discovery/discovery.css:15-20
Code:
```
/* w26.i18 (h19 §3 — the 16px drift): the SectionHeader primitive ships
 * --font-size-label (16px) where its own doc + §2's band pin the 15px
 * class. Usage-side pin until the surface lane retunes the primitive. */
.discovery .lumen-section-header__title {
  font-size: var(--font-size-section);
}
```
What is wrong: the justification is false at HEAD — `--font-size-label` is 15px (tokens.css:69), identical to `--font-size-section` (tokens.css:72). Either the surface lane already retuned the token (workaround should be retired) or the 16px claim was never true for the committed token set. The rule is a no-op: it re-pins 15px onto a rule that already computes 15px.
Why it matters: stale workaround comments teach maintainers the token system is more broken than it is; the "usage-side pin until…" pattern then spreads as cargo cult.
What correct looks like: delete the rule + comment (the primitive is compliant), or if the pin is kept as drift-insurance, rewrite the comment to say so honestly ("defensive pin — both tokens 15px at HEAD").
Confidence: HIGH — Verification: both token values read directly; SectionHeader's use at surface.css:310 read.

### F-002 [S4]
Where: ui/src/screens/discovery/discovery.css:96-102
Code:
```
.discovery__volume-id {
  max-width: 40%;
  overflow-wrap: anywhere;
}
```
What is wrong: nothing functionally — but the rule guards a lane that can never populate in a real shell (no `devices.volume-*` emitter; my DiscoveryScreen.tsx audit F-001), so like the row machinery it serves, it is defensive CSS for demo-fixtures-only data. Noted so the volume lane's CSS isn't mistaken for evidence the lane works.
Why it matters: audit bookkeeping only.
What correct looks like: nothing — keep it; the lane may ship later.
Confidence: HIGH — Verification: cross-ref DiscoveryScreen.tsx audit emitter census.

## 4. Per-function table (rule blocks)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.discovery .lumen-section-header__title` | 18-20 | NO-OP | S4 | F-001 stale pin |
| `.discovery` | 22-30 | REAL | — | 680px column, 24px lane rhythm |
| `__head` / `__title` / `__subtitle` | 34-52 | REAL | — | 15px working-screen head |
| `__section` | 56-60 | REAL | — | lane stack |
| `__count` / `__count-num` | 63-70 | REAL | — | sidebar.svg meta grammar |
| `__panel` / `__panel--prose` | 76-84 | REAL | — | Card primitive insets |
| `__volume-list` | 88-94 | REAL | S4 | list reset; demo-only data (F-002) |
| `__volume-id` | 99-102 | REAL | S4 | V7 untrusted-id clamp — good; demo-only lane |
| `__network-status` | 106-110 | REAL | — | the honest verdict line |
| light-mode panel step | 118-120 | REAL | — | Δ4/Δ16 claims verified; fallback chain |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| DiscoveryScreen.test.tsx (15 tests, baseline-passed) | DOM only | jsdom — no CSS; class contracts only |
| discovery-*.png baselines | NO as committed evidence | no visual run log (wave-wide) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the SectionHeader primitive ships --font-size-label (16px)" | discovery.css:15-16 | FALSE (stale) | token is 15px at HEAD (F-001) |
| "light surface-panel #c6c6c6 is Δ4/255 on the #c2c2c2 surface (dark's card step is Δ16)" | discovery.css:113-115 | REAL | arithmetic verified |
| "Semantic tokens only (§3.2); raw px is structural layout only" | discovery.css:13 | REAL | zero raw hex; only token math |

## 7. Dead / duplicated / unreachable code
No dead selectors. The volume-lane rules are live-for-demo-only (F-002 note).

## 8. Dependencies & flags
13 tokens, all defined. Depends on Card/SectionHeader/ListRow primitives' sheets.

## 9. Open questions for the next agent
- F-001's stale-pin pattern: audit the OTHER usage-side pins of the same era (grep `w26.i18` across ui/src CSS) — if SectionHeader's 16px was retuned globally, several sheets may carry no-op pins with false comments.
