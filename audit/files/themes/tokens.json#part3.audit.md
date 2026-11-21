# AUDIT: themes/tokens.json#part3 (lines 971–1457)
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 487 (alias-layer tail: stroke/shadow/blur/opacity/motion + the ENTIRE semantic layer: 86 tokens in 9 groups) |
| File verdict | REAL (canonical source consumed by the executed pipeline: `cargo xtask tokens` → themes/generated/tokens.css → gen-tokens.mjs → ui cascade; baseline #24 tokens:check PASS over the output) |
| Reachable from shipped entry point? | YES (artifact chain): this chunk's semantic layer produces all 86 `:root` custom properties in themes/generated/tokens.css, byte-identical copy imported by ui/src/main.tsx:27. W14 ceiling (no GUI binary) applies. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Full read of 971-1457 + header 1-70; hand count of semantic tokens = 86 (color 33, font 17, space 4, radius 11, stroke 5, shadow 2, blur 2, opacity 9, motion 3) == baseline #24's "86 semantic tokens"; scan: every semantic `$value` is an `{alias.…}` reference (zero direct primitive refs — the :1075 invariant holds); consumer census: for each of the 86 generated `--*` vars, `rg -l "var(--t)" ui/src` (excluded generated/) → 10 tokens with zero consumers, re-checked with any-mention search (2 are comment-mentions only); `rg -n "eq" themes/generated/tokens.css` → --color-eq-1..7 exist, consumed by the EQ tool css; cross-ref my setting-field.css audit (phantom --color-slider-tick) — correctly absent here. |

## 1. One-paragraph verdict
The payoff layer of the token pipeline, and it is honest: all 86 semantic tokens (my hand count matches the pipeline's own output count exactly) resolve through `{alias.…}` references only — the "never directly to a primitive" invariant holds for every entry — per-mode values appear only where modes are registered, and the descriptions carry real provenance (frame citations, wave/finding ids, WCAG notes). The verified pipeline round-trip (tokens:check PASS, sha-pinned copies) means this chunk is the live source of every color/size/radius the UI paints. Two defects: 10 of the 86 semantic tokens (11.6%) are consumed by nothing in ui/src — 8 with zero mention anywhere, 2 mentioned only in comments — so the "only consumable layer" ships dead weight into the app css; and the semantic layer mixes three naming grammars (dotted key names, nested groups, and the motion group's `duration.`-prefixed keys that mimic a subgroup its own alias source does not have).

## 2. What actually works (verified)
— verified by hand count: semantic layer = 86 tokens (33 color + 17 font + 4 space + 11 radius + 5 stroke + 2 shadow + 2 blur + 9 opacity + 3 motion) — equals baseline #24's "86 semantic tokens" and the generated `:root` declaration count (awk census in my gen-tokens.mjs audit).
— verified by scan of every `$value` in 1074-1456: all are `{alias.…}` — no semantic token bypasses the alias layer; the alias tail (971-1073) references only `{primitive.…}` — layering is clean at both seams of this chunk.
— verified by `rg`: --color-eq-1..7 (the "fixed design color, independent of the user accent" band set, EQ.svg-cited) exist in the generated css (:43-49) and are consumed by the EQ tool css; --color-focus-ring (WCAG 2.2 note), --color-content-quiet, --color-content-accelerator all have real consumers.
— verified by the executed pipeline (baseline #24, plus my sha256/diff reproduction in the gen-tokens.mjs audit): this file's output is live, byte-pinned, and shipped into the UI cascade via main.tsx:27.
— cross-check with my setting-field.css audit: the phantom `--color-slider-tick` referenced by that sheet is correctly ABSENT here — the source of truth never had it; the consumer invented it.

## 3. Findings
### F-001 [S3]
Where: themes/tokens.json:1074-1456 (semantic layer; generated into themes/generated/tokens.css and shipped via ui/src/main.tsx:27)
Code:
```
      "radius": {
        "app": {
          "$value": "{alias.radius.app}",
          "$description": "main window"
```
What is wrong: Dead tokens in the only-consumable layer. Consumer census over all 86 semantic css vars (`rg -l "var(--t)" ui/src`, generated/ excluded, re-checked with any-mention search): 10 tokens have zero consumers — 8 with no mention anywhere in ui/src (`--font-size-micro`, `--font-weight-extrabold`, `--motion-duration-slow`, `--motion-duration-step`, `--opacity-chrome`, `--opacity-scrim-subtle`, `--radius-app`, `--stroke-window`) and 2 mentioned only inside comments (`--opacity-panel` in subtitles.css:6, `--opacity-scrim` in Surface.tsx:212 — the real scrim code uses `--opacity-scrim-strong`).
Why it matters: The layer's contract is "the ONLY layer components may consume" (:1075); 11.6% of it is consumed by no one, ships as dead custom properties in the app css, and pollutes the SemanticTokenName union every component can reference. Dead tokens also inflate the "86 semantic tokens" headline the gates report.
What correct looks like: either a usage-census check in the tokens pipeline (fail on semantic tokens with zero consumers, like l10n's dead-key gate) or move unconsumed entries to a "staging" group until a consumer lands.
Confidence: HIGH — Verification: scripted census (two passes: var() usage, then any mention); 76/86 tokens verified consumed.
Severity: S3 per the scale's own "dead code" example; blast radius limited (unused css properties, no behavior impact).

### F-002 [S4]
Where: themes/tokens.json:1440-1454 (+ contrast with 1078-1109 and 1212-1229)
Code:
```
    "motion": {
      "$type": "dimension",
      "$description": "Motion durations",
      "duration.fast": {
        "$value": "{alias.motion.fast}",
```
What is wrong: Three naming grammars coexist in the semantic layer: dotted key names (`"surface.deep"`, :1082), nested groups (`font.size.micro`, :1214), and this hybrid — keys named `"duration.fast"` that fake a subgroup the alias layer does not have (alias motion is flat: `{alias.motion.fast}`). The CSS output is consistent in all three cases (--color-surface-deep / --font-size-micro / --motion-duration-fast), but a maintainer extending the file must guess which convention each group uses, and the motion keys' dotted prefix exists only to shape the CSS name.
Why it matters: Inconsistent grammar in a 1457-line canonical source invites naming drift exactly where the pipeline's naming rule (:1075 "CSS name = --{category}-{name}") has to paper over it.
What correct looks like: one convention — nested groups everywhere (motion: { duration: { fast, step, slow } }) with the CSS-name rule unchanged.
Confidence: HIGH — Verification: all three patterns read in-file; alias motion group read (flat).

## 4. Per-function table (semantic groups — the consumable layer; alias tail included)
| Group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| alias.stroke/shadow/blur/opacity/motion (tail) | 971-1073 | REAL | — | all `{primitive.…}` refs; per-mode opacity objects (dark/light) match registered modes; several [PROPOSAL] marks disclosed |
| semantic.color (33) | 1076-1210 | REAL | — | surface ramp ×10 (incl. panel-raised w/ k17 provenance, elevated +2/+3 lift note); content ×7 (quiet's non-text scope note is good); accent ×5 per-accent; record; stroke ×2; focus.ring (WCAG 2.2 3:1 note); eq.1-7 frame-cited, consumed |
| semantic.font (17) | 1211-1289 | REAL | S3 | F-001: micro + extrabold dead; label's 16→15px CONTRACT note is honest change-tracking |
| semantic.space (4) | 1290-1308 | REAL | — | all consumed |
| semantic.radius (11) | 1309-1355 | REAL | S3 | F-001: radius.app dead; 10/11 consumed |
| semantic.stroke (5) | 1356-1378 | REAL | S3 | F-001: stroke.window dead; icon-strong remaps alias.stroke-width.strong (name remap mid-reference, disclosed by structure) |
| semantic.shadow (2) | 1379-1389 | REAL | — | both consumed |
| semantic.blur (2) | 1390-1400 | REAL | — | backdrop-small has exactly 1 consumer |
| semantic.opacity (9) | 1401-1439 | REAL | S3 | F-001: 5 of 9 dead or comment-only (panel, chrome, scrim, scrim-subtle + swatch-rest consumed? — swatch-rest verified consumed) |
| semantic.motion (3) | 1440-1455 | REAL | S4 | F-002 grammar; fast consumed (19 files), step/slow dead |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| tokens.test.ts pin + tokens:check (baseline #24 PASS) | YES for pipeline drift | sha256-pinned, byte-compared — but the pin counts 86 tokens as a virtue, not flagging that 10 are dead (F-001 is exactly what a usage gate would catch) |
| no consumer census exists anywhere | N/A | the l10n gate has a two-way dead-key discipline; the token pipeline has no equivalent — F-001's root cause |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "semantic… the ONLY layer components may consume. Every token resolves through an alias, never directly to a primitive (enforced by the parser)" | :1075 | REAL | scan: 86/86 semantic $values are {alias.…}; 76/86 consumed by components (F-001 for the other 10) |
| "CSS name = --{category}-{name}" | :1075 | REAL | verified across dotted-key, nested-group, and duration.-prefixed entries (all produce valid unique names) |
| "eq.1-7 — fixed design color, independent of the user accent /* frame: EQ.svg */" | :1182-1208 | REAL | generated css :43-49; consumed by the EQ tool css |
| "$value… per-mode value [PROPOSAL]" | :5 (header, part1's scope) | REAL | per-mode objects only in alias layer here (opacity panel/chrome), modes registered dark/light |
| "surface.panel-raised — cards on the base surface (k17 P3-4 ask; light #dedede…)" | :1106-1108 | REAL-UNPROVEN (value) | token exists + consumed; light-mode hex not re-derived here (alias/primitive layers are other agents' parts) |

## 7. Dead / duplicated / unreachable code
F-001's census: 8 zero-mention + 2 comment-only tokens out of 86. Nothing duplicated; every semantic name unique.

## 8. Dependencies & flags
Upstream of the whole ui paint stack (theme-tokens crate → generated css → main.tsx:27). No feature flags. Modes/accents registry (header, part1) consistent with this chunk's per-mode usage.

## 9. Open questions for the next agent
- parts 1-2 (primitive + alias bodies): does primitive.opacity.100 exist for the chrome per-mode ref (:1023)? Does alias.color.surface.panel-raised carry the light #dedede the semantic description cites? (Owners: the part1/part2 auditors.)
- The "123/86/86" pin comment in tokens.test.ts — 123 = ? (maybe :root + light-block totals; not resolvable from part3 alone.)
- If a usage gate lands (F-001), the grandfathering question repeats l10n's: which of the 10 dead tokens are "ahead of demand" vs abandoned?
