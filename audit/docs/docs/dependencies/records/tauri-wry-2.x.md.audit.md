# CLAIM AUDIT: docs/dependencies/records/tauri-wry-2.x.md
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Claims extracted | 10 |
| REAL 7 · REAL-UNPROVEN 1 · SHALLOW 0 · STUB 0 · PAPER-ONLY 0 · FALSE 0 · UNREACHABLE 0 |
| Internal contradictions found | 0 (title-vs-admission version drift is self-annotated) |

## 1. Blunt verdict
The best-shaped record in this batch: a pre-audit that keeps its own "NOT imported" history, an admission addendum that fills exactly the [VERIFY-P1] fields it promised, and every tree-checkable assertion reproduces. The main workspace is verifiably tauri-free while the excluded subcrate carries the closure (493 locked packages) — the central containment claim is real. The remaining softness is environmental (tarball-level R1 scans and the "per-push" cargo-deny depend on CI, which is billing-blocked) and the title still reads "tauri 2.12.4 + wry 0.57.0" while the admission is 2.12.5/wry 0.57.2 — self-annotated but confusing on a fast skim.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "imported in the workspace-EXCLUDED subcrate `apps/desktop-tauri` (own lockfile) — the main workspace graph stays tauri-free" | :13-17 | REAL | root Cargo.toml:51 `exclude = ["apps/desktop-tauri"]`; `grep -c 'tauri\|wry\|tao' Cargo.lock` -> 0 (zero tauri/wry/tao entries in the main lock); apps/desktop-tauri/Cargo.lock exists with 493 packages |
| 2 | admission versions "tauri 2.12.5 / tauri-build 2.7.4 / tao 0.37.2" | :169 | REAL | apps/desktop-tauri/Cargo.lock: tauri 2.12.5, tauri-build 2.7.4, tao 0.37.2, tauri-runtime-wry 2.12.3, wry 0.57.2 (all verified by grep -A1) |
| 3 | "cargo-deny runs per-push on the subcrate lock (the `tauri-check` lane)" | :174-175, :181-182 | REAL (lane exists) / UNPROVEN (runs) | .github/workflows/ci.yml:680-747 `tauri-check` job incl. step "cargo-deny the admission crate (bans/licenses/advisories)" -> `cargo deny --manifest-path apps/desktop-tauri/Cargo.toml --exclude-dev check bans licenses advisories`; CI billing-blocked (phase7-exit.md §6) so "per-push" is nominal |
| 4 | R2 closure "adds tauri/tauri-build/wry/tao + gtk family + `webview2-com` (Windows target ... no libloading there)" | :170-174 | REAL | sublock grep: gtk/gtk-sys/gtk3-macros + 2 webkit2gtk*-sys + webview2-com present; libloading 0.7.4 present only via the tray-icon edge (deny.toml:189 wrapper scoping) |
| 5 | R1 "the tauri closure carries NO `*.c/*.h` sources and no `bindgen` crate" | :163-167 | REAL-UNPROVEN | `grep -c '^name = "bindgen"' apps/desktop-tauri/Cargo.lock` -> 0 (bindgen absent from lock — consistent); the "*.c/*.h sources" half is a tarball claim, not re-verifiable offline |
| 6 | "libloading appears only on the inactive tray-icon edge (see the allowlist §3a row)" | :166-167 | REAL | allowlist.md:129 row + deny.toml:189 `{ name = "libloading", wrappers = ["libappindicator-sys"] }`; libloading absent from the MAIN lock (grep count 0) |
| 7 | "Still PENDING ... the `[VERIFY]` frame-shape marker in ui/src/client/tauri-transport.ts (clears only on a proven event round-trip)" | :190-196 | REAL | ui/src/client/tauri-transport.ts:93 `// [VERIFY at the ADR-004 admission build] the frame shape and the` — marker still present, i.e. honestly uncleared |
| 8 | "apps/desktop-tauri/INFRA-BLOCKED.md §B" carries the launch-half blockers | :190-191 | REAL | file exists (ls -> apps/desktop-tauri/INFRA-BLOCKED.md); §B referenced consistently by ci.yml:696 ("INFRA-BLOCKED.md §B.1") |
| 9 | R3 "the CI lane installs them explicitly (`tauri-check`)" (webkit2gtk-4.1 etc.) | :176-179 | REAL | ci.yml:703-710 `sudo apt-get install ... libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libgtk-3-dev libsoup-3.0-dev ...` + pkg-config assert |
| 10 | preaudit "wry 0.57.0 ... WebKitGTK 4.1/GTK3 (Linux)" / R10 "NOT imported" (4.H1 posture kept as history) | :1, :24-27, :120-123 | REAL (self-annotated history) | title says 2.12.4/0.57.0; the :13-17 banner explicitly says the NOT-imported posture below is the historical record — the doc does not hide the supersession |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Title: "tauri 2.12.4 + wry 0.57.0" | sublock: tauri 2.12.5, wry 0.57.2 | sublock — but the doc's own :13-17 banner discloses the admission state; title is a pre-audit label, self-flagged |

## 4. Fabrication / inflation indicators
None. Pending items are listed by name (launch verdicts, TB3 security review, per-OS verdicts), the tauri-transport VERIFY marker is still literally uncleared in the tree, and the one soft phrase — "cargo-deny runs per-push" — describes a lane that exists but whose execution is CI-billing-blocked (external fact, recorded in phase7-exit.md §6).

## 5. Matrix rows
Not a matrix chunk. Cross-check performed: the admission crate's build failure in this sandbox (baseline: desktop-tauri build fails at gdk-sys, no GTK) matches the doc's own "not attempted here by design" stance (:72-75) — no contradiction with the environment facts.
