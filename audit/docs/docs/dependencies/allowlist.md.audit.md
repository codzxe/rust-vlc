# CLAIM AUDIT: docs/dependencies/allowlist.md
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Claims extracted | 12 |
| REAL 6 · REAL-UNPROVEN 2 · SHALLOW 1 · FALSE-on-current-tree 3 · UNVERIFIED 1 |
| Internal contradictions found | 1 (row version-ranges vs lock's duplicate versions) |

## 1. Blunt verdict
The Phase-1 allowlist is honest about its own gaps ("row count below is never misread as lockfile-complete", §7 audit queue) and everything it does state that is checkable offline checks out exactly (ureq dual-lock pin, tauri sublock versions, libloading ban scoping, deny.toml wiring, CI job). But as an approval record for the CURRENT tree it covers 29 of 119 registry crates (~24%): the header's "the lock now resolves 39 packages: 28 registry + 11 workspace-local" is 4x stale (now 119 registry + 39 local), and the lock contains second copies of hashbrown (0.15.5) and syn (2.0.119) that no row covers and that violate their rows' version ranges. The drift is self-flagged in direction but not in magnitude.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "the lock now resolves 39 packages: 28 registry + 11 workspace-local" | :6 | FALSE-on-current-tree (REAL 2026-10-20) | `grep -c '^name = ' Cargo.lock` = 158; registry = 119, local = 39 — registry drift 28→119 (4.25x) |
| 2 | "Enforcement: `deny.toml` + the `cargo-deny` CI job (G3/G4)" | :7 | REAL-UNPROVEN | deny.toml:85-88 allow set (MIT, Apache-2.0, Unicode-3.0, Unlicense); .github/workflows/ci.yml:490-525 cargo-deny job with final `cargo deny --workspace check licenses sources bans advisories` (:525) — but CI billing-blocked (phase7-exit.md §6), so enforcement is nominal, local-only |
| 3 | ureq "locked 3.5.0 in both the main and `fuzz/` lockfiles" | :68 | REAL | `grep -A1 '^name = "ureq"' Cargo.lock` -> 3.5.0; same in fuzz/Cargo.lock -> 3.5.0; crates/streaming/Cargo.toml:52 `ureq = { version = "=3.5.0", default-features = false, features = ["rustls", "rustls-webpki-roots"] }` — matches the row's "first active feature pin" exactly |
| 4 | tauri 2.12.5 / tauri-runtime-wry 2.12.3 / tao 0.37.2 / libloading 0.7.4 in the subcrate lock | :125-129 | REAL | apps/desktop-tauri/Cargo.lock entries verified: tauri 2.12.5, tauri-runtime-wry 2.12.3, tao 0.37.2, libloading 0.7.4 (wry present at 0.57.2, riding tauri-runtime-wry — row cites it "via", consistent) |
| 5 | libloading "BANNED in the main graph ... only through the INACTIVE `tray-icon` optional edge" | :129 | REAL | root Cargo.lock has ZERO libloading entries (`grep -c` -> 0); deny.toml:189 `{ name = "libloading", wrappers = ["libappindicator-sys"] }` — wrapper-scoped exactly as the row says |
| 6 | "`apps/desktop-tauri` is workspace-EXCLUDED (root Cargo.toml `exclude`)" | :116-117 | REAL | root Cargo.toml:51 `exclude = ["apps/desktop-tauri"]` |
| 7 | "9 path packages (`codec-api`, `localization`, ... `xtask`) are `publish = false`" | :136-138 | FALSE-on-current-tree (snapshot) | lock now has 39 local packages (command above); workspace membership grew ~4x since the snapshot |
| 8 | deny.toml license set "verified to be exactly what the lockfile consumes" | :169-171 | SHALLOW (snapshot-true) | true for the Phase-1 stack (rows' licenses match deny.toml:85-88); today's 119-crate lock adds codec-era crates whose license mix is untested here — cargo-deny green was re-proven 2026-11-10 (phase7-exit.md #15) but not since the lock grew; UNVERIFIED for current lock |
| 9 | row versions "locked" column vs Cargo.lock | :63-91 | REAL for 27/29 rows | spot-check of all 29 row crates: 27 exact matches (serde 1.0.229, serde_json 1.0.154, toml 1.1.8+spec-1.1.0, ..., zmij 1.0.23); EXCEPT hashbrown and syn — see contradiction #1 |
| 10 | "cargo test --workspace --all-features → 370 passed / 0 failed" | :213 | FALSE-on-current-tree (Phase-1 snapshot) | current baseline: 4744 passed / 2 failed — 370/0 was true only of the tiny Phase-1 workspace |
| 11 | "The CI green-run link is PENDING first push ... `git remote -v` is empty" | :216-220 | REAL (honest limitation) | corroborated by phase7-exit.md §6 external blockers: "CI Actions billing (every run dies at 0 steps)" — the enforcement story has been local-only since Phase 1 |
| 12 | "phase-4–6 codec admissions ... predate this row without §3 rows" (drift self-flag) | :55-59 | REAL (direction) / understated (magnitude) | rows cover 29/119 registry crates (24%); un-rowed includes the codec crates' own trees (syn 2.0.119, hashbrown 0.15.5 duplicates) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| hashbrown row: "locked 0.17.1, range >=0.17.1, <0.18.0" (L71) | root Cargo.lock contains hashbrown 0.15.5 AND 0.17.1 | lock (grep -A1 shows both) — the 0.15.5 copy has no row and violates the row's range; same pattern for syn (row "3.0.9, >=3.0.9" vs lock's 2.0.119 + 3.0.9) |
| "the lock now resolves 39 packages" (L6) | lock resolves 158 (119 registry) | lock — header staleness, 4x |

## 4. Fabrication / inflation indicators
None. The doc's quick-pass rows are explicitly marked `[VERIFY]` (not claimed as audited), the ureq row carries its full record chain, and the negative-fixture evidence (deny ban tripwires, sha-pin tamper test) is described as local-run with the CI link honestly pending. The one structural sin is omission-by-staleness: 90 registry crates with no approval row, softly flagged as codec-admission drift.

## 5. Matrix rows
Not a matrix chunk. Related cross-check: feature-matrix.toml H.264 row cites the h264 record chain (audited separately); no contradiction with this file's ureq/tauri rows.
