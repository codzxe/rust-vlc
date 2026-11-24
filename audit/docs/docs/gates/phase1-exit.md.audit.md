# CLAIM AUDIT: docs/gates/phase1-exit.md
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Claims extracted | 14 |
| REAL: 5 · REAL-UNPROVEN: 6 · SHALLOW: 0 · STUB: 0 · PAPER-ONLY: 0 · FALSE (stale in current repo): 1 · UNREACHABLE: 0 · UNVERIFIED: 2 |
| Internal contradictions found | 2 |

## 1. Blunt verdict
The Phase 1 exit gate's "PASS" verdict is a Lead's diary entry, not a reproducible record: every evidence cell in its command table points at `target/*.json` (build-dir artifacts, not committed — both absent from the tree) or an uncommitted "session log", and no phase-1 log exists in docs/gates/logs/ (which starts at phase 3). The doc is honest that CI links are pending, but it is NOT honest that its local-run evidence is equally uncommitted — "validated locally with recorded exit codes" (:6-7) means recorded in prose worklog lines, not in any committed artifact. The config/CI claims that survive to the present repo (deny.toml+G3/G4 job, allowlist schemars, toolchain pin, schema-validation job, catalog_shape pin) all check out.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Gate verdict: PASS (with the CI-green-run links honestly PENDING first push … every CI command was validated locally with recorded exit codes" | :5-8 | **REAL-UNPROVEN** | The recorded exit codes live in no committed artifact: `ls target/gate-report.json target/architecture-report.json` → both absent; docs/gates/logs/ has no phase-1 file (listing: phase3-* and later). Prose-in-worklog only. |
| 2 | "`cargo xtask gate` … SUCCESS — 0 of 10 failed, 5 honest skips … `target/gate-report.json`" | :14 | **REAL-UNPROVEN** | cited file absent (see above); not re-runnable within cargo-build budget |
| 3 | "`cargo test --workspace --all-features` … 529 passed / 0 failed / 3 ignored … session log" | :15 | **REAL-UNPROVEN** | "session log" not committed; count unverifiable without git |
| 4 | "`cargo fmt --all --check` PASS / clippy `-D warnings` PASS (0 warnings)" | :16-17 | **REAL-UNPROVEN (and stale)** | no committed phase-1 log; today's baseline: `cargo fmt --check` FAILED exit 1 and fmt is red in full_gate — different epoch, but nothing in-repo proves the phase-1 pass |
| 5 | "actionlint 1.7.12 over `.github/workflows/` *(now `.github/workflows-disabled/` per INFRA-BACKUP-1)* clean" | :21 | **FALSE (stale)** | `ls .github/` → `workflows` (live, 8 files incl. ci.yml, audit.yml, release.yml); no workflows-disabled dir exists. The INFRA-BACKUP-1 state was reverted later; the parenthetical now misstates the tree. |
| 6 | "`deny.toml` live (G3/G4 job)" | :30 | **REAL** | deny.toml exists (root, L177-189 libloading ban verified in the wgpu record audit); .github/workflows/ci.yml:490 `cargo-deny:` job "name: cargo-deny (G3/G4 — licenses/sources/bans/advisories)" |
| 7 | "`docs/dependencies/allowlist.md` current incl. schemars 1.2.3 + 5 transitives (MIT-only …)" | :30 | **REAL** | allowlist.md:67 schemars 1.2.3 MIT "not dual-licensed"; :69-87 the schemars family quick-pass rows |
| 8 | "rust-toolchain.toml (1.99.1 pin … pin held)" | :28 | **REAL** | rust-toolchain.toml `[toolchain] channel = "1.99.1"`, components rustfmt+clippy |
| 9 | "`product.toml` + schemas … validated by the product-config committed suite (CI schema-validation job)" | :29 | **REAL** | product.toml + product.schema.json exist; ci.yml:254 `schema-validation:` job (G1/G6) |
| 10 | "catalog_shape.rs pins the exact counts … 61 commands / 23 events" | :61-63 | **REAL (historical; superseded)** | crates/commands/tests/catalog_shape.rs:178-181 now pins `65` / `26` ("61 commands + issue #32 bookmark trio + w31.1 frame-channel bind"; "23 events + issue #42 trio") — the 61/23 numbers were phase-1-true, the catalog grew |
| 11 | "Workspace at gate: 10 crates + xtask, 39,094 lines of Rust, zero unsafe, zero unwrap/expect/panic in non-test code" | :48-50 | **UNVERIFIED** | historical workspace state; git forbidden; today's tree has far more crates and 4126 unreachable! per baseline |
| 12 | "CI green-run artifact links PENDING first push (no remote in sandbox)" | :22 | **UNVERIFIED** | cannot check remote state (git forbidden); no committed CI run URLs in the doc |
| 13 | "required checks: boundary-check, cargo-deny, schema-validation, config-propagation" (carried risk #1) | :70-72 | **REAL** | ci.yml contains all: cargo-deny (:490), schema-validation (:254), config-propagation (:416), boundary-check as gate-job step (:470 "boundary-check (gate job step 4 …)") |
| 14 | "TS output (`ui/src/generated/commands.ts`) is not yet typechecked by CI" | :81-82 | **REAL (historical)** | baseline now shows npm typecheck passing in CI — the promised 4.A1 arrival happened; claim was true at write time |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "(now `.github/workflows-disabled/` per INFRA-BACKUP-1)" (:21) | Current tree: `.github/workflows/` live with 8 workflow files | The tree wins (`ls .github/workflows/` → audit.yml, ci-full.yml, ci.yml, codeql.yml, compat-matrix.yml, release.yml, rust-clippy.yml …). Doc parenthetical is stale. |
| "catalog count is 61 commands / 23 events … catalog_shape.rs pins the exact counts" (:61-63) | catalog_shape.rs:178-181 pins 65/26 | Both true in their era; the current pin is 65/26. Any modern doc citing 61/23 from phase1-exit.md is citing a stale number. |

## 4. Fabrication / inflation indicators
- The PASS verdict itself is not fabricated evidence — it is *unbacked* evidence: the failure mode here is missing artifacts, not invented ones. The doc never claims a committed log that fails to say what it says.
- One soft inflation marker: "recorded exit codes" (:7) implies records; the records are prose lines in a worklog archive, not checkable transcripts.

## 5. Matrix rows
N/A — not a matrix chunk.

## 6. What actually works (verified)
- `ls .github/workflows/`, `cat rust-toolchain.toml`, `rg` over ci.yml job names, `sed` on catalog_shape.rs — all exit 0, outputs cited above.
- Config spine claims (§2 rows 1-3) all land on existing files/jobs.

## 7. Open questions for next agent
- Is there ANY committed phase-1 or phase-2 gate log anywhere (e.g. in docs/worklog/archive-2026-10.md as verbatim transcripts)? If not, phase 1's PASS rests on the worklog prose alone.
- phase6-exit.md (next file in this assignment) — check whether IT cites committed logs (docs/gates/logs has NO phase6 log: the listing shows phase3, phase4, phase7, merge logs only). Same missing-evidence pattern expected.
