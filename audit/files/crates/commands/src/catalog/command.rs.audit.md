# AUDIT: crates/commands/src/catalog/command.rs
| Field | Value |
|---|---|
| Auditor | W02-A08 |
| Date | 2026-11-23 |
| Lines / functions | 870 / 11 items (2 enums, 1 struct, 1 fn, 7 test fns) |
| File verdict | REAL (wire-format data catalog, executed; with false doc claims) |
| Reachable from shipped entry point? | YES — apps/cli/src/main.rs:9 `fn main` → `lumen_cli::run::run` → apps/cli/src/dispatch.rs:62 `lumen_dispatch` → `route()` matches `Command` (L79-156); control-server: apps/control-server/src/dispatch.rs → `ControlDispatch::dispatch`; desktop: apps/desktop/src/shell.rs:160 intercepts `Command::SessionSetFrameChannel` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | `./target/debug/deps/commands-cd658e8b0c406ea7 catalog::command` → 6 passed; `./target/debug/deps/catalog_shape-a6be788a761b3aed` → 4 passed; `./target/debug/deps/wire_forms-596a955b745156da` → 7 passed; `./target/debug/deps/catalog_drift-eaf83ff7e4d39d14` → 2 passed; `./target/debug/deps/catalog_emission_ledger-f20954af8148cb44` → 2 passed; rustc-compiled serde probe `/tmp/argsjunk` (linked against prebuilt libcommands rlib) → 5 cases, see F-001/F-002 |

## 1. One-paragraph verdict
This file is the real wire-format catalog: `Command` (65 variants, adjacently tagged `cmd`/`args`), `CommandResult` (32 variants, internally tagged `kind`), `CommandListing`, plus 6 round-trip tests — all executed and passing against the prebuilt binaries. The enum surface is provably consistent with `CommandId`, `catalog_data()` and the committed JSON schema (catalog_shape test, run: 4/4). But it is a *contract file, not a behavior file*: only ~15 of 65 commands are served by any shipped host (CLI/desktop/web all route to the same `route()`; 50 answer the typed refusal), 19 of 32 result variants are never constructed anywhere in the repo, and the doc comments make three claims the code contradicts: `#[non_exhaustive]` is claimed but absent; the shell-routed `video.set-fullscreen` is routed by no shell; documented numeric ranges are enforced by nothing. Worst: unknown fields inside `args` are silently dropped — executed proof below — directly contradicting the control protocol's own claim that "the catalog's own `deny_unknown_fields` cover the `args` object's interior" (crates/control-protocol/src/protocol.rs:227-229).

## 2. What actually works (verified)
— `./target/debug/deps/commands-cd658e8b0c406ea7 catalog::command` → `test result: ok. 6 passed; 0 failed` (request wire form, bookmark trio, unknown-id rejection, unit-no-args, 4 result round-trips, deep TrackInfo).
— `./target/debug/deps/catalog_shape-a6be788a761b3aed` → 4 passed: enum variants ↔ `CommandId::ALL` (65) ↔ `catalog_data()` rows ↔ committed schema tags are the same set, no orphans/missing.
— `./target/debug/deps/catalog_drift-eaf83ff7e4d39d14` → 2 passed: committed schema/TS/checksum regenerate byte-identically from these types.
— Probe (rustc, prebuilt rlibs): unit command with an args object is correctly rejected: `CASE3: REJECTED: invalid type: map, expected unit variant Command::PlaybackPause`.

## 3. Findings
F-001 [S2]
Where: crates/commands/src/catalog/command.rs:67-73 (and every struct variant, e.g. 119-122)
Code:
```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "cmd",
    content = "args",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Command {
```
What is wrong: No `deny_unknown_fields` (serde does not support it on tagged enums), so unknown fields inside `args` are silently dropped. Executed proof (rustc probe against the real crate, `/tmp/argsjunk`): `{"cmd":"playback.seek","args":{"target":{"kind":"absolute","time":1},"junk":true}}` → `CASE1: ACCEPTED (junk silently dropped)`; junk inside a nested enum variant payload also accepted (`CASE2: ACCEPTED`). Why it matters: crates/control-protocol/src/protocol.rs:227-229 asserts "The catalog's own `deny_unknown_fields` cover the `args` object's interior" — that is false; a control-server client with a typo'd/extra arg field gets silent success instead of the documented rejection (doc 05 §5.4 "unknown fields … rejected"). The conformance suite tests envelope-level extras (conformance.rs:895-902) but never args-interior extras, so the gap is untested. What correct looks like: per-variant arg structs with `deny_unknown_fields`, or a raw-`Value` post-parse key check like the envelope one. Confidence: HIGH — Verification: ran (probe output pasted above).
F-002 [S3]
Where: crates/commands/src/catalog/command.rs:135-141 (also 231-242, 280-285, 296-301)
Code:
```
    /// `playback.set-position` — seek by fraction (class A, gen-bumping;
    /// slider/scrubber ergonomics, §2.4).
    #[serde(rename = "playback.set-position")]
    PlaybackSetPosition {
        /// `0.0..=1.0` of the duration.
        fraction: f32,
    },
```
What is wrong: The documented ranges (`0.0..=1.0`, zoom `0.25..=4.0`, scale `0.2..=5.0`, delay `±60 000`, enqueue-many "non-empty … `invalid-argument`") are enforced by nothing — not the type, not serde, not the generated schema (verified: `schemas/commands.schema.json` emits `"type": "number", "format": "float"` with no minimum/maximum). Probe: `fraction: 7.5` → `CASE4: ACCEPTED (7.5 passes)`; `delayMs: -999999999` → `CASE5: ACCEPTED`. Why it matters: the schema is shipped to UI clients as the validation contract; a client validating against it will pass out-of-range values that later handlers must reject. Today no shipped host serves these commands (all refused), which is the only reason this is not S2. What correct looks like: newtype ranges (`Fraction(f32)` with constructor validation) or schemars range attributes so the emitted schema carries minimum/maximum. Confidence: HIGH — Verification: ran (probe) + read (schema JSON via python3).
F-003 [S3]
Where: crates/commands/src/catalog/command.rs:64-66
Code:
```
/// `#[non_exhaustive]` per §2.1: future minor versions add variants;
/// unknown ids are rejected with `invalid-argument` (V1) — serde does
/// that for the `"cmd"` tag automatically.
```
What is wrong: The `Command` enum carries no `#[non_exhaustive]` attribute (rg count of `non_exhaustive` in this file = 1, the doc comment itself; the derive block at 67-73 has none). The doc asserts a stability mechanism the code does not have. Also the error-code half is host-dependent: the control-protocol server answers unknown ids with protocol code `unknown-type` (conformance.rs:910 `assert_eq!(code, ProtocolErrorCode::UnknownType)`), while the CLI `--control-stdio` path answers `invalid-argument` (apps/cli/src/run.rs:341-343). Why it matters: doc-claim vs code mismatch on the catalog's forward-compat story; downstream consumers reading §2.1 will assume source-compat across minor versions that `#[non_exhaustive]` would have provided. What correct looks like: either add the attribute or delete the claim, and state the per-host error code. Confidence: HIGH — Verification: read + rg; conformance test cited.
F-004 [S3]
Where: crates/commands/src/catalog/command.rs:212-218
Code:
```
    /// `video.set-fullscreen` — **target: shell** (class S; the only
    /// shell-routed command in v1, §1.1.3).
    #[serde(rename = "video.set-fullscreen")]
    VideoSetFullscreen {
        /// Enter or leave fullscreen.
        enabled: bool,
    },
```
What is wrong: No shell routes it. The desktop shell intercepts exactly one command (`apps/desktop/src/shell.rs:160` `if let Command::SessionSetFrameChannel …`); `VideoSetFullscreen` falls through to `lumen_cli::dispatch::route` → `owner_of("video.set-fullscreen")` → refusal "video output control joins with the windowed renderer (2.D3)". Also "the only shell-routed command" is false twice over: `session.set-frame-channel` is also shell-targeted (its own doc at 89-101 says so, and spec.rs's test `shell_targeted_commands_are_the_shell_owned_surface` pins BOTH), and spec.rs:23 still says "only `video.set-fullscreen` targets the shell" — three files, three mutually inconsistent stories. Why it matters: the catalog's target metadata promises a fullscreen path that does not exist in any shipped build; the spec's own module doc contradicts its own test. What correct looks like: either the shell intercepts it or the spec/doc say "shell-routed, unimplemented until the windowed renderer". Confidence: HIGH — Verification: read + rg (zero `VideoSetFullscreen` handler matches outside catalog files).
F-005 [S4]
Where: crates/commands/src/catalog/command.rs:578-612 vs whole-repo construction sites
Code:
```
    /// `media.get-info` — the metadata snapshot.
    MediaInfo {
        /// The snapshot.
        info: MediaInfo,
    },
```
What is wrong: 19 of 32 `CommandResult` variants are never constructed anywhere in the repo (production or tests): AppliedRate, AppliedBool, AudioDevices, AppliedDelayMs, AppliedAspectRatio, AppliedCrop, AppliedZoom, AppliedEncoding, AppliedScale, AppliedRepeat, PlaylistItem, PlaylistItems, Chapters, BookmarkAdded, ConfigAreas, PluginList, PluginToggle, AppliedLogLevel, StateSnapshot (rg `CommandResult::` across apps/ + crates/ → only 13 distinct variants ever built). Why it matters: not fraud — hosts honestly refuse the commands — but `system.list-commands` (served, CLI/desktop/web) returns all 65 catalog rows with no served/capability field (`CommandListing` has id/class/since/deprecated/requires only; apps/cli/src/host.rs:1665-1676 maps `catalog_data()` unfiltered), so the introspection a UI uses "to build menus" (L670-672) advertises 50 verbs that answer `unsupported-for-host`. Confidence: HIGH — Verification: rg + read.
F-006 [S4]
Where: crates/commands/src/catalog/command.rs:807-831
Code:
```
        for (result, json) in [
            (CommandResult::Unit, r#"{"kind":"unit"}"#),
```
What is wrong: `result_variants_round_trip` covers 4 of 32 result variants; `deep_track_info_round_trips` covers 1 more. 27 variants have no round-trip pin (their wire tags are only covered transitively via the generated schema's byte-identity, which does prove the tags exist but not that payload fields serialize correctly). Minor test-coverage gap on the file that defines the wire contract. Confidence: HIGH — Verification: read + ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum Command` (65 variants) | 74-470 | REAL | — | Wire contract; all variants parse/serialize; 65↔ids↔spec↔schema proven by catalog_shape (ran). 50/65 refused by every shipped host (honest typed refusal). |
| `fn default_true` | 474-476 | REAL | — | Trivial serde default for `SubtitleAddFile.select`. |
| `enum CommandResult` (32 variants) | 491-668 | REAL (13/32) / UNREACHABLE (19/32) | S3 | 19 variants never constructed anywhere (F-005). |
| `struct CommandListing` | 675-687 | REAL | S4 | Has `deny_unknown_fields`; no served-surface field (F-005). |
| `tests::round_trip` | 697-702 | REAL | — | Test helper; `unreachable!` on error (test-only convention). |
| `request_wire_form_matches_doc_example` | 711-728 | REAL | — | Pins §5.1 JSON byte-for-byte; would fail on any serde attr drift. Ran: pass. |
| `bookmark_commands_round_trip_the_wire` | 730-779 | REAL | — | Pins bookmark trio + null label. Ran: pass. |
| `unknown_command_ids_are_rejected` | 781-791 | REAL | — | Weak assertion (`contains("variant")`) but genuine. Ran: pass. |
| `unit_commands_take_no_args` | 793-804 | REAL | — | Ran: pass. |
| `result_variants_round_trip` | 806-831 | SHALLOW | S4 | 4/32 variants (F-006). Ran: pass. |
| `deep_track_info_round_trips` | 836-869 | REAL | — | One deep payload; CJK name + HDR fields. Ran: pass. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| request_wire_form_matches_doc_example | YES | Exact-string JSON assertion on the §5.1 example; any tag/field rename breaks it. |
| bookmark_commands_round_trip_the_wire | YES | Exact wire strings for add-bookmark and Bookmarks result. |
| unknown_command_ids_are_rejected | YES (weakly) | Any serde acceptance of unknown cmd fails it; but it would not catch a change of the *error kind* (only checks the word "variant"). |
| unit_commands_take_no_args | YES | Parsing pause without args must succeed. |
| result_variants_round_trip | YES for the 4 covered | 27 variants uncovered (F-006). |
| deep_track_info_round_trips | YES | Full PartialEq on the deep struct. |
| (integration) catalog_shape / catalog_drift / emission_ledger | YES | Cross-set equality + byte-identity of committed artifacts; ran, 8/8 pass. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`#[non_exhaustive]` per §2.1" | command.rs:64 | FALSE | Attribute absent (rg count 1 = the claim itself). |
| "unknown ids are rejected with `invalid-argument` (V1)" | command.rs:65 | SHALLOW | Host-dependent: control-protocol → `unknown-type` (conformance.rs:910); CLI stdio → `invalid-argument` (run.rs:341). |
| "unknown fields … rejected" (args interior) via "the catalog's own deny_unknown_fields" | protocol.rs:227-229 | FALSE | Probe CASE1/CASE2: junk inside args accepted, silently dropped. |
| "`video.set-fullscreen` … the only shell-routed command in v1" | command.rs:212 | FALSE | No shell handler (rg); spec.rs test pins TWO shell-targeted commands. |
| "variant payloads taken verbatim from the §2 tables' Params columns" | command.rs:6-7 | REAL-UNPROVEN | Doc 05 §2 tables not diffed line-by-line by me; schema/ids/spec cross-checks pass, payload prose unverified. |
| `session.set-frame-channel` "the desktop host intercepts it before the CLI delegation" | command.rs:93-94 | REAL | apps/desktop/src/shell.rs:160-162 intercept + `bind_frame_channel` (read). |

## 7. Dead / duplicated / unreachable code
- 19 `CommandResult` variants never constructed (F-005 list). Forward contract by design; flagged so the matrix does not count them as shipped behavior.
- 50 `Command` variants with no serving handler in any shipped host (CLI SERVED=15 of 65, apps/cli/src/dispatch.rs:38-54); all answer typed refusals — honest, but "catalog row exists" ≠ "command works".

## 8. Dependencies & flags
- Uses `schemars::JsonSchema`, `serde` — no feature gates on this file; compiled in the release lane (baseline cargo-build-release-lumen-cli.txt exit 0).
- No `unsafe`, no `allow` attrs in this file.

## 9. Open questions for the next agent
- Does apps/control-server/src/dispatch.rs serve more than the CLI's 15 (e.g. read-only policy subset)? If it also forwards to the same refusal set, "control server" is a thin transport over the same 15-command surface.
- Who consumes `CommandResult::AudioVolume`/`AppliedSelector` in production (rg shows construction only in tests for these two)? If nothing produces them at runtime, `audio.get-volume`-style snapshot answers may be dead too.
- Verify whether doc 05 §5.4's "unknown fields … rejected" claim has any args-interior conformance row at all (conformance.rs covers envelope only) — candidate for the docs claim ledger.
