# AUDIT: ui/src/screens/settings/areas/codecs.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 110 / 3 (useAreaMsg seam const, HwDecodePolicyControl, CodecsArea) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web UI): App.tsx:322 `<Route path="/settings/:area">` → SettingsScreen → area registry (areas/index.tsx:277 `codecs: "video"` redirect; :255 `video: VideoAreaScreen`) → CodecsFold (areas/video.tsx:163-167 renders `<CodecsArea/>`) → HwDecodePolicyControl. Ceiling: no GUI binary exists at HEAD (W14: tauri host crate E0425); web bundle builds clean (baseline row 22). Behaviorally exercised by codecs.test.tsx through the real /settings/codecs route (baseline row 23, vitest). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | rg/sed of generated/schema.ts:966-984 + 1726-1766 (entry shapes), Screens.tsx:200-280 (PlaybackScreen/PerformanceScreen composition), SettingsSections.tsx:191 (featureExposed filter), product.toml:168+186-189, NEXT-STEPS.md (rg cache-demux/cache-network/placement/codecs/D5/D6 → ZERO hits), messages.ts:452-454+1931-1934, Dropdown.tsx props+resolveLabel, codecs.test.tsx full read, shared.tsx makeAreaMsg, areas/index.tsx registry. |

## 1. One-paragraph verdict
A small hand-authored settings area that does exactly what its honesty charter says for the headline control: the hw-decode policy Dropdown is bound to the real schema key `playback.hw-decode-policy` (featureGate "hw-decode", featureExposed false — verified against the generated schema AND product.toml:168 `hw-decode = false` with the verbatim ADR-001/F2 comment), renders visible, disabled, with the gate reason carrying ADR-001/F2 text (catalog messages.ts:1933), and is inert (test clicks the disabled trigger, asserts no listbox opens). The generated PlaybackScreen hides the same entry via SettingsSections.tsx:191's featureExposed filter, so the control appears exactly once — coherent design, all pinned by seven behavioral tests. Two defects: the two cache controls (performance.cache-demux-mib / cache-network-mib) render on BOTH the generated performance screen (Screens.tsx:274, area performance, section resources) and inside this codecs fold — live user-visible duplication of the same-key controls, and the header's claim that this is "flagged to the schema owner in ui/NEXT-STEPS.md to move placement here (D5/D6 safe)" is FALSE: NEXT-STEPS.md contains no cache/placement/codecs/D5/D6 entry (rg census zero hits) — the remediation trail does not exist. Second, the Dropdown's `onChange` is unconditionally empty: the honest-disabled guarantee rests solely on `featureExposed=false`; when Phase 7 flips exposure (product.toml F2 → schema regen), the control renders ENABLED with a no-op handler — precisely the "dead control that looks alive" the header forbids — and no test guards that transition.

## 2. What actually works (verified)
- Schema truth: SETTING_PLAYBACK_HW_DECODE_POLICY (schema.ts:966-984): key "playback.hw-decode-policy", featureGate "hw-decode", featureExposed false, 3-value enumeration, default "software-only" — matches product.toml:168 (`hw-decode = false … rule F2 keeps this false until then`) and the F2 rule text at product.toml:186-189.
- Honest-disabled control: trigger disabled, `data-gate-reason="hw-decode"`, gate note text contains ADR-001 + F2 (messages.ts:1933) — codecs.test.tsx:84-94 asserts all of it; ran green in baseline vitest (row 23).
- Inertness verified behaviorally: test :96-102 fires click on the disabled trigger, asserts no listbox and unchanged "Software only" text.
- Single render of the policy control: generated PlaybackScreen lists the entry (Screens.tsx:210-212) but SettingsSections.tsx:191 filters `row.featureExposed` → hidden there; codecs fold shows it disabled — no duplicate.
- Cache rows bound to real schema keys (schema.ts:1726-1766, min 4/max 512+1024, restartRequired true); test :56-67 pins both `data-setting` hooks; the "config-never-invents" test (:110-123) cross-checks every rendered key against SETTINGS_ENTRIES.
- Fold routing: /settings/codecs redirects to the video area; the fold is a collapsed disclosure (aria-expanded false → true), codecs rows only after expansion — test :41-54 pins it against the real routed SettingsScreen.
- i18n: all three msgids exist in the union and catalog (messages.ts:452-454, 1931-1933); lane keys via shared makeAreaMsg (shared.tsx:42-49, fallback to lane catalog after useMsg); no raw copy (the allowlist has no codecs entries — none needed).
- Dropdown contract: `label` documented "already-resolved string", `resolveLabel` defaults to identity (Dropdown.tsx:66-70,123) — passing pre-resolved `m(...)` strings is the sanctioned pattern; `disabled` prop reaches the trigger button (Dropdown.tsx:373).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/settings/areas/codecs.tsx:16-19 (header claim) + 105-106 (render); ui/src/screens/settings/generated/Screens.tsx:274 (the other render)
Code:
```
//   • cache: performance.cache-demux-mib / cache-network-mib are real schema
//     keys (advanced-flagged). NOTE: their schema placement currently also
//     renders them on the generated "performance" screen — flagged to the
//     schema owner in ui/NEXT-STEPS.md to move placement here (D5/D6 safe).
```
What is wrong: The dual render is real: PerformanceScreen renders both cache entries under section "resources" (Screens.tsx:273-275, area "performance" — a live registry area), and CodecsArea renders the same two entries via SettingField (codecs.tsx:105-106). The same setting key therefore appears as two editable controls on two different settings screens (consistent state — both bind useSettings — but duplicated UI). Worse, the claimed tracking artifact does not exist: rg over ui/NEXT-STEPS.md for `cache-demux|cache-network|placement|codecs|D5|D6` returns ZERO hits — no D5/D6 entry, no schema-owner flag.
Why it matters: User-visible duplication today, and the disclosure points at a remediation record that isn't there — the follow-up is untracked, so the "move placement here" fix has no owner-visible anchor. A header honesty claim contradicted by the repo is exactly the class this audit series flags.
What correct looks like: Either the NEXT-STEPS.md D5/D6 entry the header cites, or de-duplicate now (drop the codecs copies or exclude the two keys from the generated performance screen until the schema moves).
Confidence: HIGH — Verification: read of both render sites + rg census of NEXT-STEPS.md (file exists, zero matching lines).
### F-002 [S4]
Where: ui/src/screens/settings/areas/codecs.tsx:75-78
Code:
```
            onChange={() => {
              /* inert by design: the F2-gated policy cannot be changed yet */
            }}
            disabled={disabled}
```
What is wrong: `disabled` is derived from `entry.featureExposed` (a generated constant, false today), but the onChange handler is empty UNCONDITIONALLY — not gated on `disabled`. When Phase 7 lands and exposure flips (product.toml F2 → regenerated schema sets featureExposed true), the control renders enabled, opens its listbox, fires onChange… which does nothing. The header's charter — "never a dead control that looks alive" — is enforced only by this comment; no test covers the enabled path (all five control tests mount it in the F2-gated state).
Why it matters: The documented exposure step is a config flip + schema regen; nothing in this file or its tests forces the handler to be implemented at that moment. Latent today (featureExposed=false is regen-gated by settings:check byte-compare), live the day the flag flips.
What correct looks like: Tie inertness to the gate at runtime (`onChange={disabled ? noop : setSetting}`-style), or add a test that fails when featureExposed flips without a real handler (e.g. assert onChange writes through when not disabled, seeded by a test fixture entry).
Confidence: HIGH on the mechanism (read + test read), latent by construction.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useAreaMsg (makeAreaMsg result) | 39 | REAL | — | 3 lane-key files merged; useMsg-first fallback (shared.tsx:42-49) |
| HwDecodePolicyControl | 47-91 | REAL | S4 | Schema-bound enum Dropdown; honest-disabled pattern fully realized; F-002 inert-onchange trap; `String(value(entry.key))` verified via test :93 (default "software-only" shown) |
| CodecsArea | 94-110 | REAL | S3 | Two sections (decode/cache); cache rows duplicate the performance screen (F-001); no codec-family rows (honest absence, header :20) |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "is folded under /settings/video behind an in-flow disclosure" (:41) | YES | Real routed SettingsScreen; asserts redirect target + aria-expanded transition + codecs rows only after expansion |
| "renders the cache controls bound to their real schema keys" (:56) | YES | data-setting hooks keyed off the imported schema constants; also pins the retired "Advanced" badge absence |
| "binds the hw-decode control … full schema enumeration" (:69) | YES | aria-haspopup listbox + disabled trigger against the real Dropdown |
| "schema truth: gated on hw-decode, NOT exposed" (:79) | YES | Pins the generated constants — flips with any schema regen (would catch an accidental exposure) |
| "renders disabled with the honest reason" (:84) | YES | gate-reason attr + ADR-001/F2 text + default label "Software only" |
| "stays inert … while disabled" (:96) | YES | Click on disabled trigger → no listbox, no change (covers the CURRENT state only — not the F-002 enabled path) |
| "shows the shipped product default" (:104) | YES | defaultValue + rendered label |
| "renders no codec-family options the schema does not define" (:110) | YES | Cross-checks every rendered data-setting against SETTINGS_ENTRIES — real config-never-invents guard |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "hw-decode policy … ADR-001 is ACCEPTED … product.toml keeps features.hw-decode = false under rule F2" | :5-8 | REAL | product.toml:168 verbatim + F2 rule :186-189; schema.ts:976 featureGate/featureExposed |
| "the control renders VISIBLE, BOUND to the schema key, and DISABLED WITH THE HONEST REASON … never a dead control that looks alive" | :8-11 | REAL today / LATENT-BROKEN | Tests :69-102; but F-002: guarantee is comment-only once exposure flips |
| "their schema placement currently also renders them on the generated performance screen" | :17-18 | REAL | Screens.tsx:273-275 |
| "flagged to the schema owner in ui/NEXT-STEPS.md to move placement here (D5/D6 safe)" | :18-19 | FALSE | NEXT-STEPS.md exists; rg cache-demux/cache-network/placement/codecs/D5/D6 → zero hits |
| "the control is the settings-row grammar's CLOSED compact Dropdown (320×36), never the retired always-expanded Values-1 radio stack" | :12-14 | REAL | Dropdown closed grammar (aria-haspopup listbox, Dropdown.tsx:367); test :73; density="compact" passed (:68) |
| "Codec family toggles / decoder picking: NO schema entries → NOTHING" | :20 | REAL | No such rows; config-never-invents test :110-123 enforces it |
## 7. Dead / duplicated / unreachable code
- F-001: the codecs copies of the two cache rows duplicate the generated performance screen's rows (same schema entries rendered in two areas).
- No dead exports: CodecsArea + HwDecodePolicyControl both consumed (video.tsx:167 / codecs.tsx:100) and directly tested.
## 8. Dependencies & flags
Imports: Dropdown (components/menu), MessageKey + 3 lane-key files (i18n/keys/w26.b01|b02|f01), SettingField, generated schema constants, useSettings, makeAreaMsg. Generated layer drift-gated by settings:check (baseline row 24 PASS). Feature gate: hw-decode (product.toml F2, Phase 7 / cards 7.B). No network/bridge calls in this file — pure settings-state binding (W11 settings-persistence pin applies upstream in SettingsProvider, not here).
## 9. Open questions for the next agent
- Who owns NEXT-STEPS.md — F-001's missing D5/D6 entry should be recorded by the schema owner; if a wave owns ui/NEXT-STEPS.md, cross-count the broken reference.
- areas/video.tsx (CodecsFold host) and the performance area tests: if the schema placement ever moves, the performance-screen test expectations change with it — the two sides are coupled through schemas/settings.toml, not this file.
