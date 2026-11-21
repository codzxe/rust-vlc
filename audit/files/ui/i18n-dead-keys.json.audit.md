# AUDIT: ui/i18n-dead-keys.json
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 753 / 126 entries (1 `$comment` doc entry + 125 `{key, reason, owner}` grandfather entries) |
| File verdict | REAL (functioning, gate-enforced, re-verified end to end) |
| Reachable from shipped entry point? | N/A (config, not runtime code) — consumed by `scripts/l10n-check.mjs` rule 7 (`npm run l10n:check`, package.json:28); the 125 grandfathered keys are the dead 8.6% of the en-US catalog compiled into every web build via messages.ts. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `Read` the full JSON; python3 structural pass (125 key entries, 0 dupes, owner/reason distributions — 22 distinct reasons, all owner "w26.k18 (verified dead; Lead prunes the catalog entry)"); `Read` scripts/l10n-check.mjs (the consumer, rules 1–7, two-way dead-key gate :219-275); sample verification: 5 keys across the list (`screen.player.music`, `customize.tonemap.clip`, `settings.effects.hue`, `stream.back`, `player.controls.queue-items`) → all 5 in BOTH the MessageKey union and catalog object of messages.ts; `rg "player\.controls\.queue-items\|player\.tools\.open\|customize\.tile\.remove-hint"` over src/e2e/scripts/tools minus definition sites → zero hits (deadness sample); `rg -n '\$\{' src/i18n/keys/*.ts` → zero (no dynamic access in definition sites — blind-spot check, F-002); **`node scripts/l10n-check.mjs`** (direct node run of the read-only zero-dependency checker — not a build/test; deviation disclosed) → `PASS … 1448 MessageKeys, dead keys 125/1448 grandfathered`, EXIT=0 — **byte-identical counts to baseline `audit/runs/baseline/ui-l10n_check.log.txt`** (1448/1448, 125 dead, 6 allowlist, 13 placeholders). |

## 1. One-paragraph verdict
This file is what honest dead-code management looks like: a grandfather list where every entry carries a specific, differentiated reason (22 distinct reason texts — scaffold families superseded by named rewrites, lane files named as pruned), the consumer gate enforces BOTH directions (a new dead key fails; a grandfathered key that gains a reference or leaves the catalog fails), and the whole invariant re-verifies reproducibly — my direct run of the checker returns the exact same counts as the baseline run (1448 keys, 125 dead, 0 errors). Sampled claims check out: all 5 sampled keys exist in catalog and are referenced nowhere outside definition sites. Two hygiene-level truths remain: 8.6% of the shipped catalog is acknowledged dead weight whose prune ("the Lead's next catalog pass") has no owner or deadline — the gate stops growth but nothing forces the shrink — and the "verified dead" verdict rests on a scan that cannot see dynamic key access inside the i18n keys/ definition sites (safe today — zero template literals there — but unguarded tomorrow).

## 2. What actually works (verified)
— Verified by direct execution: `node scripts/l10n-check.mjs` → PASS, 125/1448 dead keys grandfathered, 0 errors — identical to baseline ui-l10n_check.log.txt (INDEX #24, 4/4 checks PASS). The two-way invariant holds at HEAD: every dead key is listed, every listed key is dead and in the catalog.
— Verified by python3 structural pass: 125 entries, no duplicate keys (the gate would fail duplicates — l10n-check.mjs:249-250), every entry carries non-empty key+reason (gate :241-248); the `$comment` entry is explicitly skipped by the consumer (:240).
— Verified by sample (5 keys spread across the list): union membership ✓, catalog string ✓, zero references in the scan universe (src/e2e/scripts/tools minus messages.ts and keys/) — the "verified dead" claim in `owner` is true on the sample, and the gate asserts it for all 125.
— Verified by read of the consumer: the reasons' supersession claims name their successors concretely (e.g. "live menu keys are exact literals", "lane copy pruned from w26.a14.ts by w26.k18") — the list documents WHY each family died, not just that it did.

## 3. Findings

### F-001 [S4]
Where: ui/i18n-dead-keys.json (the list as a whole; $comment :3 + all 125 entries)
Code:
```json
    "reason": "scaffold route-title keys, superseded by the per-screen title wiring of the W26 screens; messages.ts is Lead-owned — prune at the next catalog pass",
```
What is wrong: 125 of 1448 MessageKeys (8.6%) are admitted-dead catalog strings that still compile into the shipped bundle (messages.ts is `Readonly<Record<MessageKey, string>>` — the dead values ship). Every deferral ends in the same unowned promise: "prune at the next catalog pass" / "Lead prunes the catalog entry" — there is no deadline, no tracking entry, and the gate only prevents the list from growing (l10n-check.mjs:31-33 "the list must shrink, not grow moss" — nothing actually shrinks it).
Why it matters: catalog debt with an enforcement mechanism for accretion but none for retirement; 8.6% dead weight in every i18n bundle and a messages.ts the Lead must eventually prune by hand across 125 scattered entries.
What correct looks like: a tracked issue owning the prune (the $comment's "Prune entries together with the Lead's messages.ts prune" names the coordination but not the trigger), or per-entry expiry metadata the gate can escalate on.
Confidence: HIGH — Verification: counts from my gate run + baseline log; reasons read in full (python3 distribution).

### F-002 [S4]
Where: ui/i18n-dead-keys.json:3 ($comment "verified dead" semantics) + scripts/l10n-check.mjs:224-231, :169-170
Code:
```js
const KEYS_DIR = resolve(uiRoot, "src/i18n/keys") + "/";
const isDefinitionSite = (p) => DEFINITION_SITES.includes(p) || p.startsWith(KEYS_DIR);
```
What is wrong: the dead-key verdict is computed from a scan that excludes the i18n keys/ lane files as "definition sites", and the dynamic-family detection (`dynPrefixes`, :224-226) is built ONLY from scanned files. A grandfathered key that a lane file resolves dynamically (e.g. a future `keys[\`player.controls.${x}\`]` in a keys/ module or any unscanned file type — .json/.html/.css are also outside the scan) would be called dead while reachable at runtime — and the documented plan is deletion ("Lead prunes the catalog entry"), so the prune would remove live copy with the gate green throughout.
Why it matters: the hazard is latent, not active — `rg -n '\$\{' src/i18n/keys/*.ts` returns zero today, so no grandfathered key is currently resolvable through the blind spot (verified) — but the gate has no structural guard against the pattern appearing (a one-line "no dynamic access in definition sites" rule would close it).
What correct looks like: extend l10n-check with a rule failing on template-literal/computed access patterns inside src/i18n/keys/**, making the "verified dead" claim independent of where the access lives.
Confidence: HIGH (mechanism), verified N/A today (blind spot empty at HEAD).
Confidence: HIGH — Verification: read of the consumer + empty blind-spot scan.

## 4. Per-function (entry-group) table
| Entries | Count | Verdict | Severity | Notes |
|---|---|---|---|---|
| `$comment` doc entry | 1 | REAL | — | Accurately describes the gate's semantics (verified against the implementation, incl. the two-way stale rule at :267-275). |
| scaffold route-title keys (screen.*) | 10 | REAL (dead, verified) | F-001 | Sample `screen.player.music` dead ✓. |
| customize-panel family (j18 F4) | 26+1 | REAL (dead, verified) | F-001 | Sample `customize.tonemap.clip`, `customize.tile.remove-hint` dead ✓. |
| menu-bar scaffold ids | 16 | REAL (dead, verified) | F-001 | "live menu keys are exact literals" — consistent with the menu-labelKey architecture I audited in Dropdown/PlayerScreen lanes. |
| onboarding shortcut-hints + onboarding copy | 12+4+3 | REAL (dead, verified) | F-001 | — |
| hand-written effects-screen labels | 10 | REAL (dead, verified) | F-001 | Sample `settings.effects.hue` dead ✓ — consistent with my effects.tsx audit (generated schema screens replaced hand-written labels). |
| remaining families (settings/advanced/network-stream/controls/canvas-menu/seed-hint/player/stream) | 43 | REAL (dead, verified) | F-001 | Samples `stream.back`, `player.controls.queue-items`, `player.tools.open` dead ✓. |

## 5. Test honesty
The file's "test" is the gate itself: `npm run l10n:check` (baseline #24 PASS; my direct node re-run PASS with identical counts — the evidence is reproducible, the definition of a healthy data artifact). Mutation check (reasoned): delete one entry → gate FAILs with "new dead key"; add a bogus entry → FAILs with "left the catalog"; add a duplicate → FAILs "listed twice". The gate would catch every corruption class I can construct.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "A catalog key unreferenced by any ui/src\|e2e\|scripts\|tools .ts/.tsx/.mjs/.js file (outside messages.ts and keys/) is rot and FAILS l10n:check unless listed here" ($comment) | this file | REAL | l10n-check.mjs:219-264 implements exactly this; PASS run confirms. |
| "Entries whose key leaves the catalog or gains a reference also FAIL (stale)" ($comment) | this file | REAL | l10n-check.mjs:267-275. |
| "Dynamic families (prefix.${...} template keys) are treated as used by prefix" ($comment) | this file | REAL (with F-002's blind spot) | :224-231; prefixes detected only in scanned files. |
| "verified dead" (owner field, all 125) | this file | REAL | 5-key sample + gate assertion + empty definition-site template scan. |
| "Prune entries together with the Lead's messages.ts prune" ($comment) | this file | REAL-UNPROVEN | No tracked prune exists that I can verify (no git history under this session's deviation); 125 entries await it (F-001). |

## 7. Dead / duplicated / unreachable code
- The file IS the registry of dead code; no duplication (gate-enforced); no unreachable entries (two-way gate).

## 8. Dependencies & flags
- Consumer: scripts/l10n-check.mjs (zero-dependency node script); package.json:28 `l10n:check`; CI proposal noted at l10n-check.mjs:40 (.github/workflows/ui.yml — whether the workflow actually runs it is that lane's audit question, not verifiable as executed in baseline beyond run #24).

## 9. Open questions for the next agent
- Gate weakness in the SAFE direction: keys referenced ONLY from e2e/tests count as "used" (e2e is in the scan universe) — production-dead keys kept alive by test literals are invisible to this list. A cross-check of test-only-referenced keys would quantify that class.
- If the Lead's catalog prune ever lands, the expected end-state is this file shrinking to the `$comment` entry — a future audit should flag a list that has not shrunk across waves (F-001's debt metric).
