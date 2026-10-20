# Phase 1 Exit Gate Report (card 1.F1-V)

Lead gate review, 2026-10-20. Checklist source: `docs/13-agent-wave-program.md`
§4 (MP §16 Foundation gate) + the per-card acceptance criteria. Verdict per
row with evidence. **Gate verdict: PASS** (with the CI-green-run links
honestly PENDING first push — no git remote exists in the sandbox; every CI
command was validated locally with recorded exit codes, actionlint 1.7.12
clean; established precedent from the Phase 0 gate).

## 1. Gate command evidence (run on main @ `2d06607`, 2026-10-20T17:43Z)

| # | Command | Result | Evidence |
|---|---------|--------|----------|
| 1 | `cargo xtask gate` (doc 10 §2 table) | **SUCCESS** — 0 of 10 failed, 5 honest skips | `target/gate-report.json`; skips: ui×3 (no `ui/package.json` — card 4.A1), tauri (no `apps/desktop/` — Phase 4), wasm-pack (no `engine-wasm-binding` — Phase 5) |
| 2 | `cargo test --workspace --all-features` | **529 passed / 0 failed / 3 ignored** | session log; +48 over the 481 pre-1.D4 baseline |
| 3 | `cargo fmt --all --check` | PASS | gate step 1 |
| 4 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS (0 warnings) | gate step 3 |
| 5 | `cargo xtask check-architecture` | **PASS 5/5** (edges, cycles, unsafe-lints, static-mut, testkit-direction) | `target/architecture-report.json`; boundaries.toml covers all 10 crates + xtask |
| 6 | `cargo xtask codegen --check` | exit 0 — no drift, 61 commands / 23 events | session log; drift failure mode independently re-proven by Lead (mutated checksum → exit 1; restored → exit 0) |
| 7 | `cargo xtask schema-gen --check` | exit 0 (27 entries, no drift) | 1.E2 evidence, re-verified in gate |
| 8 | actionlint 1.7.12 over `.github/workflows/` | clean (ci.yml + wasm.yml, post catalog-drift-step) | Lead re-run after the 2d06607 edit |
| 9 | CI green-run artifact links | **PENDING first push** (no remote in sandbox) | honest-gate precedent; all job commands locally green, seeded-violation exits recorded in worklog 1.E1/1.E2/1.E5-R |

## 2. MP §16 Foundation-gate criteria → status

| MP §16 requirement | Status | Evidence |
|---|---|---|
| Workspace with pinned toolchain + lints | ✅ | `Cargo.toml` (workspace lints: rust+clippy deny lists), `rust-toolchain.toml` (1.99.1 pin, re-provisioned 4× across sandbox incidents — pin held), lint canary in `crates/_template` |
| `product.toml` + schemas | ✅ | `product.toml` + `product.schema.json` validated by the product-config committed suite (CI schema-validation job); `schemas/settings.toml` D1–D6 + generated outputs committed |
| Native-code audit | ✅ | `deny.toml` live (G3/G4 job); `docs/dependencies/allowlist.md` current incl. schemars 1.2.3 + 5 transitives (MIT-only, honestly recorded); all current deps pure-Rust |
| Command layer | ✅ | `crates/commands`: envelope + typed errors w/ V8 redaction + correlation (1.D3) AND the full catalog — 61 commands / 23 events as types AND data, §5.1 wire forms, schema+TS+checksum artifacts, drift-gated codegen (1.D4) |
| Plugin API | ✅ | `crates/plugin-api`: kinds, manifests, host traits (1.D2) |
| Settings-schema generator | ✅ | `crates/settings-schema` + `xtask schema-gen` (1.B2) |
| Token pipeline | ✅ | `crates/theme-tokens` + `xtask tokens`; committed outputs diff-tested (1.B4) |
| Tests per doc 10 | ✅ | 529/0/3 across 10 crates + xtask; every card landed with its AC tests (see §3) |

## 3. Phase 1 card ledger (20/20 complete)

| Wave | Cards | Status |
|---|---|---|
| 1.A bootstrap | 1.A1 workspace · 1.A2 CI skeleton · 1.A3 xtask gate | ✅ 3/3 |
| 1.B config spine | 1.B1 product-config · 1.B2 settings-schema · 1.B3 localization · 1.B4 theme-tokens | ✅ 4/4 |
| 1.C core vocabulary | 1.C1/1.C2 media-types · 1.C3 testkit | ✅ 3/3 (restored after rollback #1) |
| 1.D contracts | 1.D1 codec-api (R) · 1.D2 plugin-api · 1.D3 commands I · 1.D4 commands II | ✅ 4/4 (1.D4 completed this session from the preserved partial — 6 milestone commits, Lead-verified, merged ff) |
| 1.E enforcement | 1.E1 check-architecture · 1.E2 CI wiring · 1.E3 wasm.yml · 1.E4 rebrand · 1.E5 deny.toml | ✅ 5/5 |
| 1.F gate | 1.F1-V this report | ✅ |

Workspace at gate: **10 crates + xtask, 39,094 lines of Rust**, zero unsafe,
zero unwrap/expect/panic in non-test code (lint-enforced), boundaries.toml
fully populated.

## 4. Lead ratifications + decisions at this gate

- **D-015**: Branch-commit rights granted to implementation agents on WIP
  branches (user directive + sandbox-instability evidence: reboots reset the
  working tree to main; commits survive). AGENTS.md §3.8 deviation recorded
  in the worklog (1.SESSION-3-START). Merge to main stays Lead-only.
- **D-016**: xtask registry graduation — all six wave-1 subcommands flagged
  `implemented: true` (Lead batch-flip at the 1.D4 merge, commit `2d06607`);
  the v0 "only gate" invariant test graduated with them.
- **D-017**: The catalog count is **61 commands / 23 events** — doc 05's
  §2.3–§2.12 + §3.2 tables are the authority (the "~65/~24" in the card text
  was an approximation). catalog_shape.rs pins the exact counts.
- **R-6 (1.D4)**: schemars recorded as **MIT-only** (verified via cargo
  metadata + vendored manifest) — the brief's "MIT OR Apache-2.0" assumption
  corrected honestly in the allowlist.

## 5. Carried risks / Lead flags into Phase 2

1. **CI green-run links PENDING first push** — repo settings wiring
   (required checks: boundary-check, cargo-deny, schema-validation,
   config-propagation) happens at first push; call recorded in worklog.
2. `.github/workflows/README.md` job table synced at this gate (was stale
   since 1.E1/1.E5-R); keep syncing on every job change.
3. Feature-matrix R7/R8/R10 + R1-prefix-map/R2-P0/R4-resolution owed to
   `tools/feature-audit` (unassigned, doc 02 §4.6); l10n-audit/lint-literals/
   check-identity/config-validator xtask subcommands still owed (frozen
   registry — Lead grants ownership when scheduled).
4. Two sandbox reboots THIS session (session-3); the git-always + branch
   guard discipline is now mandatory protocol for every agent.
5. TS output (`ui/src/generated/commands.ts`) is not yet typechecked by CI —
   arrives with card 4.A1 (ui workspace + CI job).

## 6. Phase 2 launch list (Wave 2.A + parallel queue-independent)

Per doc 13 §5 dependency graph, the Lead launches in parallel immediately
after this gate (parallelism 2–4, separate crates/branches, no shared-file
conflicts):

| Card | Scope | Branch |
|---|---|---|
| 2.A1 | `crates/input` — SourceUri, file access, bounded buffered reader, Seekable | `wip/2a1-input` |
| 2.A3 | `xtask fixtures` — deterministic AVI+MJPEG+PCM and MP4+MJPEG+PCM generators (decision-independent) | `wip/2a3-fixtures` |
| 2.B1 | `crates/packet-queue` — bounded queues, backpressure, generation stale-drop | `wip/2b1-packet-queue` |
| 2.C1 | `crates/media-clock` — audio-master clock, MediaTime mapping, pause/drift | `wip/2c1-media-clock` |

Then: 2.A2 (media-probe, needs 2.A1); the **first-vertical container-pair
decision** (AVI+MJPEG+PCM self-build vs MP4+symphonia-isomp4) is taken by
the Lead at wave-2.B launch as doc 13 §5 mandates; 2.B2/2.B3 close
`not-selected` accordingly; 2.B4/2.B5 (MJPEG/PCM decoders) need 2.A3 only.
