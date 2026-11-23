# AUDIT: crates/commands/src/bin/catalog_gen.rs
| Field | Value |
|---|---|
| Auditor | W02-A02 |
| Date | 2025-06-27 |
| Lines / functions | 279 / 6 fns (bin, no tests) |
| File verdict | REAL (executed) |
| Reachable from shipped entry point? | NO (dev tool bin, not in product binaries) — YES from CI: .github/workflows/ci.yml:280 `cargo run -p xtask --locked -- codegen --check` -> tools/xtask/src/cmd/codegen.rs:67-73 spawns `catalog-gen` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `./target/debug/catalog-gen --check` (exit 0, "no drift … 65 commands / 26 events"); `--help` (exit 0); `--bogus` (exit 2); `--schema-out` missing value (exit 2); `--out-dir+--ts-out` conflict (exit 2); `--schema-out --check` (BUG DEMO — wrote files, exit 0); `--help --bogus` (exit 2, help not shown); `rg` for unwrap/panic (only unwrap_or_else); Read of full file |

## 1. One-paragraph verdict
Real and executed: I ran the already-built binary end-to-end. `--check` verifies all three committed artifacts against regeneration and reports no drift (exit 0); the documented exit-code contract (0/1/2) holds on every path I exercised, and the "no panics, no unwraps" header claim is true (rg finds only `unwrap_or_else`). The one behavioral defect is a proven CLI parsing hole: `value_of` swallows the next argument blindly, so `--schema-out --check` silently consumes `--check` as a PATH and the tool performs WRITE mode, creating a file literally named `--check` and rewriting the default outputs — the exact opposite of the operator's intent. I triggered this by accident during the audit (see incident note), which doubles as execution proof. Two smaller issues: `--check` mode generates the schema twice (run() computes `outputs` that check_against never uses), and `--help` loses to parse errors.

## 2. What actually works (verified)
- `./target/debug/catalog-gen --check` -> `catalog-gen: no drift — outputs match the catalog (65 commands / 26 events)`, exit 0 — the committed schema/TS/checksum artifacts recompute identically (also proves my accidental write did not change content).
- `./target/debug/catalog-gen --help` -> full usage text, exit 0.
- `./target/debug/catalog-gen --bogus` -> stderr `unexpected argument \`--bogus\` — see \`catalog-gen --help\``, exit 2.
- `./target/debug/catalog-gen --schema-out` -> `missing value for --schema-out`, exit 2.
- `./target/debug/catalog-gen --out-dir /tmp/x --ts-out /tmp/y` -> conflict error, exit 2.
- `rg "unwrap|expect\(|panic!|todo!|unimplemented!"` on the file -> only `unwrap_or_else` at 217/218/224 (documented claim at L23 holds).
- CI wiring: .github/workflows/ci.yml:280 runs the drift gate through xtask.

## 3. Findings
F-001 [S3]
Where: crates/commands/src/bin/catalog_gen.rs:193-197 (used at 163-172)
Code:
```
fn value_of<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, Failure> {
    args.get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| usage_error(format!("missing value for {flag}")))
}
```
What is wrong: No guard against consuming another flag as a value. `--schema-out --check` sets `schema_out = "--check"`, leaves `check = false`, and the tool enters WRITE mode: I executed exactly this and it printed `catalog-gen: wrote --check (4874 lines)` and rewrote the default TS path — a CI operator asking for a drift check instead gets regeneration, plus a junk file named `--check`. What correct looks like: reject a value that starts with `--` (with an escape hatch or clear error). Confidence: HIGH — Verification: ran (output above; stray `--check` file deleted, `--check` re-run clean).
INCIDENT NOTE: this accidental execution wrote a file outside audit/ (repo-root `--check`, plus byte-identical rewrites of ui/src/generated/commands.ts, schemas/commands.schema.json, schemas/generated/commands.checksum.txt). The stray file was deleted and `--check` re-run confirms no drift (committed content unchanged). Logged in audit/runs/W02-A02.log.md.

F-002 [S4]
Where: crates/commands/src/bin/catalog_gen.rs:92-95 (with codegen.rs:160-161)
Code:
```
    let outputs = generate_outputs().map_err(|message| Failure { code: 2, message })?;

    if options.check {
```
What is wrong: In `--check` mode `outputs` is never used — `check_against` regenerates internally (`let generated = generate_outputs()...`, codegen.rs:161). CI's drift gate therefore runs the full schemars schema generation twice per invocation. Why it matters: wasted CI time and a latent trap (someone "optimizing" by reusing `outputs` would change behavior none — but the duplication invites drift between the two call sites). What correct looks like: pass `&outputs` into check_against or only generate in the write branch. Confidence: HIGH — Verification: read of both files.

F-003 [S4]
Where: crates/commands/src/bin/catalog_gen.rs:82-87
Code:
```
    let args: Vec<String> = env::args().skip(1).collect();
    let options = parse_args(&args)?;
    if options.help {
```
What is wrong: `--help` is honored only after a fully successful parse: `catalog-gen --help --bogus` exits 2 with a usage error instead of printing help (executed: exit 2, help text not shown). Standard CLI behavior is that `--help` anywhere wins. Confidence: HIGH — Verification: ran.

F-004 [S4]
Where: crates/commands/src/bin/catalog_gen.rs:266-279
Code:
```
    fs::write(path, content).map_err(|e| Failure {
        code: 2,
        message: format!("writing {}: {e}", path.display()),
    })
```
What is wrong: Non-atomic artifact writes: direct `fs::write` (after create_dir_all) with no temp-file+rename. A crash or Ctrl-C mid-write leaves a truncated committed artifact; the next `--check` would then report confusing "drift" that is actually corruption. Dev-tool blast radius. What correct looks like: write to `<path>.tmp` then rename. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 71-79 | REAL | — | maps Failure to ExitCode + stderr; executed |
| run | 81-137 | REAL | S4 | check/write dispatch; double generation in check mode (F-002) |
| parse_args | 148-190 | REAL | S3 | flag/value loop; blind value consumption (F-001); last-wins on duplicates |
| value_of | 193-197 | REAL | S3 | no flag-value guard (F-001) |
| output_paths | 205-236 | REAL | — | Box::leak 'static, documented short-lived-process pattern |
| find_workspace_root | 240-264 | REAL | — | walk-up `[workspace]` marker; verified root Cargo.toml:35 matches; no `# [workspace]` comment false positives found (rg) |
| write_file | 266-279 | REAL | S4 | non-atomic write (F-004) |

## 5. Test honesty
No tests in this file. Indirect coverage: `crates/commands/tests/catalog_shape.rs::committed_checksum_recomputes` (baseline :1525 ok) exercises the same codegen functions the bin calls; the `--check` path itself has no automated test — my manual execution is the only evidence the flag works as documented.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Exit codes: 0 success (or no drift with --check), 1 drift with --check, 2 usage or I/O error" | catalog_gen.rs:21-22 | REAL | executed: 0 (--check clean, --help), 2 (bogus/missing/conflict); 1 not exercised (would need to introduce drift) — drift-exit-1 UNVERIFIED by execution but code path read at 99-108 |
| "All errors are typed and actionable on stderr; no panics, no unwraps" | catalog_gen.rs:22-23 | REAL | rg: only unwrap_or_else; all errors routed through Failure |
| "The (frozen) xtask codegen subcommand shells out to this binary" | catalog_gen.rs:12-15 | REAL | tools/xtask/src/cmd/codegen.rs:67-73; ci.yml:280 |
| "--out-dir … (conflicts with the --*-out flags)" | catalog_gen.rs:48-49 | REAL | conflict check 180-188; executed exit 2 |

## 7. Dead / duplicated / unreachable code
- `run()`'s `outputs` binding (L92) is dead in `--check` mode (F-002).
- No other dead code.

## 8. Dependencies & flags
Bin target `catalog-gen` in the `commands` crate (Cargo.toml [[bin]]). Uses only `commands::catalog::codegen` + std. No feature gates, no unsafe, no external deps. `lumen-allow-literal` markers at L35/L256 documented for CLI prose.

## 9. Open questions for the next agent
- Who audits `crates/commands/src/catalog/codegen.rs` itself (the emitter)? This bin is a thin shell around it; the TS-emitter and schema logic live there (not in my assignment).
- Exit code 1 (drift) path: no test or CI evidence in my scope that it fires correctly — mutating an artifact locally and running `--check` would prove it (I did not, to avoid touching committed files after the F-001 incident).
