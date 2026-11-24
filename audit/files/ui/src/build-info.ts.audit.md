# AUDIT: ui/src/build-info.ts
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A02) |
| Date | 2026-11-23 |
| Lines / functions | 9 / 2 consts |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — main.tsx -> App.tsx -> settings route -> updates-about.tsx:29 `import { UI_PACKAGE_NAME, UI_PACKAGE_VERSION } from "../../../build-info"` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg define in vite.config.ts (exit 0); rg consumers (exit 0); sed updates-about.test.tsx:15-43; rg alternate package.json readers (none) |

## 1. One-paragraph verdict
Two-line re-export of vite `define`-injected constants. Wiring verified
end-to-end: vite.config.ts:32-34 defines __UI_PACKAGE_NAME__/__UI_PACKAGE_
VERSION__ from the real manifest (package.json:2 "lumen-ui", :4 "0.1.0");
build-info.globals.d.ts:6-7 declares the global types; this file re-exports
them typed. The consumer (About panel, updates-about.tsx:29,37,44,54) is
reached from the settings route. The comment's "only path" claim is TRUE: no
other src file reads package.json or hardcodes "lumen-ui". The test
(updates-about.test.tsx:41-43) proves the injection is real — it asserts the
panel displays the injected values and that the version matches real semver,
so an unresolved define would fail the import outright.

## 2. What actually works (verified)
— verified by `rg -n "__UI_PACKAGE" vite.config.ts` -> :33-34 define entries.
— verified by `sed -n '15,43p' src/screens/settings/areas/updates-about.test.tsx`
  -> test asserts `textContent` equals the injected constants + semver regex.
— Baseline ui-test.txt: updates-about tests are inside the 136-file passing
  run (vitest resolves the same vite.config.ts define block).

## 3. Findings
None. Checked: dangling define (none — both constants defined), type safety
(globals.d.ts present), drift risk (none — single source: package.json),
boundary violation (none — no src file imports outside ui/src for this),
hardcoded fallback (none).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `UI_PACKAGE_NAME` (const) | 8 | REAL | — | Injected from pkg.name |
| `UI_PACKAGE_VERSION` (const) | 9 | REAL | — | Injected from pkg.version; semver-pinned by test |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| updates-about.test.tsx:41-43 | YES | Hardcodes no version; compares panel text to the injected constants and asserts semver shape — a missing define or a hardcoded literal both fail |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "vite.config.ts reads ui/package.json at config load and `define`s these constants" | build-info.ts:2-3 | REAL | vite.config.ts:32-34 |
| "the only path by which the manifest's name/version reach UI code" | build-info.ts:3-5 | REAL | rg for package.json/lumen-ui readers in src -> only build-info + its consumers' comments |

## 7. Dead / duplicated / unreachable code
None. Both constants consumed by the About panel.

## 8. Dependencies & flags
Vite define + ambient global declarations. No feature gates. (W15-A02's
existing build-info.globals.d.ts audit covers the types side.)

## 9. Open questions for the next agent
None.
