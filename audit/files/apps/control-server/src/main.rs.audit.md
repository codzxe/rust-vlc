# AUDIT: apps/control-server/src/main.rs
| Field | Value |
|---|---|
| Auditor | W02-A09 |
| Date | 2025-06-27 |
| Lines / functions | 270 / 6 fns + 3 types/consts |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — control-server is not in the release lane (`cargo build --release -p lumen-cli -p desktop`; baseline builds only those), and product.toml:167 gates it `control-server = false # Phase 5+ [PROPOSAL]`. The binary itself is real: built and executed by me (see §2). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `cargo build -p control-server` → exit 0; `./target/debug/lumen-control-server --help` → exit 0; `--sessions 0` → exit 2 (feature gate); `--bogus` / `--sessions notanumber` / `--config` (missing value) → all exit 2; `cargo test -p commands`, `-p media-types` (context) |

## 1. One-paragraph verdict

A small, clean, honestly-scoped binary entry: strict argv parsing with typed
usage errors, product-model loading through the schema pipeline, the doc-07
F2 feature gate (which I verified fires: `control-server = false` in
product.toml ⇒ typed message + exit 2), loopback bind with typed exit 3, and
a serve loop capped by `--sessions`. I built and drove it myself — every
documented exit code behaved as contracted. The defects: the per-start auth
token is printed to STDERR unconditionally (doc 05 §5.4 specifies an opt-in
`--print-token`; the secret lands in scrollback/journals/CI logs), main.rs
carries zero automated tests while every sibling module has a test mod, the
default paths are CWD-relative, and an explicit `--config <missing>` silently
falls back to strict defaults instead of erroring. Cross-ref: on Windows the
token entropy ALWAYS degrades to the weak time/pid mix (token.rs has no
Windows path — recorded gap).

## 2. What actually works (verified)

— `cargo build -p control-server` → exit 0. `./target/debug/lumen-control-server --help`
→ full usage text, exit 0.
— `./target/debug/lumen-control-server --sessions 0` → STDERR
`the control server is not exposed by the product model (the feature flag is false); see product.toml [features] control-server — doc 07 rule F2`,
exit 2 (PIPESTATUS) — the F2 gate is enforced, matching the header contract
"exit 2 — … feature flag off".
— `--bogus`, `--sessions notanumber`, `--config` (trailing, no value) → all
exit 2 with the documented usage-error shape.
— Exit-code mapping audited: config.rs:72-80 `AppError::exit_code` returns 2
for Config/Policy/Product/FeatureDisabled, 3 for Bind — matches the header
contract exactly (:5-9).

## 3. Findings

F-001 [S3]
Where: apps/control-server/src/main.rs:231-242
Code:
```rust
    if token_generated {
        eprintln!(
            "control-server session token (keep private): {}",
            server.token()
        );
```
What is wrong: the token — the single secret that authorizes the entire
control surface — is printed to STDERR unconditionally whenever it is
generated. Doc 05 §5.4(c) specifies opt-in: "prints it to stdout with
`--print-token`". No flag exists. STDERR of a daemon typically lands in
terminal scrollback, the systemd journal, or CI logs; anyone who can read
those gets full command access (subject to the deny-by-default policy, but
every sensitive grant in control-server.toml becomes reachable).
Why it matters: an always-on secret print is strictly weaker than the
documented opt-in design; journals outlive terminal sessions.
What correct looks like: gate on `--print-token` (doc) or write the 0600
token file (doc option (a)); keep the weak-entropy warning either way.
Confidence: HIGH — Verification: read + ran (feature gate blocked a live
run; behavior of the print path is unconditional in code).
Note: the warning half at :236-241 (WeakFallback) is correct and useful.

F-002 [S3 — cross-file, surfaced here]
Where: crates/control-protocol/src/token.rs:35-41,64-71 (surfaced by main.rs:236-241)
Code:
```rust
/// 1. `/dev/urandom` (the OS CSPRNG) — the primary, Unix-only path.
/// 2. A fallback mix of wall-clock nanos, the process id and a
///    `RandomState`-derived hash — **weak** `[VERIFY]`: ...
/// The Windows entropy path joins with the platform adapter crates
```
What is wrong: `read_urandom` opens `/dev/urandom` — on Windows this ALWAYS
fails, so every Windows start of this server authenticates with the weak
time/pid/hash token. The gap is documented ("recorded gap, not a silent
assumption") and main.rs warns, but the default posture on an entire
platform is a predictable-ish token.
Why it matters: if the Phase-5+ feature is enabled on Windows without the
platform adapter landing first, auth strength silently drops to
guess-resistant-only.
What correct looks like: refuse to start (deny-by-default) when the OS
entropy path is unavailable, rather than degrade.
Confidence: HIGH — Verification: read (no Windows run possible in sandbox).
Severity: S3 today (feature flag off, loopback-only); would be S1/S2 if
shipped enabled on Windows.

F-003 [S4]
Where: apps/control-server/src/main.rs:60-105 (whole arg surface)
Code:
```rust
fn parse_args(argv: &[String]) -> Parsed {
```
What is wrong: `parse_args`, `value_after`, `usage`, `load_product` and
`run` have ZERO automated tests — every sibling module in the app
(config.rs:210, dispatch.rs:104, run.rs:70, logging.rs:202) has a test mod;
main.rs alone relies on manual exercise (mine, today).
Why it matters: argv parsing is where regressions ship to operators; the
`--sessions`-cap semantics and exit-code contract are testable pure logic.
What correct looks like: unit tests for Parsed outcomes (help/usage/args),
value-after edge cases, and the exit-code mapping (already pure).
Confidence: HIGH — Verification: rg "mod tests" across apps/control-server/src
→ 4 hits, none in main.rs.

F-004 [S4]
Where: apps/control-server/src/main.rs:182-201
Code:
```rust
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            control_protocol::policy::PolicyFile::default()
        }
```
What is wrong: an explicitly-passed `--config /typo/path.toml` that does not
exist silently starts with strict defaults instead of erroring. The
fallback direction is SAFE (defaults are loopback/generated-token/
deny-all-sensitive), and the usage text documents "missing = strict
defaults" — but it documents it for the DEFAULT path, not for explicit
paths.
Why it matters: operator intent ("use my config") is silently discarded; a
grant the operator expected is absent and they must notice via refusals.
What correct looks like: NotFound on an explicitly-passed path = exit 2;
NotFound on the default path = strict defaults.
Confidence: HIGH — Verification: read (path provenance is available before
this branch).

F-005 [S4]
Where: apps/control-server/src/main.rs:35-39
Code:
```rust
const DEFAULT_CONFIG: &str = "control-server.toml";
/// Default product model path (repo root / product working dir).
const DEFAULT_PRODUCT: &str = "product.toml";
const DEFAULT_SCHEMA: &str = "product.schema.json";
```
What is wrong: all defaults are CWD-relative. Launching the binary from any
directory other than the repo root exits 2 with a schema-read error unless
all three flags are supplied.
Why it matters: operator surprise; the doc-header contract ("exit 2 —
config/product trouble") fires for a mere working-directory choice.
What correct looks like: resolve defaults relative to the executable or an
env-configured config dir; or at least print "run from the repo root / pass
--schema" in the error.
Confidence: HIGH — Verification: read (paths are relative by construction;
not re-run from another cwd — the feature gate fires first in this repo).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DEFAULT_*` consts | 35-39 | REAL | S4 | CWD-relative defaults (F-005) |
| `Args` / `Parsed` | 42-57 | REAL | — | clean sum type |
| `value_after` | 60-66 | REAL | S4 | untested (F-003); correct index advance |
| `parse_args` | 69-105 | REAL | S4 | unknown flags ⇒ Usage; `--sessions` u32 with honest error; verified live: bogus/bad-value/missing-value all exit 2 |
| `usage` | 108-129 | REAL | — | accurate (matches live `--help` output); documents the loopback/token/deny posture |
| `load_product` | 132-156 | REAL | S4 | schema-validated product load; untested (F-003) |
| `run` | 161-262 | REAL | S3 | full startup chain verified live (gate/bind/token/serve); token print F-001; config fallback F-004 |
| `main` | 264-270 | REAL | — | exit only when non-zero — clean |

## 5. Test honesty

No tests in this file. Manual verification performed in lieu (§2): 6 live
invocations covering help/usage/gate/flag errors. Sibling-module tests
(config/dispatch/run/logging) and crates/control-protocol/tests/conformance.rs
cover the layers this binary composes; the composition itself is untested
(F-003).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exit 0 — --help, or the session cap was served" | main.rs:5 / doc 05 §5.4 | REAL | ran: `--help` exit 0; run.rs:48-53 cap return path + test `the_cap_stops_the_loop_after_serving_that_many_sessions` |
| "exit 2 — … feature flag off" | main.rs:7 | REAL | ran: exit 2 with the F2 message under product.toml control-server=false |
| "exit 3 — bind trouble (non-loopback refusals included)" | main.rs:8 | REAL-UNPROVEN | config.rs:79 Bind→3; loopback refusal proven by conformance suite (`loopback refusal` case list, control-protocol/tests/conformance.rs:16); not re-run by me |
| doc 05 §5.4 "prints it to stdout with --print-token" | docs/05:541 | FALSE | no flag; unconditional STDERR print (F-001) |
| doc 05 §5.4 token file 0600 / --control-token-fd | docs/05:541 | FALSE | absent (see doc-ledger C70) |
| "No signal handling (std-only): SIGTERM/SIGINT end the process" | main.rs:15-16 | REAL | no signal code; serve_loop docs reiterate (run.rs:7-9) |

## 7. Dead / duplicated / unreachable code

None in this file. Cross-ref dead code: `commands::ErrorCode::NotSeekable`
dead mapping at apps/cli/src/exit.rs (see error.rs audit §7).

## 8. Dependencies & flags

- Deps: control-protocol, control-server (lib), product-config — all
  in-tree; std-only otherwise. Product gate: `[features] control-server`
  (product.toml:167, false). No cargo feature gates on the app itself.
- Not in the release lane; not in packaging (rg packaging/: only the
  attack-surface script names it, scripts/quality/22_attack_surface.sh).

## 9. Open questions for the next agent

1. Phase-5 owner: before flipping `control-server = true`, land F-001
   (opt-in token print) and the Windows entropy path (F-002) — both are
   cheap now, expensive after external users exist.
2. Should `serve_loop`'s unlimited mode (`--sessions` absent) get an idle
   re-check of the feature flag / config? (Today config is read once at
   startup — acceptable, just confirming intent.)
3. rc-smoke does not exercise this binary (baseline rc-smoke.txt has no
   control-server section) — a smoke run of `--sessions 1` + a
   hello/welcome round-trip would close the last untested composition gap.
