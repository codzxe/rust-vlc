# AUDIT: ui/i18n-allowlist.json
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / entries | 47 / 6 entries |
| File verdict | REAL |
| Reachable | Consumed by TWO gates: ui/tools/eslint-plugin-local/rules/i18n-no-literals.js:32-34 (load-time readFileSync; malformed array/entry THROWS) and ui/scripts/l10n-check.mjs:138-157+203-217 (rules 6/6b: shape with file/line context + staleness). l10n:check executed in baseline (INDEX.md row 24, 4/4 PASS). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | Read of the file; rg -F -c presence checks for every (text, scoped-file) pair — "—" ×28 in SettingsScreen.tsx (incl. JSX copy position :393), "(" ×200 / ")" in SettingsScreen + HotkeysEditor, "/" ×70 GotoTimeDialog + ×273 OnboardingScreen, "00:02:10" ×4 CustomizePanels (2 in JSX copy :692,:795), "00:25:40" ×4; duplicate-text scan (6 unique); cross-read of both consuming validators (my file-13 audit). |

## 1. One-paragraph verdict
The exemption list is exactly what an i18n honesty gate's escape hatch should be: six entries, every one punctuation or frame-pinned data — an em-dash separator, parenthesized unsaved-count punctuation (×2 files), a position/duration slash (×2 files), and two customize-band timecodes — with zero English prose exempted anywhere. Every entry carries the full v3 contract (text, non-empty files[], reason, owner); every scoped path exists; every text is LIVE in its scoped files (verified with rg presence counts, including the JSX copy positions the rule would otherwise flag: CustomizePanels.tsx:692/795 render `00:02:10` as band chrome, SettingsScreen.tsx:393 renders the em-dash between area and section names); the two timecode entries' reasons match my part2 CustomizePanels audit finding (decorative aria-hidden preview chrome, frame-pinned Current Time.svg/Duration.svg reads). Both consumers enforce the contract at gate time (the ESLint rule throws on a text-only entry at load; l10n-check reports shape errors with file/line context and fails on stale entries whose text vanished), and the executed evidence is baseline row 24. No defects; the one systemic gap (no duplicate-text detection across entries — Map.set last-wins in the rule) is filed against the RULE (my file-13 F-003, S4) and is latent against today's 6 unique texts.

## 2. What actually works (verified)
- Shape: 6/6 entries with text + non-empty files[] (all ui/-relative, forward-slashed, all paths exist) + substantive reason + owner.
- Liveness (l10n-check rule 6b's staleness check, verified statically): every text occurs in at least one of ITS OWN scoped files — counts above; the timecodes and em-dash verified in JSX COPY position, not just comments (the comments at :568/:687 alone would not exercise the rule).
- File-scoping honesty (v3): "00:02:10" is scoped to CustomizePanels.tsx ONLY — the rule's RuleTester pins that the same text FAILS in PlaylistScreen.tsx (i18n-no-literals.test.js:81-85, ran green in baseline row 23); no text is exempted repo-wide.
- No copy exemptions: all six texts are punctuation ("—", "(", ")", "/") or preformatted data ("00:02:10", "00:25:40") — nothing a translator would translate; the reasons say so and the reasons are accurate (the em-dash "same convention as product.toml title_template", the timecodes "decorative aria-hidden preview chrome", cross-verified by my CustomizePanels part2 audit).
- Dual enforcement real: rule-side throw (i18n-no-literals.js:44-53) + gate-side diagnostics (l10n-check.mjs:141-157) + staleness gate (6b, :203-217) — belt and suspenders, both green at HEAD (rows 23/24).
- Failure direction is loud: a malformed allowlist crashes the lint tool at import (ENOENT/throw) rather than silently exempting nothing/everything.

## 3. Findings
None. Zero-finding enumeration:
- Fakeness: none — pure data, no code.
- Correctness: all 12 (text, file) scoping pairs verified live by rg; 6 unique texts (no duplicate-text collision — the Map.set hazard in the rule is not triggered).
- Consistency: consumers agree on the schema (rule requires text+files, gate additionally requires reason+owner — the gate is the stricter; today's entries satisfy both).
- Hygiene: owners are card/wave identifiers (4.E1, W18-LL, w26.i05) per project convention; reasons explain WHY each text is not translatable copy.
## 4. Entry table
| Entry | Files | Live? | Reason accuracy |
|---|---|---|---|
| "—" | SettingsScreen.tsx | YES (JSX :393) | Area/section separator — matches reason |
| "(" / ")" | SettingsScreen.tsx, HotkeysEditor.tsx | YES | Count punctuation around numeric unsaved-changes data |
| "/" | GotoTimeDialog.tsx, OnboardingScreen.tsx | YES | Position/duration + shortcut-key separators between preformatted values |
| "00:02:10" | CustomizePanels.tsx | YES (JSX :692,:795) | Frame-pinned Current Time.svg band timecode, aria-hidden chrome — cross-verified (my file-5 audit) |
| "00:25:40" | CustomizePanels.tsx | YES (×4) | Duration.svg read — same class |
## 5. Test honesty
| Test | Would it fail if the data were broken? | Reason |
|---|---|---|
| i18n-no-literals RuleTester valid case (scoped "00:02:10") | YES | Would fail if the entry's text or file path drifted (the exemption must match exactly) |
| i18n-no-literals invalid case (same text, wrong file) | YES | Pins file-scoping; a v2-style global regression fails it |
| l10n:check rules 6/6b (baseline row 24) | YES | Shape errors + stale entries (text absent from ALL its files) fail the gate |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every entry is FILE-SCOPED since v3 … an entry without a non-empty files array is now a load-time config error" (rule header) | i18n-no-literals.js:18-22 | REAL | All 6 entries file-scoped; throw verified in the rule; test pins the scoping |
| "Punctuation separator … not translatable copy" (per-entry reasons) | entries 1-4 | REAL | JSX positions verified; texts are punctuation/data |
| "decorative aria-hidden preview chrome … the real control bar renders the live localized timecodes" | entries 5-6 | REAL | My CustomizePanels part2 audit (aria-hidden preview chrome; live timecodes in ControlBar) |
## 7. Dead / duplicated / unreachable code
- None. Every entry is load-bearing (each text would be a lint error without it — verified by the live JSX occurrences).
## 8. Dependencies & flags
Consumers: i18n-no-literals.js (rule), l10n-check.mjs (gate). No feature flags. Schema duplication between the two validators is intentional (different failure contexts) — documented at l10n-check.mjs:148-151.
## 9. Open questions for the next agent
- If CustomizePanels' band chrome ever renders LIVE timecodes instead of the frame-pinned literals, entries 5-6 go stale and l10n-check 6b will (correctly) fail — the gate self-cleans.
- The rule-side duplicate-text hole (my file-13 F-003) is the only structural gap touching this file's data model.
