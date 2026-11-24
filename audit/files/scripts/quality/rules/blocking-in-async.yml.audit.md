# AUDIT: scripts/quality/rules/blocking-in-async.yml
| Field | Value |
|---|---|
| Auditor | W17-A08 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines / functions | 22 / 1 ast-grep rule (async-fn containment + 7 call patterns) |
| File verdict | REAL-UNPROVEN (sound shape; zero possible hits on this tree; execution impossible here) |
| Reachable from shipped entry point? | NO — CI audit lane only: 03_astgrep.sh:17-25 scans it; 99_run_all.sh:31 (static) + :44 (quality), both `J` reporter lanes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | yaml.safe_load -> OK; `rg 'async fn' crates apps tools` -> 3 hits, 2 are doc comments (1 real fn); body scan of that fn -> 0 of 7 patterns; fs-call-site census -> 305 fully-qualified vs 70 short-form; `.recv()` census -> 6 sites, all sync/test code |

## 1. One-paragraph verdict
A structurally correct async-blocking detector whose entire live target surface is ONE async function — the whole tree has exactly one real `async fn` (the other two `rg` hits are doc comments) — and that function's only blocking call is a std Mutex lock taken via `.lock().map_err(...)?`, a form the rule's `$M.lock().unwrap()` pattern cannot match. On the current tree this rule can never fire, including on the one place its own message ("stalls the executor thread") describes. The fs patterns additionally see only fully-qualified call sites while 70 of the repo's 375 fs calls use the short `fs::read*` form.

## 2. What actually works (verified)
— verified by `python3 yaml.safe_load` -> OK: `all: [inside async fn, any: 7 patterns]`; severity `error`.
— verified by `rg 'async fn' crates apps tools --glob '*.rs'` -> 3 lines, of which 2 are `//!` doc comments (apps/desktop/src/dispatch.rs:22, apps/desktop-tauri/src/app.rs:13); the single real async fn is apps/desktop-tauri/src/app.rs:145 `async fn lumen_dispatch(`.
— verified by reading that fn's body (:145-154): no sleep/fs/TcpStream/recv/lock().unwrap call — the `inside function_modifiers regex async` containment shape is the right structural approach for this check.
— verified wiring: 03_astgrep.sh:17-25 scans the rule over `crates apps tools tests` with not_run on failure (11 rules in the dir, this one among them).

## 3. Findings
F-001 [S3]
Where: scripts/quality/rules/blocking-in-async.yml:16-22 vs apps/desktop-tauri/src/app.rs:150
Code: `        - pattern: std::fs::read($$$)` ... `        - pattern: $M.lock().unwrap()` / app.rs:150 `    let mut runtime = state.0.lock().map_err(|_| ShellError::StatePoisoned)?;`
What is wrong: the rule's policy ("Blocking call inside an async fn") is violated by the tree's only async fn — a std Mutex lock taken and held across `runtime.dispatch(&request)` and `forward_pending` (webview emit under the lock) — but the pattern list matches only `.lock().unwrap()`, not `.lock().map_err(..)?`/`.expect(..)`/`let Ok(x) = .. else`. The one real blocking-in-async instance in the repo is invisible to the rule. (The lock-across-emit is a documented deliberate decision in the code's own comments — but the rule is not the thing enforcing it, and it sees nothing.) Additionally the three `std::fs::*` patterns match only fully-qualified call sites; a census shows 305 fully-qualified vs 70 short-form `fs::read*` call sites in the tree (e.g. tools/xtask/src/cmd/rebrand.rs:680) — any short-form fs call inside async code is also invisible.
Confidence: HIGH (pattern text + code text both quoted; ast-grep concrete-syntax matching of the non-unwrap forms ruled out by pattern shape — execution itself UNVERIFIED, ast-grep absent).

F-002 [S4]
Where: scripts/quality/rules/blocking-in-async.yml:1-22 (vs tree)
Code: (rule as a whole)
What is wrong: dead-surface rule — the repo is sync-by-design (std-only posture); exactly one real async fn exists, and it contains zero matching patterns, so the rule's maximum possible hit count on this tree is 0. Quality-tier capacity spent guarding a programming style the repo does not use.
Confidence: HIGH — Verification: rg census (3 hits, 2 comments) + body read.

F-003 [S4]
Where: scripts/quality/rules/blocking-in-async.yml:3
Code: `severity: error`
What is wrong: decorative — 03_astgrep.sh never reads severity; nothing distinguishes error from warning (this rule says `error`, panic-in-lib.yml says `warning`, both equally meaningless downstream).
Confidence: HIGH — Verification: read (consumer script read in full).

F-004 [S4]
Where: scripts/quality/rules/blocking-in-async.yml:21
Code: `        - pattern: $CH.recv()`
What is wrong: `$CH.recv()` also matches async-safe receives (`rx.recv().await` on tokio/async-channel) — a canonical correct async pattern would be flagged as blocking. Latent here (the tree's 6 `.recv()` sites are all sync test/wasapi code — verified by census), but the pattern cannot distinguish blocking from async receivers.
Confidence: MEDIUM — Verification: read + census; ast-grep execution UNVERIFIED.

## 4. Test honesty
No tests, no committed astgrep json output (gitignored per W17-A06's systemic finding). Rule execution UNVERIFIED (ast-grep absent in sandbox; `/usr/bin/sg` is util-linux — W17-A02's verification).

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Blocking call inside an async fn: stalls the executor thread..." | :4-6 | Accurate as advice, SHALLOW as enforcement | the tree's one violation (app.rs:150 lock) is invisible to the patterns (F-001) |

## 6. Dead / duplicated / unreachable code
Whole rule is dead-surface on this tree (F-002). No dead keys in the YAML itself.

## 7. Dependencies & flags
ast-grep 0.51.3 (pinned; sg_run npx fallback). Consumed by 03_astgrep.sh in static+quality J tiers. `await_holding_lock = "deny"` (root Cargo.toml:66) covers only locks held across `.await` — app.rs's async fn awaits nothing, so clippy's related deny also sees nothing.

## 8. Open questions for the next agent
- None material. If the repo ever gains real async surface, F-001's pattern blind spots become live.
