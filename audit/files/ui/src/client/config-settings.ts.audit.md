# AUDIT: ui/src/client/config-settings.ts
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 301 / 5 top-level (splitSettingKey, createConfigSettingsPersistence [+ inner readKey/writeKey/resetKey/persistence], applySettingChanged) |
| File verdict | REAL (genuine backend write path over the command transport) |
| Reachable from shipped entry point? | YES (desktop lane): SettingsScreen.tsx:461 `isDesktopShell() ? createConfigSettingsPersistence(client, SETTINGS_ENTRIES)` -> client (ClientProvider) -> tauri-transport `lumen_dispatch` -> apps/desktop-tauri/src/app.rs:293 -> ShellHost -> apps/desktop/src/shell.rs (config.set-setting, :438). Browser lane: NOT used (localStorage fallback, persistence.ts:47). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "config.get-setting|config.set-setting" crates/ apps/` -> catalog command.rs:407-426 + desktop shell.rs:438; `rg -n "onWriteResult" ui/src -g '!config-settings.ts'` -> ONLY the test file; `sed -n` quote checks |

## 1. One-paragraph verdict
This is the strongest answer to the master question found in my file set:
the settings screen's Apply path really does round-trip the ENGINE's config
layer — `config.get-setting` hydration, `config.set-setting` /
`config.reset-setting` writes, `config.setting-changed` event fold — all
through the single `lumen_dispatch` IPC command. The diff-then-write
discipline (write only changed keys, reset reverted ones) is implemented
exactly as the header claims, and the test suite pins it. The defect: the
module's "never swallowed" failure contract is a lie in production — the
ONLY shipped call site passes no `onWriteResult`, so a failed write is
invisible, and the optimistic snapshot keeps the failed value, so it is
never retried either. The UI will show the setting as applied while the
engine kept the old value until the next hydrate.

## 2. What actually works (verified)
— `rg` call chain above (transport cmds exist in the Rust catalog AND the
desktop shell handles them). Tests (config-settings.test.ts, 12 cases incl.
"save writes ONLY changed keys and resets what reverted" :164, "a typed
engine refusal is reported as failed with the error code — never swallowed"
:209) run in the vitest baseline (1705 passed). Mutation reasoning: break
the diff -> :164 fails; break the reset branch -> :164 fails; break the
fold -> :272 fails.

## 3. Findings
F-001 [S2]
Where: ui/src/client/config-settings.ts:224-243 (call site SettingsScreen.tsx:461)
Code:
    save(next) {
      const changed: string[] = [];
      const reverted: string[] = [];
      for (const key of Object.keys(next)) {
        if (overrides[key] !== next[key]) changed.push(key);
      }
      ...
      overrides = { ...next };
      for (const key of changed) {
        const value = next[key];
        if (value !== undefined) writeKey(key, value);
      }
What is wrong: (a) `overrides` is replaced with `next` BEFORE the writes
settle; if a write fails (engine refusal or transport dead), the snapshot
already holds the failed value, and the next `save()` with the same desired
value sees `overrides[key] !== next[key]` as false -> the key is never
rewritten. Failed writes are permanently lost for that session. (b) The
header's contract "every write's outcome is reported through onWriteResult
... (never swallowed)" (:22-24, :59-60) is not honored by the shipped call
site: `rg onWriteResult ui/src -g '!config-settings.ts'` matches ONLY
config-settings.test.ts — SettingsScreen.tsx:461 passes no options, so the
user never learns the write failed. What correct looks like: reconcile the
snapshot from onWriteResult (revert failed keys, or re-queue them) and wire
onWriteResult to a toast/status line in SettingsScreen. Confidence: HIGH —
Verification: read + rg.
F-002 [S3]
Where: ui/src/client/config-settings.ts:231-233
Code:
      for (const key of Object.keys(next)) {
        if (overrides[key] !== next[key]) changed.push(key);
      }
What is wrong: reference equality on `SettingValue` — a mutated-in-place
string[] (same reference) is detected as unchanged and silently never
written. Safe only because callers rebuild arrays; the module itself does
not enforce that. Why it matters: a future caller holding a stable array
reference loses writes with zero signal. What correct looks like: value
equality (or document the fresh-reference precondition at the interface).
Confidence: MEDIUM — Verification: read.
F-003 [S4]
Where: ui/src/client/config-settings.ts:112-121
Code:
    } catch {
      return null; // transport unreachable — the honest unknown
    }
What is wrong: a dead IPC transport is indistinguishable from
"feature-not-exposed": hydrate() then loads schema defaults silently (one
console.debug line, :259-263) and the UI presents defaults as if they were
engine values. Acceptable for a dev shell; in the desktop build a broken
bridge renders settings that lie. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: ui/src/client/config-settings.ts:244-264
Code:
    async hydrate() {
      const results = await Promise.all(
        entries.map(async (entry) => [entry, await readKey(entry)] as const),
      );
What is wrong: save()-before-hydrate() ordering is guarded only by the
CALLER (SettingsScreen gates provider mount on `hydrated`); the module
itself would fold a premature save into a snapshot that hydrate() then
overwrites (`overrides = loaded`, :257). No internal guard. Confidence:
HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| splitSettingKey | 44-50 | REAL | — | rejects "", ".b", "a.", no-dot |
| toWireValue | 38-41 | REAL | — | copies arrays |
| createConfigSettingsPersistence | 80-276 | REAL | S2 | write path real; failure contract unwired (F-001) |
| — readKey | 100-122 | REAL | S4 | catch->null (F-003) |
| — writeKey | 125-170 | REAL | — | reports via onWriteResult (tests only) |
| — resetKey | 173-210 | REAL | — | symmetric, fine |
| — save | 224-243 | REAL | S2/S3 | optimistic snapshot, ref-eq diff (F-001/F-002) |
| — hydrate | 244-264 | REAL | S4 | no internal save-guard (F-004) |
| applySettingChanged | 285-301 | REAL | — | pure, test-pinned, unknown keys ignored |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| :164 save writes ONLY changed keys + resets reverted | YES | asserts exact transport call sequence |
| :191 write round-trips applied value | YES | asserts onWriteResult payload |
| :209 typed engine refusal reported as failed | YES | asserts error code surfaced |
| :224 unreachable transport is failed, not crash | YES | rejects the send |
| :241 setting-changed events fold into snapshot | YES | emits event, asserts next save skips folded key |
Mock is the transport seam — legitimate; the engine side is covered by the
Rust suites (other agents). What NO test covers: the production call site
without onWriteResult (F-001b).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every write's outcome is reported through onWriteResult" | :22-24 | FALSE (system level) | sole call site SettingsScreen.tsx:461 passes no options |
| "the browser dev shell keeps localStorage (the demo transport honestly answers feature-not-exposed for config.*)" | :5-7 | REAL-UNPROVEN | demo-transport not yet audited by me; behavior asserted in tests :148 |
| "an unchanged setting is never rewritten" | :11-12 | REAL | test :164 pins it |

## 7. Dead / duplicated / unreachable code
`ConfigWriteResult`/"applied" and `pendingWrites` are diagnostics consumed
only by tests — production ships them unwired (part of F-001). No dead
functions.

## 8. Dependencies & flags
Depends on ./transport types (CommandTransport.send/subscribe), settings
schema + persistence interface. No feature flags; desktop-shell gating is
runtime (`isDesktopShell()` = `__TAURI_INTERNALS__` presence, ClientProvider).

## 9. Open questions for the next agent
- apps/desktop/src/shell.rs:438 `unreachable!("config.set-setting must
  complete: {other:?}")` — if the engine can legitimately answer otherwise,
  this is a panic on the desktop bridge (S1 candidate, desktop auditor's
  call).
- Does the tauri transport's `send` reject on IPC error (writeKey's second
  callback) — verified in tauri-transport.test.ts:136 ("lumen_dispatch IPC
  failed") — good; cross-check the Rust error strings match.
