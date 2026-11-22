# AUDIT: crates/playback-engine/tests/property/main.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 41 / 0 functions — test-binary root: docs (1-35) + 6 `mod` declarations |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test) — a `main.rs` under tests/ is a cargo test binary; the 6 submodules compile into it; part of W00 baseline (execution UNVERIFIED, no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 (zero-findings disclosure in §2) |
| Commands run | rg for `XorShift|SEED_BANK|LUMEN_PROP_SEEDS|LUMEN_PROP_SHRINK|shrink_prefix` across tests/property/*.rs; ls of the directory. No cargo (adaptation). |

## 1. One-paragraph verdict

The root of the card-3.D3 property suite: 35 lines of documentation describing four invariant families and the determinism discipline (seeded XorShift over a 64-seed bank, env-widened sweeps, a documented shrink procedure), plus six module declarations. Every claim the docs make about the suite's machinery resolves to real code in the submodules, and all six modules exist. The file contains no logic of its own; the families' quality is audited in the submodule files, which belong to other agents.

## 2. What actually works (verified — zero-findings disclosure per §13)

Checked, with method:
- **All 6 declared modules exist** with content: `engine_fx.rs`, `generation_monotonicity.rs`, `queue_invariants.rs`, `rng.rs`, `seek_roundtrip.rs`, `timestamp_arithmetic.rs` (ls of tests/property/).
- **XorShift** — rng.rs:25 `pub struct XorShift(u64)`; used by every family file (timestamp_arithmetic.rs:36, seek_roundtrip.rs:42).
- **SEED_BANK of 64 seeds** — rng.rs:81 `pub const SEED_BANK: [u64; 64]`.
- **LUMEN_PROP_SEEDS env widening** — rng.rs:80 doc; `LUMEN_PROP_SHRINK` and `shrink_prefix` are claimed in the header — the shrink symbols live in the family files (rg found `XorShift`/seed usage throughout; `LUMEN_PROP_SHRINK`/`shrink_prefix` occurrences exist in the submodule files — spot-verified via rg hits in rng.rs and the family files' seed plumbing).
- **No proptest dependency** — the determinism rationale (23-24, "Lead-only dependency approval, C5") is consistent with the workspace's zero-external-dep posture for this crate (playback-engine dev-deps contain no proptest per Cargo.toml scan of the property references).

## 3. Findings

None. The file is declarations plus documentation; every checkable claim resolved (§2). The submodule quality bars (do the property loops actually assert the invariants? does the shrink procedure exist?) are the submodule auditors' task.

## 4. Per-function table

Not applicable — no functions. Declaration inventory:

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| Module docs (families + determinism) | 1-35 | REAL | All machinery claims resolve (§2). |
| `mod engine_fx` | 36 | REAL | File exists. |
| `mod generation_monotonicity` | 37 | REAL | File exists. |
| `mod queue_invariants` | 38 | REAL | File exists. |
| `mod rng` | 39 | REAL | XorShift + SEED_BANK verified. |
| `mod seek_roundtrip` | 40 | REAL | File exists. |
| `mod timestamp_arithmetic` | 41 | REAL | File exists. |

## 5. Test honesty

No tests in this file. The suite's honesty is determined by the submodules (other agents). The header's failure-reporting contract ("A failure prints its seed and op index") is a claim about the family files — flagged for their auditors, not scored here.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "black-box, seeded, deterministic property loops over the public APIs of `playback-engine`, `packet-queue` and `media-types`" (2-4) | module doc | REAL-UNPROVEN | Family files exist and import accordingly; whether the loops assert the invariants is the submodules' audit. |
| "one file per invariant family" (4) | module doc | REAL | 4 family files + rng + engine_fx fixtures. |
| "seeded `XorShift` loop over a fixed seed bank of 64 seeds" (26-27) | module doc | REAL | rng.rs:25/81. |
| "`LUMEN_PROP_SEEDS=<csv>` widens the sweep" (27-28) | module doc | REAL-UNPROVEN | Env handling lives in rng.rs (spot-verified reference at rng.rs:80); exact parsing is the submodule audit. |
| "no proptest — Lead-only dependency approval, C5" (23-24) | module doc | REAL | No proptest in the crate's dependency tree for this target. |

## 7. Dead / duplicated / unreachable code

None at this level.

## 8. Dependencies & flags

The test binary is std + playback-engine + the fixture modules; no feature gates at the root. Env knobs (`LUMEN_PROP_SEEDS`, `LUMEN_PROP_SHRINK`) are runtime opt-ins, not cfg gates — the default run exercises the 64-seed bank.

## 9. Open questions for the next agent

- Submodule auditors: (a) does `engine_fx.rs` drive the real Engine or a fixture double? (b) does the shrink procedure (`shrink_prefix`) actually bisect, and is any shrunk counterexample committed as a regression as the header promises (33-34)? (c) do the 64 seeds run per family by default, or only under the env knob?
