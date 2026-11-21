# AUDIT: themes/tokens.json#part2 (lines 486–970)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / tokens | 485 / 90 (primitive 24: space.32 + space.48, stroke.width ×5, shadow ×2, blur ×2, opacity ×10, motion ×3; alias 66: color ×33, font ×17, space ×4, radius ×11, stroke-width.icon ×1) |
| Chunk boundary | Opens at primitive.space.32 (l.486); closes mid-node on alias.stroke-width.icon's `$value` (l.970 — its `$description` at l.971 is part3's first line; the token is covered here). stroke-width.strong/max/ring/window, alias.shadow/blur/opacity/motion and the whole semantic layer are part3 (W15-A03, lines 971–1457). |
| File verdict | REAL (data) — 1 FALSE sub-claim (F-001) |
| Reachable from shipped entry point? | NO from `lumen`/`lumen-tauri` (no Cargo dependents on theme-tokens; W14: no GUI binary). Real chain: xtask tokens → tokens-gen → generated tokens.css/tokens.rs (diff-gated, baseline rows 3 & 24) → ui bridge → ui/src/main.tsx — the 78 consumed semantic vars of this design reach 70 CSS files, all inside an unshippable webview. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 7 |
| Commands run | python3 WCAG recompute of every numeric claim in range; python per-mode/layer/reference walks (part1 set); python inbound-reference census (alias orphan count); semantic-var consumer census across ui/ (rg per name, `-e` form); rg hex-declaration sweep of ui/src CSS; `sed -n` verbatim verification of all quoted lines; `rg -n on-accent themes/generated/tokens.css` |

## 1. One-paragraph verdict
The alias layer (this chunk's core) is the file's conscience and it is built honestly: all 24 per-mode values carry exactly `{dark, light}`, every reference resolves, all 33 color roles are consumed by the semantic layer (0 alias orphans), and the WCAG numbers quoted in the descriptions — including the self-incriminating histories ("was arithmetically false", "measures 2.55:1 … both sub-AA") — recompute to the exact hundredth everywhere I checked (40+ numbers, Δ≤0.01), down to the byte-verified `#DEDEDE` rect at Player - Light.svg:118. One claim is FALSE: content.tertiary asserts the delivered value is ≥4.5:1 on surface/panel/deep/muted/raised, but the dark value #828282 measures 4.14:1 on raised #222222 — the exact class of inflated contrast claim this file's own w26 history (h20 F1) condemns. The rest is hygiene debt: a mislabeled "wells Δ4" delta, a stale [PROPOSAL] tag contradicting a measured claim, 8 semantic tokens with zero consumers (2 more than W15-A05's census found), a stale "≈2.3:1" basis, a dual authoring convention with a silent-collision hazard, the undisclosed light-mode bright≡primary collapse, and the accent-swatch supply gap that forces the UI's only hardcoded-hex block.

## 2. What actually works (verified)
— verified by python WCAG recompute → every numeric claim in range is exact: content.quiet dark #6B6B6B = 3.74/3.62/3.30/3.43/3.23 + NOT-certified 2.99 raised / 2.84 hover, light #5F5F5F = 3.58/3.74/3.29/3.86/3.34 + NOT 2.78 chrome / 2.88 selected (l.718); content.tertiary light #474747 = 5.22/4.79/5.44/5.61/6.91 (l.711, light half only — see F-001); accent.bright/strong/tinted role refs consistent with primitive part; focus.ring dark 6.18:1 on #090909, light accent ring 1.40–2.39 (l.790); surface.panel-raised Δ16 dark / Δ28 light (l.676); surface.elevated +2 dark / +3 light (l.683); accent.bright light orange.450 is the correct WCAG choice a fortiori (orange.300 on current light surfaces measures 1.17–2.00, worse than the claimed failure).
— verified by `sed -n '118p' "design/figma/Player - Light.svg"` → `<rect x="254" y="1039" width="1238" height="28" rx="6" fill="#DEDEDE"/>` — surface.deep's "lone #DEDEDE rect (1238×28 r6 at (254,1039)) is surface.raised in light" (l.634) is byte-accurate, and it is the only #DEDEDE in that file.
— verified by `rg -c '#1B1B1B' design/figma/Player.svg` → 3 hits (surface.chrome dark repin basis, l.648); `rg -n 'Frame 21'` + 2 × `fill="#090909"` (dark seek-track claim, l.634).
— verified by python walk → 24 per-mode tokens, all keys exactly {dark, light}; all 66 alias references resolve into the primitive layer or extensions; layer grammar clean (alias→primitive|alias only); 0 duplicate paths.
— verified by python inbound-reference census → 0 of 86 alias tokens unreferenced: every role in this chunk maps into the semantic layer (part3) — no dead ROLES (dead semantic vars are a part3/emission concern, see F-004).
— verified by consumer census (rg per var name over ui/) → 78 of 86 semantic vars have live consumers; top: --color-surface ×70 files, --color-accent ×55, --color-content-secondary ×51, --radius-control ×38. The alias design is load-bearing.
— verified by `rg -n 'on-accent' themes/generated/tokens.css` → per-accent on-accent emission (#161616 ×6, #eaeaea blue, #ffffff violet) matches the part1 derivation rule exactly; opacity.panel/chrome per-mode values (95/100 dark, 80/75 light) match the emitted light overrides.

## 3. Findings
### F-001 [S3]
Where: themes/tokens.json:711
Code:
```
        "$description": "tertiary/disabled/OFF labels; w26.i19 one-truth repin: dark gray.425 + light gray.465 #474747 (the f15 doc/ship split-brain — gray.325 #767676 documented vs gray.460 #5f5f5f shipped, both sub-AA — is closed: the DELIVERED value now measures ≥4.5:1 on surface/panel/deep/muted/raised, h20 F1). Scope: NOT for chrome/hover/selected text."
```
What is wrong: the claim is FALSE for dark-on-raised: #828282 on raised #222222 measures **4.14:1** (< 4.5). Recomputed matrix — dark #828282: surface 5.18 ✓, deep 5.02 ✓, panel 4.58 ✓, muted 4.75 ✓, raised **4.14 ✗**; light #474747: all five pass (5.22/4.79/5.44/5.61/6.91). gray.425's own primitive description (l.100, part1) carefully claims only "surface/panel/deep" — the alias description inflates it to five surfaces and drops the raised exclusion.
Why it matters: this is the "one-truth" sentence written specifically to close the h20-F1 split-brain, and it reintroduces the same defect class it was written to end: a false WCAG certification in the canonical source. The scope note excludes chrome/hover/selected but licenses raised — surface.raised is thumbnails/elevated rows, so a consumer following this sentence ships 4.14:1 disabled/tertiary text on raised rows in dark mode. (Currently unreachable — W14 — but the claim is false regardless of reachability.)
What correct looks like: "≥4.5:1 on surface/panel/deep/muted (dark raised 4.14, light all pass); NOT for chrome/hover/selected/raised text."
Confidence: HIGH — Verification: ran (python WCAG recompute).

### F-002 [S4]
Where: themes/tokens.json:683 (cross-ref part1 l.164)
Code:
```
        "$description": "elevated panel: card-on-panel lift (i09 +2 ask). Dark = gray.775 #1B1B1B, panel +2 — … Light = gray.115 #c9c9c9, panel +3 — the smallest designated lift in the Lead-pinned light ramp; light depth steps stay compressed (wells Δ4, h20 F4 — systemic to the pinned ramp, needs a light-frame re-pin with more range). Pair with stroke.border for edge definition in light."
```
What is wrong: "wells Δ4" matches no pair of pinned light values: wells (surface.deep light) = #bababa, which is Δ8 under surface #c2c2c2. The Δ4 and Δ3 in the sentence match panel-over-surface (#c2c2c2→#c6c6c6 = 4) and elevated-over-panel (#c9c9c9−#c6c6c6 = 3) respectively — the labels "wells/panels" are attached to the wrong steps (repeated verbatim at part1 l.164).
Why it matters: the sentence is the recorded rationale for a systemic design debt ("needs a light-frame re-pin"); a future re-pinner who trusts Δ4 for wells will size the new well step against a number that belongs to the panel step.
What correct looks like: "wells Δ8 under surface, panel Δ4 over surface, elevated Δ3 over panel".
Confidence: HIGH on the arithmetic (all pairs enumerated); MEDIUM on intent (no reading of "wells Δ4" reconciles).
Verification: ran (python channel deltas over the pinned light ramp).

### F-003 [S4]
Where: themes/tokens.json:690 (contradicted by part1 l.164)
Code:
```
        "$description": "off-knob, waveform bars, tertiary fills; light value [PROPOSAL] derived"
```
What is wrong: surface.muted's light value is `{primitive.color.gray.115}`, whose own description (part1 l.164) states it was **pixel-measured** in that exact role: "w26 light-frame ramp (Player_-_Light.png pixel-measured …). Off-state fills (surface.muted light)". The alias still carries the stale "[PROPOSAL] derived" tag from before the w26 repin — the two descriptions contradict each other on whether the value was observed or derived. (surface.hover light = gray.160 and surface.selected light = gray.165 also still say "[PROPOSAL] derived", but for those only the VALUE is measured while the role assignment stays derived — defensible; muted's is directly contradicted.)
Why it matters: [PROPOSAL] vs measured is this file's provenance currency (l.2: "Every value is [VERIFY-DESIGN] … unless its description marks it [PROPOSAL]"); a stale tag corrupts the ledger in both directions — a re-miner will treat a measured value as freely re-derivable.
What correct looks like: drop "[PROPOSAL] derived" from surface.muted light (or re-scope it like hover/selected).
Confidence: HIGH — Verification: read (both descriptions quoted verbatim; both line-verified).

### F-004 [S4]
Where: themes/tokens.json:643-648 (surface.chrome) and the 86-var consumer census
Code:
```
      "surface.chrome": {
        "$value": {
          "dark": "{primitive.color.gray.775}",
          "light": "{primitive.color.gray.170}"
        },
        "$description": "title bar / control strip (alpha via opacity.chrome); dark re-pinned to the frame's #1B1B1B (Player.svg topbar/strip, g03-P2-1 + h03 #3) — w26.i02"
```
What is wrong: 8 of 86 emitted semantic tokens have ZERO consumers anywhere in ui/ (rg per name, whole ui/ tree): `--font-size-micro` (6px), `--font-weight-extrabold`, `--motion-duration-slow`, `--motion-duration-step`, `--opacity-chrome`, `--opacity-scrim-subtle`, `--radius-app`, `--stroke-window` — 9.3% dead supply (superset of W15-A05's 6; the census here adds extrabold and chrome). Most pointed: surface.chrome's own description instructs "alpha via opacity.chrome" — the color is consumed (files: 20+), the opacity it mandates is consumed by nothing, so the documented pairing is half-unwired. `--font-size-micro` is additionally a 6px text token waiting to be misused (A05's a11y note).
Why it matters: the semantic layer is "the ONLY layer components may consume" (l.1075, part3) — unconsumed supply is dead weight in every generated artifact, and an instruction ("pair with") that resolves to a no-op misleads maintainers.
What correct looks like: delete the 8 (or wire them); if --opacity-chrome stays, the chrome surfaces must actually use it.
Confidence: HIGH — Verification: ran (rg census over ui/, excluding generated copies; also zero Rust-const consumers outside the generated file itself — theme-tokens has no dependents).

### F-005 [S4]
Where: themes/tokens.json:747
Code:
```
        "$description": "hover/lighter accent; w26.f15: light repinned to the WCAG-adjusted orange.450 TEXT step (e17 F2 — orange.300 ≈2.3:1 on light failed) w26.gfix-C: dark bright repinned to the contract §2 hover #e8851a (orange.300)."
```
What is wrong: "orange.300 ≈2.3:1 on light" is stale: #E8851A measures 2.36 on #F0F0F0 / 2.34 on #EFEFEF — the PRE-repin light surfaces. On the current Lead-pinned light ramp it measures 1.51 (surface) / 1.58 (panel) / 1.17 (chrome) / 2.00 (raised).
Why it matters: the conclusion (repin to orange.450) survives — the current values are WORSE than the quoted failure — but the number as stated is not reproducible against the ramp the file pins; it is the same stale-basis pattern as F-005 part1 (the "102" corpus count).
What correct looks like: "orange.300 measured ≈2.3:1 on the pre-repin light surfaces (1.2–2.0 on the pinned ramp)".
Confidence: HIGH — Verification: ran (python WCAG recompute on both surface generations).

### F-006 [S4]
Where: themes/tokens.json:622-634
Code:
```
      "surface": {
        "$value": {
          "dark": "{primitive.color.gray.900}",
          "light": "{primitive.color.gray.135}"
        },
        "$description": "base surface: main window, menus, icon rail, dropdown lists"
      },
      "surface.deep": {
```
What is wrong: alias.color mixes two authoring conventions for the same path space: `surface` is a nested JSON group while its nine siblings (`"surface.deep"`, `"surface.panel"`, …) are literal keys containing dots. parse.rs builds paths by string concatenation (walk_group: `format!("{path}.{key}")`), so a dotted key and a nested group are indistinguishable — and the token index is a HashMap with last-wins semantics and no duplicate check (parse.rs:68-73). A future author who "normalizes" `surface.deep` into the `surface` group creates a second token with the identical path and the parser silently keeps one.
Why it matters: the format contract (l.5, part1) documents nested groups and `{dotted.path}` references but never states that dots inside group KEYS are path separators — the equivalence is only visible by reading the parser. Committed file has 0 duplicate paths (verified), so this is a trap, not a defect today.
What correct looks like: one convention (nested groups), or a documented rule + a parser-side duplicate-path check (cross-ref W15-A02, whose F-006 covered duplicate modes, not duplicate paths).
Confidence: HIGH — Verification: read (walk_group + index construction traced; python dupes check = 0).

### F-007 [S4]
Where: themes/tokens.json:692-697 vs 727-732 (ruling on W15-A05's cross-ref §3.2)
Code:
```
      "content.primary": {
        "$value": {
          "dark": "{primitive.color.gray.250}",
          "light": "{primitive.color.gray.750}"
        },
```
```
      "content.bright": {
        "$value": {
          "dark": "{primitive.color.gray.100}",
          "light": "{primitive.color.gray.750}"
        },
```
What is wrong: in light mode content.bright ≡ content.primary (both gray.750 #161616) — the emphasis hierarchy that dark mode keeps (#EAEAEA vs #C6C6C6, 1.35:1 apart) vanishes. The VALUE is disclosed (l.732: "light value measured (#161616 emphasis glyphs, Player - Light.svg)") but the ROLE consequence — "bright stops being brighter than primary in light" — is stated nowhere; a component author has no warning that heading-vs-body contrast in light mode relies on weight/size alone.
Why it matters: measurement-driven and honest at the value level; the missing piece is the one sentence a consumer needs. Not a data error — an undisclosed design consequence.
What correct looks like: append to l.732: "note: light collapses bright onto primary — the frames show no lighter ink; create hierarchy with weight/size, not color, in light mode."
Confidence: HIGH — Verification: read + ran (values identical; dark pair distinct; 36 ui files consume --color-content-bright).
Ruling on A05's second cross-ref (font-family-time ≡ sans): INTENTIONAL and documented at the source (l.376 part1: "timecode face — same UI stack … tabular-numeric rule lives here as prose") — a deliberate seam for a future tabular face, not drift; the consumer-side degeneracy A05 filed at dialogs.css F-001 is the cost of that seam. No finding here beyond the note.

### F-008 [S4]
Where: themes/tokens.json:242-276 (accent group) — consumer cross-ref ui/src/components/form/form.css:495-535
Code:
```
/* The 9 accent swatch primaries, frame-measured (Settings - global.svg,
 * frame order — orange first, the W26 default). Per-accent primary tokens do
 * not exist: the generated per-accent blocks are :root[data-accent]-scoped,
 * so a group showing all nine at once cannot consume them. Token seam
 * recorded in states.md + the lane log. */
```
What is wrong: the token model cannot express "all 9 accent primaries visible at once" — accent primitives exist (this file, l.244-275) but the only emitted surface is the single-active-accent `[data-accent]` block — so the Settings swatch row hardcodes all 9 hexes in CSS (stylelint-disabled). The 9 values currently MATCH the primitives byte-for-byte (verified: #d37918/#e15032/#c49e23/#19a234/#18aad3/#185dd3/#8e42ff/#f052b3/#afafaf), but no gate connects them: re-pin accent.red in tokens.json and the swatch row silently goes stale (tokens:check only verifies generation; stylelint disable suppresses the only other guard). Same pattern, smaller: options.css:311 hardcodes #303030 ("no token — token-lane handoff") and form.css:62 consumes a nonexistent `--color-stroke-input` with an off-ramp #3a3a3a fallback.
Why it matters: these 12 declarations are the UI's only active bypasses of the "semantic layer is the only consumable layer" rule (out of 78 live vars / 900+ var() uses — the rule otherwise holds), and the swatch block is the one bypass with an unguarded drift path back to this file.
What correct looks like: emit per-accent-primary semantic vars (e.g. `--color-accent-primary-<name>` for swatch rows) or a generated swatch list in the ui bridge (tokens.ts already carries ACCENTS); add the missing input-stroke token the consumer is waiting for.
Confidence: HIGH — Verification: ran (values diffed against primitives; consumer census; gate inventory).

## 4. Per-token-group table (data file: groups are the units)
| Group (lines) | Tokens | Verdict | Severity | Notes |
|---|---|---|---|---|
| primitive.space.32 + .48 (486–493) | 2 | REAL | — | "[PROPOSAL]" grid steps; 48 consumed via alias.item-pitch, 32 record-only (part1 F-006) |
| primitive.stroke.width (495–519) | 5 | REAL | — | icon 1.5 ×3464 corpus-exact (part1); window 20px mined value — dead downstream (F-004) |
| primitive.shadow (521–544) | 2 | REAL | — | measured feGaussianBlur chains; overlay dy "[PROPOSAL]" disclosed |
| primitive.blur (545–556) | 2 | REAL | — | Figma 50→25px CSS halving documented |
| primitive.opacity (557–600) | 10 | REAL | — | 0.95 ×169 corpus-exact; keys are percent labels — fine |
| primitive.motion (601–616) | 3 | REAL | — | fast/step/slow; step+slow unconsumed downstream (F-004) |
| alias.color surfaces (622–691) | 10 | REAL | S4 | F-002 (elevated Δ mislabel), F-003 (muted [PROPOSAL] stale), F-006 (dual authoring); #DEDEDE rect + #1B1B1B + Frame 21 claims byte-verified |
| alias.color content (692–737) | 7 | REAL | S3/S4 | F-001 (tertiary raised 4.14), F-007 (bright≡primary light); quiet/accelerator numbers all exact |
| alias.color accent (738–766) | 5 | REAL | S4 | F-005 (≈2.3:1 stale basis); per-accent emission verified |
| alias.color record/stroke/focus/eq (767–819) | 11 | REAL | — | focus.ring numbers exact; eq.1–7 resolve 1:1 |
| alias.font (821–899) | 17 | REAL | — | pure pass-through of primitive font layer; time≡sans ruling in F-007 |
| alias.space (901–919) | 4 | REAL | — | grid/inset/track-inset/item-pitch; all consumed |
| alias.radius (920–966) | 11 | REAL | — | window=card=14 by design ("floating windows … r14"); app/ring/card-lg dead downstream (F-004, part3 emission) |
| alias.stroke-width.icon (967–972, straddles boundary) | 1 | REAL | — | $value at l.970 (this chunk), $description at l.971 (part3); token complete and correct |

## 5. Test honesty (gates executing this file — none in-file)
| Gate | Would it fail if this file were broken? | Reason |
|---|---|---|
| crates/theme-tokens/tests/diff.rs + pipeline.rs (baseline row 3) | YES for values/structure | Regeneration byte-compare + shape/value assertions (W15-A02/A05) |
| ui tokens:check (baseline row 24) | staleness only | Verifies bridge copy freshness, not semantic correctness |
| Mutation reasoning (F-001): flip content.tertiary claim to include a failing surface | NO test fails | Descriptions are unasserted prose — F-001/F-002/F-003/F-005 are invisible to every gate. pipeline.rs recomputes per-accent contrast (the crate's ramps) but not the surface-pair claims in role descriptions. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| content.tertiary "DELIVERED value now measures ≥4.5:1 on surface/panel/deep/muted/raised" | l.711 | FALSE (dark×raised 4.14) | F-001 recompute |
| "wells Δ4" light-ramp compression | l.683, l.164 | FALSE (wells Δ8; Δ4 is the panel step) | F-002 |
| surface.muted light "[PROPOSAL] derived" | l.690 | FALSE (pixel-measured per l.164) | F-003 |
| "orange.300 ≈2.3:1 on light" | l.747 | REAL only vs pre-repin surfaces (1.2–2.0 current) | F-005 |
| All other WCAG numbers in range (40+) | l.634–790 | REAL | §2 recompute, all exact |
| "lone #DEDEDE rect (1238×28 r6 at (254,1039))" | l.634 | REAL | Player - Light.svg:118 byte-match |
| Per-mode roles flip dark/light correctly | l.622–747 | REAL | 24/24 per-mode objects exactly {dark,light}; emitted light overrides match (A05 census) |
| "alpha via opacity.chrome" (chrome pairing) | l.648 | FALSE at consumption | --opacity-chrome zero consumers (F-004) |
| "semantic layer is the only consumable layer" (role of this layer) | l.2/l.1075 | REAL with 12 disclosed bypass declarations | F-008; 78/86 vars live, 900+ var() uses |

## 7. Dead / duplicated / unreachable code
- 8 zero-consumer semantic vars (F-004) — supply-side dead weight (emission is part3/A05).
- 20 zero-inbound primitive orphans catalogued in part1 F-006 (12 gray, 4 radius, 4 space — of the space orphans, only `space.32` (l.486-489) sits in this chunk; 12/16/24 are part1's l.474-485).
- alias layer itself: 0 orphans of 86 (verified) — no dead roles.
- font.family.time ≡ sans: intentional seam (F-007 ruling).
- Whole file dev-lane only; nothing shipped reads it (header).

## 8. Dependencies & flags
None (data). Same parser/pipeline as part1. The alias→semantic 1:1 mapping means this chunk defines every name the UI consumes; the ui bridge (ui/scripts/gen-tokens.mjs) copies the emission verbatim (baseline row 24 PASS).

## 9. Open questions for the next agent
- part3 / A03: the semantic layer duplicates every alias 1:1 ({alias.color.surface} → {alias.color.surface}) — is any semantic token NOT a pure pass-through? If none, the three-layer grammar is 86 lines of ceremony; that ruling belongs to the layer's owner.
- ui form-lane owner: form.css:62 waits on a --color-stroke-input token that tokens.json never defines (F-008) — the seam is documented on the consumer side only.
- Whoever re-pins accents: the form.css swatch block (F-008) is the unguarded drift path; wire it or add a check before the next accent change.
