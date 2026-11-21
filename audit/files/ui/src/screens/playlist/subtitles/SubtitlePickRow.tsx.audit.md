# AUDIT: ui/src/screens/playlist/subtitles/SubtitlePickRow.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 80 / 2 (SubtitlePickRowProps interface, SubtitlePickRow component) |
| File verdict | REAL |
| Reachable from shipped entry point? | PARTIAL — tree-mounted (PlaylistScreen → AddSubtitlesDialog → SubtitlePickList → SubtitlePickRow; the dialog opens in the real /playlist flow, g19 capture `1440-06-subs-real`), but NO shipped data path can ever render a row: candidates default `[]` (AddSubtitlesDialog.tsx:96) and the candidates producer (folder-scan service / OpenSubtitles client) does not exist (AddSubtitlesDialog.tsx:27, k15 P2-3, g19 P3-5) — rows appear only under the dev harness fixtures and tests. No shipped binary exists (W14 pin). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "SubtitlePickRow" ui/src` (5 consumers: list, harness, index barrel, 2 tests); subtitle one.svg + subtitle multiple.svg read in full (geometry/colors extracted); Add Subtitles to Playlist/Folder-2.svg rect extraction (band x20 w1008, Vector 44 x1001–1015); tokens.css values (--color-content-secondary #8f8f8f, --color-accent-icon #d37918, --font-size-body 14px); subtitles.css:253-320 read; docs/waves/w26/logs/w26.b11.md + w26.k15.md + findings/w26.g19.md read (vision-loop status, delta documentation search — none); SubtitlePickList.test.tsx read (row + multi-mode coverage) |

## 1. One-paragraph verdict
Correct, honest presentational row — every frame-anatomy claim in its header checks out against the real SVGs (band, insets, glyph geometry, check dimensions and color, hover-cycle variant), and the a11y split (semantics in the list, spread via optionProps) is real and tested. Two substantive problems: the header's color-delta parenthetical is FALSE — the filename renders #8f8f8f against the frame's #969696 (Δ7/255, not "1/255-class") and NO lane log documents it; and the row cannot render once in any shipped flow because the candidates producer does not exist (documented honestly in three places, but it makes this component fixture-only code today). The g19 verification of this exact fragment ended MATCH for subtitle_one (deterministic geometry comparison; the vision loop itself was 429-blocked wave-wide).

## 2. What actually works (verified)
- Frame anatomy vs subtitle one.svg (read in full): selected band `<rect x="20" y="100" width="479" height="36" rx="6" fill="#090909"/>` — 36px, rx6, #090909 selected-only ✓; glyph paths x31–49/y28–48 = 18×20, stroke-width 1.5 #6B6B6B ✓ (11px off band left edge: 31−20 ✓); filename path starts x66.7 → 17.7px past glyph end (comment "17px") ✓; Vector 44 `M472 117.5L476.667 123L486 112` = 14×11, stroke-width 2, #D37918 ✓, 13px off right edge (499−486 ✓).
- CSS implements the claims: subtitles.css:253-264 (height 36px, radius var(--radius-control), padding 11/13px, --color-content-secondary), :268-271 (selected band = --color-surface), :297-307 (check = --color-accent-icon #d37918, overriding the icon base ink), :311-320 (cycle display:none → inline-flex on hover/focus of --multiple only).
- Token check: --color-accent-icon #d37918 ≡ frame #D37918 exact (tokens.css:33); --font-size-body 14px within the frame's 13–14px claim.
- subtitle multiple.svg carries exactly 2 × #8F8F8F vectors (42/43 — the stacked arrows) and the CSS hover-reveal matches the "Property 1=hover" state.
- g19 findings record (docs/waves/w26/findings/w26.g19.md): subtitle_one.png verdict **MATCH (fragment-exact)** from harness captures `H-subtitle-rows`/`H-folder-online`; console errors zero; vision loop 429-blocked with deterministic geometry comparison standing in.
- Presentational split verified: SubtitlePickList.tsx:176-190 spreads exactly {role, tabIndex, "aria-selected", onClick, onFocus} — no id/className/ref keys, so the spread-last hazard (F-003) is untriggered.
- Baseline vitest 1705/1705 pass (INDEX #23), including SubtitlePickList.test.tsx's row tests.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/playlist/subtitles/SubtitlePickRow.tsx:9-10
Code:
```tsx
//   * filename 13–14px #969696 (→ content-secondary; 1/255-class delta
//     documented in the lane log) starting 17px past the glyph;
```
What is wrong: Two false assertions in one parenthetical. (1) Magnitude: the row name renders `--color-content-secondary` = **#8f8f8f** (tokens.css:41) against the frame's **#969696** (subtitle one.svg, both text paths `fill="#969696"`) — a **Δ7/255** per-channel delta, not "1/255-class". This repo's own vocabulary treats that band as significant (w26.j10 "Δ≤1/255 claimed and plausible"; w26.h16 tracks "Δ4/255 to Δ11/255" as flag-worthy vision findings). (2) Provenance: "documented in the lane log" — w26.b11.md is 18 lines with ZERO color content; the only frame-color record (g19) notes `name #969696 at x66.7` without any delta ruling. No document anywhere in docs/waves/w26/ records this delta.
Why it matters: The comment launders a real 7-unit ink deviation as a documented sub-unit rounding difference — exactly the pattern (comment asserts, record absent) this audit exists to catch; a token-lane fix or a Lead ruling can never be tracked against a citation that does not exist.
What correct looks like: "frame #969696 → token #8f8f8f (Δ7/255, WCAG-motivated, ruling pending — cf. the glyph's Δ23 ruling in subtitles.css:282-284)".
Confidence: HIGH — Verification: read; token value and both SVG fills extracted; b11 log read in full; rg over docs/waves/w26 for the delta — no record.

### F-002 [S3]
Where: ui/src/screens/playlist/subtitles/SubtitlePickRow.tsx:41-48 (component under audit) + AddSubtitlesDialog.tsx:27,96 (data path)
Code:
```tsx
export function SubtitlePickRow({
  fileName,
```
```tsx
//     the CURRENT session only — so no shipped host can produce candidates or
```
What is wrong: The component is correct but UNRENDERABLE in any shipped flow: `candidates = []` is the dialog default and the producer seam (folder-scan service / OpenSubtitles client) does not exist, so the listbox mounts with zero options and the real /playlist flow shows the honest empty state (g19 P3-5, capture `1440-06-subs-real`). Every rendered instance of this row in existence comes from the dev harness fixtures or tests.
Why it matters: Under the wave's hostile lens ("is any UI element wired to a real backend?") this is a fixture-only component: the click affordance (optionProps.onClick → toggle) can never fire for a user. Honest — the gap is admitted in AddSubtitlesDialog.tsx:27 and k15 P2-3 "(a) candidates producer… (Lead decision)" — but it is still dead-in-product code awaiting a seam.
What correct looks like: The Lead decision k15 defers to: either land a candidates producer (engine folder-scan command) or gate the flow's entry points until one exists.
Confidence: HIGH — Verification: read; default traced (AddSubtitlesDialog.tsx:96), seam absence confirmed in k15 log and g19 P3-5.

### F-003 [S4]
Where: ui/src/screens/playlist/subtitles/SubtitlePickRow.tsx:50-60
Code:
```tsx
    <div
      id={id}
      ref={ref}
      className={[
```
```tsx
      {...optionProps}
```
What is wrong: `optionProps` spreads LAST, so any `id`, `className`, or `ref` key inside optionProps silently clobbers the structural props computed above (React last-prop-wins). The prop is typed bare `HTMLAttributes<HTMLDivElement>`, which permits all three.
Why it matters: The doc comment promises optionProps carries "role/aria/tabIndex/handlers"; nothing enforces that. Today's only caller is clean (verified), so latent — but a future caller passing a test className wipes the selected-band class and every state style with it.
What correct looks like: Destructure the behavioral subset explicitly (`{role, tabIndex, onClick, onFocus, ...}`), or spread optionProps first and re-apply id/className/ref after.
Confidence: HIGH — Verification: read; caller's spread keys enumerated (SubtitlePickList.tsx:176-190).

### F-004 [S4]
Where: ui/src/screens/playlist/subtitles/SubtitlePickRow.tsx:62-69
Code:
```tsx
      <span className="lumen-plsubs-row__icon" aria-hidden="true">
        <SubtitleFileGlyph size={19} />
      </span>
```
What is wrong: Rendered glyph drifts from the frame the header pins: SubtitleFileGlyph draws `width={19} height={(19*20)/18}=21.1` against the frame's 18×20 (comment line 7 claims "18×20"); the check uses the square Icon primitive at size 15 (15×15 box) against Vector 44's 14×11. Both are decorative, aria-hidden, sub-2px deltas inside a 36px row — but neither the comment nor any measurement record accounts for them.
Why it matters: The header sells exact frame anatomy; the deltas are small but unrecorded, unlike the two ink deltas that DID get documented treatment.
What correct looks like: size={18} on the file glyph (height auto-scales to 20), or a comment recording the 19px choice as deliberate (optical sizing).
Confidence: HIGH — Verification: read; SubtitleGlyphs.tsx geometry math traced (viewBox 0 0 18 20).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SubtitlePickRowProps` | 24-39 | REAL | — | honest prop docs; k15 note on id ("no longer wires aria-activedescendant") verified true (list test pins its absence, SubtitlePickList.test.tsx:118-121) |
| `SubtitlePickRow` | 41-79 | REAL | S3/S4 | pure render; className join + filter(Boolean) correct; selected/multiple modifier classes match CSS; `selected ? check : (multiple && cycle)` correctly suppresses the cycle on selected rows (CSS comment "never on selected"); spread-last hazard (F-003); glyph drift (F-004); data-path dead in shipped flows (F-002) |

## 5. Test honesty (if tests exist in/for this file)
SubtitlePickList.test.tsx:53-88 covers the row directly.
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders the filename with option semantics and the selected band + check" (:54-67) | YES | rerender selected→ asserts --selected class appears + check node exists (and absent before) |
| "renders the frame's subtitle-file glyph (aria-hidden)" (:69-73) | YES (weakly) | asserts svg presence under __icon; does not check aria-hidden attr or geometry |
| "spreads optionProps (role, aria-selected, handlers)" (:75-87) | YES | queries by role option {selected:true} and fires the spy through the spread |
| Mutation check (reasoned): delete the `multiple && (…cycle…)` branch (lines 71-77) | NOT caught | no test anywhere queries `.lumen-plsubs-row__cycle` — the multiple variant's ONE distinctive affordance is unpinned (rg over the test file: zero "cycle" hits) |
| Mutation check: swap spread order (optionProps before id/className) | NOT caught | no test passes conflicting keys — F-003's latency is unguarded |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "row band 479–1008 wide × 36px, rx6 — fill #090909 … SELECTED state only" | SubtitlePickRow.tsx:5-6 | REAL | subtitle one.svg rect w479 h36 rx6 #090909 (selected group only); Folder-2.svg bands w1008 h36 rx6 — the two host widths; CSS --selected = --color-surface |
| "leading subtitle-file glyph (18×20, stroke 1.5 #6B6B6B …) 11px off the row's left edge" | :7-8 | REAL | SVG paths x31–49 y28–48, stroke-width 1.5 #6B6B6B; 31−20=11 |
| "filename 13–14px #969696 … 1/255-class delta documented in the lane log" | :9-10 | **FALSE** | token #8f8f8f vs frame #969696 = Δ7/255; no lane log documents any delta (F-001) |
| "trailing orange check (Vector 44: 14×11, stroke 2 #D37918) … 13px off the right edge" | :11-12 | REAL | SVG Vector 44 M472→486 / y112–123, sw2, #D37918; 499−486=13; token #d37918 exact |
| "MULTIPLE variant shows the stacked ←/→ cycle glyph (#8F8F8F) on hover instead" | :13-15 | REAL | subtitle multiple.svg 2×#8F8F8F (Vectors 42/43); CSS .lumen-plsubs-row__cycle display:none → inline-flex on :hover/:focus-visible |
| "The row is PRESENTATIONAL: selection semantics … belong to SubtitlePickList" | :17-18 | REAL | list spreads role/tabIndex/aria-selected/handlers (SubtitlePickList.tsx:176-190) |

## 7. Dead / duplicated / unreachable code
- Unreachable-with-data in shipped flows (F-002): fixture/test-only component until the Lead lands a candidates producer. No duplicated code; no dead branches (the `multiple &&` expression renders nothing for false — correct React).

## 8. Dependencies & flags
Imports: Icon (check glyph, variant="strong" — verified exported), SubtitleFileGlyph/CycleGlyph (local frame ports, currentColor-tinted). No engine/bridge imports — nothing to wire; the data seam is the dialog's problem, not this file's. No feature flags. i18n: none (filename is verbatim data).

## 9. Open questions for the next agent
- The cycle-glyph test gap (§5) is a two-line fix for whoever touches SubtitlePickList.test.tsx next.
- F-001's phantom citation pattern ("documented in the lane log") is worth a cross-file rg by a docs-wave agent: how many comments cite lane-log records that do not exist?
