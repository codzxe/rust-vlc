# AUDIT: ui/src/screens/settings/areas/privacy-network.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 134 / 1 (NetworkArea) |
| File verdict | REAL (honest staged-inert network area; header misstates where the privacy-half telemetry rows live — contradicted by its own test file) |
| Reachable from shipped entry point? | YES in the web UI (route /settings/network → SettingsScreen → areas/index.tsx:22 import + registry; privacy-network.test.tsx:95-98 pins one area root). NO desktop binary exists (W14-A01 E0425). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read areas/privacy-network.test.tsx (4 NetworkArea tests + 4 privacy-half tests); sed schema.ts:686-745 (SETTING_PRIVACY_TELEMETRY/CRASH_REPORTS/USAGE_STATS: screen "privacy", section "data", featureExposed true — but NOT in SETTINGS_ENTRIES :2514-2521 nor any SETTINGS_AREAS section) + :146-154 (global.update-checks: Global screen, defaultValue false); rg `network` in areas/index.tsx (import :22 + registry); rg `settings.network` in i18n/keys/w26.i15.ts (lane copy exists) + messages.ts (26 central matches); Read staged.tsx (StagedRow semantics); sed quote verification. Baseline: ui-vitest.log.txt 136 files / 1705 passed (includes privacy-network.test.tsx). |

## 1. One-paragraph verdict
The rendered component is the honest-staged pattern again, done correctly: all 9 §C10 frame rows (Buffering dropdown + four @0.3-dimmed buffering inputs + two Proxy rows + two Security rows) render full-strength disabled with the staged tooltip, the seam marker, and the operable ⓘ on exactly the 5 frame-circle rows; no live [data-setting] exists anywhere in the tree (test-pinned); the Advanced bar is an honest inert disclosure. The product truth — no network stack exists to configure (no proxy, no TLS verification path; the engine's URL open path refuses unimplemented schemes) — is disclosed on every row. The one defect is the header's privacy-half note: it claims the telemetry rows "stay schema-backed on the generated privacy screen (privacy.telemetry / crash-reports / usage-stats / update-checks)" — FALSE for the first three (they are ui=false, CLI/config-only, absent from SETTINGS_ENTRIES and every area section, and the test file this comment cites asserts exactly that absence) and wrong-screen for update-checks (it is global.update-checks on the Global screen's startup section). The defaults-OFF half of the claim is true (telemetry false, crash-reports "off", usage-stats false, update-checks false — all read).

## 2. What actually works (verified)
— verified by privacy-network.test.tsx:35-98 (9 staged rows all data-staged="schema-seed" + title containing "Staged"; 4 dimmed rows; disabled switch/textbox; zero [data-setting]; frame-verbatim values "System proxy"/"Normal"/"Balanced"; 5/9 ⓘ census operable with reveal "network settings schema"; no socks inventions; one area root at /settings/network) + baseline ui-vitest (136 files, 1705 passed).
— verified by reading the telemetry schema entries (schema.ts:686-745): all three carry `screen: "privacy", section: "data"` yet are members of NO SETTINGS_AREAS section and NOT of SETTINGS_ENTRIES — the ui=false exclusion the test proves behaviorally (:108-109 isScreenRow=false, :131-132 nothing renders at /settings/privacy).
— verified by reading the defaults: telemetry false (:691), crash-reports "off" (:712), usage-stats false (:733), update-checks false (schema.ts:150) — the header's "all default OFF per product.toml §2.6 opt-in rule" is accurate.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/settings/areas/privacy-network.tsx:18-22 (header) vs the test it cites (privacy-network.test.tsx:102-110, :128-132)
Code:
```ts
// The PRIVACY half of the card's "Privacy & Network" group stays
// schema-backed on the generated privacy screen (privacy.telemetry /
// crash-reports / usage-stats / update-checks — all default OFF per
// product.toml §2.6 opt-in rule; asserted in privacy-network.test.tsx, not
// duplicated here — no double definition).
```
What is wrong: The claim "stays schema-backed on the generated privacy screen" is false for three of the four named keys. privacy.telemetry / privacy.crash-reports / privacy.usage-stats are ui=false rows (schemas/settings.toml:460-476): their generated constants exist (schema.ts:686-745) but they are in NO SETTINGS_AREAS section and NOT in SETTINGS_ENTRIES (verified :2514-2521) — nothing renders them anywhere in the UI. The cited test asserts the OPPOSITE of the comment: privacy-network.test.tsx:108-109 (`isScreenRow(SETTING_PRIVACY_TELEMETRY)).toBe(false)`) and :128-132 (nothing telemetry-shaped renders at /settings/privacy); its own header says "the telemetry rows relocated off the screen (g11-P2-1: ui=false, CLI/config-only)". The fourth key, update-checks, is global.update-checks and renders on the GLOBAL screen's startup section (schema.ts:146-154, SETTINGS_ENTRIES :2492), not the privacy screen.
Why it matters: The comment cites a test as proof of a statement the test disproves — the worst kind of stale doc, because a reader trusting the citation never opens the test. Someone auditing "where can a user see telemetry switches?" is told: privacy screen; truth: nowhere.
What correct looks like: "The privacy half's telemetry rows are CLI/config-only (ui=false, g11-P2-1 — isScreenRow asserted false in privacy-network.test.tsx); update-checks lives on the Global screen's startup section."
Confidence: HIGH — Verification: schema entries + SETTINGS_ENTRIES/AREAS membership + both test blocks read; the cited test's assertions quoted above.
### F-002 [S4]
Where: ui/src/screens/settings/areas/privacy-network.tsx:26-28
Code:
```ts
import { StagedAdvancedBar } from "./staged";
import { StagedRow } from "./staged";
import { useStagedMsg } from "./staged";
```
What is wrong: Three separate imports from "./staged" (the same pattern as recording.tsx:29-31 and privacy.tsx:39-40 — a family-wide assembly habit).
Why it matters: Hygiene; one statement suffices.
What correct looks like: `import { StagedAdvancedBar, StagedRow, useStagedMsg } from "./staged";`
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `NetworkArea` | 35-134 | REAL | S3 | Pure composition: 3 sections × StagedRow (9 rows: 1 enum + 4 dimmed text + 2 proxy text/enum + 2 security switch/enum) + StagedAdvancedBar; all copy through the staged seam (lane w26.i15 + central); no state, no transport, no engine imports; the only defect is the header's privacy-half claim (F-001). The staged proxy-password row renders an EMPTY disabled TextField — no secret ever displayed (safe). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :35 staged rows inert, 9 count, seam + title markers, dimmed ×4, disabled controls, zero data-setting | YES | The full honesty contract, DOM-pinned. |
| :67 ⓘ census 5/9 operable (dimmed rows carry none) | YES | Count + click-driven reveal. |
| :87 invents no live proxy options | YES | Negative assertion incl. /socks/i. |
| :95 routed at /settings/network, one area root | YES | Registry wiring. |
| :102/:112/:135/:150 privacy-half tests (schema truth, History rows, last.fm rows, no Browse) | YES | They test privacy.tsx's screen, not this file's component — misplaced but honest (they pin the g11 relocations). |
| Gap: nothing pins the header's privacy-half location claim — F-001's falsehood is invisible to the suite (the tests it cites assert the opposite, but no test reads comments). | | |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The PRIVACY half … stays schema-backed on the generated privacy screen (privacy.telemetry / crash-reports / usage-stats / update-checks)" | privacy-network.tsx:18-20 | FALSE | F-001: ui=false (not on any screen); update-checks is on Global; cited test asserts absence. |
| "all default OFF per product.toml §2.6 opt-in rule" | privacy-network.tsx:20-21 | REAL | schema.ts:691 (false), :712 ("off"), :733 (false), :150 (false). |
| "no network [[setting]] keys exist in schemas/settings.toml yet" | privacy-network.tsx:6-7 | REAL | rg `network.` schema keys → none in SETTINGS_ENTRIES (only privacy.*/global.* etc.); all rows seam="schema-seed". |
| "§C10 ships the four buffering inputs … as disabled groups @0.3" | privacy-network.tsx:12-14 | REAL | Test :59 (4 × data-staged-dimmed) + :61 (disabled). |
| "asserted in privacy-network.test.tsx, not duplicated here — no double definition" | privacy-network.tsx:21-22 | REAL (assertions exist) but they assert the OPPOSITE of the comment | F-001. |

## 7. Dead / duplicated / unreachable code
No dead code in the component. Duplicated: the triple-import pattern (F-002). Product-dead-by-design: every row (no network stack exists to configure — disclosed via staged markers).

## 8. Dependencies & flags
staged.tsx (StagedRow/StagedAdvancedBar/useStagedMsg → i18n keys w26.i15, all present in the lane file and centrally in messages.ts — 26 matches), staged.css. No transport, no engine imports, no feature flags.

## 9. Open questions for the next agent
- The telemetry constants carry `featureExposed: true` (schema.ts:692, :713, :734) while being excluded from every screen by ui=false — if the generator ever starts deriving exposure from featureExposed alone, dead rows would appear; the settings-schema owner should make the two flags consistent.
- Same header-drift family as recording.tsx F-001: the staged-area headers describe schema/yield facts that only the tests hold — one owner should sweep all five staged-area headers (library/network/recording/extensions + this) against their test files.
