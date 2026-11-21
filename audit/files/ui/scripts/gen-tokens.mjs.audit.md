# AUDIT: ui/scripts/gen-tokens.mjs
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 153 / 3 (main, renderTs, top-level catch) |
| File verdict | REAL (executed: baseline #24 `tokens:check` PASS; I independently recomputed the sha and the byte-identity of both outputs) |
| Reachable from shipped entry point? | N/A (dev tooling) — wired as `npm run tokens:generate` / `tokens:check` (ui/package.json:22-23); `--check` executed in baseline run #24; its OUTPUT (ui/src/tokens/generated/tokens.css) is reachable: imported by ui/src/main.tsx:27 (`import "./tokens/generated/tokens.css";`), 17+ consumer files import tokens.ts. NOT run by any CI workflow (F-002). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "tokens:generate|tokens:check" ui/package.json .github/workflows/ui.yml` → package.json:22-23 wired, ZERO workflow hits; `rg -n "tokens:check|settings:check|hotkeys:check" .github/workflows/*.yml` → absent from all 12 workflows; `sha256sum themes/generated/tokens.css` → 26c55ab2b319… matches TOKENS_CSS_SHA256 (tokens.ts:15) and PINNED_SHA256 (tokens.test.ts:54); `diff themes/generated/tokens.css ui/src/tokens/generated/tokens.css` → identical; `rg -o 'data-accent="…"'` → 9 real accents + `<id>` placeholder; `sed -n '88,110p' product.toml` → [brand] accent_color = "#4f8cff"; `awk '/^:root \{/,/^\}/'` → 86 declarations = baseline's "86 semantic tokens"; read of the whole file. |

## 1. One-paragraph verdict
A real, executed, zero-dependency codegen bridge: it copies the Rust-owned themes/generated/tokens.css verbatim into ui/, derives typed accessors (modes, accents, 86 semantic names, pinned sha256), and offers a working `--check` drift gate. Baseline #24 ran it PASS and I reproduced its guarantees offline (sha matches, both outputs byte-identical, 86 root declarations counted, 9 accents confirmed). Four defects: the header credits product.toml as the source of the "orange" default accent while product.toml's actual brand field is a provisional blue `#4f8cff`; the "drift is caught" claim is only true for humans who run the script locally — CI runs none of tokens/settings/hotkeys:check; the generated doc comment hard-codes "The 9 accent selections" over a dynamically parsed list; and `--check` reports a missing file as "differs from".

## 2. What actually works (verified)
— verified by baseline #24 (audit/runs/baseline/ui-tokens_check.log.txt): `gen-tokens --check: OK (86 semantic tokens, 2 modes, 9 accents, sha256 26c55ab2b319…) EXIT=0`.
— verified by `sha256sum themes/generated/tokens.css` → 26c55ab2b3193445728537df9cbb878bc77a46b7752714f1ff45752489648a20 == TOKENS_CSS_SHA256 (ui/src/tokens/generated/tokens.ts:15) == PINNED_SHA256 (ui/src/tokens/tokens.test.ts:54). The chain crate-css → copy → typed sha is currently consistent.
— verified by `diff` of the two css files → byte-identical; and `awk` count of `:root` declarations → 86, matching the script's own parse and the baseline count.
— verified by `rg` → the generated css is the live runtime cascade (ui/src/main.tsx:27), and tokens.ts consumers span theme/, screens/, components/ (17 files).
— Regex engine traced by hand: `blockRe` correctly binds `:root[data-theme="light"]` and per-accent blocks; `declRe` collects only custom-property names; the only comment in tokens.css is a `/* … */` header with no `}`, so no brace mis-slicing is possible today.

## 3. Findings
### F-001 [S3]
Where: ui/scripts/gen-tokens.mjs:30-35 (+ product.toml:91-97)
Code:
```
// Doc 08 §3.3: dark is the default mode, orange the default accent (W25
// Figma redesign, 2026-11-11 — the VLC orange, fidelity spec §1). These are
// product constants (product.toml brand fields) — the non-default values are
// parsed from the CSS selectors below.
const DEFAULT_MODE = "dark";
const DEFAULT_ACCENT = "orange";
```
What is wrong: The comment asserts orange is a "product constant (product.toml brand fields)". product.toml's [brand] section carries `accent_color = "#4f8cff"` — a BLUE hex, self-flagged `# [VERIFY] against design package token extraction` and "this literal is provisional" (product.toml:93-95). The two declared sources of truth disagree: the token pipeline's default accent is VLC orange (#d37918 in `:root`, verified), the rebrand manifest's brand accent is blue #4f8cff. DEFAULT_MODE="dark" does match [defaults] theme="dark" (product.toml:104-106), so the drift is accent-specific.
Why it matters: product.toml §8/§9 makes [brand] the rebrand authority and forbids other files hardcoding brand colors; a `cargo xtask rebrand` consumer reading accent_color would paint the app blue while the token cascade paints it orange. The comment launders a contradiction as settled fact.
What correct looks like: either product.toml accent_color = the orange hex (or an accent id) with the [VERIFY] resolved, or gen-tokens stops citing product.toml for a constant it does not read.
Confidence: HIGH — Verification: both files read; :root accent values read.

### F-002 [S3]
Where: ui/scripts/gen-tokens.mjs:113-115 (+ .github/workflows/ui.yml:100-128)
Code:
```
// Regenerate with \`npm run tokens:generate\` in ui/. Drift is caught by
// src/tokens/tokens.test.ts (pinned sha256 + structure) and
// \`npm run tokens:check\`.
```
What is wrong: Of the four baseline gates (#24), CI wires only l10n:check. `rg -n "tokens:check|settings:check|hotkeys:check" .github/workflows/*.yml` → zero hits across all 12 workflows; ui.yml's steps are typecheck (100), lint+lint:css (105-111), l10n:check (118), test (122), test:e2e (128). So the only CI-visible defense is tokens.test.ts's PINNED_SHA256 — which is hand-bumped in the same commit as the regenerated file (its own comment "w26.k18: +surface.panel-raised, +font.size.scale-label (123/86/86)" documents the manual bump ritual) and compares the COPY against itself, never against themes/generated/tokens.css. Crate-side drift (cargo xtask tokens output changes, ui copies not regenerated) passes CI green.
Why it matters: The pipeline's single-source-of-truth claim ("Rust-owned source of truth", :6-7) is enforced only by developer discipline; the advertised gate is a no-op in the exact place (CI) where drift lands.
What correct looks like: add `npm run tokens:check` (and settings:check/hotkeys:check) to ui.yml beside l10n:check — one line each, all four gates already exist as npm scripts.
Confidence: HIGH — Verification: workflow files read; baseline #24 log read; test pin mechanics read.

### F-003 [S4]
Where: ui/scripts/gen-tokens.mjs:133
Code:
```
/** The 9 accent selections (doc 08 §3.3). \`orange\` is the product default. */
```
What is wrong: The generated doc comment hard-codes the count "9" while `accentList` is parsed dynamically from the css selectors. Today 9 is correct (verified: blue, cyan, gray, green, orange, pink, red, violet, yellow) — the first accent added or removed silently makes every regenerated tokens.ts carry a wrong count in its public API doc.
Why it matters: The script exists to eliminate hand-maintained duplication; this line re-introduces exactly that, in generated output consumed by 17+ files.
What correct looks like: `${accentList.length} accent selections` in the template, or drop the count.
Confidence: HIGH — Verification: template read; accent list enumerated from tokens.css.

### F-004 [S4]
Where: ui/scripts/gen-tokens.mjs:75-90
Code:
```
if (oldCss !== css) {
  console.error(
    `gen-tokens --check: ${outCssPath} differs from ${crateCssPath} — run \`npm run tokens:generate\``,
  );
```
What is wrong: In `--check` mode a MISSING output file (readFile → null via `.catch(() => null)`) is reported as "differs from". null !== css is true, but the diagnosis printed names the wrong condition; same for oldTs ("is stale").
Why it matters: A new clone or a deleted generated file yields a misleading error that sends the user diffing two files instead of regenerating.
What correct looks like: branch on `oldCss === null` → "missing — run npm run tokens:generate".
Confidence: HIGH — Verification: read; null path traced.
Severity: S4 — exit code and remedy (the run command) are still correct.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| constants/paths | 19-35 | REAL | S3 | F-001: product.toml authority claim contradicted by product.toml itself; paths resolve correctly from import.meta.url |
| main() | 37-104 | REAL | S3/S4 | F-002 gate not CI-wired; F-004 null-vs-differs message; parse loop correct (86/2/9 reproduced); --check exits 1 on drift, 0 on match — no false-PASS path; early exit when 0 semantic tokens found (:64-67) |
| renderTs() | 106-148 | REAL | S4 | F-003 hard-coded "9"; JSON.stringify(css) embeds full 8.6 KB css as a TS string constant (deliberate, documented for vitest css-stub); template is deterministic + sorted (idempotence claim holds) |
| main().catch | 150-153 | REAL | — | prints message, exit 1; no swallowed errors |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| baseline #24 `tokens:check` | YES (as executed) | it byte-compares both outputs against the crate css — I reproduced the comparison with sha256sum/diff |
| ui/src/tokens/tokens.test.ts (pins TOKENS_CSS_SHA256, runs in CI via `npm run test`) | NO for crate-side drift | the pin is hand-bumped with each regeneration and never sees themes/generated/tokens.css (vitest stubs raw css imports) — it catches neither stale copies nor pin/copied-file divergence committed together (see F-002) |
| no self-tests for gen-tokens.mjs | N/A | the parser (regexes over generated css) has no unit tests; a malformed tokens.css (e.g. a comment containing `}` between `:root` and `{`) would mis-parse silently |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "zero deps" node script | :9 | REAL | imports are node:crypto/fs/promises/path/url only |
| "regenerate (idempotent, sorted output)" | :13 | REAL | semantic.sort(); Set-derived mode/accent lists; template deterministic — verified by structure, not by execution (REQUEST-ORCH-RUN not needed; --check PASS over committed output implies regeneration reproduces it byte-for-byte, else the baseline check would have failed) |
| "The script NEVER touches the Rust side" | :16-17 | REAL | only writes under ui/src/tokens/generated/ |
| "These are product constants (product.toml brand fields)" | :32-33 | FALSE | product.toml [brand] accent_color = #4f8cff ≠ orange (F-001) |
| "Drift is caught by … npm run tokens:check" | :113-115 | MISLEADING | true locally, false in CI — no workflow runs it (F-002) |
| "The 9 accent selections" | :133 | REAL today / FALSE on change | count hard-coded (F-003); 9 verified |

## 7. Dead / duplicated / unreachable code
None. Both outputs are live (css via main.tsx:27; ts via 17 importers). No dead branches.

## 8. Dependencies & flags
node: builtins only. `--check` flag is the only CLI flag; unknown flags are silently ignored (argv.includes) — harmless for a dev script. Depends on `cargo xtask tokens` having produced themes/generated/tokens.css (crate 1.B4) — present and committed.

## 9. Open questions for the next agent
- Who validates product.toml [brand].accent_color against the token cascade? (rebrand xtask — if it exists, does it compare #4f8cff to #d37918 and scream?)
- tokens.test.ts pin comment says "(123/86/86)" while baseline reports 86 semantic tokens — what are the 123? (raw vs semantic count in the crate-side output — worth one line in the tokens.json#part3 or theme-tokens audits.)
