# AUDIT: ui/src/screens/settings/SettingsProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 159 / 5 (SettingsProvider, useSettings, defaultsFor + context/type) |
| File verdict | REAL (correct, well-tested state machine; the end-to-end persistence verdict is PAPER-ONLY in the only real host — see F-001) |
| Reachable from shipped entry point? | NO binary at HEAD (ui-wide). SPA-internal: SettingsScreen.tsx:496-498 mounts it for /settings/:area; OnboardingScreen.tsx:767 for /onboarding. Executed under vitest (baseline run #23, 18 tests). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | Read complete; mount-site read SettingsScreen.tsx:449-498 (desktop shell → createConfigSettingsPersistence over config.set/get/reset-setting + subscribe feed; hydration gate); `rg -rn "config.set-setting" ui/src` (client bridge exists, config-settings.ts writeKey); W11 prior reconciled: audit/runs/W11-A02.log.md:27 — engine-side config.set-setting writes go to a dead in-memory BTreeMap, zero persistence, SettingApplied+restart_required returned for writes guaranteed lost; persistence.ts head read (localStorage seam, per-key value-type validation on load); test inventory: SettingsProvider.test.tsx 18 tests (state machine + engine feed). |

## 1. One-paragraph verdict
The provider itself is a genuinely correct draft/persisted/dirty/restart state machine — 18 honest tests cover every transition including the engine event feed and the dirty-draft-wins rule — and the desktop-shell mount genuinely wires Apply through the engine's config commands (config.set/get/reset-setting over the client bridge, with a hydration gate). The verdict that matters is end-to-end: W11-A02 proved the ENGINE side of that bridge writes into a dead in-memory BTreeMap with zero persistence — every successful Apply returns SettingApplied+restart_required for a write that is guaranteed lost on process exit. The net effect is an inversion: settings actually survive a restart only in the browser dev shell (localStorage), while the real desktop host loses them by design of the engine's config store.

## 2. What actually works (verified)
— State machine verified by tests (baseline run #23): clean start, override loading with unknown-key dropping, per-key dirty tracking, apply saving only non-default overrides, revert-to-APPLIED (not pre-edit), resetField/resetArea, restart-pending recording and clearing.
— Engine feed (subscribe) is real: config.setting-changed moves persisted + clean drafts, dirty drafts win, unknown keys and unrelated events ignored, unmount releases the feed (tests :129-207).
— Mount wiring verified: SettingsScreen.tsx:461-465 — in the desktop shell, persistence = createConfigSettingsPersistence(client, SETTINGS_ENTRIES) (config.* command bridge), with a hydration gate that never renders schema defaults over unread engine values (:480-492).
— localStorage seam (browser dev shell) does real per-key value-type validation on load (persistence.ts:19-34, anti-tamper).

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/settings/SettingsProvider.tsx:108-125 (apply) with the upstream dead store at the engine config layer
Code:
```
      apply() {
        const changed: SettingEntry[] = SETTINGS_ENTRIES.filter((e) => draft[e.key] !== persisted[e.key]);
        if (changed.length === 0) return;
        setPersisted((prev) => {
```
What is wrong: Not a bug in this file — a verdict on the chain this file terminates. `apply()` commits to the persistence seam; in the desktop shell that seam dispatches `config.set-setting`, and the engine accepts the write (SettingApplied + restart_required). But W11-A02 (audit/runs/W11-A02.log.md:27, config runtime.rs F-001 S2) proved the engine-side store is a dead in-memory BTreeMap: sole reader is config.get-setting, nothing ever persists it, the engine has no settings surface, and the `#[ignore]`d gap test admits it. So in the only real host: Apply "succeeds", the restart notice renders, and the value evaporates at process exit.
Why it matters: The user-visible settings contract ("persisted preferences") is false end-to-end in the real host — the W11 headline, confirmed here from the UI side. The browser dev shell's localStorage seam ironically persists better than the engine.
What correct looks like: The engine's config store gaining a real backing (file-backed preferences) — a config-lane fix, not a UI fix; until then the UI could badge the desktop Apply as session-only.
Confidence: HIGH — Verification: read of this file + mount site; engine-side fact cited from W11-A02's committed audit (not re-derived here).
(Severity S2 recorded in this file's ledger as the settings-state owner; the primary finding already exists in W11-A02's ledger — cross-referenced, not duplicated as new evidence.)

### F-002 [S4]
Where: ui/src/screens/settings/SettingsProvider.tsx:111-122
Code:
```
        setPersisted((prev) => {
          const next = { ...prev };
          for (const e of changed) next[e.key] = total(draft[e.key]);
```
What is wrong: `persistence.save(overrides)` (:121) executes INSIDE the setPersisted updater — a side effect in a function React requires to be pure. Under StrictMode (dev) updaters double-run, saving twice (idempotent here, so no data harm).
Why it matters: React contract violation with a lucky idempotency; a future non-idempotent seam (the config bridge's writeKey with pending counters — note config-settings.ts tracks `pending`) would misbehave under double-invocation.
What correct looks like: Compute `next` outside the updater, then `setPersisted(next)` + `persistence.save(...)` as separate statements.
Confidence: HIGH — Verification: read; config-settings.ts pending-counter read.

### F-003 [S4]
Where: ui/src/screens/settings/SettingsProvider.tsx:97, 105-107, 148
Code:
```
    const total = (v: SettingValue | undefined): SettingValue => v as SettingValue;
```
What is wrong: `setValue`/`value` accept any string key. For a key outside SETTINGS_ENTRIES, `value()` returns `undefined` cast to SettingValue (the seeding invariant covers schema keys only), and `setValue` mutates draft with a key that never appears in dirtyKeys (the filter iterates schema entries) yet makes `isDirty(key)` true.
Why it matters: Untyped surface on an otherwise total-by-construction store; no current caller passes bogus keys (call sites use schema constants), so latent.
What correct looks like: `key: SettingKey` branded type from the generated schema.
Confidence: HIGH — Verification: read + trace of the filter/updater paths.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SettingsContextValue` (type) | 16-38 | REAL | — | Full documented API. |
| `defaultsFor` | 42-46 | REAL | — | Schema defaults; O(entries). |
| `SettingsProvider` | 48-153 | REAL | F-001, F-002, F-003 | Lazy persisted init (unknown keys dropped); engine-feed effect (re-subscribes per persisted change — closure-correct); memoized context; apply/revert/resetField/resetArea/value. |
| `useSettings` | 155-159 | REAL | — | Throws outside provider. |
| (engine-feed effect) | 78-91 | REAL | — | Dirty-draft-wins rule; unknown keys ignored; cleanup returns unsubscribe. |

## 5. Test honesty (SettingsProvider.test.tsx, 18 tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "apply commits draft to persisted and saves only non-default overrides" (:68) | YES | Asserts the saved overrides object exactly. |
| "revert after apply restores the APPLIED value, not the pre-edit one" (:80) | YES | The classic revert bug, pinned. |
| "a dirty draft wins: the engine's move does not clobber the user's edit" (:167) | YES | Emits config.setting-changed with a dirty draft; asserts draft intact. |
| "records restart-required keys changed by apply" (:108) / "clears restart-pending…" (:119) | YES | Restart semantics pinned. |
All 18 behavior-level; the persistence seam is doubled (legitimately — it's the dependency). Passed in baseline run #23.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "persisted = schema defaults + user overrides from the persistence seam" | :10 | REAL (mechanically) / PAPER-ONLY (end-to-end) | Mechanism verified; engine-side store dead (W11-A02 F-001) → F-001. |
| "in the desktop shell, Apply round-trips the REAL engine config over the client bridge" | SettingsScreen.tsx:452-456 | REAL (dispatch) / FALSE (persistence) | Bridge verified (config-settings.ts writeKey → config.set-setting); engine drops the value (W11-A02). |
| "config-never-invents, doc 05 §6.4" (unknown keys dropped) | :60-61 | REAL | Loader loop + test :39. |

## 7. Dead / duplicated / unreachable code
None. All context fields consumed (SettingField/SettingWidgets/areas screens read draft/value/isDirty; apply bar reads dirtyKeys/restartPending).

## 8. Dependencies & flags
generated/schema (SETTINGS_ENTRIES — another agent), persistence.ts (localStorage seam), client/config-settings.ts (engine bridge), client/transport Event type. No feature flags; host selection via isDesktopShell() at the mount site.

## 9. Open questions for the next agent
- config-settings.ts owner (unassigned in my list): writeKey's onWriteResult failure surface — does the settings UI surface failed writes (feature-not-exposed in browser) to the user, or swallow them?
- Engine config lane (W16/W17?): the dead-store fix is the single unblocker for this entire chain; F-001 should be tracked there.
