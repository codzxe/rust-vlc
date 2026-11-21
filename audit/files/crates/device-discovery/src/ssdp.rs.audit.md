# AUDIT: crates/device-discovery/src/ssdp.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-21 |
| Lines / functions | 584 / 14 (2 builders/ctors + 1 parser + 2 config impls + 4 searcher methods + 1 struct + 6 tests; plus `Default` impl) |
| File verdict | UNREACHABLE (code is REAL and executed — see verified — but no shipped entry point links it) |
| Reachable from shipped entry point? | **NO.** `SsdpSearcher` is used only by `crates/device-discovery/src/lib.rs:123` (`Discovery::probe`). **No app depends on `device-discovery` at all**: rg over `apps/*/Cargo.toml` + `tools/` for `device-discovery` → zero matches; the only external mention of `device_discovery::` is a doc comment in `crates/plugin-api/src/discovery.rs:4` (no Cargo dependency). The `lumen` CLI (apps/cli → streaming) and the tauri shell never link this crate. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "SsdpSearcher|ssdp::"` apps/ crates/ tools/ → only device-discovery internal + its tests. `rg "device-discovery" apps/*.toml tools/` → nothing. `rg -n -i "ssdp|upnp" docs/feature-matrix.toml` → row `sd-upnp` (blocked, empty evidence). Read `tests/ssdp.rs` (the loopback responder integration suite). Baseline cites: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2397-2403` (7 unit tests ok) and `:2426-2437` (`tests/ssdp.rs` — 5 passed in 1.04s, real UDP). |

## 1. One-paragraph verdict
This is the most honest file audited so far in W10-A06: the module doc's five-finding verdict matches the code exactly (std-only UDP lane implemented; description/SOAP lane correctly *not* pretended; wasm carries a typed blocked verdict; multicast-LAN behavior marked `[VERIFY]`, and the live-sandbox claims are scoped to "zero responders here"). The transport logic is proven by real-socket loopback integration tests that pass in the citable baseline run. The defects are small and local: the parser enforces a 256-byte cap on `LOCATION` while the struct doc promises 2048 (valid long-URL devices get skipped as malformed), "distinct devices" vocabulary misdescribes USN-keyed dedup, `with_limits` doesn't enforce its own documented `wait ≥ mx` relationship, and the feature-matrix row `sd-upnp` still says **blocked/empty evidence** while this tested code sits in-tree. Nothing here is reachable from the shipped binary — no app links the crate.

## 2. What actually works (verified)
— `SsdpSearcher::search` sends a spec-shaped M-SEARCH over a real UDP socket and collects/dedups/skips responses: verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2426-2437` — `tests/ssdp.rs`: `loopback_roundtrip_collects_parses_and_dedups ... ok`, `loopback_roundtrip_skips_malformed_and_keeps_going ... ok`, `empty_wire_is_an_honest_empty_result ... ok`, `max_responses_stops_the_window_early ... ok`, `multicast_api_calls_succeed_or_fail_typed ... ok` — "test result: ok. 5 passed; 0 failed" (real sockets, 1.04s).
— In-file unit suite (7 tests) all ok: `orchrun-test-workspace-nofailfast.log.txt:2397-2403`.
— `build_msearch` byte shape: the `\`-continued format literal produces exact CRLF headers with no leading spaces (test `msearch_bytes_match_the_uda_shape` asserts the full literal; integration test asserts the actual wire bytes at the responder: `HOST: 239.255.255.250:1900\r\n`, `MAN: "ssdp:discover"\r\n`, `ST: …\r\n`, ends `\r\n\r\n` — tests/ssdp.rs:137-142).

## 3. Findings

### F-001 [S3] — LOCATION length contract is contradictory: doc promises ≤2048, parser enforces ≤256
Where: ssdp.rs:109-111 (doc) vs 210-216 (enforcement); loop cap at 172-176
Code:
```rust
    /// The device DESCRIPTION URL (UDA 1.1 §1.2.2 `LOCATION`). Non-empty,
    /// control-char-free, ≤ [`LOCATION_CAP`] bytes.
    pub location: String,
```
(and)
```rust
    let required = |field: &str, value: Option<String>| match value {
        Some(v) if v.len() > FIELD_CAP => {
            Err(malformed(format!("{field} exceeds {FIELD_CAP} bytes")))
        }
```
What is wrong: `LOCATION_CAP` is 2048 (line 82) and the generic per-header check (172-176) allows any header value up to that; but `required()` then rejects LOCATION/ST/USN values over `FIELD_CAP` = **256** (line 83). A spec-conformant device advertising a 257–2048-byte description URL (long paths + query tokens occur on real TVs/renderers) passes the loop check and is then discarded as `MalformedResponse` — counted in `skipped_malformed`, device invisible.
Why it matters: the struct's public contract (`≤ LOCATION_CAP bytes`) is false; devices at the long tail of real-world LOCATION lengths are silently dropped in a subsystem whose whole selling point is dirty-world tolerance.
What correct looks like: apply `LOCATION_CAP` to `location` (e.g. a per-field bound map: LOCATION→2048, ST/USN→256) and fix the doc to name the enforced number.
Confidence: HIGH — Verification: read (both constants and both checks traced; no test covers a >256-byte LOCATION, so the suite cannot catch it).

### F-002 [S4] — "distinct devices (unique USNs)" is the wrong unit: one device yields many USNs
Where: ssdp.rs:235-237 (config doc), 284-285 (outcome doc)
Code:
```rust
    /// How many distinct devices (unique USNs) to collect before
    /// stopping. Clamped to 1..=[`MAX_RESPONSES_CEILING`].
    pub max_responses: usize,
```
What is wrong: with `ST: ssdp:all` (the common browse case), one physical device answers once per advertised USN — `uuid:X::upnp:rootdevice`, `uuid:X::uuid:X`, `uuid:X::urn:…` — each a distinct dedup key. The cap therefore counts *responses/USNs*, not devices; a default `max_responses: 32` fills with ~3–8 actual devices on chatty stacks.
Why it matters: API consumers (a future UI discovery tree) will read "32 devices" semantics and get 4; tuning the window from the doc leads to wrong conclusions. Behavior itself is safe and bounded — the vocabulary lies.
What correct looks like: either dedup on the UUID prefix of USN (the part before `::`) when present, or document `max_responses` as "distinct USNs (≈ responses), not devices".
Confidence: HIGH — Verification: read (dedup key is `response.usn.clone()` at 415; UDA 1.1 §1.2.2 USN composition is standard knowledge).

### F-003 [S4] — `with_limits` does not enforce the documented `wait ≥ mx` relationship
Where: ssdp.rs:238-241 (doc) and 259-271 (impl)
Code:
```rust
    /// The receive window. Should cover `mx` seconds for
    /// spec-conformant responders (plus margin); clamped to
    /// 200 ms..=60 s.
    pub wait: Duration,
```
What is wrong: the doc says the window "should cover `mx` seconds … plus margin", but `with_limits` clamps each field independently (`mx.clamp(1,5)`, `wait.clamp(200ms,60s)`) — `with_limits(_, 5, 8, 200ms)` is accepted and guarantees that spec-conformant responders answering in the 0..=5 s window are mostly missed. The production default (mx 1 / 1500 ms) is consistent; the constructor is a footgun the doc disclaims in prose but the code does not enforce.
Why it matters: silent result truncation for any caller that tunes `mx` without reading the field docs; a clamp `wait ≥ mx + margin` (or a typed refusal) is one line.
Confidence: HIGH — Verification: read.
What correct looks like: `wait: wait.max(Duration::from_secs(mx as u64) + READ_SLICE*2).clamp(min, 60s)`.

### F-004 [S4] — feature-matrix row `sd-upnp` says blocked/empty while this tested implementation exists in-tree
Where: docs/feature-matrix.toml:2621-2632 (row) vs this file + tests
Code (matrix):
```toml
id = "sd-upnp"
…
status = { windows = "blocked", linux = "blocked", macos = "blocked", web = "blocked" }
evidence = ""
```
What is wrong: the row claims UPnP discovery is blocked on all platforms with no evidence, but `crates/device-discovery` (card 7.D1) implements the SSDP discovery step with real-socket tests passing in the baseline. The drift direction is under-claiming (the opposite of the repo's usual inflation), but it is still drift: any tooling or reader trusting the matrix (e.g. `cargo xtask parity-report`, feature-audit) sees "blocked".
Why it matters: the matrix is the project's claimed single source of feature truth; a landed-and-tested lane invisible to it makes both directions of audit unreliable. (Caveat that keeps this S4: end-to-end multicast discovery against real hardware is genuinely `[VERIFY]` per this file's own honest verdict — "blocked" is not crazy, but "blocked + empty evidence + no mention of the in-tree partial lane" is stale, not deliberate.)
What correct looks like: status `partial` with evidence pointing at `crates/device-discovery/src/ssdp.rs` + `tests/ssdp.rs` + the baseline run, notes carrying the `[VERIFY]` multicast caveat.
Confidence: HIGH — Verification: read of both; baseline run cited in §2.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `build_msearch` | 93-102 | REAL | — | Byte-exact UDA 1.1 §1.2.3 shape; `\`-continuation verified to strip indentation; pinned by 2 tests incl. wire-byte assertions in tests/ssdp.rs. `mx`/`search_target` interpolated unvalidated — caller-controlled, but config clamps mx and ST is a caller string (host API, fine). |
| `SsdpResponse` (struct) | 107-121 | REAL | — | Data-only; field doc bug F-001. |
| `parse_search_response` | 131-224 | REAL | S3 (F-001) | Dirty-world tolerant (case-insensitive, LF-only, colon-less skip, optional-degrade); typed errors name defects; bounded (HEADER_CAP 64, LOCATION_CAP 2048 pre-check). Duplicate required headers: last wins silently (acceptable). UTF-8 strictness rejects Latin-1 stacks (defensible; spec says ASCII). |
| `SsdpSearchConfig::new` | 248-255 | REAL | — | Production default (multicast group, MX 1, 32, 1500 ms) — internally consistent. |
| `SsdpSearchConfig::with_limits` | 259-271 | REAL | S4 (F-003) | Independent clamps; no cross-field wait≥mx enforcement. |
| `Default for SsdpSearchConfig` | 274-278 | REAL | — | Delegates to `new`. |
| `SsdpSearchOutcome` (struct) | 282-288 | REAL | S4 (F-002) | Doc vocabulary defect. |
| `SsdpSearcher::new` | 300-302 | REAL | — | Trivial ctor. |
| `SsdpSearcher::config` | 306-308 | REAL | — | Pure accessor. |
| `SsdpSearcher::probe` | 317-345 | REAL | — | Bind-only probe (nothing sent — honest); wasm arm returns typed BlockedOnWeb; error path maps to `Capability::from_probe_error`. Note: probe proves UDP bind, not multicast capability — the doc says exactly that ("proves the API, not the network"). |
| `SsdpSearcher::search` | 357-372 | REAL | — | wasm typed refusal; native delegation. |
| `SsdpSearcher::search_native` | 375-442 | REAL | — | Bounded (deadline + max_responses + DATAGRAM_CAP 4096 + 200 ms read slices, no busy-spin); USN dedup; malformed skipped+counted. No multicast group join needed for unicast M-SEARCH responses (correct); no source validation (inherent to SSDP, unauthenticated by design). `skipped_malformed: u32` cannot overflow within a ≤60 s window. Verified executed (baseline §2). |
| tests (6 fns) | 473-583 | REAL | — | All exact-assert; mutation: break `pad`-equivalent (e.g. drop `MAN` header) → `msearch_bytes_match_the_uda_shape` fails; break required-header rejection → `missing_required_headers_are_typed_errors` fails. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `msearch_bytes_match_the_uda_shape` (474) | YES | Full-literal equality on the built message. |
| `spec_shaped_response_parses_every_field` (489) | YES | Asserts every field incl. max_age=1800 and exact SERVER string. |
| `parse_is_dirty_world_tolerant` (505) | YES | Lower-case/LF-only/unknown-header vector + malformed optional degrade. |
| `missing_required_headers_are_typed_errors` (530) | YES | Three absence vectors, asserts the `required` detail text. |
| `bad_status_lines_and_encodings_are_typed_errors` (549) | YES | NOTIFY/500/invalid-UTF-8 → err. |
| `config_clamps_to_spec_plausible_bounds` (556) | YES | Both clamp directions asserted. |
| `constants_are_the_spec_values` (579) | WEAK | Asserts literals against literals (239.255.255.250/1900/4) — tautology-adjacent, but it pins the constants against accidental edits; acceptable as a tripwire. |
| tests/ssdp.rs (5 integration tests, other file) | YES | Real sockets; wire bytes asserted from the responder side (not a mock echo); dedup, skip-count, early-stop, empty-wire all asserted with exact values. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "SSDP is UDP multicast; the in-tree https client can never serve it" (finding 1) | ssdp.rs:6-15 | REAL | ureq is TCP-only (streaming Cargo.toml:52, no UDP feature); transport split is architectural. |
| "What is implemented is exactly what is provable … transport logic proven with real UDP round-trips against an in-process loopback responder (tests/ssdp.rs)" | ssdp.rs:27-34 | REAL | Baseline orchrun log:2426-2437, 5/5 ok. |
| "Web: BLOCKED, typed" | ssdp.rs:36-41 | REAL | `#[cfg(target_family = "wasm")]` arms at 318-327/358-367. |
| "nothing fetches descriptions, and nothing pretends to" | ssdp.rs:54-56 | REAL | No description-fetch code in crate (rg: only `location: String` as data). |
| `sd-upnp` status blocked, evidence "" | feature-matrix.toml:2629-2630 | FALSE (stale — under-claim) | F-004. |
| "SsdpResponse.location … ≤ LOCATION_CAP bytes" | ssdp.rs:109-111 | FALSE | F-001: enforced cap is FIELD_CAP 256. |

## 7. Dead / duplicated / unreachable code
- Whole crate is unreferenced by any app/tool — compiled as a workspace member, tested, shipped nowhere. (Contrast: dash is at least linked-into-but-uncalled via streaming; device-discovery is not even linked.)
- Duplicated vocabulary worth cross-referencing (not this file's defect): `plugin-api/src/discovery.rs` defines its own `Device`/`ServiceDiscovery` trait while `device-discovery/src/device.rs` defines another — two parallel service-discovery vocabularies with no bridge (only a doc-comment mention). For the device.rs/plugin-api owners.

## 8. Dependencies & flags
- std-only (no external deps; matches the crate's zero-dep posture). `#[cfg(target_family = "wasm")]` gating is the only platform switch — note wasm lane is UNVERIFIED by any test (no wasm target in baseline; the blocked-verdict arms are compile-checked only on native).
- Constants: SSDP_TTL=4, DATAGRAM_CAP=4096, HEADER_CAP=64, LOCATION_CAP=2048, FIELD_CAP=256, MAX_RESPONSES_CEILING=256, READ_SLICE=200ms — all read, all plausible; F-001 is the interaction bug between two of them.

## 9. Open questions for the next agent
- For the context.rs (A09) / monitor.rs (A08) owners: does anything consume `Discovery::ssdp()` at all, or is the accessor itself dead? (rg found only lib.rs wiring.)
- For the feature-matrix wave: sd-upnp row update (F-004) — and sd-directory-watch (2596) also says not_started/empty evidence while volume.rs/events.rs/monitor.rs exist in the same crate; same staleness pattern worth a row-level sweep.
- Who is intended to call `Discovery::probe()`? plugin-api's parallel trait suggests the host was supposed to bridge them; nothing does today.
