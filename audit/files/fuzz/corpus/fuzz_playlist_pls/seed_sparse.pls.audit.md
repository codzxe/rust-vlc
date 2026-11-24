# AUDIT: fuzz/corpus/fuzz_playlist_pls/seed_sparse.pls
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 118 / 4b71b36ed97fc025 |
| File verdict | REAL (hand-authored hostile seed) |
| Reachable? | YES — cargo-fuzz corpus convention; presence enforced by fuzz_registry_check.sh:64 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Hand-authored PLS seed (fuzz/README.md:106-107 "hand-authored
text/cookie/skeleton seeds" class). Content verified by read: 9 lines with
deliberate adversarial shape — leading `;` comment outside the header, a
numeric GAP (file1 then File5 — entries 2-4 missing), mixed key casing
(`file1` vs `File5` vs `TITLE5`), a `#` comment INSIDE the section, and a
declared `NumberOfEntries=2` that matches the sparse entries (5 claimed
positions, 2 actual). This exercises the parser's sparse-index and
case-sensitivity handling.
## 2. Verified
— Read (9 lines, 118 B); sha256 4b71b36ed97fc025….
— Consumer: fuzz/Cargo.toml bin fuzz_playlist_pls (3-seed corpus: basic,
dirty, sparse); the playlists crate's PLS parser is the target
(fuzz/README.md:85).
## 3. Findings
None.
