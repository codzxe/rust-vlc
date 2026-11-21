# AUDIT: crates/streaming/src/schemes.rs#part2
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-21 |
| Lines / functions | 607 (532–1138 of 1138) / 18 tests + `use` block |
| Chunk boundary | part2 = 532–1138 = the entire `#[cfg(test)] mod tests` (equal split is 569; moved lines 532–568 from A04's part1 so the tests module is not cut mid-test — flagged in log). A04's part1 = 1–531, all production code. |
| File verdict | REAL (tests executed; production `classify` UNREACHABLE from shipped, `open_http` REACHABLE) |
| Reachable from shipped entry point? | Tests: executed (baseline). Production code they cover: `SchemeRegistry::classify` — NO shipped caller (`rg "SchemeRegistry::classify"` → only schemes.rs itself: fuzz + tests; the CLI matches schemes itself at `apps/cli/src/demux_host.rs:427`). `SchemeRegistry::new()` + `open_http` — YES: `apps/cli/src/demux_host.rs:491` `streaming::schemes::SchemeRegistry::new().open_http(...)` in the default http(s) open lane. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "classify\|SchemeRegistry\|open_http" apps/cli/src` (CLI matches scheme strings itself at 427; calls new().open_http at 491); `rg -n "SchemeRegistry::classify" crates apps tests` (self-only); `rg -n "ureq\|rustls" crates/streaming/Cargo.toml` (ureq =3.5.0, rustls + webpki-roots, real dep); `sed -n '688,760p' crates/streaming/src/client.rs` (mock testkit is plaintext; "the ureq adapter cannot be fronted with TLS in-tree"); `sed -n '915,1000p' crates/streaming/src/client.rs` (config-pin test + two `#[ignore]` live TLS tests); `rg -c "scheme-registry-begin" docs/formats/network-http.md` (markers exist); baseline: orchrun-test-workspace-nofailfast.log.txt:5139-5157 (18/18 schemes tests ok). |

## 1. One-paragraph verdict
This is the test half of the scheme registry, and the tests themselves are unusually strong: wire-level assertions (Range headers on the mock server log), a doc-record enforcement test that parses `docs/formats/network-http.md`'s scheme table and cross-checks every row, retry-weather end-to-end with a fake clock asserting bounded backoff, and an 18/18 pass in baseline. The hostile question of this wave — "TLS claim vs evidence" — lands here with a hard answer: not one test in this file (or the crate) ever exercises the production transport `SchemeRegistry::new()` builds; every transport test injects `MockClient` over a plaintext loopback, the in-tree testkit admits it cannot front TLS, and the only tests that would touch real rustls are `#[ignore]`d live-network tests that no committed run has ever executed. The registry's own headline claim ("everything … arrives here first") is also false: the shipped CLI dispatches schemes itself and never calls `classify`.

## 2. What actually works (verified)
— verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5139-5157` → all 18 `schemes::tests` pass, incl. `record_table_matches_the_registry`, `retrying_registry_opens_through_scripted_weather`, `registry_opens_http_through_the_mock_testkit`.
— verified by `rg -c "scheme-registry-begin" docs/formats/network-http.md` → 2 (begin+end markers present); rows counted at lines 53-65 (12+ schemes) — the doc-enforcement test has a real ledger to enforce.
— verified by `rg -n "ureq" crates/streaming/Cargo.toml` → `ureq = { version = "=3.5.0", default-features = false, features = ["rustls", "rustls-webpki-roots"] }` — the production transport is a real pinned dependency compiled into the shipped `lumen` binary (reachable via demux_host.rs:491).

## 3. Findings

### F-001 [S2]
Where: crates/streaming/src/schemes.rs:876-877 (and the whole transport-test set 873-1039); evidence gap proven by crates/streaming/src/client.rs:966-998
Code:
```
        let server = MockServer::spawn(MockConfig::body(body.clone())).must();
        let registry = SchemeRegistry::with_fetch(Arc::new(MockClient));
```
What is wrong: the production registry constructor `SchemeRegistry::new()` (schemes.rs:236-240, the ureq+rustls "hardened transport") is never constructed by any test in this module — every transport test substitutes `MockClient`, and client.rs's own test comment admits "the ureq adapter cannot be fronted with TLS in-tree (plaintext mock testkit)". The only real-TLS tests are `live_example_com_returns_200` / `live_httpbin_redirect_chain_lands_on_200` (client.rs:966+), both `#[ignore = "live network required: run with --ignored"]` — never run in the committed baseline (24 ignored). The record's row (`docs/formats/network-http.md:55`) claims "rustls TLS, verification always on … the one `TlsConfig` site is test-pinned" — what is pinned is `!tls.disable_verification()` (a config field check, client.rs:924-927), not a handshake.
Why it matters: a security-relevant claim (TLS verification always on, Mozilla roots, rustls provider) is documented as tested when no executed run has ever completed a TLS handshake through this stack; a regression inside ureq/rustls integration (wrong provider wiring, root bundle not loaded, SNI/hostname-verification breakage) would pass the entire committed suite.
What correct looks like: an in-tree TLS test (self-signed cert + local rustls server, asserting both success against a trusted root and typed failure against an untrusted one), or a recorded orchestrator run of the two live tests with output committed under audit/runs/.
Confidence: HIGH — Verification: read of both files; baseline log confirms 24 ignored and no TLS output anywhere in audit/runs/baseline/.

### F-002 [S3]
Where: crates/streaming/src/schemes.rs:4-5 (claim, part1's header) vs apps/cli/src/demux_host.rs:415-437 (reality)
Code:
```
//! Everything the player can be asked to open arrives here first
//! (doc 13 §9 wave 6.A; doc 06 §1.2 "access factory" row):
```
(and, in the shipped CLI:)
```
            match parsed.scheme().as_str() {
                "file" => {
                    ...
                "http" | "https" => open_http_source(url, fetch)?,
```
What is wrong: the module's architectural claim — the registry is "the single point" every source reference passes through — is false for the only shipped consumer: the CLI parses the URI itself (`SourceUri::parse`), string-matches `file`/`http`/`https`, and never calls `SchemeRegistry::classify` (rg proves zero production callers). Two dispatchers exist; the registry one is dead code in the shipped binary.
Why it matters: the duplicated dispatch can drift (a scheme `classify` admits that the CLI lane refuses, or normalization rules diverging); the file: rules are defended by mirrored tests (`file_uri_drive_and_unc_forms_mirror_the_input_resolver`, 592-618, same rows as `input`'s), but nothing defends the scheme *set*.
What correct looks like: the CLI's open path calls `SchemeRegistry::classify` and matches on `AccessKind`, or the module doc is corrected to say the registry is a library surface the composition site currently does not use.
Confidence: HIGH — Verification: read of both sites + rg over apps/, crates/, tests/ for callers.

### F-003 [S4]
Where: crates/streaming/src/schemes.rs:625-630
Code:
```
        match SchemeRegistry::classify("HTTP://Example.com") {
            Ok(AccessKind::Http { url }) => {
                assert_eq!(url.scheme, "http");
                assert_eq!(url.host, "Example.com");
```
What is wrong: the test pins that the host keeps its original case ("Example.com"); RFC 3986 §6.2.2.1 calls for lowercase host normalization. The port (fetch.rs) not lowercasing means `Example.com` and `example.com` are distinct keys wherever hosts key maps — notably the redirect follower's visited set, weakening (budget still bounds it) loop detection by case-splits.
Why it matters: the test blesses a normalization gap instead of exposing it; scheme lowercasing is done (line 627 asserts it), so the asymmetry is deliberate-but-undocumented.
What correct looks like: normalize the host to lowercase in `Url::parse` (fetch.rs owner) and assert `example.com` here.
Confidence: HIGH — Verification: read; fetch.rs is A01's file — cross-ref in §9.

### F-004 [S4]
Where: crates/streaming/src/schemes.rs:1116-1121
Code:
```
            for i in 0..bytes.len() {
                let mut mutated = bytes.to_vec();
                mutated[i] ^= 0x20;
                fuzz_scheme_classify(&mutated);
```
What is wrong: the doc comment (1090-1093) promises "bit flips, truncations, and splices" — the "bit flip" family is a single fixed mask (`^= 0x20`, case flips), not bit flips; a mutation that flips a high bit or injects `%`/`:`/`\` bytes comes only from the splice family by accident.
Why it matters: the sweep's stated coverage overstates what it does; narrow but it is the exact "name promises more than the body" pattern.
What correct looks like: iterate a few masks (`0x01..=0x80`) or a deterministic PRNG byte overwrite family, and keep the `passes > 300` floor.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `bare_paths_classify_local` | 545-561 | REAL | — | 5 shapes incl. drive-letter and spaces; asserts verbatim passthrough. Would fail if classify mangled bare paths. |
| `file_uris_normalize_to_local_paths` | 565-583 | REAL | — | Authority folding, case-insensitive scheme, percent-decode rows with exact expected paths. |
| `file_uri_drive_and_unc_forms_mirror_the_input_resolver` | 592-618 | REAL | — | Mirrors input's resolver rows incl. exact boundary (`/C:x` untouched); UNC refusals typed. Strongest classification test here. |
| `http_urls_classify_normalized` | 622-643 | REAL | S4 (F-003) | Pins default-port explicitness and path default; pins host case NON-normalized. |
| `https_classifies_exactly_like_http_post_adr011` | 655-685 | REAL | — | ADR-011 pin: https shares the http arm; 443 default; empty authority malformed. A re-introduced scheme-specific https refusal fails loud. |
| `unsupported_schemes_are_refused_typed` | 691-711 | REAL | — | 9 schemes incl. `https+invalid` grammar edge; asserts the named scheme. |
| `malformed_sources_are_typed_with_the_port_chain` | 716-749 | REAL | — | Empty/control/relative/`%zz` rows; asserts the port's error rides `source()` (real chain walk); non-UTF-8 escape refusal. |
| `remote_file_authority_refuses_typed` | 753-761 | REAL | — | Host named in the refusal. |
| `codes_are_stable_and_distinct` | 767-789 | REAL | — | Dedup check over the 3 codes. |
| `display_is_structured_and_credential_free` | 793-822 | REAL | — | Exact Display string; asserts `secret` never appears for userinfo-bearing http and rtsp sources; query redaction via SourceUri. Genuine credential-leak tripwire. |
| `engine_bridge_names_the_scheme` | 826-850 | REAL | — | Stage/kind/format-name/source-chain all asserted. |
| `error_is_send_and_sync` | 854-866 | REAL | — | Bounds + real thread crossing. |
| `registry_opens_http_through_the_mock_testkit` | 873-907 | REAL | S2 (F-001) | End-to-end classify→open_http→reads over mock; asserts probe `Range: bytes=0-1023` on the wire and byte-exact payload — but over `MockClient`, never the production transport. |
| `unsupported_scheme_refuses_before_any_socket` | 912-930 | REAL | — | No server spawned; refusal asserted. |
| `retrying_registry_opens_through_scripted_weather` | 942-999 | REAL | — | Scripted 503/500 recovery; asserts served==2, sleeps ≤2 and ≤ max_ms, ≥3 wire requests. Would catch unbounded backoff or lost retries. |
| `zero_retry_policy_propagates_the_weather_typed` | 1005-1039 | REAL | — | Fail-fast typed refusal (`streaming.ranged.fetch-failed`), no sleeps. |
| `record_table_matches_the_registry` | 1049-1086 | REAL | — | Parses the doc's marker-delimited table; every supported row must classify, every refused row must refuse naming its scheme; ≥12 rows asserted. Verified markers/rows exist (rg). Genuine doc-enforcement. |
| `fuzz_lite_mutations_never_panic` | 1094-1136 | REAL | S4 (F-004) | 12 seeds × (case-flip/truncate/splice) families, >300 passes; absence-of-panic contract; narrow mutation mask (F-004). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 16 classification/refusal/pin tests | YES | Exact expected values/variants/codes; any classify regression flips an arm to `unreachable!`. |
| registry_opens_http_through_the_mock_testkit | YES | Byte-exact round-trip + wire Range assertion; mutating the probe window or chunking breaks it. |
| retrying/zero_retry weather tests | YES | Wire request counts, served counts, sleep bounds all asserted against scripted weather. |
| record_table_matches_the_registry | YES for registry↔doc drift; NO for doc-vs-reality outside the table | It enforces the table's probes, but a scheme row missing from the doc entirely is invisible (rows ≥ 12 floor only). |
| fuzz_lite_mutations_never_panic | Only for panics on these mutation families | Contract is absence-of-panic; narrow mask (F-004). |
| (module-level gap) production transport | NO test breaks if UreqClient/rustls breaks | F-001 — the suite is blind to the production transport beyond client.rs's plaintext socket tests. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Everything the player can be asked to open arrives here first" | schemes.rs:4-5 | FALSE | CLI matches schemes itself (demux_host.rs:427); classify has zero production callers (rg). |
| "the hardened ureq transport … rustls TLS with verification always on" | schemes.rs:233-235, network-http.md:55 | REAL-UNPROVEN | Config pinned (`!disable_verification`, client.rs:924-927); no executed TLS handshake anywhere (live tests #[ignore]d; baseline 24 ignored) — F-001. |
| "an unsupported scheme must be refused before any socket — pinned by test" | schemes.rs:335-337 | REAL | `unsupported_scheme_refuses_before_any_socket` (912-930) + classify purity (no I/O in part1 code). |
| record table = decision ledger, test-enforced | network-http.md markers; test 1049-1086 | REAL | Markers exist (rg -c = 2); test passes in baseline (5151). |
| "RTSP family pending 6.G1-I" refusal policy | network-http.md:58-60 | REAL (refusal only) | `unsupported_schemes_are_refused_typed` covers rtsp/rtp/udp/mms — refusal is the shipped behavior; no RTSP code exists to audit. |

## 7. Dead / duplicated / unreachable code
`SchemeRegistry::classify` and everything only it exercises (`classify_file_uri`, `split_scheme`, `percent_decode`, `strip_drive_authority_slash`, `AccessKind::LocalFile` arm — part1 code) are UNREACHABLE from shipped entry points: the CLI's own dispatch handles file/http/https and refuses everything else with its own error (exit 69 lane). The tests exercise them, so the code is not dead in the library sense — but in the shipped binary the entire local-path classification arm is dormant.

## 8. Dependencies & flags
Tests depend on: mockhttp (MockServer/MockClient/ScriptedStatus), clock (FakeClock), testkit (Must), plugin_api::SourceStream, media_types SourceUri — all dev/in-crate. No feature gates in this range (`#[cfg(test)]` only). The `lumen-allow-literal` comments mark test vectors for the repo's lint-literals gate — verified present on every URL literal (spot-check 551, 595-600, 624, 658-660, 807, 820-821, 1100-1105).

## 9. Open questions for the next agent
- A01 (fetch.rs): does `Url::parse` lowercase hosts? F-003 says no (pinned by test here); decide if normalization belongs in the port.
- A01/A10 (client.rs parts): the two `#[ignore]`d live TLS tests are the only TLS evidence path — request an orchestrator run (`REQUEST-ORCH-RUN` candidate) or accept REAL-UNPROVEN for the TLS claim.
- A04 (schemes.rs part1): the `Unsupported` arm's `into_engine_error` maps Malformed to `ErrorKind::Unsupported` — misleading kind for a malformed (not unsupported) source; check who consumes it.
