# AUDIT: apps/desktop-tauri/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-21 |
| Lines / functions | 69 / 10 config keys ([package]×5, [[bin]], [lints.rust]×4, [lints.clippy]×7, [dependencies]×4, [build-dependencies]×1) |
| File verdict | REAL-UNPROVEN (manifest complete and self-consistent; the `lumen-tauri` entry point it defines has zero committed build/test evidence) |
| Reachable from shipped entry point? | YES by definition — this crate IS the `lumen-tauri` shipped entry point (bin, manifest:30-32); its build is UNVERIFIED (baseline #2 exit 101) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | Read of manifest + `sed -n '13,21p;34,47p;57,68p'` (quote verify); Grep main Cargo.lock for `libloading\|libappindicator\|tauri\|tray-icon` → 0 matches (workspace graph tauri-free, as claimed); Grep apps/desktop-tauri/Cargo.lock → tauri 2.12.5 (:3688), wry 0.57.2 (:4961), tao 0.37.2 (:3631), tauri-build 2.7.4 (:3739), tray-icon 0.25.1 (:4190), libappindicator-sys 0.9.0 → libloading 0.7.4 (:1832-1840, :1858); `rg -n '^name = '` apps/desktop/Cargo.toml + crates/commands/Cargo.toml → package names `desktop`/`commands` (path deps resolve); `rg -c '^name = '` apps/desktop-tauri/Cargo.lock → 493; Grep boundaries.toml for `desktop-tauri` → 0 matches; Grep tools/xtask for `desktop-tauri` → only rebrand.rs (tauri.conf.json tokens); Read .github/workflows/ci.yml:673-760 (tauri-check lane exists: apt webkit2gtk, ui/dist build, cargo test/clippy/deny --manifest-path, ldd smoke); Read docs/dependencies/allowlist.md §3a (tauri >=2.12.5,<2.13.0; tauri-build >=2.7.4,<2.8.0; tao >=0.37.2,<0.38.0); Read INFRA-BLOCKED.md; baseline #2/#3/#4/#25 |

## 1. One-paragraph verdict
The manifest is well-organized and its central design claim holds: the crate is workspace-excluded, the main workspace lockfile is verifiably tauri-free (Grep → zero tauri/libloading/libappindicator entries), and its own lockfile pins exactly the versions the dependency record admits (tauri 2.12.5, wry 0.57.2, tao 0.37.2, tauri-build 2.7.4). But the audit found a real admission-control hole: the manifest requires `tauri = "2"` (any 2.x) while the comment claims that is "per the record's admission range" — the actual admitted range is `>=2.12.5, <2.13.0` (allowlist.md:125), and no committed gate enforces that range (cargo-deny checks bans/licenses/advisories only; deny.toml carries no tauri version constraint). One `cargo update` away from an unaudited webview shell with every gate green. On top of that, the shipped `lumen-tauri` binary has never been built or tested in any committed evidence (baseline #2 failed on missing GUI libs; `--workspace` lanes structurally exclude it), and the crate's self-declared edge list and lint mirror have no machine checker.

## 2. What actually works (verified)
— Workspace exclusion is real and effective: main Cargo.lock contains no tauri/wry/libloading/libappindicator/tray-icon packages (Grep → 0 matches), matching root Cargo.toml:42-51 and the record's "main workspace graph stays tauri-free" claim (tauri-wry-2.x.md:13-18).
— Subcrate lockfile matches every allowlist §3a pin: tauri 2.12.5 (Cargo.lock:3688), tauri-build 2.7.4 (:3739), tao 0.37.2 (:3631), wry 0.57.2 (:4961) — allowlist.md:125-128.
— The lint mirror (manifest:34-47) is a verbatim copy of the root `[workspace.lints]` tables (root Cargo.toml:53-66) — verified key-by-key, both rust and clippy tables, identical deny set.
— Path dependencies resolve: apps/desktop package name is `desktop` (Cargo.toml:19), crates/commands is `commands` (Cargo.toml:15) — the manifest's `desktop = { path = "../desktop" }` / `commands = { path = "../../crates/commands" }` are correct.
— The tauri-check CI lane exists in ci.yml (:673+, apt installs webkit2gtk-4.1, builds ui/dist first, then `cargo test/clippy/deny --manifest-path apps/desktop-tauri/Cargo.toml --locked`) — the design's enforcement story is at least written down.

## 3. Findings
### F-001 [S2]
Where: apps/desktop-tauri/Cargo.toml:57-60
Code:
```
# ADR-004 admission (records/tauri-wry-2.x.md): the OS webview. Version
# "2" per the record's admission range; wry + tao ride its own graph
# in THIS crate's lockfile.
tauri = "2"
```
What is wrong: The comment claims the version requirement follows "the record's admission range". The admitted range is `>=2.12.5, <2.13.0` (docs/dependencies/allowlist.md:125, the Lead allowlist row the record mandates — tauri-wry-2.x.md:144-146); `"2"` means `>=2.0.0, <3.0.0`. Nothing machine-enforces the narrower range: deny.toml's [bans] has no tauri entry (no version pins at all — Read of deny.toml:170-398), and the tauri-check lane's cargo-deny step runs `check bans licenses advisories` (ci.yml:754-757), which cannot fail on an in-range-but-unaudited 2.13.0. allowlist.md:120-121's claim "Enforced by the `tauri-check` lane's cargo-deny step with this same repo-root table" is therefore false as stated. Same gap for `tauri-build = "2"` (line 68) vs the admitted `>=2.7.4, <2.8.0` (allowlist.md:126).
Why it matters: This crate is the E1 OS-webview exception — the highest-risk external surface the repo admits, governed by an explicit audit-record regime (doc 11). A `cargo update` in this crate (or a transitive bump) can land tauri 2.13+ / tauri-build 2.8+ — outside every audited range — with all CI gates green; only the procedural "same-PR rule" (a reviewer noticing the lockfile diff) stands in the way. The admission-control system this repo advertises has a silent bypass for its single most security-sensitive dependency.
What correct looks like: `tauri = "~2.12.5"` (or `=2.12.5` + a renovate-style bump flow that re-opens the record), plus a real enforcement hook: deny.toml `[bans] deny` entries like `{ crate = "tauri@<2.12.5,>=2.13" }`-style specs, or an xtask check comparing the subcrate lock against allowlist.md §3a ranges.
Confidence: HIGH — Verification: read of manifest, allowlist.md:114-132, deny.toml, ci.yml:673-760; lockfile Grep.

### F-002 [S3]
Where: apps/desktop-tauri/Cargo.toml:13-21,34-47
Code:
```
# Edges (this crate only — boundaries.toml governs workspace members;
# this one is excluded, so its edges are declared here):
...
# Lints: `[lints] workspace = true` cannot resolve for an excluded
# crate (root Cargo.toml note) — the policy is mirrored inline below.
```
What is wrong: Both "declarations" are prose with no reader. boundaries.toml has no desktop-tauri entry (Grep → 0 matches); xtask check-architecture parses workspace members only (baseline #11: "unsafe-lints 39/39 inherit" — 39 members, this crate not among them); no xtask command parses this manifest (Grep `desktop-tauri` in tools/xtask → only rebrand.rs's tauri.conf.json token rewrite). The lint mirror (lines 34-47) matches the root tables today (verified verbatim), but if the root `[workspace.lints]` gains or changes a lint, this copy silently diverges — nothing diffs them. The crate's internal layering edges (desktop L6, commands L3) are likewise never boundary-checked by any committed gate.
Why it matters: The ADR-004 exclusion bought a clean main graph at the cost of dropping the excluded crate out of every structural gate; the manifest's "edges are declared here" creates the impression of checked declarations where none exist. The only enforcement that does touch this crate is the tauri-check lane's clippy (enforces the mirrored lints at compile time) — a CI-lane claim with no committed run log (see F-004).
What correct looks like: An xtask check (or check-architecture step) that (a) parses apps/desktop-tauri/Cargo.toml, (b) diffs its `[lints.*]` tables against the root `[workspace.lints]` tables, and (c) validates its internal deps against a boundaries.toml `[excluded.desktop-tauri]` row.
Confidence: HIGH — Verification: read + Grep (boundaries.toml, tools/xtask) + baseline #11.

### F-003 [S3]
Where: apps/desktop-tauri/Cargo.toml:30-32,49-68 (whole crate)
Code:
```
[[bin]]
name = "lumen-tauri"
path = "src/main.rs"
```
What is wrong: `lumen-tauri` is one of the two shipped entry points, and no committed evidence shows it has ever compiled or run: baseline #2 `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml` → exit 101 (missing gdk-3.0/gtk pkg-config; INDEX.md: "tauri shell UNVERIFIED-BUILD"); baselines #3/#4 (`cargo test --workspace`) structurally exclude this crate, so its tests (including the §7.4 anti-bypass audit `tests/single_command.rs`) have no baseline run either. INFRA-BLOCKED.md honestly lists the pending half: launch verdicts (§B.2), icons (§B.3), dialog plugin (§B.4), ADR-004 per-OS verdicts + TB3 security review (§B.5), WebView2 bootstrapper decision (§B.6).
Why it matters: A shipped GUI entry point whose entire verification story is CI-lane assertions in comments (ci.yml run IDs 43453921560, 43455796103 quoted in ci.yml:745-760) — not committed logs. The wave-0 baseline could not execute it (sandbox limitation, recorded as such in INDEX.md:96), so within this audit the entry point is REAL-UNPROVEN end to end.
What correct looks like: A committed tauri-check/tauri-launch run log (or orchestrator-run of the §C recipe on a webview-equipped host) under audit/runs/; until then every "the crate compiles" claim should carry UNVERIFIED.
Confidence: HIGH — Verification: baseline #2/#3/#4 + INFRA-BLOCKED.md read; REQUEST-ORCH-RUN logged for a webview-host build.

### F-004 [S4]
Where: apps/desktop-tauri/Cargo.toml:13-18,61-62
Code:
```
#   desktop  (path, L6) — the shell: ShellRuntime, files, window config
#   commands (path, L3) — the shared wire envelope (WireRequest etc.);
...
serde = { version = "1", features = ["derive"] }
```
What is wrong: The "Edges … declared here" list names three edges (desktop, commands, tauri) but the manifest declares a fourth dependency, `serde` (line 62) — mentioned only in a separate inline comment, not in the edge declaration the header presents as this crate's boundary contract. Since the comment IS the only edge declaration (F-002), the one place that documents the contract is incomplete.
Why it matters: Inconsistent self-declared contract; a future auditor reading only the header undercounts the crate's external surface by one crate (serde, plus tauri-build in [build-dependencies], which the header also omits).
What correct looks like: List all five edges (desktop, commands, tauri, serde, tauri-build[build]) in the header, or drop the pretense of an edge declaration and point at a machine-readable source.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: apps/desktop-tauri/Cargo.toml:20-21 (cross-ref: INFRA-BLOCKED.md:21)
Code:
```
# ADR-004 admission close package (issue #67)
```
(Cross-referenced claim, not in this file: INFRA-BLOCKED.md:21 "Own lockfile (`Cargo.lock`, 468 packages incl. the tauri closure)".)
What is wrong: The lockfile now carries 493 `name =` entries (`rg -c '^name = '` → 493), not the 468 the admission package records — 25 packages of silent growth since 2026-11-13 with no note. The manifest itself is unaffected; the drift is in the crate's own admission documentation, recorded here because the manifest is the anchor file for the admission state.
Why it matters: Admission-package numbers that nobody re-checks are how unaudited growth hides; the same-PR lockfile-delta rule clearly has not been re-anchored since w26–w30 added release-lane work.
What correct looks like: INFRA-BLOCKED.md states a count with a "as of commit X" qualifier, or drops the count.
Confidence: HIGH — Verification: `rg -c '^name = '` on apps/desktop-tauri/Cargo.lock → 493 (this count is my own command, not a baseline run).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package]` (name/version/edition/publish) | 23-28 | REAL | — | `lumen-desktop-tauri` 0.1.3 = the release twin of `lumen-cli` 0.1.3 (apps/cli/Cargo.toml:34); edition 2024; publish=false. |
| `[[bin]] lumen-tauri` | 30-32 | REAL-UNPROVEN | S3 (F-003) | src/main.rs exists; entry point never built in any committed run. |
| `[lints.rust]` mirror | 34-38 | REAL | — | Verbatim match of root Cargo.toml:53-57; enforced only by the tauri-check clippy step (CI claim); no sync checker (F-002). |
| `[lints.clippy]` mirror | 40-47 | REAL | — | Verbatim match of root Cargo.toml:59-66; same enforcement gap. |
| `desktop` dep | 53 | REAL | — | Path resolves to package `desktop` (apps/desktop/Cargo.toml:19). |
| `commands` dep | 56 | REAL | — | Path resolves to package `commands` (crates/commands/Cargo.toml:15). |
| `tauri = "2"` dep | 60 | REAL-UNPROVEN | S2 (F-001) | Lock pins 2.12.5 (admitted); requirement floats to <3; range unenforced. |
| `serde` dep | 62 | REAL | — | features=["derive"] for ShellError (src/app.rs); S4 omission from edge list (F-004). |
| `tauri-build = "2"` build-dep | 68 | REAL-UNPROVEN | S2 (F-001) | Lock pins 2.7.4 (admitted >=2.7.4,<2.8.0); same float problem. |

## 5. Test honesty (if tests exist in/for this file)
No tests in this file. The crate's `tests/single_command.rs` (§7.4 anti-bypass audit) and `tests/e2e.rs`-adjacent suites have NO baseline execution (workspace-excluded); their "Local simulation PASS; CI compiles it for real" claim (INFRA-BLOCKED.md:31-33) is recorded evidence only — UNVERIFIED in this audit.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the main workspace graph stays exactly the pre-admission posture" | manifest:7-8 / tauri-wry-2.x.md:13-15 | REAL | Main Cargo.lock Grep → zero tauri-family entries. |
| 'Version "2" per the record's admission range' | manifest:57-58 | FALSE | Admitted range is >=2.12.5,<2.13.0 (allowlist.md:125); "2" spans <3 — see F-001. |
| "Enforced by the `tauri-check` lane's cargo-deny step with this same repo-root table" | allowlist.md:120-121 | FALSE (as enforcement of the version range) | The deny step checks bans/licenses/advisories (ci.yml:754-757); deny.toml has no tauri version constraint — nothing fails on an out-of-range but clean 2.13. |
| "the crate compiles in its dedicated CI lane (tauri-check, Linux + webkit2gtk)" | INFRA-BLOCKED.md:4-5 | REAL-UNPROVEN | Lane exists in ci.yml:673+; no committed run log; baseline #2 (the only attempted build) exit 101. |
| "468 packages incl. the tauri closure" | INFRA-BLOCKED.md:21 | FALSE (stale) | Actual count 493 (`rg -c '^name = '`). |
| Lint policy mirrored inline | manifest:20-21 / root Cargo.toml:48-50 | REAL | Verbatim match verified; but see F-002 (no sync enforcement). |

## 7. Dead / duplicated / unreachable code
- None within the manifest itself. The crate is an entry point, not library code — no orphan deps detected (desktop/commands/serde all used per header comments; src presence verified via LS).

## 8. Dependencies & flags
- Internal (path): desktop, commands — both resolve; both would be L6→L6 / L6→L3 edges if any layering gate covered this crate (none does — F-002).
- External: tauri "2" (lock: 2.12.5), serde "1" +derive (lock has own serde in subcrate graph), build-dep tauri-build "2" (lock: 2.7.4).
- deny.toml posture for this graph: licenses MIT/Apache-2.0 in allow set; not banned; the libloading wrapper `{ name = "libloading", wrappers = ["libappindicator-sys"] }` (deny.toml:189) matches this lockfile's real edge (libappindicator-sys 0.9.0 → libloading 0.7.4, lock:1832-1840) — but the SAME wrapper is what fails the MAIN workspace's `cargo deny check` (baseline #25: advisories FAILED, "unused wrapper") because libappindicator-sys exists only here. Cross-file S3 for the deny.toml owner: the shared-config-two-graphs design leaves the main workspace deny gate RED.
- Features: none declared; tauri's optional tray-icon feature edge is inactive (tray-icon 0.25.1 + libappindicator present in lock as resolvable optional deps — Cargo.lock keeps optional deps regardless of feature state).

## 9. Open questions for the next agent
- deny.toml / root-manifest auditor: baseline #25's advisories FAIL (unused wrapper libappindicator-sys) means `cargo deny --workspace check` (ci.yml:519-520) is red on main TODAY — was the cargo-deny CI job running when deny.toml:189 landed? Comments elsewhere say "billing-blocked cargo-deny CI job" (deny.toml:106-107 context) — the gate lapsed, the drift landed.
- Whoever audits apps/desktop-tauri/src/** and tauri.conf.json: verify the CSP claims (INFRA-BLOCKED.md:34-38) and the single-command anti-bypass test actually parse the registration site rather than a copy.
- release.yml auditor: verify lumen-tauri really ships in every payload (INFRA-BLOCKED.md §B.1 DONE claim) — the packaging gate PASS (baseline #17) does not by itself prove payload contents.
