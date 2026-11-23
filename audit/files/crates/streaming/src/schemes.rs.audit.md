# AUDIT: crates/streaming/src/schemes.rs
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-23 |
| Lines / functions | 1137 / 15 production fns/methods + 18 tests |
| File verdict | REAL for the shipped surface (`open_http`), with `classify` — the file's headline "single dispatch point" — UNREACHABLE from shipped entry points (test/fuzz-only) |
| Reachable from shipped entry point? | PARTIAL YES: `lumen open http://…` → apps/cli/src/parse.rs → demux_host.rs:427 `"http" \| "https" => open_http_source(url, fetch)` → demux_host.rs:491 `SchemeRegistry::new().open_http(...)` → schemes.rs:348. `classify` itself: NO shipped caller (verified by repo-wide grep) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 4 |
| Commands run | `cargo test -p streaming --lib schemes::` -> 18 passed / 0 failed (exit 0); repo-wide `rg` for callers; `sed -n` quote verification |

## 1. One-paragraph verdict
This is the scheme registry, and the scheme answer is blunt: exactly three schemes classify to a real strategy — `file` (→ `AccessKind::LocalFile`), `http`, `https` (→ `AccessKind::Http` through the fetch port's `Url::parse`). Every other scheme string — ftp, sftp, rtsp, rtp, udp, data, gopher, mms, mmsh, smb, and any unlisted one like rtmp — hits the catch-all `SchemeError::Unsupported` typed refusal; there is no fourth lane, no partial ftp, no hidden rtsp. The code is genuinely total, pure, and well-tested (18 tests pass under my run, including a machine-checked doc↔code table). The rot is in the claims around it: the module doc says "Everything the player can be asked to open arrives here first" — false, the shipped CLI never calls `classify`, it ships its own divergent splitter in parse.rs and its own refusal message in demux_host.rs; and the protocol record (docs/formats/network-http.md) still says the CLI refuses every URL, stale since the w31.3 wiring landed.

## 2. What actually works (verified)
— verified by `cargo test -p streaming --lib schemes::` -> `test result: ok. 18 passed; 0 failed` including `registry_opens_http_through_the_mock_testkit` (real mock socket, byte-exact round-trip, probe `Range: bytes=0-1023` asserted on the wire) and `record_table_matches_the_registry` (re-parses docs/formats/network-http.md's 13-row table and cross-checks every row against `classify`).
— verified by repo-wide grep: `SchemeRegistry::new().open_http` is called from apps/cli/src/demux_host.rs:491 (the shipped HTTP lane); `classify` has zero non-test callers.

## 3. Findings
F-001 [S3]
Where: crates/streaming/src/schemes.rs:1-5
Code:
```
//! Scheme dispatch registry (card 6.A3) — the single point mapping a
//! source reference to an access strategy.
//!
//! Everything the player can be asked to open arrives here first
```
What is wrong: the claim is false for the shipped product. `lumen open` never routes through `classify`: apps/cli/src/parse.rs:505-510 builds `SourceSpec::Url`/`File` with its own `is_url` check, and apps/cli/src/demux_host.rs:427-436 matches `"http" | "https"` (else its own `UnsupportedForHost` refusal). The registry's entire classification surface — the drive-letter rule, the UNC refusal, `RemoteFileHost`, percent-decoding — is dead code in production; a CLI user typing `file://nas/x` gets the CLI's generic refusal, never `streaming.scheme.file-host`. Why it matters: the file's reason to exist (single dispatch point, "cannot drift") is not honored by the only shipped consumer. What correct looks like: the CLI open lane calls `SchemeRegistry::classify` and maps `AccessKind`/`SchemeError` (the composition the doc already describes).
Confidence: HIGH — Verification: ran (repo-wide rg: classify callers = schemes.rs tests + https_activation.rs tests + netfuzz).

F-002 [S3]
Where: apps/cli/src/parse.rs:505-510 vs crates/streaming/src/schemes.rs:366-381 (cross-file drift evidence against this file's design claim)
Code (parse.rs):
```
    let is_url = source.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
```
What is wrong: two shipped scheme grammars coexist and disagree: `split_scheme` requires the first char to be ASCII-alphabetic and treats a single-letter prefix (`c:`) as a local path; the CLI's `is_url` accepts any alphanumeric first char and has no single-letter rule. `c://x`: registry says bare local path; CLI says URL → `UnsupportedForHost`. `1://x`: registry says bare path; CLI says URL-refused. The registry's doc pins the disambiguation rules "so Phase-6 dispatch and `input` cannot drift" — the CLI drifted anyway because it never adopted the registry.
Confidence: HIGH — Verification: read (both grammars compared line-by-line).

F-003 [S3]
Where: docs/formats/network-http.md:7-11 (record for this file)
Code:
```
**The CLI open path does not dispatch through the registry yet** — the
composition card (probe-over-network + product-limits join; the
`open-path-for-http` handoff named by 6.A2a/6.A2b) owns that wiring;
today every `SourceSpec::Url` still refuses at
```
What is wrong: stale in the opposite direction — since wave 31.3 the CLI DOES open http(s) through `SchemeRegistry::new().open_http` (demux_host.rs:427, 491). The record under-claims (says refused; code opens). A reader auditing "does HTTP open work in the CLI" from the record gets the wrong answer; the "Honest gaps — CLI dispatch not wired" section (L174-182) is equally stale. The record's own enforcement test only covers the scheme table, not the status prose.
Confidence: HIGH — Verification: read (code cited) + ran (the 18-test suite includes the registry-over-socket open).
Note: record-table itself is accurate and machine-enforced; `rtmp` is refused by the catch-all but has NO row in the table (the "decision ledger" is incomplete for it) — S4 hygiene, folded here.

F-004 [S4]
Where: crates/streaming/src/schemes.rs:343-349
Code:
```
    pub fn open_http(
        &self,
        url: &Url,
        config: RangedConfig,
    ) -> Result<RangedResource, EngineError> {
        RangedResource::open(Arc::clone(&self.fetch), &url.to_string(), config)
    }
```
What is wrong: takes a parsed `Url`, stringifies it, and `RangedResource::open` re-parses the string — the open lane parses the same URL three times (demux_host.rs:480 `Url::parse`, then here, then inside RangedResource). Wasted work on a hot open path; also a place where the two parsers could theoretically disagree.
Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/streaming/src/schemes.rs:264-272
Code:
```
    pub fn with_retrying_fetch(
        fetch: Arc<dyn HttpFetch>,
        policy: RetryPolicy,
        clock: Arc<dyn WallClock>,
    ) -> Result<SchemeRegistry, &'static str> {
```
What is wrong: stringly-typed error (`&'static str` from `RetryPolicy::validate`) on a public constructor, against the file's own house style of typed errors with dotted codes. Also: this constructor is test-only in practice (W10-A04 established the shipped lane builds `SchemeRegistry::new()` — no retry).
Confidence: HIGH — Verification: read + prior W10-A04 log.

F-006 [S4]
Where: crates/streaming/src/schemes.rs:127-130 and 389-403
Code:
```
    RemoteFileHost {
        /// The non-local authority as written (host or host:port).
        host: String,
    },
```
What is wrong: the `file:` arm never splits userinfo off the authority — `file://alice@nas/media` reports `host: "alice@nas"` and the Display prints it; a username rides into logs under a field documented as "hosts are not credentials". The http arm rejects userinfo at the port; the file arm has no equivalent.
Confidence: HIGH — Verification: read (no '@' handling in classify_file_uri).

F-007 [S4]
Where: crates/streaming/src/schemes.rs:288-293 and 464-501
Code:
```
        if source.chars().any(|c| c.is_ascii_control()) {
            return Err(SchemeError::Malformed {
```
What is wrong: the control-byte check runs BEFORE percent-decoding, so `file:///a%00.wav` / `%0A` decode to NUL/control bytes inside the accepted path; the resulting `PathBuf` fails later at OS open with a raw IO error instead of the typed `Malformed` the registry promises. Same for `file:///a.wav?x=1` — the query is not a file-URI component but rides into the path verbatim and fails late. No panic, no security hole; late untyped failure where the module's contract is "every input is a strategy or a typed refusal".
Confidence: HIGH — Verification: read (decode order traced).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SchemeError::code | 135-141 | REAL | — | 3 stable dotted codes, distinctness pinned by test |
| SchemeError::into_engine_error | 149-177 | REAL | — | Stage Open, scheme in FormatInfo, port error chained (test-pinned) |
| Display for SchemeError | 181-195 | REAL | — | Structured, credential-free (test asserts "secret" never appears) |
| Error::source | 198-206 | REAL | — | Chain preserved for Malformed |
| SchemeRegistry::Default::default | 227-230 | REAL | — | Delegates to new() |
| SchemeRegistry::new | 236-240 | REAL | — | UreqClient transport — the shipped production lane |
| SchemeRegistry::with_fetch | 244-246 | REAL | — | Mock/test injection seam |
| SchemeRegistry::with_retrying_fetch | 264-272 | REAL-UNPROVEN (unreachable in shipped lane) | S4 | Test-only; stringly error (F-005) |
| SchemeRegistry::classify | 281-327 | REAL (code) / UNREACHABLE (shipped) | S3 | Pure+total, 18 tests — but no shipped caller (F-001) |
| SchemeRegistry::open_http | 343-349 | REAL | S4 | Reachable via demux_host.rs:491; re-parse waste (F-004) |
| split_scheme | 366-381 | REAL | S3 | Correct RFC 3986 grammar; drifted twin in parse.rs (F-002) |
| classify_file_uri | 389-435 | REAL | S4 | RFC 8089 folding verified; userinfo-in-authority gap (F-006) |
| strip_drive_authority_slash | 442-454 | REAL | — | §E.3.1 mirror, boundary-exact (test rows `/C:x` untouched) |
| percent_decode | 464-501 | REAL | S4 | Rejects bad escapes + non-UTF-8; produces NUL/ctrl bytes un-rejected (F-007) |
| fuzz_scheme_classify | 516-524 | REAL | — | 64 KiB bound, non-UTF-8 early-return; no cargo-fuzz target (Lead-owned, recorded) |
| tests (18) | 532-1137 | REAL | — | See §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| bare_paths_classify_local | YES | 5 shapes incl. drive-letter and spaces, exact PathBuf equality |
| file_uris_normalize_to_local_paths | YES | 7 rows, authority folding + decode + case |
| file_uri_drive_and_unc_forms_mirror_the_input_resolver | YES | 6 positive + 2 UNC-refusal rows, exact paths |
| http_urls_classify_normalized | YES | Asserts scheme/host/port/path normalization fields |
| https_classifies_exactly_like_http_post_adr011 | YES | ADR-011 pin incl. default-port and empty-authority refusal |
| unsupported_schemes_are_refused_typed | YES | 9 schemes incl. `https+invalid`, scheme name asserted |
| malformed_sources_are_typed_with_the_port_chain | YES | 5 shapes + chained port error string + non-UTF-8 escapes |
| remote_file_authority_refuses_typed | YES | Host name asserted |
| codes_are_stable_and_distinct | YES | Dedup check |
| display_is_structured_and_credential_free | YES | Exact Display string; asserts `secret` absent for http AND rtsp userinfo |
| engine_bridge_names_the_scheme | YES | Kind/stage/format-info asserted; chain survives |
| error_is_send_and_sync | YES | Thread round-trip |
| registry_opens_http_through_the_mock_testkit | YES | Real mock socket; byte-exact body; `Range: bytes=0-1023` on the wire |
| unsupported_scheme_refuses_before_any_socket | YES | No server spawned; refusal is pure |
| retrying_registry_opens_through_scripted_weather | YES | 503→500→recovery over socket; sleep count + ceiling asserted on FakeClock |
| zero_retry_policy_propagates_the_weather_typed | YES | Fail-fast boundary + no sleeps |
| record_table_matches_the_registry | YES | Re-parses the doc; every row cross-checked; `rows >= 12` |
| fuzz_lite_mutations_never_panic | WEAK-PASS | Only proves absence-of-panic on 3 deterministic mutation families; no coverage-guided fuzzing exists (doc admits cargo-fuzz target is Lead-owned) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Everything the player can be asked to open arrives here first" | schemes.rs:4-5 | FALSE | CLI never calls classify (F-001); parse.rs:505 + demux_host.rs:427 |
| 3 supported schemes: file/http/https; all else refused typed | schemes.rs:7-24 + record table | REAL | classify 300-326; 18 tests; record cross-check test |
| "the registry is uniform, the record is the decision ledger" | schemes.rs:22-24 | PARTLY FALSE | `rtmp` refused by catch-all but absent from the ledger table |
| "CLI open path does not dispatch through the registry yet… every SourceSpec::Url still refuses" | docs/formats/network-http.md:7-11 | FALSE (stale) | demux_host.rs:427/491 opens http(s) through the registry (F-003) |
| "Two rules mirror input's resolver exactly… cannot drift" | schemes.rs:34-40 | FALSE (drifted anyway) | parse.rs is_url grammar disagrees (F-002) |
| "SchemeError never embeds the source reference" | schemes.rs:47-49 | REAL | Display test asserts credential absence; F-006 is a username-in-host edge |
| fuzz contract absence-of-panic, 64 KiB ceiling | schemes.rs:503-530 | REAL | fuzz_lite test + bound check at 517-519 |

## 7. Dead / duplicated / unreachable code
- `classify`, `split_scheme`, `classify_file_uri`, `strip_drive_authority_slash`, `percent_decode`, `fuzz_scheme_classify`: unreachable from shipped entry points (tests + https_activation.rs + netfuzz only) — the CLI duplicates their behavior in parse.rs/demux_host.rs with drift (F-001/F-002).
- `with_retrying_fetch`: test-only (W10-A04 finding, re-confirmed: callers schemes.rs:958/1015 are tests).

## 8. Dependencies & flags
- No feature gates; compiled unconditionally. Deps: ureq client (crate::client), ranged, reconnect, clock, fetch — all internal to `crates/streaming`.
- The shipped lane instantiates `UreqClient` (real network transport) — see client.rs audit (W10-A04) for that half.

## 9. Open questions for the next agent
- Should apps/cli/parse.rs + demux_host.rs be re-pointed at `SchemeRegistry::classify` (kills the drift in F-002)? That is a product decision, not a streaming-crate fix.
- Is `rtmp` refusal policy supposed to be recorded in the scheme table (my task prompt names it; the ledger has no row)?
