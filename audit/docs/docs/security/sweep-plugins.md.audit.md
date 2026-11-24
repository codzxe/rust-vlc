# CLAIM AUDIT: docs/security/sweep-plugins.md
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Claims extracted | 11 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 7 REAL / 4 REAL-UNPROVEN / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A security-sweep report whose code-facing claims are ALL verified in the tree: all five named regression tests exist (byte-verified test names + line numbers), the F1 bounded-slurp fix is literally in imports.rs (`file.take(cap + 1)`), the F2 threshold enforcement is in (`escalation_or_status` in the read_guest arms), fsjail's verify_open re-check exists, and — the most valuable honesty — the claim that `HOST_MAX_ACTIVE_INSTANCES`/`HOST_TOTAL_HEAP_BYTES` are declared-but-unwired is TRUE: zero callers outside limits.rs. This doc discloses its own unfixed caps rather than claiming them enforced. Only the "50/50 green" run count is historical (the tree now carries 26 integration + ~44 lib tests — later waves added crash-isolation/conformance tests).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Regression: fs_open_slurp_is_bounded" | L49-50 | REAL | crates/plugin-host-wasm/tests/plugin_sandbox_security.rs:688 (grep, ran) |
| 2 | "Regression: oob_probes_trip_the_disable_threshold" | L65-66 | REAL | plugin_sandbox_security.rs:641 (grep, ran) |
| 3 | "Regression: devices_list_is_bucket_metered" | L75-76 | REAL | plugin_sandbox_security.rs:748 (grep, ran) |
| 4 | "swap_between_resolve_and_open_is_denied + unswapped_path_passes_reverification" | L87-89 | REAL | crates/plugin-host-wasm/src/fsjail.rs:326 + :370 (grep, ran) |
| 5 | "read through `Read::take(cap + 1)` so at most cap+1 bytes ever cross" | L47-48 | REAL | imports.rs:511 `let mut bounded = file.take(cap + 1);` (grep, ran) |
| 6 | "read_guest returns ReadErr::{Status, Fault}; escalation_or_status faults at threshold" | L61-63 | REAL | imports.rs:125,140,160 — escalation_or_status wired into both OOB arms (grep, ran) |
| 7 | "HOST_MAX_ACTIVE_INSTANCES (4) and HOST_TOTAL_HEAP_BYTES (256 MiB) ... no in-tree caller enforces them" | L110-114 | REAL | limits.rs:86/89 declare them; repo-wide grep for callers outside limits.rs -> ZERO (ran). Honest unwired-cap disclosure — the doc says the emperor has no clothes and the tree agrees |
| 8 | "PathJail::verify_open re-canonicalizes the same path after File::open ... before any byte is read" | L83-86 | REAL | fsjail.rs:160 `pub fn verify_open` + module doc L16 (grep, ran) |
| 9 | "cargo test -p plugin-host-wasm — ACTUAL: 50/50 green (35 lib, 2 hello-world, 4 install-lifecycle, 9 sandbox-security)" | L16-18 | REAL-UNPROVEN (historical) | not re-run (cargo budget); breakdown matches the then-existing files; today's tree has 5 integration files / 26 integration tests + crash_isolation.rs (added after) — count is stale, not fabricated |
| 10 | "Extension signature verification (ed25519) ... today's status is honestly `Unverified`" | L128-129 | REAL (admission) | honest status disclosed instead of claimed |
| 11 | "residual window ... needs openat/O_NOFOLLOW — libc, MP §9 ADR territory → 8.A2" | L90-92 | REAL | the fsjail.rs module doc carries the [VERIFY] window (read) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| — | — | none found |

## 4. Fabrication / inflation indicators
NONE. The doc's value is in what it refuses to claim: unwired caps (L110-114), unresolved TOCTOU residual (L90-92), Unverified signature status (L128-129) — all three verified true in code.

## 5. Matrix rows
N/A.
