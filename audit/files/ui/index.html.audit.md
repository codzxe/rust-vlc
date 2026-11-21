# AUDIT: ui/index.html
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 12 / 1 document (vite entry) |
| File verdict | REAL (the entry document that baseline #22 built — `tsc --noEmit && vite build` OK, 386ms, this file is the input) |
| Reachable from shipped entry point? | YES as artifact: the webview entry (tauri.conf.json frontendDist/devUrl target the vite build rooted here); W14 ceiling — the host crate cannot compile, so no shipped binary loads it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read of the file; `rg -n "csp" apps/desktop-tauri/tauri.conf.json` → :23 strict CSP (default-src 'self', no unsafe-eval); `ls ui/public` → absent (no favicon asset — non-issue for a webview, see §7); `sed -n '91,100p' product.toml` → [brand] carries icons/accent/logo but no name field (no rebrand claim touches this title); vite.config.ts defines read (build-info defines for the About panel); cross-ref my App.tsx audit (per-route document titles set at runtime from msgids). |

## 1. One-paragraph verdict
A minimal, correct vite entry: charset, viewport, static "Lumen" title (replaced per-route at runtime by App.tsx — verified in my App.tsx audit), one root mount, one module script. Security posture lives where it belongs for tauri — the host injects a strict CSP (tauri.conf.json:23: `default-src 'self'; script-src 'self'; … object-src 'none'; frame-src 'none'`), so a CSP meta here would be redundant. One hygiene note: `lang="en"` is frozen — accurate today (the UI ships exactly one catalog, en-US; W11: the localization crate is orphaned), but it is the outermost layer of the hard-coded-en-US story and nothing marks it for the day a second locale lands.

## 2. What actually works (verified)
— verified by baseline #22 (audit/runs/baseline/ui-build.log.txt, EXIT=0, built in 386ms): this document is the entry the production build compiles from — it is not dead weight.
— verified by `rg` in tauri.conf.json:23: the host's CSP covers script/style/img/media/font/connect/worker/frame sources strictly; no `unsafe-eval`; the html correctly defers to it.
— verified by reading App.tsx (my prior audit): document.title is set per-route from msgids (:261 area titles), so the static `<title>Lumen</title>` is only the pre-hydration value.
— `main.tsx` (the script target :10) verified earlier: imports fonts.css FIRST, then the generated tokens cascade (:27), then App.

## 3. Findings
### F-001 [S4]
Where: ui/index.html:2
Code:
```
<html lang="en">
```
What is wrong: The document language is hard-coded and nothing tracks it. Today this is honest — the app ships exactly one locale (en-US; the settings language picker offers only "English", the localization crate is orphaned per W11, my l10n-check.mjs audit found the gate guards a single catalog). But `lang` is the last mile of any localization effort: screenreader pronunciation, hyphenation, and font selection all key off it, and when a second catalog lands this line must become dynamic (or server/webview-set) — no TODO, comment, or doc marks it.
Why it matters: It is the outermost, easiest-to-forget layer of the frozen-en-US debt; shipping a Spanish catalog with `lang="en"` is an a11y defect that no existing gate (tokens/settings/hotkeys/l10n checks) would catch.
What correct looks like: a one-line comment here ("single-catalog en-US; revisit with locales") or the lang set from the same source the l10n gate counts catalogs from.
Confidence: HIGH — Verification: file read; W11 + my l10n-check.mjs audit findings (messages.ts:1757 single language option).

## 4. Per-function table
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| document (doctype/html/head/body) | 1-12 | REAL | S4 | charset/viewport/title/mount/script all standard; F-001 lang |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| baseline #22 build | YES (entry breakage) | vite build fails on a malformed entry html |
| App.test.tsx title tests (adjacent) | YES | per-route titles asserted — the static title's successor behavior is covered |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| (implicit) the entry document of the UI | vite default + tauri.conf frontendDist | REAL | baseline #22 built from it |
| (implicit) CSP handled host-side | tauri.conf.json:23 | REAL | strict CSP present in the host config |

## 7. Dead / duplicated / unreachable code
None. Note: no `ui/public/` dir exists, so no favicon ships — a browser-tab favicon 404s in the dev shell; a tauri webview does not fetch favicons, and window/taskbar icons come from apps/desktop-tauri/icons. Not a defect.

## 8. Dependencies & flags
None beyond the vite convention. The mount id `root` matches main.tsx's createRoot target (verified in my main.tsx reads). No inline scripts/styles (CSP-clean by construction).

## 9. Open questions for the next agent
- None. If locales ever land, F-001's comment/TODO is the cheapest insurance.
