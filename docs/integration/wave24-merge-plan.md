# Wave 24 integration — merge plan & battery record

Date: 2026-11-11 · Integrated on `w24-integration-rehearsal` (39+2 merges) · Battery executed by the Lead.

## Merge order (executed)

b (soak #54) → c (packaging #56) → d (compat #53) → e (docs #58) → f (phase7 gate #51)
→ g (gate fixes) → h (packaging fixes) → i (docs fixes) → j (docs fixes 2) → k (gate fixes 2)
→ l (compat audit) → m (soak fixes) → n (g nits) → p (compat fixes) → q (packaging P4s)
→ r1–r14, r16 (review reports) → s (IN-6 fix) → t (m+q audit) → u (ps1 fix) → v (gate addendum)
→ w (plan-doc fix) → x (p worklog) → y (packaging follow-ups) → ac (follow-up ledger)
→ a (web matrix + budgets #49 — last implementation lane)

Conflict resolutions (all unions): worklog.md lane-order unions (2 marker-cleanup
amends); xtask cmd registry keeps both compat-matrix + web-budget; full_gate.sh
steps 10 (web-budget) + 11 (packaging) in numeric order; .gitignore target-xtask
dedupe; feature-matrix.toml kept both lanes' row changes.

## Integration fixes (Lead, committed on the rehearsal branch)

1. `tests/integration/first-vertical/src/scenarios.rs` — lumen_binary_path() honors
   CARGO_TARGET_DIR (relocated shared cache; the OF-4 hermetic rule).
2. `tests/rebrand/rebrand_skeleton.rs` — README rewrite count + totals line derived
   from the source README (case-insensitive), not pinned (OF-2 derivation principle;
   lane E's README truth pass changed 7→6).
3. `apps/web/src/host.rs` — the wasm32 decoder-seam refusal test is
   cfg(not(builtin-codecs)): decoder-less builds refuse typed; tree-shake builds
   with builtin codecs decode by design (7.K1).

## Battery results (every command, exit status)

| # | Check | Result |
|---|---|---|
| 1 | `cargo fmt --all --check` | CLEAN |
| 2 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | CLEAN (1m53s) |
| 3 | `cargo test --workspace --all-features` | **4488 passed / 0 failed / 0 ignored** (piecewise: 4139 excl. playback-engine + 349 playback-engine — the sandbox's 10-min call cap forces the split; both halves green) |
| 4 | `cargo build -p lumen-web -p theme-tokens --target wasm32-unknown-unknown --locked` | PASS |
| 5 | xtask check-architecture | PASS |
| 6 | xtask codegen --check | PASS |
| 7 | xtask schema-gen --check | PASS |
| 8 | xtask fixtures --check | PASS |
| 9 | scripts/fuzz_registry_check.sh | PASS |
| 10 | xtask lint-literals | PASS |
| 11 | xtask check-identity | PASS |
| 12 | xtask sbom --check | PASS |
| 13 | xtask web-budget --check (NEW — full_gate step 10) | PASS (5 rows, no breach, no growth) |
| 14 | scripts/packaging/gate.sh (full_gate step 11) | PASS |
| 15 | ui: tsc --noEmit | 0 errors |
| 16 | ui: eslint | 0 errors (28 pre-existing warnings) |
| 17 | ui: vitest run | 718/718 passed (80 files) |
| 18 | ui: l10n:check | PASS (611 keys, 0 missing/dup/orphan) |
| 19 | ui: settings:check | OK (5 areas, 21 ui entries, 32 total) |
| 20 | workflows YAML (6 files) | all parse |

`full_gate.sh` itself: steps 1–2, 4–11 verified via the direct invocations above;
step 3 (the monolithic test run) exceeds the sandbox's 10-minute single-call cap,
so the suite was executed in two green halves — same coverage, honestly recorded.

## Known state (honest)

- CI Actions remain billing-blocked (0 steps ever executed; issues #53/#56 track) —
  this local battery is the verification vehicle.
- The consolidated follow-up ledger (`docs/reviews/wave24-followup-ledger.md`,
  merged from w24/ac) records 101 findings: 71 fixed+verified, 7 fixed (verification
  pending at ledger time — J/X/Y chains, since merged), 5 deferred with owners,
  13 unique open (top items closed by this integration: R11 P1 closed by lane A's
  final push; IN-2 closed by full_gate step 10; IN-5 closed by the web-budget gate
  + web-column pass).
- User-owned blockers (unchanged): CI billing, 3-OS runners, Apple R-35, Windows
  cert, ADR-010 patent sign-off.

## Lead-decision items executed at merge

- Merge order above; union resolutions as recorded.
- IN-6 (browser-matrix name drift): fixed by lanes S/V/W chain.
- IN-9 (.gitignore dedupe): fixed in-rehearsal.
- The 3 integration fixes above.
- Issue closes: #49, #51, #53, #54, #56, #58 (with evidence comments).
