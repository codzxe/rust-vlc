# CLAIM AUDIT: docs/05-command-layer.md
| Field | Value |
|---|---|
| Auditor | W02-A09 |
| Date | 2025-06-27 |
| Claims extracted | 99 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 40 / 11 / 19 / 0 / 10 / 16 / 3 |
| Internal contradictions found | 4 |

## 1. Blunt verdict

The catalog-as-types, the envelope, the correlation machinery, the schema-first
codegen and the control-server wire protocol are REAL — implemented, tested,
and I executed the `commands` crate tests myself (57 unit + 15 integration,
all pass; several pin this doc's tables and wire examples byte-for-byte). The
doc's "Status: implemented" is nevertheless an overstatement: the §7
"testkit conformance suite" does not exist in any shape the doc describes
(testkit is an allocation counter), the §8.1/§8.3/§8.4 data files
(`config/keymaps/default.toml` in the documented format,
`schemas/keymap.schema.json`, `layouts/menus.toml`, `layouts/ui-actions.toml`)
do not exist, four of doc §5.4's security controls are "recorded gaps"
(rate limits, concurrency cap, Origin check, token file/fd distribution), and
every host serves only a small subset of the catalog (CLI 15/65, web ~13,
control-server 2) with typed refusals for the rest. This is a Wave-1 planning
doc whose header simultaneously claims "planning document — type sketches, no
implementation" and "Status: implemented" — the honest status is: §1–§3
contract types + data: implemented; §5 hosts: partial; §7 suite, §8 data
layer: design only.

## 2. Claim ledger

Evidence key: `T` = test I ran (`cargo test -p commands`, exit 0, 57+2+2+4+7
passed); `B` = orchestrator baseline file; otherwise path:lines.

| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| C01 | "Status — implemented (awaiting Lead review)" | :6 | SHALLOW | Core types/wire REAL (C04-C48); but §7 suite absent (C82-C86), §8 data files absent (C91-C97), §5.4 gaps (C70,C74,C76,C77), hosts serve subsets (C25) |
| C02 | "Task — 1.A5 (Wave 1, planning document — type sketches, no implementation)" | :5 | SHALLOW | Contradicts :6 "implemented"; see §3 contradictions |
| C03 | Owning crates: "crates/testkit (conformance)" | :9 | FALSE | crates/testkit/src/lib.rs:1-15: "The Lumen test kit, v0 (doc 03 §8…)" — alloc_count + faults + Must only; no host abstraction, no scenarios |
| C04 | "exactly one command layer… same catalog… re-exported (never re-implemented) by every host" | :20 | REAL | apps/desktop/src/dispatch.rs:211-212 wraps `lumen_cli::dispatch::lumen_dispatch`; crates/control-protocol/src/protocol.rs:28-30 `use commands::WireRequest` (boundaries.toml same_layer_pairs "control-protocol->commands"); ui/src/client/transport.ts imports ../generated/commands |
| C05 | "Enforcement is automated (§7.4)" [UI import bans] | :25 | FALSE | ui/eslint.config.js has no `no-restricted-imports`/dependency-cruiser (rg: 0 matches for "no-restricted\|dependency-cruiser"); §7.4.1/2 absent |
| C06 | Shell-routed commands stay in catalog, declare `target: engine/shell` | :26 | REAL | crates/commands/src/catalog/spec.rs:84-105 CommandTarget; spec test `shell_targeted_commands_are_the_shell_owned_surface` T |
| C07 | V1: "deny_unknown_fields; unknown fields, variants, IDs rejected with invalid-argument" | :33 | REAL | 22 uses in catalog/types.rs + command.rs; control-protocol REQUEST_KEYS check protocol.rs:224; T: `closed_shapes_reject_unknown_fields`, `unknown_command_ids_are_rejected` |
| C08 | V2: "ranges validated before dispatch… violations return out-of-range" | :34 | SHALLOW | OutOfRange exists (error.rs:83) and engine seek rejects bad percent (playback-engine/src/seek.rs `SeekFailure::OutOfRange`); no pre-dispatch range validation for volume/rate/zoom/delay in any host — those commands are unserved (apps/cli/src/dispatch.rs:38-54 SERVED list) |
| C09 | V3: "no sentinel values… Duration::Unknown, Option<TrackId>" | :35 | REAL | crates/commands/src/catalog/types.rs Duration enum Known/Unknown/Infinite (TS mirror ui/src/generated/commands.ts) |
| C10 | V4: "no `as` casts on the request path (workspace lints already deny the usual foot-guns)" | :36 | REAL-UNPROVEN | B: cargo-clippy.txt "-D warnings: CLEAN"; per-cast audit of the request path not performed |
| C11 | V5: "reference validity checked at execution… typed not-found" | :37 | SHALLOW | ErrorCode::NotFound real (error.rs:85); used by apps/web (host.rs/open.rs) + CLI exit map; `tracks.get-info` unserved in hosts — execution-time check unexercised |
| C12 | V6: "Strings ≤ 4096 / 1024 bytes… playlist ≤ 10 000… depth ≤ 8" | :38 | PAPER-ONLY | rg 4096/4_096/10_000 across crates/commands, apps/{cli,desktop,web}: 0 relevant matches; only 1 MiB control frames (server.rs:83) + 1024 error-message cap (error.rs:139) exist |
| C13 | V7: "untrusted strings… length-capped, UTF-8-validated" | :39 | SHALLOW | control-protocol/src/protocol.rs:280-286 `redact_name` 64-char cap; error.rs MESSAGE_CAP=1024; no general caps on track names/metadata strings |
| C14 | V8: "URLs have credentials stripped, tokens/secrets never appear" | :40 | REAL | error.rs:146-151 redact masks `user:pass@` and collapses paths; T: `error_construction_redacts_userinfo`, `error_construction_collapses_absolute_paths`, `details_are_redacted_too` |
| C15 | "Every command response carries request_id… event carries seq, session, gen, origin" | :44 | REAL | crates/commands/src/envelope.rs:99-134 CommandResponse/Event; T: `response_shape_carries_all_correlation_fields` |
| C16 | C1–C8 truthful-UI "hard contract" | :48-65 | SHALLOW | Host half REAL (C17, C41); UI half: no promise/waitFor client API, no no-early-success harness (C86); player context consumes seek-completed (ui/src/screens/player/context.tsx) |
| C17 | C8: "engine still emits the completion event as a self-transition (from == to)" | :65 | REAL | types.rs:320-325 `if self == other { return true; // C8 }`; T: `self_transitions_are_always_valid` (types tests) |
| C18 | §1.5 S/A classes with the doc's example lists | :69-74 | REAL | spec.rs catalog_data; T: `classification_matches_section_1_5_examples`, `class_counts_match_the_doc_05_tables` |
| C19 | S budget "≤ 50 ms [PROPOSAL]" | :71 | PAPER-ONLY | No budget measurement anywhere (rg "50 ms" budget: none) |
| C20 | §1.6 envelope type sketches (RequestId/SessionId/Generation/…) | :78-122 | REAL | envelope.rs:27-134 matches sketch incl. `gen` rename; T: `ids_are_plain_newtypes` etc. |
| C21 | §2.1 stable kebab IDs ↔ PascalCase variants; mapping mechanical | :130-141 | REAL | catalog/ids.rs macro; T: `doc_05_examples_round_trip`, `every_id_round_trips_and_is_unique` |
| C22 | "IDs are stable forever within major version 1"; enum #[non_exhaustive] | :141 | REAL | ids.rs:47 `#[non_exhaustive]`; growth 61→65 was additive (checksum/golden tests); T: `catalog_sizes_match_doc_05` asserts 65 |
| C23 | §2.2 common types (Timestamp, Duration, Volume, Rate, TrackId, TrackSelector, SeekTarget, SourceSpec) | :182-221 | REAL | catalog/types.rs; TS mirror in ui/src/generated/commands.ts (SeekTarget union incl. relative/percent/chapter) |
| C24 | §2.3–2.12 tables: every command's params/class/result/errors defined | :225-334 | REAL | catalog/command.rs (870 LOC, one variant per id, args from tables) + spec.rs rows; T: `tables_cover_every_id_exactly_once_in_order` |
| C25 | [implied by Status] the catalog commands are executable | :6 | SHALLOW | apps/cli/src/dispatch.rs:38-54 SERVED = 15 ids; apps/web/src/host.rs:1403-1430 serves ~13; apps/control-server/src/dispatch.rs:55-71 serves 2, all else typed not-ready; remainder answers typed `unsupported-for-host` — honest but far from "implemented" breadth |
| C26 | session.open: emits opening→buffering→playing, tracks.list-ready + media.info-ready before completion, gen 0 | :227 | REAL-UNPROVEN | playback-engine/src/{session,events}.rs emit TracksListReady/MediaInfoReady (rg files); B: rc-smoke.txt "command-layer-play … PASS"; not executed by me end-to-end |
| C27 | session.close "frees decoder threads with a bounded join deadline (2 s [PROPOSAL])" | :228 | UNVERIFIED | Engine-internal; 2 s deadline not found in commands crate; playback-engine close path outside my file set |
| C28 | playback.seek: absolute/relative clamped to [0, duration]; "clamping is truth, not failure" | :238 | REAL-UNPROVEN | playback-engine/src/seek.rs:157-188 doc-comments + clamping impl ("Clamped to [0, duration] when the duration is known"); not executed by me |
| C29 | Superseded seeks emit no terminal event | :238 | REAL-UNPROVEN | correlation.rs DeliveryLog validates "exactly zero or one terminal seek event per gen" on observed streams (T: `the_full_host_side_pipeline_over_a_command_storm`); engine-side suppression not executed |
| C30 | video.set-fullscreen "target: shell", CLI: unsupported-for-host | :269 | REAL | spec.rs VideoSetFullscreen target: Shell; CLI `unserved()` returns UnsupportedForHost for video.* (apps/cli/src/dispatch.rs:164-174) |
| C31 | system.get-engine-info decoders `{name, kind, hardware, backend, active}` — truthful hardware | :328 | REAL | types.rs:553-564 DecoderInfo fields exact; B: mp4-smoke.md engine-info "decoders: mjpeg, pcm_s16le" (truthful — no fake hw flags) |
| C32 | system.list-commands "lets UIs… build menus… without hardcoding" | :334 | REAL | apps/control-server/src/dispatch.rs:84-101 catalog_listings() from live spec; T: `the_catalog_lists_all_65_commands_from_the_live_spec` |
| C33 | §2.13 Stage (13) + ErrorCode (17) taxonomy, kebab-case wire | :340-361 | REAL | commands/src/error.rs:15-103 exact variant sets; T: `command_error_roundtrips_on_the_wire` |
| C34 | "a failed open or seek is never reported as success" | :363 | REAL-UNPROVEN | Engine refusals are typed errors (B: mp4-smoke HEVC exit 65 "no decoder for hevc"); full engine audit outside my set |
| C35 | unknown ErrorCode values degrade to generic failure (older clients) | :363 | SHALLOW | Types allow forward-compat; no client-side degrade logic found in ui/src/client |
| C36 | §3.2 event table (23 events, payload shapes, gen column, rate column) | :377-401 | REAL | catalog/event.rs + spec.rs event_catalog_data (26 rows incl. issue-#42 additive trio per §6.2); T: `pipeline_scoped_set_matches_section_3_2`, `event_wire_form_matches_doc_example` |
| C37 | "state-changed, seek-completed, seek-failed, playback.error, tracks.changed, tracks.list-ready, media.info-ready are the completion evidence events" | :403 | REAL | catalog/event.rs completion-evidence set; T: `completion_evidence_set_matches_section_1_4` |
| C38 | time-tick "never queued — at most one tick in flight… newer tick replaces an undelivered older one" | :416 | UNVERIFIED | Rate recorded as data ("config-interval-never-queued", spec.rs:718-721); actual throttle lives in engine/host event loops — not traced end-to-end here |
| C39 | Burst bound "no more than 1 000 events/s… [PROPOSAL]" | :419 | PAPER-ONLY | No burst-bound implementation found (rg "1000 events"/burst bound: none) |
| C40 | §3.3 PlaybackState 8 states + legal transitions + "conformance-tested" | :407-412 | REAL | types.rs:288-330 `can_transition_to` matches §3.3 verbatim incl. 3.C1 edges and C8 self-transitions; engine state walk B: mp4-smoke.md Idle→…→Idle exit 0 |
| C41 | request_id "monotonic per host connection… starting at 1… engine echoes… never interprets" | :427 | REAL | correlation.rs RequestIdAllocator (T: `request_ids_are_monotonic_and_never_reused`, `responses_must_echo_allocated_requests`); control server enforces (C75) |
| C42 | §4.2 gen-bumping closed list (seek, set-position, step-frame, set-chapter, stop, set-video/audio-track, session.close) | :433 | REAL | spec.rs gen_bumping:true on exactly those 8 ids (rg); T: `gen_bumping_set_is_exactly_section_4_2` |
| C43 | "Accepted responses to gen-bumping commands carry the new gen" | :435 | REAL | envelope.rs:64-77 + StalenessFilter::observe_response; T: `accepted_responses_raise_the_generation_watermark`; engine-side minting UNVERIFIED |
| C44 | Host staleness rule: drop pipeline events with gen < highest observed | :436 | REAL | correlation.rs StalenessFilter; T: `stale_pipeline_events_are_dropped_current_ones_kept`, `watermarks_are_per_session` |
| C45 | "engine emits Accepted for R before any event caused by R; delivery preserves causal order end-to-end in every binding" | :443 | SHALLOW | Envelope/ordering types real (DeliveryLog T: `seq_must_be_strictly_increasing`); end-to-end per-binding proof absent — the §5.1-mandated Tauri funnel (C50) does not exist, no TauriMockHost (C83) |
| C46 | Control server "subscribe { topics } / unsubscribe… topic syntax: exact ID, area.*, area.>" | :450 | SHALLOW | protocol.rs:172-181 Subscribe/Unsubscribe REAL but exact-id strings only (server.rs subscriptions Vec<String>); wildcard `area.*`/`area.>` NOT implemented |
| C47 | "Default subscription after hello: nothing (deny-by-default)" | :450 | REAL | server.rs Session::new subscriptions empty; only subscribed kinds pass (no event source in v1 control-server, so unexercised on the wire) |
| C48 | Reconnect/hydration "call system.get-state-snapshot, then subscribe; seq gap detection [PROPOSAL]" | :451 | PAPER-ONLY | No reconnect logic in control client (client.rs is TestClient); control-server answers get-state-snapshot not-ready (dispatch.rs:66-70) |
| C49 | Desktop: "registers a single Tauri command lumen_dispatch" | :459-465 | REAL | apps/desktop-tauri/src/app.rs:293 `.invoke_handler(tauri::generate_handler![lumen_dispatch])`; apps/desktop-tauri/tests/single_command.rs enforces exactly-one |
| C50 | "the shell must forward both through the same tokio::mpsc before touching the webview" | :479 | FALSE | app.rs:150-168 forward_pending emits under the runtime mutex and returns the response after; no tokio::mpsc anywhere in desktop-tauri (rg mpsc: 0) |
| C51 | Wire examples: `{"requestId":42,"cmd":"playback.seek",…deltaMs:-10000…}` etc. | :469-477 | REAL | crates/commands/tests/wire_forms.rs:56,95 pin all three doc examples; T: wire_forms 7/7 pass |
| C52 | Events forwarded via channel "lumen://event" | :474-479 | REAL | app.rs:100 `pub const EVENT_CHANNEL: &str = "lumen://event"`; ui tauri-transport.ts LUMEN_EVENT_CHANNEL |
| C53 | "cargo xtask schema-gen emits (a) schemas/commands.schema.json… CI fails on drift" | :481 | SHALLOW | Artifacts + CI drift REAL: schemas/commands.schema.json + ui/src/generated/commands.ts + schemas/generated/commands.checksum.txt; ci.yml:279-280 `cargo run -p xtask --locked -- codegen --check`; BUT command is `codegen` (xtask mod.rs:95-99), not `schema-gen`, and TS path is ui/src/generated/commands.ts, not `ui/packages/commands/src/types.ts` |
| C54 | "(c) validation JSON Schemas reused by the control server and the keymap validator" | :481 | FALSE | Control server never loads the schema artifact (protocol.rs:220-224 does key-set + serde checks only); no keymap validator exists (C92) |
| C55 | "No TS type is ever hand-written" | :481 | REAL-UNPROVEN | ui/src/generated/commands.ts is generated (header "@generated… NEVER hand-edit"); full UI tree not swept for hand-written command types |
| C56 | "`ui/packages/client` wraps invoke+listen… implementing the staleness filter and C1–C6 once" | :483 | SHALLOW | ui/src/client/transport.ts + tauri-transport.ts implement send/subscribe seam only; NO staleness filter, NO waitFor/seekAndComplete in TS (rg waitFor/staleness: 0 real hits); no packages/ dir |
| C57 | §5.2 wasm32-unknown-unknown binding with same catalog | :487 | REAL-UNPROVEN | apps/web/src reuses commands crate (host.rs:1403+ match on commands::Command); B: rc-smoke.txt web-ui-build/browser-load PASS, wasm build passes |
| C58 | Web capability gaps "typed, not silent" (unsupported-for-host) | :492 | REAL | apps/web/src/open.rs:133-157 returns ErrorCode::UnsupportedForHost with tests asserting it (:535,:556) |
| C59 | "subtitle.add-file accepts a transferred Blob; session.open accepts… blobs [PROPOSAL]" | :493 | PAPER-ONLY | apps/web/src/protocol.rs: "the catalog's `SourceSpec` has no [blob] variant yet… host accepts `SourceSpec::Url { url: "[blob]:<name>" }`" — workaround, not the documented variant |
| C60 | Worker plan: engine in dedicated Web Worker, postMessage, transferables | :490 | REAL | apps/web/worker.js exists; apps/web/src/protocol.rs defines host↔worker message lanes |
| C61 | WebGPU/WebGL rendering + WebAudio output | :491 | REAL-UNPROVEN | apps/web/src/render/ + audio/ dirs exist; not audited (doc 09 domain) |
| C62 | COOP/COEP headers "served by apps/web" | :490 | UNVERIFIED | Not checked; apps/web serving stack outside my file set |
| C63 | CLI "--json emits every event as one NDJSON line… --control-stdio" | :499-500 | REAL | apps/cli/src/run.rs:298-319 machine mode; tests `control_stdio_answers_ndjson`, `control_stdio_malformed_request_is_typed` |
| C64 | §5.3 Top-30 subcommand mapping (open/close/play/pause/toggle/stop/seek/volume/mute/tracks/…/log-level) | :504-531 | SHALLOW | apps/cli/src/help.rs SUBCOMMANDS: open, close, play, pause, toggle, stop, seek, info, time, duration, tracks, engine-info, settings = ~15 of the doc's ~30 rows; volume/mute/rate/step/audio-track/video-track/subtitle-track/subtitle-add/chapters/devices/screenshot/enqueue/next/prev/playlist save-load/shuffle/repeat/plugins/log-level NOT implemented (typed unsupported-for-host) |
| C65 | "lumen open <path> [--start-time 30s] [--rate 1.5] …" | :506 | SHALLOW | parse.rs implements --start-time/--start-paused/--no-video/--no-audio; `--rate` absent (rg "rate" parse.rs: no flag) |
| C66 | "Settings surface as generated long flags… CLI never defines an option outside the one schema" | :533 | REAL-UNPROVEN | help.rs:123-127 settings-derived flags sections; full schema-only option audit is doc-07 domain |
| C67 | "collisions… rejected at build time by cargo xtask config-validator" | :533 | REAL-UNPROVEN | tools/xtask/src/cmd/config_validator.rs exists + CI runs it (B: xtask-config-validator.txt FAILURE on brand assets — it executes); short-flag-collision check not verified |
| C68 | Control server "Loopback-only bind… non-loopback address is a startup error" | :539 | REAL | server.rs:301-368 bind() refuses + re-verifies bound addr; conformance tests loopback refusal |
| C69 | "control-server.enabled defaults to false" | :539 | REAL | product.toml:167 `control-server = false # Phase 5+ [PROPOSAL]` |
| C70 | Token distribution "(a) file mode 0600, (b) --control-token-fd, (c) --print-token" | :541 | FALSE | Only STDERR print exists: apps/control-server/src/main.rs:230-235 `eprintln!("control-server session token (keep private): {}", …)`; rg control-token-fd/0o600/runtime-dir: 0 matches |
| C71 | "256-bit random token per start… constant-time… close 4401" | :541 | REAL | token.rs:28-48 (base64 of 32 bytes); server.rs constant_time_eq + Unauthorized close; T (conformance): `wrong_token_closes_4401_and_counts` |
| C72 | "5 failures impose a 60 s lockout and a log entry" | :541 | REAL | server.rs:102-128 AuthLimits::documented {5, 60s}; conformance `five_failures_lock_out_even_the_right_token` (B: cargo-test-workspace.txt passes) |
| C73 | Read-only mode: unauthenticated clients get "fixed safe subset (media.get-info, system.get-engine-info, tracks.list, snapshot) and nothing else" | :541 | SHALLOW | policy.rs:65-70 READ_ONLY_SAFE = exactly those 4 ids (REAL); but CatalogDispatch answers the 3 non-introspection ones not-ready anyway (no engine) — subset is policy-real, service-moot |
| C74 | "Every frame: max 1 MiB; schema-validated against schemas/commands.schema.json" | :542 | SHALLOW | 1 MiB caps REAL (server.rs:69-94, conformance `oversized_length_claim_closes_1009_before_allocation`); "schema-validated against the artifact" FALSE — raw key-set + serde only (protocol.rs:233-276), artifact never loaded |
| C75 | "request_id monotonicity enforced per connection (anti-replay)" | :542 | REAL | server.rs:766-777 non-monotonic id ⇒ protocol-error + close |
| C76 | "per-connection concurrency cap (8 in-flight) and rate limit (100 req/s burst 200)" | :542 | FALSE | apps/control-server/src/lib.rs:33-35: "the [PROPOSAL] rate limits and the Origin allowlist of doc 05 §5.4 are recorded gaps"; server is single-connection synchronous (server.rs:30-33) |
| C77 | "Origin check… server rejects any origin not on the configured allowlist" | :543 | FALSE | Same recorded gap (lib.rs:33-35); no Origin handling in http.rs upgrade path |
| C78 | "Tokens never logged; URLs credential-stripped before logging" | :544 | REAL-UNPROVEN | logging.rs advertises token-free JSON logs (lib.rs:19-22); full log-line sweep not done |
| C79 | Protocol envelope `{"v":1,"type":"hello"|"welcome"|…}` | :540 | SHALLOW | protocol.rs:6-12 real envelope; `v` appears on `welcome` only, `hello` carries `versions` list — substance REAL, per-frame shape differs from the sketch |
| C80 | §6.2 additive-only rules within a major | :556-560 | REAL | Practiced: catalog grew 61→65 via documented additive entries (ids.rs:90-92,175-179); checksum+golden tests make retyping/renaming fail (T: `mutated_table_entry_changes_the_checksum`) |
| C81 | §6.4 unavailable commands return "feature-not-exposed… with the missing feature named in details" | :568 | SHALLOW | Hosts return `unsupported-for-host` with owner detail (apps/cli/src/dispatch.rs:164-174); `FeatureNotExposed` is never produced by any Rust host (rg: only error.rs def + ui MockTransport); product.toml∩features gating not consulted per-command |
| C82 | "crates/testkit defines the host abstraction and one data-driven conformance suite; scenario file tests/contract/scenarios/*.toml" | :576-593 | FALSE | No tests/contract/ dir (find: 0); no scenario tomls; testkit = alloc_count/faults/Must (lib.rs:33-34) |
| C83 | Four hosts "TauriMockHost / WasmNodeHost / CliHost / ControlHost" | :601-604 | FALSE | No types with these names (rg outside docs: 0); real but disjoint tests exist: crates/control-protocol/tests/conformance.rs, apps/cli/tests/*, apps/desktop-tauri/tests/single_command.rs — no shared host abstraction |
| C84 | Scenario categories incl. stale-generation races, ordering invariants, redaction, throttle conformance | :595 | PAPER-ONLY | No scenario engine; some invariants covered by type-level tests in commands crate (T) |
| C85 | "The suite is merge-blocking in CI and runs on Windows, Linux and macOS" | :606 | FALSE | ci.yml:43-48: "THIS workflow (merge-blocking): ubuntu-only… ci-full.yml [PROPOSAL, non-blocking]: the parked 3-OS matrix" |
| C86 | §7.3 "typed client must not resolve andComplete() before the terminal event… makes C1–C8 executable" | :610 | PAPER-ONLY | No andComplete/waitFor anywhere in ui/src/client (rg: 0) |
| C87 | §7.4.1/2 eslint/dependency-cruiser import bans in CI | :614-615 | FALSE | ui/eslint.config.js contains neither (rg no-restricted: 0 matches) |
| C88 | §7.4.3 "architecture test reads the desktop shell's invoke_handler registration" | :616 | REAL | apps/desktop-tauri/tests/single_command.rs (scans app.rs for generate_handler![…]) |
| C89 | §7.4.4 "only @lumen/wasm-client may import the wasm package (build-time module graph assertion)" | :617 | PAPER-ONLY | No wasm-client package, no module-graph assertion (rg wasm-client in ui/: 0) |
| C90 | §7.4.5 cargo boundary check; "the testkit depends on production crates, never the reverse" | :618 | REAL | boundaries.toml (root) + tools/xtask check_architecture; testkit in dev_only allowlist (boundaries.toml `dev_only = ["testkit"]`) |
| C91 | Keymap at "config/keymaps/default.toml… [[binding]] rows… schema = keymap.v1" | :626-637 | FALSE | Actual keymaps/default.toml:28-31 self-flags: "doc 05 §8.1 specifies `config/keymaps/default.toml` with `[[binding]]` rows…; this seed follows card 0.B3 and D-003 (`keymaps/*.toml`, section-grouped `binding-id = { … }` entries)" — different path AND format |
| C92 | "Validated by schemas/keymap.schema.json and cargo xtask config-validator" | :626 | FALSE | schemas/keymap.schema.json does not exist (ls schemas/: commands/settings/product/packaging only); config-validator does not mention keymaps (rg in tools/xtask: only feature_audit.rs) |
| C93 | "Validator failures are hard errors: unknown command ID, args schema, duplicate key+mode…" | :639 | PAPER-ONLY | No validator; UI consumption seam ui/src/screens/player/keymap.ts parses TOML subset and rejects duplicate binding ids, but ships "a verbatim excerpt of the three shell-relevant bindings" hardcoded (keymap.ts:7-12) — the loader "has not landed" |
| C94 | §8.2 default keymap seed (Space toggle, arrows ±10 s, S/N/P, media keys…) | :645-671 | REAL | keymaps/default.toml:83-102: Space→toggle-play-pause, Left/Right→seek ±10000 ms, S→stop, N/P→playlist next/prev — matches doc rows; seed is a superset pinned to VLC 3.0.23 + design pkg (doc table marked [PROPOSAL], superseded) |
| C95 | §8.3 "layouts/menus.toml declares menu trees… dynamic submenus… populated from queries" | :677 | FALSE | No layouts/ dir and no menus.toml anywhere (find: 0); menus live in UI code (doc 08 domain) |
| C96 | "Menu items whose command is not in the effective feature set render disabled from data" | :690 | PAPER-ONLY | Depends on §6.4 feature-data plumbing (C81 SHALLOW) and menus.toml (C95 FALSE) — neither exists |
| C97 | §8.4 "layouts/ui-actions.toml (id, label_key, capability…), schema-validated" | :694 | FALSE | File does not exist (find ui-actions.toml: 0); keymap seed references ui_action ids but there is no registry file |
| C98 | "Unfinished: None. All eight assigned areas are covered (§1–§8)" | :711 | SHALLOW | The doc covers §1–§8 as DESIGN; §7 conformance suite and §8.1/8.3/8.4 data layer are unimplemented (C82-C97) — "covered" ≠ implemented, in tension with :6 |
| C99 | playback.seek Errors (specific): "not-ready, not-seekable, out-of-range" | :238 | FALSE | crates/playback-engine/src/event.rs seek_error_wire maps `SeekFailure::NotSeekable => (Control, Unsupported, …)` and `OutOfRange => (Control, Unsupported, …)`; `commands::ErrorCode::NotSeekable` is never constructed in production code (rg: only tests + cli/src/exit.rs dead mapping) — the documented typed codes never appear on the wire |

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| :6 "Status: implemented" | :5 "planning document — type sketches, no implementation"; §7/§8 unimplemented | The types/data/wire ARE implemented (tests ran); the conformance suite and data-layer files are NOT — "implemented" is inflated |
| :9 "crates/testkit (conformance)" | crates/testkit/src/lib.rs:1 "test kit, v0 (doc 03 §8; card 1.C3)" — alloc counters + faults | testkit is not the conformance crate; doc 05's §7 owner does not exist |
| :481 TS types at "ui/packages/commands/src/types.ts", client "ui/packages/client" | ui/src/generated/commands.ts + ui/src/client/ (no ui/packages/) | The artifacts exist at different paths; the doc's paths are stale |
| :542 control server "schema-validated against schemas/commands.schema.json" | protocol.rs:220-226 "validate_request_shape checks the raw JSON object's key set… because #[serde(flatten)] envelopes cannot carry deny_unknown_fields" | Key-set + serde validation only; the schema artifact is never loaded — the doc's claim is false as written |

## 4. Fabrication / inflation indicators

1. **"Status: implemented"** on a self-declared planning doc — the single
   biggest inflation; ~21 of 94 claims are PAPER-ONLY and 12 FALSE.
2. **§7 conformance suite** is described in operational detail (scenario TOML
   grammar, host table, CI OS matrix) that matches nothing in the tree; the
   merge-blocking CI is ubuntu-only and the 3-OS matrix is explicitly parked
   (ci.yml:43-48). Writing "merge-blocking… Windows, Linux and macOS" for a
   non-existent suite is documentation-fiction.
3. **§5.4 security posture**: the doc states rate limits, concurrency caps,
   Origin checks and three token-distribution mechanisms as design; the code
   honestly records them as gaps (apps/control-server/src/lib.rs:31-36) — the
   doc never mentions the gaps.
4. **Hardcoded keymap excerpt in production UI** (ui/src/screens/player/
   keymap.ts:7-12 "seeded from a verbatim excerpt of the three shell-relevant
   bindings") while the doc claims schema-validated keymap files with user
   layering.
5. Counter-indicator (to the repo's credit): the commands crate's tests pin
   THIS doc's tables (class counts, gen-bumping set, completion-evidence set,
   wire examples) — the implemented half is aggressively drift-guarded, and
   every host refuses unserved commands with typed errors instead of faking
   success.

## 5. Matrix rows

N/A (not a matrix chunk). Cross-reference for matrix auditors: doc 05 §5.2
"Feature-matrix web column tracks each gap" — see C58 (gaps typed, real) and
B: xtask-parity-report.txt (0/252 verified — the matrix itself is the
problem, not the typed refusals).
