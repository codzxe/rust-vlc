# AUDIT: crates/playlists/tests/fixtures/dirty.pls
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Type / size | `file` → "PLS playlist, ASCII text"; 17 lines, LF endings |
| File verdict | REAL — every one of the 17 lines is damage deliberately placed and asserted by an exact-warning-list test that passes |
| Reachable from shipped entry point? | N/A (test fixture); the parser it exercises is library code, unwired (see pls.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `cat -A` (17 LF lines, no CR); ran `./target/debug/deps/pls_suite-3d474893475cc239` → `dirty_fixture_degrades_honestly ... ok` (14/14 suite pass, exit 0); sed on pls_suite.rs:129-162 |

## 1. Blunt verdict
Model dirty-world fixture: each line is a distinct damage class and the consuming test (`dirty_fixture_degrades_honestly`, pls_suite.rs:130) asserts the EXACT ordered warning list — 10 warnings with exact line numbers (BadIndex 2/3, BadLength 4/8, DuplicateKey 9, EmptyLocation 10, NotAKeyValuePair 11, UnexpectedVersion 17, OrphanMetadata 4, EntryCountMismatch 99 vs 3) — plus the surviving 3 entries with last-value-wins on the duplicate File2, the kept empty File3, and the INI-ignored File9 under `[other-section]`. I ran the suite: passes. Every line of the fixture is accounted for; nothing in it is dead weight.

## 2. Evidence
- Fixture lines map 1:1 to expectations: `File0`/`Filex` (bad indexes), `Length1=not-a-number`/`Length2=abc`, `Title1` orphan, duplicate `File2`, `File3=` empty, `garbage line without equals`, `[other-section]` + `File9` (ignored), re-opened `[playlist]`, `NumberOfEntries=99` (lying count), `Version=1`.
- Execution: pls_suite 14/14 pass (my run), including this test.
- The fixture also underpins the fuzz-lite mutation base? No — mutations use a valid PLS; this file is the deterministic damage oracle. Consistent.

## 3. Notes
No defects. One observation, not a finding: the fixture does not include my pls.rs F-002 edge (duplicate `FileN=` with an EMPTY value — where "last value wins" does not hold); adding a line 18 `File2=` would pin that semantic. The current test's duplicate uses a non-empty value, which is the case the code handles as documented.
