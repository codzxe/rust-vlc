# AUDIT: ui/src/components/icon/Icon.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 448 / 5 describes, 27 `it` blocks, 4 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest lane (`ui/package.json` `"test": "vitest run"`); baseline executed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read full file; rg `\bit\(` → 27; rg baseline → pass line |

## 1. One-paragraph verdict
A strong component test suite: 27 tests, all mutation-sensitive. It verifies
rendered SVG geometry (stroke 1.5, viewBox, currentColor), the full registry
(76 names, pinned count + uniqueness), 24×24-grid coordinate bounds for every
glyph, exact mined path-data fingerprints (regression pins against hand-edits,
issue #35), emphasis-variant stroke widths, two-tone faces with inert-in-mono
cascade checks, and IconWrapper hit-area presets — including CSS assertions
by reading icon.css from disk and regex-matching rules (token discipline, no
raw hexes in declarations). The two defects are stylistic: a
`import.meta.dirname` cast hack and a hand-rolled CSS regex parser.

## 2. What actually works (verified)
— Baseline: audit/runs/baseline/ui-test.txt:39 `✓ src/components/icon/Icon.test.tsx (27 tests) 318ms` — matches rg count (27).
— Geometry pins are exact-string asserts (L209-217, L230-249): any hand edit
to mined geometry fails the suite; the comment states the policy
(L199-201): "If these change, geometry was re-drawn by hand instead of mined
— re-derive from the sheet, don't edit the constants."
— Grid-bounds sweep (L136-147) checks every numeric coordinate of every
element of all 76 glyphs against [-1, 25] with 1u slack — real invariant.
— Negative regex tests: L395-396 asserts no bare `.lumen-icon__detail` rule
exists (mono is byte-identical), L445-446 asserts no raw hex colors in
declarations (comments stripped first). Both would catch token-discipline
regressions.

## 3. Findings
F-001 [S4]
Where: :8-9
Code: `const { dirname } = import.meta as ImportMeta & { dirname?: string };`
`const iconCss = readFileSync(\`\${dirname}/icon.css\`, "utf8");`
What is wrong: relies on the non-standard `import.meta.dirname` (Node ≥20.11
ESM feature) via a type cast instead of `import.meta.url` + `fileURLToPath`;
the cast hides the platform dependency. Works under vitest; brittle if the
runner changes. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: :11-14
Code: `const cssRule = (selector: string): string => { const match = iconCss.match(new RegExp(\`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[^{]*\\{([^}]*)\\}\`));`
What is wrong: hand-rolled CSS matching — `[^{]*` can bridge selector
boundaries and `[^}]*` breaks on nested braces or minified CSS; a refactor of
icon.css formatting could silently empty a rule match (tests then pass on
`""`-containment only if they assert substrings — they do, via toContain, so
a formatting change degrades to weaker checks rather than failure).
Confidence: MEDIUM — Verification: read.

## 4. Per-function table (every test)
| Describe (lines) | Tests | Verdict | Severity | Notes |
|---|---|---|---|---|
| Icon 1.5-stroke set (26-132) | 6: svg attrs; every registry name renders; registry exactly 76 + unique; size→width/height; aria-hidden vs role=img+title; disabled class + 5 CSS token rules | REAL | — | count pinned `iconNames.length` 76 (L93) |
| mined glyph geometry (134-263) | 7: all glyphs in 24×24 grid w/ slack; transport filled paths; volume family shared speaker + wave counts (structure-abstracted); per-element stroke-width=2 info; exact path pins (issue #35); w26.a07 pins; alias identity folder/open-folder info/about | REAL | — | structure-abstraction (L165) compares command shape, coordinates abstracted — documented rationale |
| emphasis variants (265-299) | 4: default 1.5 + no modifier; strong=2; max=3; variant keeps a11y contract | REAL | — | |
| two-tone mode (301-398) | 6: class add + mono never; dedicated faces carry added geometry fingerprints; folder alias same face; fallback to shared glyph (settings identical + hasTwoToneVariant false); dedicated set exactly {check,folder,open-file,open-folder,playback}; detail markers inert + token cascade from CSS source | REAL | — | L378 "j04-7 test hook" sorts to avoid registry-order leakage |
| IconWrapper (400-448) | 4: md default 30×32 frame; sm/md/lg presets (24/30×32/44 px from CSS); active accent token; hover backdrop + no raw hex | REAL | — | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| registry holds exactly the doc-named icons (L47-95) | YES | exact count 76 + uniqueness + arrayContaining doc names |
| pins mined sheet geometry (L198-220) | YES | exact `d` prefixes/full strings |
| grid sweep (L135-148) | YES | numeric bounds on every coordinate of all 76 glyphs |
| detail markers inert (L385-397) | YES | regex absence proof on declarations-only CSS |
| all 27 | YES (mutation reasoning) | no tautologies; every test asserts observable DOM/CSS content |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "32 frozen (wave-1) + 44 frame-mined w26.a07 additions = 76; the 9 dead names were pruned" | file L91-93 | REAL | L93 asserts length 76; uniqueness L94 |
| "If these change, geometry was re-drawn by hand instead of mined" | file L199-201 | REAL | exact pins below it |

## 7. Dead / duplicated / unreachable code
None. All helpers (cssRule, coordNumbers, structure, glyph/d, geometry/face)
are used; no `.skip`/`.only`/`.todo`.

## 8. Dependencies & flags
First-party only (./index, ./icons) + RTL/vitest/node:fs. No env keys, no
network.

## 9. Open questions for the next agent
None — the companion icons.tsx registry (76 glyphs) was cross-checked only
through this suite; a dedicated icons.tsx audit would verify glyph provenance
against design/figma sheets.
