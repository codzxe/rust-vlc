# AUDIT: Cargo.toml (workspace root)
| Field | Value |
|---|---|
| Auditor | W01-A10 |
| Date | 2026-11-23 |
| Lines / functions | 66 / 4 sections ([workspace], [workspace.lints.rust], [workspace.lints.clippy]; no [profile]/[workspace.package]/[workspace.dependencies]) |
| File verdict | REAL — a real, consistent, honestly-commented workspace manifest with two stale comments and standard hygiene gaps |
| Reachable from shipped entry point? | YES — every build of lumen-cli/desktop resolves through it (baseline release lane exit 0) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | greps of members/lines/workspaces/lints across the tree; wc; no builds (baseline cited) |

## 1. One-paragraph verdict
The manifest does what it says: 39 members (31 `crates/*` incl. the intentional `_template` member, 4 `apps/*` minus the ADR-004-excluded `apps/desktop-tauri`, 3 explicit `tools/` entries, 1 integration suite), one exclude, a lint block that is verbatim identical to doc 10 §3.1, and inheritance verified — I grepped all 9 non-crate members' manifests for `[lints] workspace = true` (all present), the baseline arch gate reports "39/39 crates inherit [lints] workspace = true" (full_gate.txt:46), and the canary proof is recorded with verbatim exit-101 output in docs/worklog/archive-2026-10.md:672-673. Nested tool workspaces (tools/vta ×5, tools/bench-vs-vlc, fuzz) all declare their own [workspace], so the member set is exactly what the globs say. Defects are hygiene-level: the header's membership narrative omits the `tools/mem-audit` + `tools/sweep` members; the single-line-members justification is stale (the arch reader now parses multi-line arrays — fixed in "W25 CI-repair"); there is no [profile] tuning anywhere in the main workspace despite a wasm size-budget gate and perf-parity goals; and no [workspace.package]/[workspace.dependencies] centralization for 119 registry deps.

## 2. What actually works (verified)
- Members ↔ reality: `ls crates/` = 31 dirs; `ls apps/` = 5 (desktop-tauri excluded); + tools/xtask, tools/mem-audit, tools/sweep, tests/integration/first-vertical = 39 members — matches full_gate.txt:46 "39/39" and the SBOM's "39 workspace/path" components (xtask-sbom baseline).
- Every non-crate member carries `[lints] workspace = true` (grep of all 9 manifests, incl. apps/web and apps/control-server); crates/_template/Cargo.toml:21-22 too.
- Lint block vs doc 10 §3.1: line-for-line identical (4 rust denies + 7 clippy denies, same order) — the "verbatim" claim at Cargo.toml:31 is TRUE.
- Canary claim (L32-33): proof run recorded — docs/worklog/archive-2026-10.md:672-673, `cargo clippy -p template --all-targets -- --cfg lint_canary` → exit 101 with both inherited tables firing (unwrap_used + unused_variables). Not re-run (clippy banned); worklog is the run record.
- desktop-tauri exclusion rationale (L48-49 "the policy is mirrored inline in its Cargo.toml"): VERIFIED — apps/desktop-tauri/Cargo.toml has its own `[lints.rust]`/`[lints.clippy]` (L34/L40).
- No accidental members: tools/vta/{ac3,dnx,mpeg4,ruopus,vp9}, tools/bench-vs-vlc, fuzz all declare their own `[workspace]` (grep).
- No .clippy.toml exists → clippy defaults apply; `unwrap_used`/`expect_used` therefore deny in test code too — consistent with the tree-wide `Must`-trait pattern and the census (19 raw `unwrap(` tree-wide, none executable in main-workspace non-test code).

## 3. Findings
F-001 [S4] Stale justification: the single-line members array is credited to a parser limitation that no longer exists.
Where: Cargo.toml:37-41 vs tools/xtask/src/cmd/check_architecture.rs:73-75
Code (Cargo.toml, verbatim):
```
# Single-line members array: the xtask TOML reader (check-architecture)
# does not support multi-line arrays — the salvage's multi-line form
# broke the arch gate; restored to the reader's shape with the same
# membership (the 2.E1 pre-announced edit, unchanged in content).
```
Code (check_architecture.rs:73-75, verbatim):
```
//! * The TOML subset rejects multi-line inline tables and dotted keys —
//!   loudly, with file:line, never silently. Multi-line ARRAYS parse
//!   (W25 CI-repair: member manifests ship them).
```
What is wrong: the reader HAS supported multi-line arrays since the W25 CI repair ("member manifests ship them"), so the root manifest's stated constraint is obsolete. Harmless today (single-line still parses) but the comment teaches a false fact about the toolchain, and the root manifest's legal TOML shape is still being dictated by a hand-rolled parser's history rather than by cargo.
Confidence: HIGH — Verification: read (both files).

F-002 [S4] Membership narrative omits two members.
Where: Cargo.toml:3-29 (header) vs :41 (members)
Code (members line, verbatim):
```
members = ["crates/*", "apps/*", "tools/xtask", "tests/integration/first-vertical", "tools/mem-audit", "tools/sweep"]
```
What is wrong: the header meticulously documents each pre-announced edit (xtask/1.A3, apps-glob/2.D2-2.D3, first-vertical/2.E1) and says "Further membership changes require Lead approval (AGENTS.md §3.7)" — but `tools/mem-audit` and `tools/sweep` appear in the members line with no comment, no card reference, and no worklog pointer. Either the approval is unrecorded in the file that promises the record, or the header was never updated. (apps/control-server also joined silently, but via the pre-announced glob — no edit needed.) Also a third TOML parser note: check_architecture has its own parser (parse_toml, L286) distinct from cmd::toml_lite — the "xtask TOML reader" is not one thing.
Confidence: HIGH — Verification: read + grep (no comment mentions mem-audit/sweep in this file).
UNVERIFIED: whether Lead approval for mem-audit/sweep exists elsewhere (worklog search out of scope for this file; next agent can grep the worklog).

F-003 [S4] No [profile] sections in the main workspace — release lane and wasm builds use stock cargo defaults.
Where: Cargo.toml (absent); repo-wide `[profile` grep hits only fuzz/Cargo.toml:59 and tools/vta/mpeg4/Cargo.toml:18 (both separate workspaces)
What is wrong: no opt-level/LTO/strip/codegen-units/panic settings anywhere that affects `cargo build --release -p lumen-cli -p desktop` or the wasm lane. The project simultaneously runs a wasm size-budget gate (web-budget, ceilings of 3/6 MiB gzip) and perf-parity goals, yet ships with default release settings (opt-level 3, lto off, codegen-units 16, no strip). Not proven harmful (defaults are sane; the smoke passed at ~19ms); it is an unexamined default for a performance-positioned product. Correct looks like: a deliberate [profile.release] (even if it only documents "defaults chosen deliberately").
Confidence: HIGH (absence verified) — impact UNPROVEN (no A/B measurement exists).
Confidence: HIGH — Verification: ran (repo-wide grep).

F-004 [S4] No [workspace.package] and no [workspace.dependencies].
Where: Cargo.toml (absent)
What is wrong: 39 members each re-declare name/version/edition/publish and their own dependency versions (119 registry components per the SBOM). Nothing centralizes `version = "0.1.x"` or shared dep versions, so per-crate drift is possible (e.g. two crates on different serde minor versions resolve independently). The SBOM exists to notice such drift after the fact, but the standard Cargo mechanism to prevent it is unused. Given the std-only policy keeps most crates at zero deps, impact is limited.
Confidence: HIGH — Verification: read (manifest) + baseline SBOM component count.

## 4. Per-key inventory (config checklist)
| Key / section | Verdict | Notes |
|---|---|---|
| `[workspace] resolver = "2"` | REAL | implied by edition 2024 anyway; explicit is fine |
| `members` (glob + 4 explicit) | REAL | 39 members verified against the tree |
| `exclude = ["apps/desktop-tauri"]` | REAL | ADR-004; mirrored lints verified in the excluded manifest |
| `[workspace.lints.rust]` (4 denies) | REAL | verbatim doc 10 §3.1; inheritance proven (canary worklog record + arch gate 39/39) |
| `[workspace.lints.clippy]` (7 denies) | REAL | same; known blind spots (unreachable!, indexing) owned by check-panics per its docs — and check-panics is unwired (my file-1 audit) |
| `[profile.*]` | ABSENT | F-003 |
| `[workspace.package]` | ABSENT | F-004 |
| `[workspace.dependencies]` | ABSENT | F-004 |

## 5. Test honesty
No tests in this file. The related proofs: `crates/_template/src/lint_canary.rs` (cfg-gated sanctioned violations; proof commands documented in-file, output recorded in worklog — REAL-UNPROVEN for my own re-run, clippy banned) and the arch gate's lint-inheritance check (baseline PASS).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Lint policy: doc 10 §3.1 verbatim" | Cargo.toml:31-32 | REAL | line-for-line identical block (doc 10:191-204) |
| "Inheritance is proven by the canary" | Cargo.toml:32-33 | REAL (cited record) | worklog archive-2026-10.md:672-673, exit 101 both tables |
| "`crates/_template` … intentionally a MEMBER" + rationale | Cargo.toml:6-12 | REAL | member via glob; template manifest carries `[lints] workspace = true`; publish=false |
| "the xtask TOML reader … does not support multi-line arrays" | Cargo.toml:37-39 | FALSE (stale) | check_architecture.rs:73-75 says arrays parse since W25 (F-001) |
| membership edit history | Cargo.toml:13-29 | INCOMPLETE | mem-audit + sweep undocumented (F-002) |
| "Every member inherits it via `[lints] workspace = true`" | Cargo.toml:31-33 | REAL | 9/9 non-crate manifests grepped + baseline 39/39 |

## 7. Dead / duplicated / unreachable code
- No dead keys. `unused_must_use`/`unused_imports` denies are exercised by real code (`#[must_use]` attributes throughout, e.g. demux ts API).
- Duplicated parsers context: the root manifest's parseability is constrained by xtask's own readers (check_architecture's parse_toml + toml_lite) — three hand-rolled TOML/JSON-ish parsers exist inside xtask (see my config_validator audit and the toml_lite sibling audit).

## 8. Dependencies & flags
- Virtual manifest (no [package]) — correct for a workspace root. No external deps. Toolchain pinned separately (rust-toolchain.toml: 1.99.1, minimal + rustfmt/clippy — cross-ref; audited by the toolchain-pin owner).

## 9. Open questions for the next agent
- Find the Lead-approval record for the `tools/mem-audit` and `tools/sweep` member additions (worklog/AGENTS.md §3.7 trail) — or confirm the header simply wasn't updated.
- deny.toml (root) — separate file, not in my assignment; cross-check its crate allowlist against the 39-member set.
- Whether [profile.release] defaults were ever measured against tuned settings for the wasm budget (docs/web/size-report.md owner).
