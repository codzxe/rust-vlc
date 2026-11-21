# AUDIT: rust-toolchain.toml
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-21 |
| Lines / functions | 12 / 3 config keys (channel, profile, components) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (build configuration). Effective on every rustup-invoked command in the repo; honored by all 26 baseline runs (INDEX.md: "Rust 1.99.1 (repo pin)") and asserted by 4 CI workflows |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep `dtolnay/rust-toolchain@` in .github/workflows/*.yml (4 gate workflows pin d124f86f… "1.99.1 branch", each with a 3-way assert; audit.yml uses nightly deliberately for Miri/Kani formal lanes) · grep `wasm32\|--target\|target add` in workflows+scripts (wasm lanes self-install targets: ci.yml:570, compat-matrix.yml:125) · grep `cargo doc\b` in workflows/scripts/gate.rs (zero — minimal profile needs no extra components beyond fmt/clippy) · sed -n '142,152p' ci.yml (assert drops the build hash) · sed -n '1,11p' rust-toolchain.toml (quote verify) · grep `1\.99` in docs/README/AGENTS (no divergent version claims) |

## 1. One-paragraph verdict
A correct, minimal toolchain pin: exact channel (not a drifting alias),
minimal profile, and exactly the two components the committed gates need
(rustfmt, clippy — both proven in use by baseline runs #5/#7). Pin consistency
is enforced unusually well: four gate workflows (ci, ci-full, release,
compat-matrix) install the same dtolnay action ref AND run a three-way assert
(channel == action == effective rustc). The one defect is precision theater:
the header records the rustc build hash (58cf8f9c3) as the verified identity,
but the CI asserts compare only the version string — every workflow's sed
discards the `(hash date)` part of `rustc --version` — so the recorded
hash-level identity is claimed, never checked.

## 2. What actually works (verified)
— Pin honored by the baseline: INDEX.md line 5 "Rust 1.99.1 (repo pin)";
  all 26 baseline commands ran under it, including `--locked` builds (#1-#4)
  which also validate the Cargo.lock v4 ↔ toolchain pairing.
— Components sufficient: `cargo fmt --check` (baseline #7 — ran, found 1
  dirty file, i.e., rustfmt present and executing) and `cargo clippy …
  -D warnings` (baseline #5/#6 — clean, clippy present). No committed lane
  needs any other component (grep `cargo doc` in workflows/scripts/gate.rs:
  zero; wasm lanes self-install their target at ci.yml:570 /
  compat-matrix.yml:125; audit.yml's nightly lanes declare their own
  components `miri, rust-src`).
— Consistency guards: ci.yml:141-152, ci-full.yml:92-102, release.yml:163-173,
  compat-matrix.yml:111-117 all pin
  `dtolnay/rust-toolchain@d124f86f28ff42855784ff3319f48ac182c248f8` ("1.99.1
  branch — matches rust-toolchain.toml") and fail on mismatch — a bump of the
  pin without the workflows (or vice versa) cannot pass CI.
— Edition 2024 crates (all manifests audited this wave) require rustc ≥1.85;
  1.99.1 satisfies. The `cargo::rustc-check-cfg` build-script syntax
  (crates/_template/build.rs) requires cargo ≥1.77; satisfied.

## 3. Findings
### F-001 [S4] — recorded build hash is never checked; asserts pin the version only
Where: rust-toolchain.toml:2-4 (the claim), .github/workflows/ci.yml:145 (the gap)
Code:
```toml
# Verified on the Phase-1 host (2026-10-19):
#   rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1,
#   rustfmt 1.10.0-stable (58cf8f9c35 2026-10-13), clippy 0.1.99.
```
What is wrong: The header documents hash-level toolchain identity
(58cf8f9c3). Every CI assert extracts only the version:
ci.yml:145 `eff="$(rustc --version | sed -n 's/^rustc \([^ ]*\) (.*)$/\1/p')"`
— the `\([^ ]*\)` group stops at the space before `(58cf8f9c3 2026-10-13)`,
so the parenthesized build hash is discarded. A toolchain reporting
`rustc 1.99.1 (anythingelse anydate)` passes all four asserts.
Why it matters: The file implies reproducible build identity ("Verified …
58cf8f9c3"), but nothing verifies it; the claim is decorative. For an audit
that treats evidence claims as findings, a recorded-but-unchecked identity
marker is exactly the kind of paper assurance to flag.
What correct looks like: Either drop the hash from the header (version-only
pin, honestly enforced), or extend the asserts to compare the full
`rustc --version` output including the hash (the header already records the
expected string).
Confidence: HIGH — Verification: read (sed extraction pattern verified in
ci.yml; same pattern in ci-full/release/compat-matrix).

## 4. Per-key table
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `channel = "1.99.1"` | 9 | REAL | — | Exact version pin (not stable/rolling); honored by baseline; enforced 3-way in 4 workflows |
| `profile = "minimal"` | 10 | REAL | — | rustc+std+cargo only; rustdoc ships inside rustc, so `cargo test` doctests work (baseline #3 ran the full workspace test suite incl. doctests under this profile) |
| `components = ["rustfmt", "clippy"]` | 11 | REAL | — | Exactly the gate tools; both exercised in baseline #5/#7; nothing else needed by any committed lane (grep-verified) |

## 5. Test honesty
No tests. The de-facto test is the 4-workflow assert block (quoted in §3),
which tests version equality and nothing else — see F-001.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1, rustfmt 1.10.0-stable, clippy 0.1.99" | rust-toolchain.toml:2-4 | REAL-UNPROVEN | Version verified by baseline pin + CI asserts; the build hash and component versions have NO committed check or run output (the baseline .log files that might have recorded `rustc -vV` were never committed — see my .gitignore audit F-001) |
| "profile/components guarantee the doc 10 §2 gate tools (fmt, clippy) are present on a fresh rustup install" | rust-toolchain.toml:5-6 | REAL | Standard rustup semantics + both tools proven executing in baseline #5/#7 |
| "Lead-owned" (doc 10 §2.5) | rust-toolchain.toml:1 | REAL-UNPROVEN | Process claim; cannot be verified from code |

## 7. Dead / duplicated / unreachable code
None. Three keys, all live. audit.yml's `dtolnay/rust-toolchain@nightly`
(audit.yml:266,335,412) is NOT a violation of this pin: those are Tier-4
formal/UB evidence lanes (Miri/Kani) that require nightly, documented as such
and running with `continue-on-error: true` (audit.yml:259) — the gate
workflows all use the pin.

## 8. Dependencies & flags
No cargo dependencies. Interacts with: Cargo.lock v4 (needs cargo ≥1.78 ✓),
edition 2024 manifests (need ≥1.85 ✓), `cargo::` build-script syntax in
_template (needs ≥1.77 ✓), wasm lanes (install their own targets ✓), nightly
formal lanes (self-contained ✓). No feature flags implicated by the pin.

## 9. Open questions for the next agent
- Wave 17 (scripts/CI auditor): confirm whether `scripts/full_gate.sh` also
  runs under the pin or trusts the ambient toolchain (its hard-coded Lead
  path is already a recorded S3 in INDEX.md tooling notes).
- If the Lead ever bumps the channel, four workflow action refs must move in
  lockstep — the asserts will catch a partial bump (good), but only at
  version granularity (F-001).
