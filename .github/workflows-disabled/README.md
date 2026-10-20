# CI workflows

Owner: task 1.A2 (Wave 1.A). This directory holds the GitHub Actions
pipeline. `ci.yml` is the fast tier skeleton; later Phase-1 cards activate
the stub jobs inside it (and 1.E3 may add `wasm.yml`).

**Honest status (2026-10-19):** this sandbox repository has no git remote
and no GitHub Actions runner. The workflow is written and validated with
actionlint 1.7.12 locally, and every cargo command it invokes was run green
locally (exit codes recorded in the 1.A2 worklog entry) — but no real CI run
exists yet.

**green-run artifact link: PENDING first push to remote (blocked: no remote
in sandbox)** `[PROPOSAL: gate evidence on first push]`.

## Job map (`ci.yml`)

| Job | OS | Command (doc 10 §2.1 verbatim unless noted) | Status |
|---|---|---|---|
| `fmt` | ubuntu | `cargo fmt --all -- --check` (#1) | active |
| `actionlint` | ubuntu | actionlint 1.7.12 over `.github/workflows/` (doc 10 §10.5) | active |
| `deps` | ubuntu | `cargo metadata --format-version 1 --locked` (doc 11 §3.7 lockfile assertion); `cargo tree --workspace` + `--edges normal,build,dev` (#5, §2.3); uploads `evidence-deps-ubuntu-latest-<run_id>` (§4.5) | active |
| `check` | 3-OS matrix | `cargo check --workspace --all-targets --all-features` (#2) | active |
| `clippy` | 3-OS matrix | `cargo clippy --workspace --all-targets --all-features -- -D warnings` (#3) | active |
| `test` | 3-OS matrix | `cargo test --workspace --all-features` (#4); `needs: [fmt, check]` (doc 10 §4.2 cheap-first ordering) | active |
| `boundary-check` | ubuntu | `cargo xtask check-architecture` (G5) | active — 1.E1 landed (Lead sync) |
| `schema-validation` | ubuntu | product.toml + settings schema + command-catalog drift (`cargo xtask codegen --check`) + fixtures drift (`cargo xtask fixtures --check`) + feature-matrix v0 (G1/G6) | active — 1.E2 landed, catalog-drift step added at 1.D4 merge, fixtures-drift step added at 2.A3 merge (Lead sync) |
| `wasm-build` | ubuntu | wasm32 build + size report (G12) | active — 1.E3 landed, lives in `wasm.yml` |
| `cargo-deny` | ubuntu | `cargo deny check …` (G3/G4) | active — 1.E5 landed |

Matrices use `fail-fast: false` (doc 10 §4.2). Concurrency: per-branch group
with `cancel-in-progress` on pull-request events only (doc 10 §10.1);
`main` and merge-queue runs never cancel. Triggers: `push` (main),
`pull_request`, `merge_group` (doc 10 §4.4 merge queue), `workflow_dispatch`.

## Stub activation protocol

Each stub job **runs** but its placeholder step always exits 0 — always
green, gates nothing (actionlint rejects `if: false` constant conditions,
and doc 10 §10.5 mandates a clean actionlint pass; hence the exit-0 form of
the Lead's stub spec). Job names carry a visible `[STUB until <card> …]`
suffix. `continue-on-error: false` is already set, so activation is:
**(1)** replace the placeholder step with the real command, **(2)** add the
`dtolnay/rust-toolchain` + `Swatinem/rust-cache` steps (copy from `check`),
**(3)** drop the `[STUB]` suffix from `name:`. The activating cards: 1.E1
boundary-check, 1.E2 schema-validation, 1.E3 wasm-build (owns `wasm.yml` —
may supersede its stub here), 1.E5 cargo-deny. **Do not add a stub check to
required checks (branch protection) until it is activated.**

## Action pins

Pinned by full commit SHA; versions resolved via `git ls-remote`
(2026-10-19). Keep this table and the header table in `ci.yml` in sync on
every bump.

| Action | Version | Commit SHA | Why |
|---|---|---|---|
| `actions/checkout` | v7.0.1 | `3d3c42e5aac5ba805825da76410c181273ba90b1` | official GitHub checkout |
| `dtolnay/rust-toolchain` | `1.99.1` branch | `d124f86f28ff42855784ff3319f48ac182c248f8` | installs exactly the rust-toolchain.toml pin (the per-version branch hard-codes it); bootstraps rustup where the image lacks it (ubuntu-24.04 images ship no Rust — verified 2026-10-19) |
| `Swatinem/rust-cache` | v2.9.2 | `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` | doc 10 §10.1 cache (keys on `Cargo.lock` + `rust-toolchain.toml` + job) |
| `actions/upload-artifact` | v7.0.2 | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` | official artifact upload (doc 10 §4.5 evidence) |

actionlint is not an action: the `actionlint` job curls the pinned
download script from the `v1.7.12` tag and installs binary 1.7.12 to `/tmp`.

## Design decisions and open items

- **Toolchain drift guard.** The toolchain pin lives in rust-toolchain.toml
  (source of truth) *and* in the action ref. Every Rust job asserts
  pin == action-installed == effective `rustc`, so a bump that misses either
  side — or a runner `RUSTUP_TOOLCHAIN` override (env vars beat toolchain
  files in rustup) — fails loudly instead of silently testing the wrong
  toolchain.
- **`--locked` posture.** Doc 10 §2.1 gate strings are verbatim (doc 10
  §10.4 parity: CI and `cargo xtask gate` run identical strings); doc 11
  §3.7 ("builds are `--locked` in CI") is honored by the `deps` job's
  `cargo metadata --locked` assertion on every run. `[PROPOSAL]` the strong
  form — routing CI jobs through `cargo xtask ci <job>` (doc 10 §10.5) —
  resolves the tension permanently once 1.A3 lands.
- **Runner labels.** `-latest` per Lead instruction; doc 10 §4.1 proposes
  `ubuntu-24.04` / `windows-2025` / `macos-15` as a budget decision with
  doc 15. One-line matrix edit to switch.
- **Evidence uploads.** Only `deps` uploads evidence today; the full
  `evidence-<job>-<os>-<run_id>` coverage of doc 10 §4.5 lands with 1.E2
  (needs junit-style test output — nextest is `[PROPOSAL]`).
- **Required checks.** Branch protection (doc 10 §4.4) is Lead-only repo
  settings; do it after the first green run on a remote exists.
