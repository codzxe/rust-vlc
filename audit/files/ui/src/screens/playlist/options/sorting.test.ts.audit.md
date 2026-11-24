# AUDIT: ui/src/screens/playlist/options/sorting.test.ts
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A10) |
| Date | 2026-06-14 |
| Lines / functions | 164 / 2 helpers + 6 describe blocks (22 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:101 `✓ src/screens/playlist/options/sorting.test.ts (22 tests) 9ms`; subject sorting.ts (198 L) ships via the playlist options menus |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `grep -n sorting.test audit/runs/baseline/ui-test.txt` -> :101; `wc -l` sorting.ts -> 198 L; read-through of all 22 tests |

## 1. One-paragraph verdict
Pure-model suite with genuinely hostile cases: natural sort (Episode 2 < Episode 10, never lexical), case-fold placement ("episode 2" between "Episode 2" and "Episode 10"), title-null falling back to source basename, metadata-absent sorts as stable no-ops (honest unknowns), numeric track/disc ordering (2 before 10), unknowns-last for artist/album/disc, tie stability, non-mutation, and clock formatting edge cases (null, negative -> null). One honesty defect: a test named "every key maps to its lane i18n key" asserts exactly one mapping.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:101 (22/22 pass; lane 1705 passed :178-179). Mutation reasoning: a lexical comparator fails :52; a metadata-fabricating comparator fails :70-77 (no-op assertions); a mutating sort fails :133-137; a non-stable sort fails :139-142.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/options/sorting.test.ts:43-45
Code:
```
  it("every key maps to its lane i18n key", () => {
    expect(sortLabelKey("title-asc")).toBe("playlist.options.sort.title-asc");
  });
```
What is wrong: the name promises all 16 SORT_KEYS map to their lane keys; the body checks a single key — 15 mappings unverified, so a broken label map for "artist-asc" through "disc-number-desc" passes silently and the options menu renders untranslated keys. Why it matters: names promising more than the body is the exact test-dishonesty pattern the audit exists to catch; the pin is trivially cheap to complete (`for (const k of SORT_KEYS) expect(sortLabelKey(k)).toBe(...)`). Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| entry helper + ENTRIES | 17-29 | REAL | — | covers null title, case variant, basename fallback |
| SORT_KEYS suite | 31-46 | REAL | S4 | F-001: 16-length + order pins good; label-map pin single |
| natural order suite | 48-66 | REAL | — | natural asc/desc + uri natural both ways |
| honest unknowns suite | 68-84 | REAL | — | no-op stability for meta-free keys |
| metadata seam suite | 86-130 | REAL | — | 8 tests incl. unknowns-last both directions |
| contract details suite | 132-143 | REAL | — | non-mutation + total tie stability |
| helpers suite | 145-164 | REAL | — | clock zero-padding, null/negative, display-title fallbacks, sortUsesMeta |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| natural sort :49-53 | YES | exact array equality incl. basename fallback |
| no-op unknowns :69-83 | YES | exact id order |
| seam sorts :95-129 | YES | exact orders with unknown-last pinned |
| label map :43-45 | PARTIALLY | one key only (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 22 tests run | baseline ui-test.txt:101 | REAL | `✓ ... (22 tests) 9ms` |

## 7. Dead / duplicated / unreachable code
None. formatDurationClock/entryDisplayTitle/sortUsesMeta all covered.

## 8. Dependencies & flags
Imports resolve against sorting.ts (198 L) and PlaylistProvider types.

## 9. Open questions for the next agent
None.
