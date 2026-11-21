# AUDIT: apps/control-server/src/dispatch.rs
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-21 |
| Lines / functions | 197 / 9 (4 production items + 3 tests + 2 test helpers) |
| File verdict | REAL (product-disabled surface) |
| Reachable from shipped entry point? | NO from `lumen`/`lumen-tauri` — the served surface is the standalone `lumen-control-server` binary (main.rs:246 `CatalogDispatch::new(engine_info)`), which builds (baseline #1) and whose tests pass, but the product manifest disables the feature: product.toml:167 `control-server = false  # Phase 5+ [PROPOSAL]` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "CatalogDispatch\|catalog_listings" apps/` (constructor: main.rs:246; lib.rs:46 re-export); `rg -n "pub struct CommandSpec\|since\|pub const ALL" crates/commands/src/catalog/{spec,command,ids}.rs` (CommandSpec carries NO since/deprecated/requires fields — the listing's constants are not spec-derivable); `rg -n "dispatch::tests\|control_server" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (all 3 tests passed :1460-1462); `rg -n "control" product.toml` (feature false :167); `sed -n` quote verification |

## 1. One-paragraph verdict
A small, honest, correctly-shaped v1 stub-with-integrity: two introspection commands served from real catalog data, every other command answered with a typed `not-ready` (stage `control`, explicit message naming the gap) — never a fake success. The production half is 78 lines with no hidden state and no invented wire vocabulary. Two defects, both about promises vs proof: the test that guards the file's central promise ("never a fake success") samples 2 of the ~63 refused commands, so a regression serving a fake success on any other command passes unnoticed; and the listing's `since`/`deprecated`/`requires` fields are hard-coded constants while the docs claim the rows come "straight from the live spec table" — the spec table genuinely carries no such fields, so the constants are unavoidable today but the claim overstates the derivation. Note the whole surface is product-disabled (`control-server = false`), so none of this ships to users yet.

## 2. What actually works (verified)
- All 3 tests passed in the orchestrator rerun: `dispatch::tests::{the_catalog_lists_all_65_commands_from_the_live_spec, engine_info_and_list_commands_serve_the_catalog_subset, everything_else_is_a_typed_not_ready_never_a_fake_success}` (orchrun-test-workspace-nofailfast.log.txt:1460-1462); the `lumen_control_server` binary target also built and ran 0 tests (log :1512). — verified by reading the cited log.
- The catalog listing is derived from the live table: `CommandId::ALL` + `command_spec(id)` (dispatch.rs:85-100), so id and class track the spec — the count pin (65) and first-id pin (`session.open`) both passed. — verified by reading + passing test.
- Typed refusal shape: `Stage::Control` + `ErrorCode::NotReady` + the NOT_READY message — asserted by the passing test. — verified by reading + passing test.

## 3. Findings

### F-001 [S3]
Where: apps/control-server/src/dispatch.rs:169-195
Code:
```rust
    fn everything_else_is_a_typed_not_ready_never_a_fake_success() {
        let mut dispatch = CatalogDispatch::new(engine_info());
        let commands = [
            Command::PlaybackPlay,
            Command::SessionOpen {
```
What is wrong: The test name promises "everything else"; the body probes exactly two commands (`PlaybackPlay`, `SessionOpen`) of the ~63 that fall into the `_ =>` refusal arm. The module's central promise — "an honest gap, never a silent no-op and never a fake success" (lines 12-13) — is guarded by a two-sample probe. A mutation that adds one more served arm (e.g., `Command::PlaybackStop => Ok(...)`) passes this test untouched; a client would then receive a fabricated success for an engine command from a build that embeds no engine.
Why it matters: This catch-all honesty is the file's entire reason to exist (v1 embeds no engine); its only regression guard is trivially bypassable for 61 of 63 commands.
What correct looks like: Iterate `CommandId::ALL`, dispatch each (skipping the two served ids), and assert the typed refusal for every one — the catalog table is already enumerable in the test.
Confidence: HIGH — Verification: read + mutation reasoning (no run needed).

### F-002 [S4]
Where: apps/control-server/src/dispatch.rs:81-101
Code:
```rust
/// Every catalog command as a `system.list-commands` row, straight from
/// the live spec table (doc 05 §2.12: UIs and the control server build
/// menus and validate keymaps without hardcoding the catalog).
pub fn catalog_listings() -> Vec<CommandListing> {
```
What is wrong: "Straight from the live spec table" is true for `id` and `class` only. `since: "1"`, `deprecated: None`, and `requires: Vec::new()` are hard-coded constants for every row (lines 95-97) — necessarily so, because `CommandSpec` (spec.rs:133-147) carries no such fields. The doc comment cites doc 05 §6.1 ("every command appeared in catalog major 1") for the constant, but the code cannot track any future divergence: the day the catalog mints a `since 2` row or a deprecation, this table silently serves stale metadata while still claiming live derivation.
Why it matters: The claim "never a copy" (module docs, lines 9-10) is 2/5 fields true; the drift risk is latent, not current.
What correct looks like: Either add `since`/`deprecated`/`requires` to `CommandSpec` (single source of truth), or scope the comment to "id and class from the live spec table; the v1 namespace constants per doc 05 §6.1".
Confidence: HIGH — Verification: read (spec.rs:133-147 confirms the fields don't exist).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-15 | REAL | — | "v1 embeds no engine" honest; "never a copy" → F-002 |
| `NOT_READY` const | 33-36 | REAL | — | names the served pair; honest |
| `CatalogDispatch` (struct) | 38-44 | REAL | — | stateless beyond the info block |
| `CatalogDispatch::new` | 46-51 | REAL | — | trivial |
| `Dispatch::dispatch` | 53-79 | REAL | — | two served arms + catch-all refusal; `session: None`, `generation: Generation(0)` per the session-less wire shape; `&mut self` never mutates (trait shape) |
| `catalog_listings` | 81-101 | REAL | S4 | F-002 constants vs claim; `filter_map` silently drops ids without spec (none today — pinned by test count) |
| tests: `engine_info` helper | 108-120 | REAL | — | builds a real EngineInfo |
| `the_catalog_lists_all_65_commands_from_the_live_spec` | 122-141 | REAL | — | count/first-id/contains pins real; `len == ALL.len()` partially tautological but pins spec coverage through the `filter_map` |
| `engine_info_and_list_commands_serve_the_catalog_subset` | 143-167 | REAL | — | request_id echo + host + count asserted |
| `everything_else_is_a_typed_not_ready_never_a_fake_success` | 169-195 | REAL | S3 | F-001 two-sample probe |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| the_catalog_lists_all_65_commands_from_the_live_spec | YES | 65-count + first-id + membership pins; a catalog change without pin update fails |
| engine_info_and_list_commands_serve_the_catalog_subset | YES (narrowly) | request_id echo, host, count |
| everything_else_is_a_typed_not_ready_never_a_fake_success | ONLY FOR THE 2 SAMPLED COMMANDS | F-001: 61 of 63 refusal arms unprobed |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "v1 embeds no engine … only catalog introspection is served" | module docs 4-6 | REAL | the two arms are the whole match (55-65) |
| "never a silent no-op and never a fake success" | module docs 12-13 | REAL for the 2 probed + 61 unprobed arms (code read); TEST GUARD WEAK | F-001 |
| "built from `commands::catalog::spec`, never a copy" | module docs 9-10 | SHALLOW | id/class derived; since/deprecated/requires are constants (F-002) |
| "UIs and the control server build menus … without hardcoding the catalog" | doc 05 §2.12 via comment 81-83 | REAL-UNPROVEN for this server (the UI half is not this file); the server half verified |

## 7. Dead / duplicated / unreachable code
- None dead in-file. The whole surface is product-disabled (product.toml:167) — reachable only by explicitly running the `lumen-control-server` binary.

## 8. Dependencies & flags
Depends on the `commands` crate (catalog spec, envelope, error) and `control_protocol::server::Dispatch` — all default-on, no feature gates in this file. No engine dependency (by design).

## 9. Open questions for the next agent
1. (control-server run.rs/main.rs owners — W13-A01) What does the production `engine_info` (main.rs:246) advertise for `decoders`/`features`? If it is a hard-coded literal like the CLI's engine_info (baseline standing defect), this dispatch faithfully serves a false block — the defect would live one file over.
2. (commands crate auditors) Doc 05 §6.1's "every command appeared in catalog major 1" — verify against the catalog's own history (the w31.1 frame-channel mint) when the `since` semantics get a real field (F-002).
