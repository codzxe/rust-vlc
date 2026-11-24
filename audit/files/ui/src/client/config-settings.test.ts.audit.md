# AUDIT: ui/src/client/config-settings.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 308 / 11 |
| File verdict | REAL — wire-level persistence tests at the transport seam; exact request/args/failure assertions |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:76 `✓ src/client/config-settings.test.ts (11 tests) 160ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg baseline; rg schema value types |

## 1. One-paragraph verdict
Honest seam testing of the settings write path: a ScriptedTransport records
every request and answers from a queue, and the tests assert the exact wire
sequence — hydrate reads per-key with only non-default engine values kept,
save writes ONLY changed keys (set for flips, reset for reverted overrides),
engine refusals and dead transports surface as `failed` write results with
the reason (never swallowed), `config.setting-changed` events fold so
engine-side moves are not rewritten, and dispose releases the subscription.
The pure fold (`applySettingChanged`) gets its own degenerate-input tests.

## 2. What actually works (verified)
— Baseline ui-test.txt:76: 11 tests, 160 ms, green.
— Mutation reasoning: if save() wrote all keys, L181's `setCalls).toHaveLength(1)`
fails; if the reset-on-revert regressed, L187 fails; if the error were
swallowed, L215-221 fail; if the fold ignored engine events, L253/258 fail.
All 11 fail on their named behavior.

## 3. Findings
F-001 [S4]
Where: ui/src/client/config-settings.test.ts:95
Code:
  type: typeof defaultValue === "boolean" ? "bool" : "string",
What is wrong: the `entry()` factory can only mint "bool"/"string" entries —
the real schema has 10 value types (schema.ts:9-20: int/float/enum/
list<string>/duration-ms/path/url/percent). No test covers int/float/enum/
path hydration or write — the numeric/typed-value paths of the persistence
layer are unexercised here.
Confidence: HIGH — Verification: read (schema union quoted).
F-002 [S4]
Where: :141-143 `const areas = transport.sent.map((r) => r.cmd === "config.get-setting" && r.args);` — a truthy-coerce map producing `args | false`; works via toContainEqual but is a sloppy shape (hygiene).
Confidence: HIGH — Verification: read.

## 4. Per-function table (describe blocks)
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| ScriptedTransport | 20-54 | REAL | records requests; rejects when unscripted (used for the dead-transport test) |
| splitSettingKey | 117-124 | REAL | degenerate shapes refused (null) |
| hydration | 126-161 | REAL | per-key reads; not-exposed host → defaults + one honest log |
| writes | 163-238 | REAL | write-only-changed; reset-on-revert; applied/failed results; unreachable |
| live feed | 240-270 | REAL | event fold; no-rewrite discipline; dispose hygiene |
| applySettingChanged | 272-308 | REAL | default-clears-override; unknown keys/unrelated events ignored |

## 5. Test honesty
All 11 tests assert exact wire shapes and would fail on their named
regressions (§2). Test names match bodies.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No webview is needed: the transport is scripted at the seam" | file L16-17 | REAL | no DOM usage in file |
| "the engine's config layer over the command transport" | file L11-12 | REAL | all cmds asserted are config.get/set/reset-setting |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
vitest vi.waitFor for async settles; generated/commands types imported as
types only. No network.

## 9. Open questions for the next agent
- Does generated.test.ts (also mine) cover the numeric/enum entries this file
  cannot mint? (F-001 follow-up.)
