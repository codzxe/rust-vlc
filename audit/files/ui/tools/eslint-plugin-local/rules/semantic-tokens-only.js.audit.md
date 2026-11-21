# AUDIT: ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 128 / 6 module constants + create, isFrameMarked, checkColorLiteral, checkDimensionLiteral, literalValue, 4 AST visitors + export |
| File verdict | REAL |
| Reachable from shipped entry point? | Tooling (not the lumen binary). Executed in baseline via vitest: vite.config.ts:41-44 includes `tools/eslint-plugin-local/**/*.test.js`; the RuleTester suite (semantic-tokens-only.test.js, 16 cases) ran green inside "npm test → 1705 passed / 1 skipped" (INDEX.md row 23). Repo-wide wiring: eslint.config.js:110 `"local/semantic-tokens-only": "error"` over `src/**/*.{ts,tsx}`, run by `npm run lint` → CI ui.yml:102-105. No baseline row runs `npm run lint` → whole-tree pass UNVERIFIED. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:4 |
| Commands run | sed -n verification of :31-35, :69-81, :91-97; Read of test file/index.js/eslint.config.js; rg `#[0-9a-fA-F]{3,8}\b\|rgba?\(` over ui/src *.{ts,tsx} (all hits = comments, tests, generated/, i18n/, frameData.ts — zero unmarked production literals); rg numeric `fontSize/radius: <number>` (zero hits); rg `hsl(\|hwb(\|oklch(\|lch(\|lab(\|color-mix` (3 hits, all tests); Read of frameData.ts markers; Read of scripts/lint-css.mjs (CSS-side gate claim). |

## 1. One-paragraph verdict
The rule enforces doc 08 §3.2 token discipline over .ts/.tsx with a genuinely strict color check (ANY string literal containing #hex or rgb(/rgba( is flagged, position-independent — inline styles, data modules, everything), a narrower dimension check (px strings on font-size/radius keys in object properties and JSX attributes), and a contract §4.4 `frame:` citation escape that is real and honest: frameData.ts's two exported hexes carry verbatim `/* frame: Video - Colors.svg … */` citations, the RuleTester suite pins the escape in both directions (cited hex passes :45-50; a NON-frame comment does not exempt :84-88), and the v3 .ts extension closed the documented v2 hole. Four latent gaps, none with a live offender in the tree: numeric dimension values (`fontSize: 14` — React treats numbers as px) escape because literalValue only accepts strings; non-hex/rgb color notations (hsl(), hwb(), lab/lch/oklch(), color(), named colors, interpolated `` `#${h}` `` templates whose quasis-join is just "#") escape HEX_OR_RGB_RE; the `frame:` marker regex matches ANY "frame:" text on the literal's line — an object key `frame:` would exempt an unrelated same-line hex; and the header's §4.4 exemption ("a literal preceded by … a frame-citing comment is allowed") is implemented for COLORS ONLY — a frame-measured `fontSize: "15px"` still fails, an undocumented asymmetry. Repo-wide `npm run lint` has no baseline row — the RuleTester execution is the only committed evidence.

## 2. What actually works (verified)
- Executed evidence: baseline row 23 (vitest 1705 passed) includes this suite via vite.config.ts:41-44; the 16 RuleTester cases pin every branch (exemptions for test/generated/css/i18n filenames, token values, frame citations valid + rejected, hex in .ts, dimension px in object + JSX attribute).
- Color check breadth: `Literal` + `TemplateLiteral` visitors fire on ALL string literals — rg census confirms zero unmarked production hex/rgb literals in ui/src (every hit is a comment, test, generated/i18n file, or the two frame-marked frameData.ts exports).
- Dimension key regex handles kebab and camel: `font-?size|radius` case-insensitive matches `fontSize`, `borderRadius`, `border-top-left-radius` (test :29 pins borderRadius + fontSize).
- File-type gate (:54-62): test/generated/i18n/non-.ts(x) return {} — the i18n catalog hex (`messages.ts:2653 "settings.subtitles.text-color.value": "#FFFFFF"`) is correctly exempt as displayed copy, not paint (test :54-57 pins exactly that fixture).
- Frame escape discipline: nearest-ancestor-with-comments decides and STOPS (:74-79) — a non-citation comment between the citation and the literal blocks the exemption (test :84-88); Program stops file-header exemptions.
- CSS-side counterpart is real: header claims .css is Stylelint territory — ui/stylelint.config.js exists and references semantic tokens; `npm run lint:css` → scripts/lint-css.mjs launches the locally-installed stylelint 18 (package.json devDeps) offline-first; CI ui.yml:111 chains it after `npm run lint`.

## 3. Findings
### F-001 [S4]
Where: ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js:91-97
Code:
```
    const checkDimensionLiteral = (node, keyName, value) => {
      if (keyName !== null && typeof value === "string" && PX_RE.test(value)) {
        context.report({ node, messageId: "rawDimension" });
      }
    };
    const literalValue = (expr) =>
      expr !== null && expr.type === "Literal" && typeof expr.value === "string" ? expr.value : null;
```
What is wrong: `literalValue` accepts only string literals, and `checkDimensionLiteral` requires `typeof value === "string"`. `style={{ fontSize: 14 }}` or `{ borderRadius: 8 }` — React interprets unitless numbers as px — pass untouched.
Why it matters: The most idiomatic React way to write a raw dimension is exactly the shape that escapes the gate. rg census: zero numeric fontSize/radius values in ui/src today (all dimensions live in .css = stylelint territory), so latent.
What correct looks like: Accept numeric literals on dimension keys too (`typeof expr.value === "number"` → flag), or document the string-only scope next to the header's v1 note.
Confidence: HIGH — Verification: read + rg `fontSize|radius…: <number>` (no hits).
### F-002 [S4]
Where: ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js:31, 104-106
Code:
```
const HEX_OR_RGB_RE = /#[0-9a-fA-F]{3,8}\b|rgba?\(/;
…
      TemplateLiteral(node) {
        checkColorLiteral(node, node.quasis.map((q) => q.value.raw).join(""));
      },
```
What is wrong: Only #hex and rgb()/rgba() notations are recognized. `hsl(…)`, `hwb(…)`, `lab()/lch()/oklch()`, `color()`, and CSS named colors (`"red"`) escape. Interpolated templates escape structurally: `` `#${hex}` `` joins quasis to `"#"`, which matches nothing — the template check only catches no-substitution smuggles (test :69-73 pins that one case).
Why it matters: The gate's promise is "raw colors rejected"; the modern color functions are the first-class spelling in current CSS. rg census: zero offenders in ui/src (3 color-mix hits are test regexes) — latent.
What correct looks like: Extend the regex (`hsla?(|hwb(|lab(|lch(|oklch(|color(`), or disclose the hex/rgb-only scope in the header (it currently reads as if "raw colors" generally are rejected, :7).
Confidence: HIGH — Verification: read + rg census.
### F-003 [S4]
Where: ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js:34, 69-73
Code:
```
const FRAME_MARKER_RE = /frame\s*:/i;
…
      const line = sourceCode.lines?.[node.loc?.start.line - 1] ?? "";
      if (FRAME_MARKER_RE.test(line)) {
        return true;
      }
```
What is wrong: The same-line test scans the ENTIRE source line for ANY "frame:" substring — not just a comment citation. An object key or property named `frame:` (e.g. `{ frame: 1, color: "#fff" }` on one line, realistic in a video-player codebase full of frame numbers) exempts the hex without any measurement record. Same for a variable `frame:` in a label position.
Why it matters: The escape is the rule's pressure valve; a false marker silently disables the color check for that line. Current tree: the only frame-marked sites are frameData.ts:9-16 with genuine citations — latent.
What correct looks like: Restrict the same-line test to comment text (tokenize the line, test only `//`/`/* */` segments), or require the citation shape `frame: <name>.svg` like the ancestor-comment path in practice does.
Confidence: HIGH on the mechanism (read), MEDIUM that no non-citation `frame:` co-line hex exists (line-level scan of rg hex hits showed all are comments).
### F-004 [S4]
Where: ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js:15-23 (header) vs 82-95 (implementation)
Code:
```
// Frame-measured ports are exempt per
// contract §4.4: a literal preceded by (or sharing a line with) a
// `frame:`-citing block comment naming a frame is allowed
```
What is wrong: The documented exemption is stated for "a literal" — unqualified. The implementation consults `isFrameMarked` ONLY in `checkColorLiteral` (:82-90); `checkDimensionLiteral` (:91-95) never checks it. A frame-measured `fontSize: "15px"` / `radius: "6px"` with a proper `/* frame: … */` citation still fails lint, contradicting the header.
Why it matters: Doc-vs-code drift inside the rule's own header; anyone porting a frame-measured dimension (the exact workflow the contract promises) hits a false positive. No live case (zero px strings on dimension keys in ui/src) — latent.
What correct looks like: Either call `isFrameMarked(node)` in the dimension path too, or scope the header sentence to color literals.
Confidence: HIGH — Verification: read of both sites; test file has no frame-marked dimension case (consistent with the narrower implementation, inconsistent with the header).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HEX_OR_RGB_RE / PX_RE / DIMENSION_KEY_RE / FRAME_MARKER_RE | 31-34 | REAL | S4 | F-002 color-notation gaps; F-003 marker breadth |
| isTestFile | 35 | REAL | — | Same regex as sibling rule |
| create | 52-62 | REAL | — | Exemption gate: test/generated/i18n/non-ts(x); generated regex `([\\/])generated\1` matches any path segment named generated |
| isFrameMarked | 69-81 | REAL | S4 | F-003 same-line over-match; ancestor walk stops at first commented node (correct) and at Program |
| checkColorLiteral | 82-90 | REAL | — | Frame-marked consult (colors only — F-004 asymmetry) |
| checkDimensionLiteral | 91-95 | REAL | S4 | F-001 string-only; F-004 no frame consult |
| literalValue | 96-97 | REAL | S4 | F-001 numeric rejection |
| Literal visitor | 99-103 | REAL | — | Color check on every string literal — broad, catches inline styles |
| TemplateLiteral visitor | 104-106 | REAL | S4 | F-002 interpolation join |
| Property visitor | 107-116 | REAL | — | Dimension keys in object literals; computed keys skipped (unknowable, honest); key regex covers kebab+camel |
| JSXAttribute visitor | 117-123 | REAL | — | Dimension attrs; `typeof name !== "string"` guards namespaced names |
| default export | 128 | REAL | — | index.js:27 |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| valid: token dimensions, width:'420px' (out of scope), test/generated/css/i18n filenames, var() in .ts, frame-cited hex (block + same-line), catalog hex | YES | Each pins a distinct gate branch; filename exemptions fail if the gate regex breaks |
| invalid: raw hex, fontSize px, template rgb, JSX borderRadius px, hex in .ts, NON-frame comment + hex, .ts dimension | YES | The non-frame-comment case (:84-88) is the escape-abuse regression test — mutation: delete `FRAME_MARKER_RE.test(c.value)` → it fails; mutation: make isFrameMarked always true → :61/:81 hex cases fail |
| (whole-suite reach) | YES | Baseline vitest row 23 execution proof |
| Missing coverage | — | No case for numeric dimensions (F-001), non-rgb color functions (F-002), frame-marked dimensions (F-004) — consistent with the holes |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "TS source may express colors and font-size/radius dimensions ONLY through semantic tokens — raw #hex, raw rgb()/rgba(), and raw px … are rejected" | :5-8 | REAL | Visitors :98-123; tests pin each; rg census: zero unmarked raw literals in production src |
| "Exemptions: test files and generated files … plain .css files are Stylelint territory (ui/stylelint.config.js)" | :8-11 | REAL | :54-62 gate; stylelint.config.js exists; lint-css.mjs + CI ui.yml:111 run it |
| "v3 … covers .ts files too" | :13-14 | REAL | :58 suffix check; test :81 (.ts hex fails) |
| "Frame-measured ports are exempt per contract §4.4: a literal preceded by (or sharing a line with) a frame:-citing block comment … is allowed" | :15-23 | FALSE-by-omission | F-004: implemented for colors only, dimensions never consult isFrameMarked |
| "promoted verbatim from ui/eslint.config.js (issue #36) … the rule is path-free" | :1-3 | REAL (path-free) / REAL-UNPROVEN (verbatim) | No path resolution in the rule (only filename suffix/segment regexes); no git history access to verify "verbatim" |
| "The message-catalog surface (src/i18n/**) is exempt from the COLOR check" | :25-29 | REAL | :57 gate; messages.ts:2653 is the live fixture; test :54-57 |
## 7. Dead / duplicated / unreachable code
- None. isTestFile duplicated verbatim from the sibling rule i18n-no-literals.js:57 (2-line duplication, acceptable for rule isolation — table note, not a finding).
## 8. Dependencies & flags
Zero deps (pure AST + regex). Consumers: index.js:27, eslint.config.js:110 (error, src/**/*.{ts,tsx}), CI ui.yml:102-105. Complementary gates: stylelint (CSS files) + lint-css.mjs launcher (offline-first, npx fallback is network-touching but loudly disclosed — cross-ref, not counted). NOTE: `npm run lint` appears in NO baseline row — repo-wide enforcement is wiring-only evidence (CI green-run claims in ui.yml comments are unexecuted in committed evidence).
## 9. Open questions for the next agent
- AccentColorPicker.tsx (my file 17): confirm accent hex display copy reaches the screen without tripping this rule (generated token import = exempt path; raw hex strings would need the i18n/data exemption or frame marker).
- If any wave adds hsl()/oklch() or numeric inline dimensions, F-001/F-002 promote to S3 with that file as proof.
