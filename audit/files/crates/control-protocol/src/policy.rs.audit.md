# AUDIT: crates/control-protocol/src/policy.rs
| Field | Value |
|---|---|
| Auditor | W02-A03 |
| Date | 2026-11-23 |
| Lines / functions | 415 / 11 prod items (2 const lists, 1 fn, 7 methods/impls, 3 serde structs + 3 sections), 6 tests |
| File verdict | UNREACHABLE (code is real and tested; no shipped entry point compiles it) |
| Reachable from shipped entry point? | NO — control-protocol is a dependency of apps/control-server ONLY (rg over all Cargo.tomls: 2 hits, the crate itself + apps/control-server); the release lane `cargo build --release --locked -p lumen-cli -p desktop` never compiles it (baseline cargo-build-release-lumen-cli.txt has no "Compiling control-protocol" line; desktop log has none); product.toml:167 `control-server = false # Phase 5+ [PROPOSAL]`, and apps/control-server/src/config.rs test `repo_product_today_refuses_the_feature_gate` (baseline :1659, ok) proves the binary refuses to start under the real product model. If built with the flag flipped: main → run → server → `self.policy.check(id, self.authenticated)` (crates/control-protocol/src/server.rs:788) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `./target/debug/deps/control_protocol-30329beb10b168e9 policy::tests` -> 6 passed, 0 failed (exit 0); python3 count of CommandId enum -> 65; rg sweeps (dependents, callers, doc 05 §5.4) |

## 1. One-paragraph verdict
The policy code is honest and fail-closed: `check()` denies unauthenticated sessions everything except the four read-only ids (only in read-only mode), denies authenticated sessions every ungranted sensitive op, and the caller (server.rs:786-794) refuses requests whose command id cannot be extracted rather than executing them — I verified the seam reads `None => Err(...Internal...)`. The deny-by-default semantics, the READ_ONLY_SAFE list, and the "connection survives, only the op is refused" contract all match doc 05 §5.4 verbatim, and the real-TOML claim in the test comment is backed by apps/control-server/tests/e2e.rs (tests pass in baseline :1681-1686). The damning fact is reachability: none of this ships. The crate is compiled by exactly one binary (apps/control-server) that the release lane never builds and that refuses to start under the repo's own product.toml (`control-server = false`). The security posture advertised in the module doc ("deny-by-default is structural, the same posture as the loopback bind") describes a server that no shipped user can run. Residual nits: a stale "64 ids" count in the operator-facing error (catalog is 65), a bind doc narrower than the enforced set, an operator-misleading `read_only` name, and `sensitive_allow` silently accepting non-sensitive ids.

## 2. What actually works (verified)
— verified by `./target/debug/deps/control_protocol-30329beb10b168e9 policy::tests` -> `test result: ok. 6 passed; 0 failed` (exit 0). Baseline corroborates (cargo-test-workspace.txt:1584-1590).
— verified by reading the enforcement seam crates/control-protocol/src/server.rs:786-794: `request_command_id` returning `None` produces a typed `Internal` refusal, never execution; policy check failure sends a `Denied` frame and the connection continues (server.rs:796-801) — exactly the module doc's contract.
— verified by reading apps/control-server/tests/e2e.rs:169-247 (operator TOML parse, dirty TOML typed error, typoed grant startup error, non-loopback bind refused) — all `ok` in baseline :1681-1684, so the policy.rs test comment "Real-TOML parsing is covered by the app's integration tests" is true.

## 3. Findings
F-001 [S3]
Where: crates/control-protocol/src/policy.rs:1-20 (module doc), whole file
Code:
```
//! The classification itself ([`sensitive_command_ids`]) is a closed,
//! documented list derived from the doc 05 §2 tables' own notes (e.g.
//! §5.4: "`session.open` stays privileged because it touches the local
//! filesystem"); membership is asserted against the live catalog in
```
What is wrong: the entire deny-by-default policy is unreachable from the shipped product. Proof chain: (1) only apps/control-server depends on control-protocol (rg over every Cargo.toml); (2) the release lane builds only lumen-cli and desktop, and neither build log contains control-protocol; (3) product.toml:167 `control-server = false` — the app's own feature gate refuses startup (config.rs:149-159 `require_feature`, tested by `repo_product_today_refuses_the_feature_gate`, baseline ok). The module presents itself as the shipped security posture ("the same posture as the loopback bind"); in the shipped builds this posture does not exist. This is pre-built Phase-5 infrastructure honestly flagged `[PROPOSAL]` in product.toml — not fraud — but per the audit's reachability rule it is dead code in the release lane, and any security claim resting on it (MP §12 "local control server binds to loopback") is currently vacuous. Confidence: HIGH — Verification: read + rg + baseline logs.
F-002 [S4]
Where: crates/control-protocol/src/policy.rs:81-89
Code:
```
            "sensitive_allow entry {:?} is not a catalog command id: {} \
             (see the 64 ids in the doc 05 §2 tables / system.list-commands)",
```
What is wrong: the catalog has 65 ids, not 64 (python3 count of the `CommandId` enum: 65; apps/control-server/src/dispatch.rs:130 pins `assert_eq!(listings.len(), 65)` and passes in baseline :1663 — its own comment says w31.1 minted `session.set-frame-channel` and "w31.1 missed these control-server pins"). The operator-facing error message understates the catalog by one and was missed by the same w31.1 sweep that fixed dispatch.rs. Confidence: HIGH — Verification: ran (count) + read.
F-003 [S4]
Where: crates/control-protocol/src/policy.rs:126
Code:
```
    /// Bind host; MUST be `127.0.0.1` or `::1` (refused otherwise).
    pub bind: String,
```
What is wrong: the enforced set is wider — server.rs:319 accepts `"127.0.0.1" | "::1" | "[::1]" | "localhost"` (and re-verifies the bound socket `is_loopback()` at server.rs:344). The doc says `localhost` is refused; it is not. No security impact (every accepted spelling is loopback), but the doc misstates the contract. Confidence: HIGH — Verification: read (server.rs:315-344) + baseline e2e `a_non_loopback_bind_in_the_operator_config_is_refused_at_bind ... ok`.
F-004 [S4]
Where: crates/control-protocol/src/policy.rs:130-132
Code:
```
    /// Read-only mode (doc 05 §5.4): unauthenticated clients admitted
    /// to [`READ_ONLY_SAFE`] and nothing else.
    pub read_only: bool,
```
What is wrong: the name promises a read-only server; the semantics (and doc 05 §5.4, and `check()` L215-225) only widen admission for tokenless clients — an authenticated client in read-only mode may still `playback.play`, `audio.set-volume`, mutate playlists, etc. The doc comment is scoped correctly, so this is a naming trap for operators, not a behavior bug. Confidence: HIGH — Verification: read (check() logic + doc 05 §5.4).
F-005 [S4]
Where: crates/control-protocol/src/policy.rs:187-206
Code:
```
        for entry in &file.policy.sensitive_allow {
            match entry.parse::<CommandId>() {
                Ok(id) => {
                    grants.insert(id);
                }
```
What is wrong: `sensitive_allow` accepts any valid catalog id — granting an ordinary command (e.g. `playback.play`) parses, is stored, is advertised by `granted()` as a "granted sensitive id" in the welcome capabilities, and silently does nothing (ordinary ids bypass the sensitive check regardless). Unknown ids are typed errors, but valid-but-ordinary ids should also be rejected (or at least warned) so operator mistakes surface at startup like typos do. Confidence: HIGH — Verification: read (is_sensitive path in check()).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| sensitive_command_ids | 33-55 | REAL | — | closed 10-id list; every id exists in catalog (round-trip test); classification matches doc 05 categories (fs/settings/plugins) |
| is_sensitive | 58-60 | REAL | — | linear contains over 10 — fine (cold path) |
| READ_ONLY_SAFE | 65-70 | REAL | — | exactly doc 05 §5.4's four ids; snapshot type carries no paths/URIs (types.rs:757-777) so V8 holds |
| PolicyConfigError Display/Error | 81-92 | REAL | S4 (F-002) | stale "64 ids" count |
| PolicyFile + sections (serde) | 111-162 | REAL | S4 (F-003, F-004) | deny_unknown_fields strict; defaults = strict posture (tested) |
| ServerPolicy::deny_all_sensitive | 176-178 | REAL | — | honest alias of Default |
| ServerPolicy::from_file | 187-206 | REAL | S4 (F-005) | unknown ids typed errors; ordinary ids silently accepted |
| ServerPolicy::check | 215-238 | REAL | — | fail-closed both branches; matches doc 05 §5.4 exactly |
| ServerPolicy::granted / read_only | 241-248 | REAL | — | accessors |
| request_command_id | 256-262 | REAL | — | serde roundtrip tag recovery; `None` handled fail-closed by caller (server.rs:789-794, verified) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| sensitive_list_is_exactly_the_documented_categories | YES (for staleness) | len==10 pin + every id round-trips through FromStr + 3 positive/3 negative spot checks; a removed catalog id breaks it. Cannot (mechanically) prove no OTHER command deserves the class — inherent design-judgment limit, spot-checks are explicit |
| deny_by_default_and_granted_when_enabled | YES | asserts error code/stage/message, grant flow, non-granted stays denied, ordinary passes |
| unknown_grant_entries_are_typed_errors | YES | typo entry must error with the entry echoed |
| unauthenticated_sessions_get_the_read_only_subset_only | YES | default-deny + 3 safe ids pass + 3 ordinary/sensitive ids fail |
| policy_file_serde_shape_is_strict | YES | strict/unknown-key/defaults assertions; JSON-projection scope is disclosed in-comment and the real-TOML claim verified against e2e.rs:169-200 |
| request_id_extraction_covers_payload_shapes | YES | unit, unit, and struct-variant payload shapes all extract the right id |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "membership is asserted against the live catalog in tests, so a catalog rename cannot silently rot it" | policy.rs:13-17 | REAL | round-trip test; wire-string rename keeps variant (same command), variant rename is a compile error |
| "deny-by-default is structural, the same posture as the loopback bind" | policy.rs:10-11 | REAL-UNPROVEN (in shipped builds: vacuous) | code real + tested; crate not in release lane (F-001) |
| "Wrong-class refusals … the connection SURVIVES; only the op is refused" | policy.rs:19-20 | REAL | server.rs:796-801 sends Denied frame, Flow::Continue |
| "unknown keys are errors" (PolicyFile serde) | policy.rs:95-97,112 | REAL | deny_unknown_fields; dirty-TOML e2e test ok (baseline :1683) |
| "Bind host; MUST be 127.0.0.1 or ::1 (refused otherwise)" | policy.rs:126 | FALSE (incomplete) | server.rs:319 also accepts `localhost`/`[::1]` (F-003) |
| "the 64 ids in the doc 05 §2 tables" | policy.rs:86 | FALSE (stale) | catalog = 65 (counted; dispatch.rs:130 pin) |
| read-only safe set = media.get-info, system.get-engine-info, tracks.list, snapshot | policy.rs:62-70 vs doc 05 §5.4 | REAL | identical lists, compared verbatim |

## 7. Dead / duplicated / unreachable code
- The whole file is unreachable from the shipped release lane (F-001) — kept as honest Phase-5 [PROPOSAL] infrastructure.
- No dead functions inside the file; every item has an in-crate or app caller (server.rs, config.rs, main.rs).

## 8. Dependencies & flags
- control-protocol depends on `commands` (WireRequest, CommandId, CommandError) + serde; no feature gates in this file.
- Single consumer: apps/control-server (not shipped, feature-flagged off in product.toml).
- No testkit dependency here; tests use only in-crate fixtures.

## 9. Open questions for the next agent
- Who is `control_protocol::client` for, and does any shipped surface (desktop remote-control) plan to embed the server? If Phase 5 ships, re-audit F-002..F-005 against the then-live catalog.
- `SessionSetFrameChannel` (the 65th id, w31.1) was added without touching this file — confirm no future fs/settings-touching command can be minted without a policy.rs review; consider asserting the sensitive list against a catalog annotation (class field) instead of a hand list.
