# AUDIT: schemas/generated/settings-help.txt
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 442 / 0 (generated data file; 121 flags across 11 areas) |
| File verdict | FALSE (core claim "settings surface as long flags" contradicted by the shipped CLI — executed; per-area reference content itself is accurate and drift-free) |
| Reachable from shipped entry point? | NO at runtime — the CLI regenerates help from the embedded schema (`apps/cli/src/help.rs:156-160`), it never reads this file. Consumed by CI/tests (settings-schema integration `committed_outputs_have_no_drift`) and `xtask rebrand` (token rewrite list, rebrand.rs:672). |
| Findings | S0: 1 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/lumen --video-hue 50 open download/big_buck_bunny.mp4` -> `error: unknown command `--video-hue``, exit 64; `./target/debug/lumen --generate-manifest` -> `error: unknown command `--generate-manifest``, exit 64; `./target/debug/lumen --help` / `--help video` -> Areas block + video section byte-identical to golden (diff -> AREA-SECTION-IDENTICAL); `cargo test -p settings-schema --test integration` -> 8 passed, 0 failed (incl. `committed_outputs_have_no_drift`); awk per-area flag counts -> all 11 index rows accurate, total 121 |

## 1. One-paragraph verdict
This golden is accurately generated and provably in sync (the crate's own drift test passes; the Areas index counts match the flag lists machine-counted; the runtime `--help <area>` output is byte-identical to the per-area sections). But the file's central promise — and the promise the shipped binary prints — is false: "Settings surface as generated long flags (--<area>-<key> <value>)". Not one of the 121 documented flags is parsed by `lumen`; every one dies with `error: unknown command` (executed, exit 64). Settings are only writable through the REPL `setting set <area.key> <value>` verb. The file also reserves `--generate-manifest`, a flag that exists nowhere in the codebase (executed: unknown command), and doc 07 §5.3 G2 still promises it "dump[s] the full option table as TOML/JSON for scripts".

## 2. What actually works (verified)
— `cargo test -p settings-schema --test integration` -> 8 passed (`committed_outputs_have_no_drift ... ok`): golden is in sync with schemas/settings.toml.
— `diff <(./target/debug/lumen --help video | tail -n +4) <(sed -n '/^video$/,/^$/p' ...)` -> AREA-SECTION-IDENTICAL: runtime area help == golden section.
— awk count of `^  --` flags per area: audio 11, controls 9, engine 6, extensions 6, global 13, interface 18, performance 15, playback 17, privacy 11, recording 2, video 13 (total 121) — every index row (L14-24) is accurate.

## 3. Findings
F-001 [S0]
Where: schemas/generated/settings-help.txt:7 (claim) + apps/cli/src/parse.rs:154-169 (contradiction)
Code: (verbatim, settings-help.txt:7-9)
```
  Settings surface as generated long flags (--<area>-<key> <value>).
  `lumen --help` lists areas with counts; `lumen --help <area>` lists that
  area's flags (the VLC --longhelp pattern, generated).
```
Code: (verbatim, apps/cli/src/parse.rs:155-169)
```
        match arg.as_str() {
            "--json" => invocation.json = true,
            "--control-stdio" => invocation.control_stdio = true,
            "--version" | "-v" => invocation.version = true,
            "-h" | "--help" => {
                invocation.help = Some(None);
```
What is wrong: the only argv flags `parse_argv` recognizes are `--json`, `--control-stdio`, `--version`/`-v`, `-h`/`--help` (everything else is pushed into `command_words` and rejected as an unknown verb). Executed proof: `./target/debug/lumen --video-hue 50 open download/big_buck_bunny.mp4` -> stderr `error: unknown command `--video-hue`` + usage, exit 64. All 121 flags documented in this golden (and re-printed by the shipped `lumen --help`) are unusable. Why it matters: the file (and the binary's own help) documents a settings CLI surface that does not exist; a user following the help verbatim gets exit 64. The only real settings write path is the REPL verb `setting set <area.key> <value>` (help.rs SUBCOMMANDS). What correct looks like: either parse the generated flags (map `--<area>-<key>` → `config.set-setting`) or change the generated claim to "settings are set via `setting set` / `--help <area>` is reference only". Confidence: HIGH — Verification: ran (both the flag rejection and the byte-identical help output).
F-002 [S3]
Where: schemas/generated/settings-help.txt:10-11
Code: (verbatim)
```
  Reserved (non-settings) flags: -h/--help, -v/--version, --json,
  --control-stdio, --generate-manifest.
```
What is wrong: `--generate-manifest` is a phantom. rg over crates/, apps/, tools/, tests/, ui/, scripts/ finds it only in the generator (emit.rs:290), a reserved-name list (validate.rs:43), and doc 07 §5.3 G2 — never an implementation. Executed: `./target/debug/lumen --generate-manifest` -> `error: unknown command `--generate-manifest`` (exit 64). The runtime `--help` reserved list (help.rs) omits it, so the two "generated" surfaces disagree with each other and with the binary. Why it matters: the golden is committed documentation; it reserves a name for a feature (doc 07: "dumping the full option table as TOML/JSON for scripts") that was never built. What correct looks like: implement the flag or delete it from emit.rs's reserved list and doc 07. Confidence: HIGH — Verification: ran + rg.
F-003 [S4]
Where: schemas/generated/settings-help.txt:226-231 (pattern; 24 occurrences)
Code: (verbatim)
```
  --interface-start-maximized <on|off>  [advanced] [restart required]
      settings.interface.start-maximized
      default: -> product.toml window.start_maximized
```
What is wrong: 24 of 121 defaults are pointers, not values (`rg -c "default: ->"` -> 24). The runtime `--help <area>` regenerates from the embedded schema alone (`generate_str(schema, GenTarget::CliHelp)` — help.rs:156-160, no product.toml input), so the user can never learn the actual default from help; they must open product.toml and look the key up. What correct looks like: resolve product.toml defaults into the help text at generation time (schema-gen already reads product.toml for validation). Confidence: HIGH — Verification: read + rg count.
F-004 [S4]
Where: schemas/generated/settings-help.txt:315-317
Code: (verbatim)
```
  --playback-start-paused <on|off>
      settings.playback.start-playing
      default: -> product.toml playback.start_paused
```
What is wrong: polarity-inverted naming: the flag says start-paused, the settings key says start-playing, and the product default key is `start_paused`. schemas/settings.toml:656-658 admits the mess: "The KEY keeps its start_paused semantics; the UI inverts the display through the registered special widget". A `setting set playback.start-playing true` user gets the opposite of what the key name says (assuming the seam ever honors it — see F-001). What correct looks like: one name, one polarity, everywhere. Confidence: HIGH — Verification: read (settings.toml L656-662, product.toml L192).

## 4. Per-area inventory (data file — no functions; per-area table instead)
| Area | Index count | Actual flags (awk) | Match | Notes |
|---|---|---|---|---|
| audio | 11 | 11 | YES | |
| controls | 9 | 9 | YES | |
| engine | 6 | 6 | YES | marked engine-internal: ui = false, consistent |
| extensions | 6 | 6 | YES | --extensions-load-on-startup carries `[gate: wasm-extensions]` |
| global | 13 | 13 | YES | --global-language enum `<en-US>` matches the single shipped catalog |
| interface | 18 | 18 | YES | |
| performance | 15 | 15 | YES | |
| playback | 17 | 17 | YES | includes F-004 inverted pair |
| privacy | 11 | 11 | YES | lastfm-password documented as plain `<string>` — see §9 |
| recording | 2 | 2 | YES | `[gate: recording]` |
| video | 13 | 13 | YES | |

## 5. Test honesty
No tests in this file. Consumers' tests verified: `cargo test -p settings-schema --test integration` -> 8 passed, incl. `committed_outputs_have_no_drift` (would fail if this golden drifted from schemas/settings.toml) and `stale_output_drift_fails_check_with_exit_one`. `apps/cli/src/run.rs:916-928` tests `--help`/`--help engine`/`--help no-such-area` — but NO test anywhere parses a settings-derived flag, which is exactly why F-001 survived.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Settings surface as generated long flags (--<area>-<key> <value>)" | this file L7; help.rs L123 | FALSE | executed: `--video-hue 50` -> unknown command, exit 64 (F-001) |
| "`lumen --help` lists areas with counts; `lumen --help <area>` lists that area's flags" | L8-9 | REAL | executed: both outputs match golden (diff identical) |
| "`--generate-manifest`" reserved | L11 + doc 07 §5.3 G2 | FALSE | executed: unknown command, exit 64; no implementation in tree (F-002) |
| "Generated by `cargo xtask schema-gen` ... DO NOT EDIT" | L2 | REAL | integration test `committed_outputs_have_no_drift` passes; regeneration idempotent byte-for-byte |

## 7. Dead / duplicated / unreachable code
- The whole file is unreachable from the shipped binary at runtime (CLI regenerates from the embedded schema); it exists for CI drift checks, rebrand token rewrites, and human reference.
- `--generate-manifest` (L11): dead name (F-002).

## 8. Dependencies & flags
Generated artifact of `settings-schema` (emit.rs G2, `emit_help`). Flags/metadata rendered: `[advanced]`, `[restart required]`, `[gate: <feature>]` (wasm-extensions on L118, hw-decode on L335, recording on L391-394) — gate names cross-check against the engine feature registry per doc 07; not re-verified here (UNVERIFIED — out of this file's scope; owner: feature-matrix/xtask auditors).

## 9. Open questions for the next agent
- Who owns parse.rs/help.rs (W?-A?): F-001 is primarily a CLI defect; the golden merely faithfully documents the lie the generator emits. Cross-file S0 — coordinate before counting matrix rows.
- `--privacy-lastfm-password <string>` (L382-384): is a settings key holding a password ever persisted in plaintext? Owner: settings-schema/privacy auditors.
- 24 pointer-defaults (F-003): schema-gen reads product.toml for validation already (config_validator) — cheap fix, confirm no design reason for pointers.
