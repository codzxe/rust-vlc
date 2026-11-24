# AUDIT: ui/src/screens/settings/persistence.test.ts
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| Lines / functions | 130 / 2 helpers + 3 describe blocks (13 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:121 `✓ src/screens/settings/persistence.test.ts (13 tests) 9ms`; subject persistence.ts (88 L) is the settings-override store behind the shipped settings screens |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `grep -n persistence.test audit/runs/baseline/ui-test.txt` -> :121; `wc -l` persistence.ts -> 88 L; read-through of all 13 tests |

## 1. One-paragraph verdict
Direct unit coverage of exactly the branches the header says the shell tests miss: corrupt JSON, non-object stores (array/null/scalar), unknown-key dropping, the tampered `{"interface.keep-above": "yes"}` repro (known key, wrong type — dropped so the schema default wins), enum membership validation both directions, list<string> element typing, int-vs-string mismatch, save round-trip, and quota-refusing storage (DOMException from a spied setItem, asserting no crash). The memory double is labeled and pinned as a mounting-only seam. Tests are parameterized against the REAL SETTINGS_ENTRIES schema, not a fixture schema.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:121 (13/13 pass; lane 1705 passed :178-179). Mutation reasoning: removing per-key type validation fails :65-70 (the documented regression); dropping unknown keys' removal fails :51-56; a crashing save on quota errors fails :119.

## 3. Findings
None. Checked for: tautologies (none — each load() expectation is a distinct object shape), mocks of the subject (none — only Storage.prototype is spied for the quota branch, which is boundary, not subject), unwired tests (13/13 run), copied blocks (enum/list/int cases each target different type machinery). Zero findings is the honest result.

## 4. Per-function table (file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| keyOf / seed helpers | 20-30 | REAL | — | seed writes real localStorage; afterEach clears + restores mocks |
| load suite | 32-105 | REAL | — | 10 tests: missing/corrupt/non-object, unknown-key drop, bool/enum/list/int validation |
| save suite | 107-122 | REAL | — | JSON round-trip; quota DOMException swallowed without crash, spy verified called |
| memory double suite | 124-130 | REAL | — | load {} / save no-op / stays {} |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tampered-store repro :65-70 | YES | expects {} (drop), was the documented silent-true bug |
| enum membership :72-85 | YES | both directions pinned with real schema entries |
| quota swallow :115-121 | YES | throwing spy + not.toThrow + called |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 13 tests run | baseline ui-test.txt:121 | REAL | `✓ ... (13 tests) 9ms` |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
SETTINGS_ENTRIES/SettingValue imported from the real generated schema (used for enum/int/list lookups — schema-dependent, will break loudly if schema changes shape, which is correct).

## 9. Open questions for the next agent
None.
