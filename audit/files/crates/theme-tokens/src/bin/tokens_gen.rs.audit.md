# AUDIT: crates/theme-tokens/src/bin/tokens_gen.rs
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines / functions | 311 / 9 (main, run, parse_args, value_of, output_paths, find_workspace_root, drift, write_file, usage_error) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO by design (codegen tool) — reachable via `cargo xtask tokens` subprocess bridge (tools/xtask/src/cmd/tokens.rs:61) and `cargo run -p theme-tokens --bin tokens-gen`; EXECUTED directly (`./target/debug/tokens-gen --check` → exit 0). Its output tokens.rs is `include_str!`-equivalent cargo of the shipped theme-tokens lib. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/tokens-gen --check` (exit 0, counts printed); `--help` (exit 0); `--bogus` (exit 2); `--check --out-dir X --css-out Y` (exit 2, conflict caught); `--out-dir /tmp/tg-test` regen + `diff -q` vs both committed artifacts (identical); `cargo test -p theme-tokens --offline` (38 tests, all ok) |

## 1. One-paragraph verdict
A small, honest, working generator CLI. It parses themes/tokens.json through the crate's real `parse`, emits CSS + Rust through the real `emit`, supports a real drift check with correct exit codes (0/1/2 as documented), and its committed outputs are byte-identical to regeneration — verified by execution, not just by the test suite. Error handling is typed (`Failure` + exit code), no unwrap/expect/panic anywhere. The only defect found is a flag-parsing wart where a missing option value silently swallows the next flag.

## 2. What actually works (verified)
— `./target/debug/tokens-gen --check` → `tokens-gen: no drift — outputs match …/themes/tokens.json` + `123 primitive / 86 alias / 86 semantic tokens; 2 mode(s) (dark default); 9 accent(s) (orange default)`, exit 0.
— Regeneration to a scratch dir then `diff -q` → both `tokens.css` and `tokens.rs` byte-identical to the committed artifacts (idempotence proven by execution).
— `--bogus` → exit 2 `unexpected argument`; `--out-dir` + `--css-out` → exit 2 conflict error; `--help` prints the usage. Exit-code contract (0 success / 1 drift / 2 usage+IO) matches the doc comment at L19-21.
— `cargo test -p theme-tokens --offline` → 38 passed, 0 failed (unit 22, diff 2, pipeline 12, semantic_lint 2).

## 3. Findings
F-001 [S4]
Where: crates/theme-tokens/src/bin/tokens_gen.rs:189-199, 215-219
Code (189-193, verbatim):
```
            "--source" | "--css-out" | "--rust-out" | "--out-dir" => {
                let value = value_of(args, i, arg)?;
                let path = PathBuf::from(value);
                match arg {
                    "--source" => options.source = Some(path),
```
Code (215-219, verbatim):
```
/// The value following the flag at `index` (already validated to exist).
fn value_of<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, Failure> {
    args.get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| usage_error(format!("missing value for {flag}")))
```
What is wrong: `value_of` only checks that an argument EXISTS, not that it isn't itself a flag. Executed: `./target/debug/tokens-gen --source --check` → `tokens-gen: reading --check: No such file or directory`, exit 2 — the user's `--check` was silently eaten as a path. Why it matters: a mistyped invocation produces a confusing I/O error instead of a usage error; with `--check` swallowed, a CI script could write outputs where it meant to verify. What correct looks like: reject values beginning with `--` for these four options. Confidence: HIGH — Verification: ran (output pasted).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 67-75 | REAL | — | maps Result to ExitCode, single eprintln |
| run | 77-163 | REAL | — | full pipeline; --check vs write; counts printed (executed) |
| parse_args | 174-212 | REAL | S4 | flag-value swallowing (F-001); conflict check works (executed) |
| value_of | 215-219 | REAL | S4 | exists-check only (F-001) |
| output_paths | 222-238 | REAL | — | defaults match committed layout (verified by regen diff) |
| find_workspace_root | 242-265 | REAL | — | walks up to a Cargo.toml containing `[workspace]`; substring check could false-positive on a stray manifest, none exists in tree |
| drift | 269-296 | REAL | — | missing file counts as drift (stderr note, exit 1 path); first-diff line heuristic sound |
| write_file | 298-311 | REAL | — | create_dir_all + typed IO error |
| usage_error | 60-65 | REAL | — | code 2 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tests/diff.rs (2) | YES | runs the real generator against committed artifacts and asserts no drift (I also ran the binary directly — same result) |
| tests/pipeline.rs (12) | YES | parse→emit round-trips over real fixtures |
| tests/semantic_lint.rs (2) | YES | consumes fixtures/semantic_lint/clean.css (my file #11) |
| No test covers | — | flag-value edge (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "emits BOTH committed outputs deterministically" | L4-8 | REAL | regen diff identical (ran) |
| "Exit codes: 0 success (or no drift with --check), 1 drift with --check, 2 usage, I/O or parse/validation error" | L19-21 | REAL | exit 0 (check), 2 (bogus arg, conflict) executed; exit 1 path exercised by tests/diff.rs |
| "The (frozen) xtask `tokens` subcommand shells out to this binary" | L10-13 | REAL | tools/xtask/src/cmd/tokens.rs:61 spawns it (read) |

## 7. Dead / duplicated / unreachable code
None. All functions called; no `#[allow]`; no dead branches found.

## 8. Dependencies & flags
Depends only on the parent crate's `parse`/`emit` (no external crates — verified: no `use` of third-party libs beyond std). No feature gates.

## 9. Open questions for the next agent
- None for this file. The lib-side parse.rs/emit.rs/model.rs/lint.rs belong to another agent's scope; my execution covers them only through the binary's end-to-end behavior.
