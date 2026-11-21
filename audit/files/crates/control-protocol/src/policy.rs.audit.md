# AUDIT: crates/control-protocol/src/policy.rs
| Field | Value |
|---|---|
| Auditor | W02-A09 |
| Date | 2026-11-21 |
| Lines / functions | 415 / 17 (2 const tables, is_sensitive, 4 config structs + error type, ServerPolicy ×5 methods, request_command_id, 6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen-control-server` binary → `apps/control-server/src/config.rs:24-25` uses `PolicyFile`/`ServerPolicy`; per-request path `crates/control-protocol/src/server.rs:786-788` `request_command_id(&request)` → `self.policy.check(id, self.authenticated)`; crate re-exported at lib.rs) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | `sed -n` full read of policy.rs; `rg "sensitive\|privileged" docs/05-command-layer.md` (§5.4:541 privilege rationale; §2.9:291 enqueue row); `rg "PlaylistEnqueue|PlaylistPlayItem" crates/commands/src/catalog/command.rs` (SourceSpec payloads); `rg "enum SourceSpec" -A 12 types.rs` (File{PathBuf} variant); `sed -n '60,120p' apps/control-server/src/dispatch.rs` (NotReady fall-through — F-001 mitigation); `rg "ALL.len" crates/commands/src/catalog/ids.rs` (65); `rg "policy::tests::" audit/runs/baseline/test-workspace.log.txt` (6/6 ok at 1439-1444); `rg "PolicyFile" apps/control-server/src/config.rs` (app wiring) |

## 1. One-paragraph verdict
This is a real, tested, deny-by-default policy engine: a closed 10-id sensitive classification, a 4-id read-only safe subset that matches doc 05 §5.4's named list exactly, typed startup errors for grant typos, strict serde config shapes, and 6/6 unit tests green in the baseline run plus conformance-level denial tests. Its one structural defect is a hole in the very boundary it exists to enforce: `session.open` is sensitive "because it touches the local filesystem" (doc 05 §5.4:541), but `playlist.enqueue` (which carries an arbitrary `SourceSpec::File { path }` and is NOT sensitive) followed by `playlist.play-item`/`next`/`prev`/`skip` (the "open lifecycle" family, also NOT sensitive) opens the same local file with nothing but token authentication — no `sensitive_allow` grant. The shipped binary currently cannot be exploited this way only because its dispatch answers every non-introspection command with an honest `not-ready` (no engine embedded); the policy crate is the reusable boundary the engine-embedding composition will inherit, and it classifies the playlist open-lifecycle lane as ordinary. Three lesser defects: a stale "64 ids" count in an operator-facing error (catalog is 65), silent acceptance of no-op grants, and a test that checks 3 of the 4 read-only-safe ids.

## 2. What actually works (verified)
— All 6 unit tests passed in baseline: `audit/runs/baseline/test-workspace.log.txt:1439-1444` (`policy::tests::* … ok`).
— READ_ONLY_SAFE matches doc 05 §5.4:541's named subset exactly ("`media.get-info`, `system.get-engine-info`, `tracks.list`, snapshot") — compared by eye, both sides read.
— Deny-by-default + grant flow verified at three levels: unit (`deny_by_default_and_granted_when_enabled`), conformance (`sensitive_command_denied_by_default_and_connection_survives`, `sensitive_command_allowed_when_config_grants_it`, `unauthenticated_read_only_mode_serves_the_safe_subset_only` — all baseline ok), app e2e (`a_typoed_sensitive_grant_is_a_typed_startup_error`, `a_non_loopback_bind_in_the_operator_config_is_refused_at_bind`).
— Sensitive list is compile-coupled to the catalog (uses `CommandId` variants; a rename breaks the build) and round-trip-asserted (`sensitive_list_is_exactly_the_documented_categories`).
— `PolicyFile` config family uses `deny_unknown_fields` on every section — pinned by `policy_file_serde_shape_is_strict` (unknown key `"hostname"` errors).

## 3. Findings

### F-001 [S2]
Where: crates/control-protocol/src/policy.rs:33-55 (the classification) vs docs/05-command-layer.md:541 and §2.9:291
Code:
```rust
pub const fn sensitive_command_ids() -> &'static [CommandId] {
    &[
        // session: opens LOCAL filesystem sources (doc 05 §5.4 — "session.open
        // stays privileged because it touches the local filesystem").
        CommandId::SessionOpen,
```
What is wrong: The classification rule (module doc, lines 30-32: "commands that mutate settings, touch filesystem paths, or change plugin state") is not applied to the playlist open-lifecycle lane. `Command::PlaylistEnqueue { source: SourceSpec, .. }` (command.rs:290-295) accepts `SourceSpec::File { path: PathBuf }` (types.rs:222-227) — an arbitrary local path — and is NOT sensitive (doc 05 §2.9:291: "Does not open the item"). `playlist.play-item`/`next`/`prev`/`skip` (doc 05 §2 class-A, "open lifecycle") perform the actual open and are NOT sensitive. So the composite sequence enqueue(file path) → play-item(id) opens an arbitrary local file over the control wire with only token authentication, while the identical effect via `session.open` requires an explicit operator `sensitive_allow` grant. The documented privilege rationale ("`session.open` stays privileged because it touches the local filesystem", doc 05 §5.4:541) is defeated by a two-command detour.
Why it matters: The sensitive-op policy is the operator's only control over filesystem-touching operations on this surface. A token-authenticated client (the token is printed to the terminal / written 0600; any local process or a compromised remote-UI page that captured it) can open arbitrary local files as media — metadata and demux-error oracles disclose file structure, and the file's content is decoded locally. Not S1 because the shipped binary currently cannot reach it: `apps/control-server/src/dispatch.rs:65-70` answers every non-introspection command `NotReady` ("this control-server build embeds no engine"), so no file is opened today. The defect is in the reusable policy boundary that the "engine-embedding composition" (dispatch.rs:4-5) will wire up.
What correct looks like: Either classify `PlaylistPlayItem`/`PlaylistNext`/`PlaylistPrev`/`PlaylistSkip` sensitive (they open sources), or gate file-scheme sources at the dispatch/policy seam regardless of which command carries them (a `source_is_local(SourceSpec)` check alongside the id check).
Confidence: HIGH — Verification: read of policy.rs, command.rs:290-306, types.rs:222-233, doc 05:541/§2.9:291, dispatch.rs:65-70.

### F-002 [S4]
Where: crates/control-protocol/src/policy.rs:85-88
Code:
```rust
            "sensitive_allow entry {:?} is not a catalog command id: {} \
             (see the 64 ids in the doc 05 §2 tables / system.list-commands)",
```
What is wrong: The catalog has 65 command ids, not 64 — `crates/commands/src/catalog/ids.rs:279` asserts `CommandId::ALL.len(), 65`, and the app e2e asserts 65 listings ("65 since w31.1's session.set-frame-channel"). The operator-facing hint embeds a stale count that drifted when the 65th command landed and nothing in this crate's tests pins it.
Why it matters: An operator following the hint counts 65 ids against a message claiming 64 — small, but it is exactly the class of stale-literal drift this repo's lint-literals regime exists to catch, in a user-facing string.
What correct looks like: Derive the count (`CommandId::ALL.len()`) into the message, or drop the number.
Confidence: HIGH — Verification: read + ids.rs:279 + e2e.rs:313-315.

### F-003 [S4]
Where: crates/control-protocol/src/policy.rs:187-201
Code:
```rust
        for entry in &file.policy.sensitive_allow {
            match entry.parse::<CommandId>() {
                Ok(id) => {
                    grants.insert(id);
                }
```
What is wrong: Any valid catalog id is accepted into the grant set, including ids that are not sensitive (e.g. `playback.play`, `media.get-info`). Such a grant is a silent no-op: no error, no warning, no effect. The doc comment three lines up (180-182) states the design rule "typos must surface at startup … never silent no-ops" — an operator granting an ordinary id (a plausible mistake: granting the command they *think* guards the filesystem, e.g. `playlist.play-item` per F-001's confusion) gets silent nothing.
Why it matters: Grant-list review is the operator's security workflow; a grant that neither errors nor does anything is invisible in review and audit.
What correct looks like: `from_file` should reject (or at least warn via a typed outcome) entries that parse but are not in `sensitive_command_ids()`.
Confidence: HIGH — Verification: read; no test grants a non-sensitive id.

### F-004 [S4]
Where: crates/control-protocol/src/policy.rs:340-363 (test) 
Code:
```rust
        assert!(policy.check(CommandId::MediaGetInfo, false).is_ok());
        assert!(policy.check(CommandId::SystemGetEngineInfo, false).is_ok());
        assert!(policy.check(CommandId::TracksList, false).is_ok());
```
What is wrong: The test asserts the unauthenticated-ok path for 3 of the 4 `READ_ONLY_SAFE` ids — `SystemGetStateSnapshot` is never asserted on the ok path (only implicitly via the const table). Additionally, `sensitive_list_is_exactly_the_documented_categories` (277-293) verifies internal consistency (len, round-trips, spot checks) — its name promises conformance to "the documented categories" it never consults.
Why it matters: If `SystemGetStateSnapshot` were dropped from `READ_ONLY_SAFE` (e.g. someone decides snapshots leak state), no test fails — the read-only contract silently narrows. Test names that overclaim breed false confidence.
What correct looks like: Loop over `READ_ONLY_SAFE` and assert each `check(id, false)` is ok in read-only mode; rename the list test to what it does.
Confidence: HIGH — Verification: read; grep for `SystemGetStateSnapshot` in the test module → only the const table, no check() call.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `sensitive_command_ids` | 33-55 | REAL | S2 (F-001) | 10 ids, compile-coupled to catalog; misses the playlist open-lifecycle lane (file-path enqueue + play) |
| `is_sensitive` | 58-60 | REAL | — | Linear scan over 10 ids, fine on cold path |
| `READ_ONLY_SAFE` | 65-70 | REAL | — | Exact match to doc 05 §5.4:541's named subset |
| `PolicyConfigError` (+Display/+Error) | 73-92 | REAL | S4 (F-002) | Stale "64 ids" literal; message otherwise actionable |
| `PolicyFile`/`ServerSection`/`AuthSection`/`PolicySection` | 111-162 | REAL | — | `deny_unknown_fields` + `default` on all four; strict-shape pinned by test; TOML parsing deliberately owned by the host app (comment 366-371) |
| `ServerPolicy::deny_all_sensitive` | 176-178 | REAL | — | = `Default`; the strict default posture |
| `ServerPolicy::from_file` | 187-206 | REAL | S4 (F-003) | Typed errors for unknown ids (good); silent no-op grants for non-sensitive ids |
| `ServerPolicy::check` | 215-238 | REAL | S2 (via F-001) | Unauth ⇒ read-only safe subset only; auth ⇒ non-granted sensitive denied; correct per-command logic, wrong classification input; `with_detail("command", …)` keeps the id machine-readable |
| `ServerPolicy::granted` | 241-243 | REAL | — | Iterator for caps/log |
| `ServerPolicy::read_only` | 246-248 | REAL | — | Flag accessor used by server.rs welcome capabilities |
| `request_command_id` | 256-262 | REAL | — | Cold-path: full `to_value` round-trip to read the `"cmd"` tag; `unwrap_or(Value::Null)` swallows a serde failure into the caller's "internal invariant breach" error — acceptable; ~2× message-size transient allocation at control-server rates |
| test `sensitive_list_is_exactly_the_documented_categories` | 277-293 | REAL | S4 (F-004) | Pins len=10 + round-trips + 3 spot categories; name overclaims doc conformance |
| test `deny_by_default_and_granted_when_enabled` | 296-323 | REAL | — | Real 3-way behavior: denied by default, granted via file, others still denied, ordinary passes |
| test `unknown_grant_entries_are_typed_errors` | 326-337 | REAL | — | Typo surfaces with the offending entry in the message |
| test `unauthenticated_sessions_get_the_read_only_subset_only` | 340-363 | REAL | S4 (F-004) | 3 of 4 safe ids asserted; negative cases (play/open/config) all covered |
| test `policy_file_serde_shape_is_strict` | 366-401 | REAL | — | Unknown-key error, full-shape parse, empty-file defaults — pins the serde contract the host's TOML parser rides on |
| test `request_id_extraction_covers_payload_shapes` | 404-414 | REAL | — | No-payload, unit-payload, struct-payload commands |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| sensitive_list_is_exactly_the_documented_categories | PARTIAL | Catches a variant rename (compile), a broken FromStr, a list edit changing len. Does NOT catch a *wrong* classification decision (F-001's missing playlist lane passes all its asserts). |
| deny_by_default_and_granted_when_enabled | YES | Mutation: make `check` ignore grants → `policy.check(ConfigSetSetting, true).is_ok()` assert fails. |
| unknown_grant_entries_are_typed_errors | YES | Mutation: skip unparseable entries → Ok(policy) hits `unreachable!("typo accepted")`. |
| unauthenticated_sessions_get_the_read_only_subset_only | PARTIAL | Mutation: drop `MediaGetInfo` from READ_ONLY_SAFE → fails. Mutation: drop `SystemGetStateSnapshot` → passes silently (F-004). |
| policy_file_serde_shape_is_strict | YES | Mutation: remove `deny_unknown_fields` → the `"hostname"` dirty-case assert fails. |
| request_id_extraction_covers_payload_shapes | YES | Mutation: return None always → all three asserts fail. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "commands that mutate settings, touch filesystem paths, or change plugin state" are the sensitive class | policy.rs:30-32 | SHALLOW | Rule applied to 10 ids; the playlist file-path enqueue + open-lifecycle lane escapes it (F-001) |
| "`session.open` stays privileged because it touches the local filesystem" | docs/05-command-layer.md:541 | FALSE (as a boundary) | Bypass via enqueue(SourceSpec::File) + play-item, neither sensitive (F-001) |
| read-only safe subset = "media.get-info, system.get-engine-info, tracks.list, snapshot" | docs/05-command-layer.md:541 | REAL | READ_ONLY_SAFE:65-70 matches 1:1; tested at unit + conformance level |
| "deny-by-default is structural" (grants are the only opt-in) | docs/05-command-layer.md:541, policy.rs module doc | REAL | `ServerPolicy::default()` = empty grants; conformance tests baseline-green |
| "typos must surface at startup — never silent no-ops" | policy.rs:180-182 | SHALLOW | True for unknown ids; false for valid-but-non-sensitive grants (F-003) |
| "membership is asserted against the live catalog in tests" | policy.rs:16-17 | REAL (compile+round-trip) | Ids are `CommandId` variants (rename = build break); round-trip assert at 281-284 — but no doc-conformance assert (F-004) |
| `playlist.enqueue` "Does not open the item" | docs/05-command-layer.md:291 | REAL | Classification is per-command accurate; the composite bypass is the hole (F-001) |

## 7. Dead / duplicated / unreachable code
- No dead code in this file. `sensitive_command_ids` is re-exported (lib.rs) and used by the app's config validation; `granted()` has no caller in the workspace (rg: only tests/none) — check: `rg "granted()" apps crates` — it is a public API accessor with zero production callers; candidate dead-public-API (S4-adjacent, noted for the hygiene wave; not filed separately).

## 8. Dependencies & flags
- Deps: `commands` (wire types, CommandId, CommandError) + `serde` (config shapes) + std BTreeSet. No feature flags; compiled into `lumen-control-server`. No unsafe/unwrap/expect in production paths (the single `unwrap_or` is the documented Null fallback at 257).

## 9. Open questions for the next agent
- Security wave (8.A2 owner): F-001 needs a decision — classify the open-lifecycle playlist commands sensitive, or gate `SourceSpec::File` at the seam. Also the Origin allowlist gap (lib.rs honest-scope) compounds this: a browser page that obtains the token has no origin check either.
- commands crate auditors: is `session.set-frame-channel` (the 65th id) supposed to be sensitive? It reconfigures the frame channel — check §2.1's notes when auditing ids.rs/spec.rs.
- Dispatch auditor: when the engine-embedding composition lands, `CatalogDispatch`'s NotReady fall-through must NOT silently become a pass-through without revisiting F-001.
