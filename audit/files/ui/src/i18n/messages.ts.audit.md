# AUDIT: ui/src/i18n/messages.ts
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 3017 / 5 (union + catalog are data: 1448 keys each) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — useMsg() imported by 84+ ui/src files (rg -l useMsg ui/src, excluding i18n/) -> screens -> App.tsx -> main.tsx (web-ui build PASS in baseline rc-smoke) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | `node ui/scripts/l10n-check.mjs` (exit 0, PASS line pasted below); `rg -l useMsg|isMessageKey ui/src`; python3 key-consumption analysis; `sed -n` quote verification |

## 1. One-paragraph verdict
This is the hand-written en-US message catalog: a 1448-key `MessageKey` union
(L17-1496), a matching `Record<MessageKey,string>` catalog (L1498-2976), and a
5-function runtime (pseudo transform, locale switch, key guard, `useMsg`). The
mechanism is real and really rendered — `useMsg()` is the single text source for
the whole UI and `l10n:check` passes on the actual file. It is also 100%
frontend-local: no transport command fetches locale packs; the Rust
crates/localization catalog is not wired to this UI at all (header admits "the
real generated locale packs arrive with the i18n pipeline" — they have not
arrived). 125 of 1448 keys (8.6%) are dead weight, grandfathered with "prune at
the next catalog pass" reasons that never executed.

## 2. What actually works (verified)
— `node ui/scripts/l10n-check.mjs` -> exit 0:
`PASS l10n:check — 1448 MessageKeys, 1448 en-US strings: missing 0, duplicates 0, orphans 0, key-grammar ok, placeholders 13 ok, allowlist 6 entries (0 stale), dead keys 125/1448 grandfathered`
— Union keys == catalog keys: python3 set comparison -> True (1448/1448).
— Consumption: 1323/1448 keys referenced by scanned ui/src|e2e|scripts|tools code
(l10n-check rule 7 scan universe; 125 dead are grandfathered in ui/i18n-dead-keys.json).

## 3. Findings
F-001 [S3]
Where: ui/src/i18n/messages.ts:17-1496 (union) + ui/i18n-dead-keys.json
Code: (sample dead key from the union, verified unreferenced)
```
  | "screen.player.music"
  | "screen.miniplayer"
  | "screen.playlist"
```
What is wrong: 125/1448 catalog keys are referenced by NO scanned source file. The
grandfather file's own reasons say "prune at the next catalog pass" — the prune
never happened; the two-way gate froze the rot instead of removing it.
Why it matters: 8.6% of the shipped catalog is dead payload compiled into the
bundle; it also inflates the "1448 keys" integrity number cited as a quality signal.
What correct looks like: the promised prune pass deleting the 125 entries.
Confidence: HIGH — Verification: ran (l10n-check output above + python3 scan).

F-002 [S3]
Where: ui/src/i18n/messages.ts:2989-2995
Code:
```
let active: LocaleTag = "en-US";

/** Select the active catalog ("en-US" | "pseudo"). Unknown tags fall back to
 * the reference catalog — selection can never crash rendering (doc 07 §6.3). */
export function setActiveLocale(tag: string): void {
  active = tag === PSEUDO_LOCALE_TAG ? PSEUDO_LOCALE_TAG : "en-US";
```
What is wrong: the only locale selection that exists is en-US and the `pseudo`
debug locale. There is no user-facing language switch, and `active` is bare
module state with no subscription — a mid-session `setActiveLocale` would not
re-render anything (only caller is main.tsx:51, pre-render). The settings key
`settings.global.language` is itself in the dead-key grandfather list ("relabeled
away").
Why it matters: locale support is exactly one locale; any "language" setting UI
cannot be wired to this file.
Confidence: HIGH — Verification: `rg setActiveLocale ui` -> only main.tsx:23,51.

F-003 [S3]
Where: ui/src/i18n/messages.ts:1-13 (header) vs ui/src/client/* (transport)
Code:
```
// The real generated locale packs arrive with the i18n pipeline (doc 07); this
// hand-written table is the scaffold stand-in and
// every entry honestly labels its screen as a placeholder.
```
What is wrong: master-question answer for i18n — the UI text pipeline is wired to
no backend. `rg -n "locale|translat|i18n|lang" ui/src/client/*.ts` finds zero
translation commands in the transport layer; the UI never calls
crates/localization. The "generated locale packs" remain future work.
Why it matters: the Rust localization crate and the UI catalog can drift with no
gate comparing them (l10n-check itself notes "the Rust-side crates/localization
audit owns the cross-catalog comparison").
Confidence: HIGH — Verification: read + rg (client dir listing + grep output above).
Severity note: kept S3 not S2 because en-US rendering is genuinely functional;
this is an unwired seam, not a broken user-visible path.

F-004 [S4]
Where: ui/src/i18n/messages.ts:6-7
Code:
```
// this hand-written table is the scaffold stand-in and
// every entry honestly labels its screen as a placeholder.
```
What is wrong: only 45 of 1448 catalog entries contain the word "placeholder"
(`grep -c placeholder` -> 45). The file stopped being an "honestly labeled
scaffold" around wave 26; the header now misdescribes 97% of the file.
Confidence: HIGH — Verification: ran grep.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MessageKey union (type) | 17-1496 | REAL | — | 1448 keys; duplicates 0 (verified by l10n:check) |
| messages (catalog const) | 1498-2976 | REAL | — | 1448 entries, set-equal to union (python3 check); 125 dead (F-001) |
| pseudoCatalog (const) | 2985-2987 | REAL | — | eager toPseudo() over 1448 strings at module load; only used when active=="pseudo" |
| setActiveLocale | 2993-2995 | REAL | S3 (F-002) | 2-value switch; no reactivity; only debug caller |
| activeLocale | 2998-3000 | REAL | — | trivial getter; callers in tests/harness |
| isMessageKey | 3005-3007 | REAL | — | hasOwnProperty guard; used for config label_key validation (doc 07 §7.2) |
| useMsg | 3011-3017 | REAL | — | closure over active; `catalog[key] ?? messages[key]` fallback is dead-in-practice (type forbids missing keys) but harmless |

## 5. Test honesty
ui/src/i18n/messages.test.ts + pseudo.render.test.tsx exist (other agents' scope
if assigned); l10n:check (ui/scripts/l10n-check.mjs) is the real gate and I ran
it: PASS. It would fail on a genuinely missing/duplicate/orphan/dead key —
mutation reasoning: deleting a catalog entry breaks rule 1 (missing); adding an
unreferenced key breaks rule 7 unless grandfathered. Honest gate.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every entry honestly labels its screen as a placeholder" | messages.ts:6-7 | FALSE for current content | 45/1448 entries say placeholder (grep) |
| "real generated locale packs arrive with the i18n pipeline" | messages.ts:5-6 | PAPER-ONLY | no locale command in ui/src/client (rg) |
| "Integrity ... gated by npm run l10n:check" | messages.ts:11-13 | REAL | ran it: exit 0 PASS |

## 7. Dead / duplicated / unreachable code
125 dead catalog keys (F-001) — full list with reasons in ui/i18n-dead-keys.json
(not reproduced here; verified count 125 by python3). The per-lane registration
files ui/src/i18n/keys/w26.*.ts duplicate a subset of catalog entries by design
(Lead merges then leaves the lane file); they are definition sites excluded from
usage scans — see individual key-file audits later in this run.

## 8. Dependencies & flags
Imports only ./pseudo (PSEUDO_LOCALE_TAG, toPseudo). No feature gates. No
backend imports — pure frontend module.

## 9. Open questions for the next agent
- Does crates/localization contain the same 1448 keys, and does any shipped
  entry point (lumen-cli / desktop) ever serve its catalogs? (localization-crate
  agent's scope; UI side confirmed unwired.)
- Who consumes ui/i18n-allowlist.json's 6 entries and are their texts really
  exempt-worthy? (l10n-check rule 6b verified 0 stale.)
