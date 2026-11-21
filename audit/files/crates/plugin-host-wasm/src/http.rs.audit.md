# AUDIT: crates/plugin-host-wasm/src/http.rs
| Field | Value |
|---|---|
| Auditor | W12-A01 |
| Date | 2026-11-21 |
| Lines / functions | 270 / 12 (1 const, 2 structs, 2 enums + 2 trait impls + 2 inherent impls of 5 methods total, 4 tests + 1 helper) |
| File verdict | REAL (policy gate verified and unit-tested; the transport it gates for does not exist anywhere in production — by documented design) |
| Reachable from shipped entry point? | NO, twice over. (1) No shipped app embeds `plugin-host-wasm` at all: Grep `plugin-host-wasm` → zero hits in apps/cli, apps/desktop, apps/desktop-tauri, apps/web Cargo.tomls; the app-level plugin registry is explicitly unwired (baseline `orchrun-test-workspace-nofailfast.log.txt:3880-3882`: `gap_plugins_{crashed,failed,loaded} ... ignored, gap: … not wired — handoff H-8/H-9/H-10`). (2) Even inside the host, the seam's only production state is `transport: None` (`HostServices::default()`, state.rs:24) → `http_get` answers `E_TRANSPORT` before any fetch (imports.rs:768-775); the only `Some(transport)` construction in the entire repo is the test double (`tests/plugin_sandbox_security.rs:627`). Within the host's own test suites the gate IS executed: 4/4 `http::tests` (baseline :4512-4515) and the security suite's http row. The wasmi host itself does execute real plugins end to end in tests (baseline :4569-4596: `plugin_examples_conformance` 5/5, `plugin_hello_world` 2/2 — "hello_world_extension_runs_and_reports_manifest ... ok", `plugin_install_lifecycle` 4/4, `plugin_sandbox_security` 9/9) — but no shipped entry point reaches any of it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `Read` full file; `Grep ParsedUrl\|HttpTransport\|NoTransport\|http_get` (consumer census: imports.rs:690/717 — the wasmi import; state.rs:11/23-24 — the Option<Box<dyn>> seam; lib.rs:95 re-export); `sed -n` imports.rs:685-790 (the full http_get policy chain: read_guest caps → parse → allowlist → `http_admit` rate → out_cap vs `io_transfer_bytes` → `io_bucket.take` → transport dispatch); `Grep transport: Some\|set_transport` (exactly ONE construction, in tests); `Grep plugin-host-wasm` over apps/ (zero embedders); baseline log greps (:3880-3882 gap rows, :4495-4515 unit+http tests, :4569-4608 the four plugin integration suites); `sed -n` verbatim checks at 4-6, 73-77, 119-132, 172-177, state.rs:22-25. |

## 1. One-paragraph verdict
The policy half of the plugin HTTP seam is genuinely well-built: https-only with no downgrade, exact-host allowlist that survives the subdomain/suffix/path-smuggling tricks (test-pinned), userinfo and IP-literal and non-443-port refusals, case normalization, and a transport trait whose only in-tree implementation honestly answers "unavailable" — with 4/4 unit tests plus the security suite proving the gate. But the file's headline claim — "enforces everything policy-side before any transport runs" — has a hole: the path/query bytes are never validated, so a CRLF-laden path passes every check and is reconstructed verbatim into the URL handed to whatever transport someday gets wired; and the `NoTransport` type that the module doc calls "the v1 default" is dead weight — the actual default is `transport: None` in `HostServices`, and `NoTransport` is constructed nowhere outside its own test. The deeper truth this file exposes for the wave: the plugin HTTP capability is a gate in front of a door that has no house behind it — no embedder, no transport, no shipped reachability — honestly documented, tested in isolation, and unreachable from anything a user can run.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt:4512-4515` → 4/4 `http::tests` pass (https-only + normalization, userinfo/IP/port refusals incl. the explicit-443 exception, exact-match allowlist vs three smuggling shapes, honest unavailability).
— verified by read: the allowlist is exact-match (`eq_ignore_ascii_case` on the whole host, :181-185) and the adversarial test (:243-250) covers the three classic bypasses — subdomain (`api.musicbrainz.org`), suffix (`musicbrainz.org.evil.test`), path-embedded host — all correctly refused.
— verified by read: the policy chain is real and ordered — `ParsedUrl::parse` (this file) → `allowed_by` (this file) → `http_admit` rate → `out_cap` vs `io_transfer_bytes` → `io_bucket.take` (imports.rs:717-767) — so parse+allowlist genuinely run in front of every possible transport, as the module doc claims (:15-16).
— verified by read: `NoTransport::fetch` answers `TransportError::Unavailable` with a message that says exactly why (:50-53) — the honest-refusal posture the doc claims; pinned by its test and by the security suite's http row.
— verified by read: IPv6 literals are refused via the bracket check (:156) and the port parse happens before it, so `[::1]:443` cannot slip through the port exception.

## 3. Findings

### F-001 [S3]
Where: crates/plugin-host-wasm/src/http.rs:124-132, :172-176 (claim at :4-6)
Code:
```
        let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        let mut tail = &rest[authority_end..];
        if let Some(hash) = tail.find('#') {
            tail = &tail[..hash];
        }
```
(+ :172-176:)
```
        Ok(ParsedUrl {
            host,
            path: tail.to_string(),
```
What is wrong: the host half of the URL is validated (scheme, userinfo, port, IP-literal, DNS-label grammar) but the PATH AND QUERY (`tail`) receive zero byte validation — no control-character rejection, no space rejection, no length bound. `https://allowlisted.host/a\r\nX-Injection: 1` parses to `Ok`, passes `allowed_by`, and `parsed.url()` (:188-190) reconstructs the CRLF verbatim into the string handed to `transport.fetch`.
Why it matters: the module's contract is "enforces **everything policy-side** before any transport runs" (:4-6) and the trait doc tells transport authors the URL is "already validated" (:63-64) — both are false for the path. The moment the Lead wires the planned ureq client (the documented next step, :9-13), a malicious guest gets header-injection-shaped URLs past the gate unless the transport independently re-validates. The sibling crate already knows better: apps/web's `validate_http_url` (fetch_port.rs:292) rejects control bytes and spaces at its boundary. Today the gap is unreachable (no transport exists anywhere), which is why this is S3 and not S1.
What correct looks like: reject `tail` bytes outside the RFC 3986 pchar set (or minimally: `is_ascii_control() || b == b' '` → `PermissionDenied`), before the `Ok`.
Confidence: HIGH — Verification: read (byte flow traced parse → url() → fetch signature); no test pins path bytes — the suite's URL vectors all carry clean paths.

### F-002 [S4]
Where: crates/plugin-host-wasm/src/http.rs:73-83 vs crates/plugin-host-wasm/src/state.rs:23-24
Code:
```
/// The v1 default transport: none. Every `http_get` observes the
/// typed `E_TRANSPORT` status — the host is honest that fetching is
/// unavailable rather than pretending success or panicking.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoTransport;
```
(+ state.rs:22-25:)
```
    /// The HTTP transport (default [`NoTransport`]).
    pub transport: Option<Box<dyn HttpTransport>>,
```
What is wrong: `NoTransport` is not the default of anything. `HostServices` derives `Default`, so `transport` defaults to `None`, and the `http_get` handler's `None` arm answers `E_TRANSPORT` (imports.rs:768-775) — the `NoTransport` struct is never constructed outside its own unit test (:262). Two vocabularies express one refusal; state.rs's doc names the one that is never used.
Why it matters: dead public API (`lib.rs:95` re-exports it) plus a doc that misstates the default — a future embedder reading state.rs:23 will set `Some(Box::new(NoTransport))` believing they are choosing the default, when `None` was already it.
What correct looks like: either delete `NoTransport` (None is the refusal) or make the field's default literally it — and fix state.rs's doc to say "default: none (`None`)".
Confidence: HIGH — Verification: read + construction census (`Grep NoTransport` → definition, re-export, test use only).

### F-003 [S4]
Where: crates/plugin-host-wasm/src/http.rs:119-123
Code:
```
        if rest.contains('@') {
            return Err(HostError::PermissionDenied {
                detail: "userinfo in URL is refused".to_string(),
            });
        }
```
What is wrong: the check refuses '@' ANYWHERE in `rest` — authority, path, or query — while the doc claims only "no userinfo (`user@host` is refused)" (:98-99). `https://allowlisted.org/users/@alice/profile` (a legal, common REST shape) is denied as "userinfo".
Why it matters: fail-closed, so no security hole — but a documented rule and the enforced rule differ, and legitimate plugin use-cases (the metadata-provider pattern this crate's examples model) hit a refusal whose message names the wrong reason. The correct scope is `authority.contains('@')` (checked after `authority_end` is computed).
What correct looks like: move the '@' check below the `authority_end` slice and test it against `authority` only; add the path-'@' case to the test matrix.
Confidence: HIGH — Verification: read; the test suite has no path-'@' vector, so the drift is unpinned.

### F-004 [S4]
Where: crates/plugin-host-wasm/src/http.rs:90-91 (behavior at :124-132)
Code:
```
    /// Path+query (starts with `/`; `""` becomes `"/"`).
    pub path: String,
```
What is wrong: `https://host?q=1` parses with `path == "?q=1"` — the authority ends at `?` before any `/`, so the tail never starts with `/`. The doc's "starts with `/`" holds only for URLs that carry a slash; the field doc overstates the invariant. (Reconstruction through `url()` is still correct — scheme+host+tail round-trips.)
Why it matters: doc drift on a public field; a consumer branching on `path.starts_with('/')` would mishandle query-only URLs.
What correct looks like: "Path+query, the URL's tail (may start with `?` when no path precedes the query); empty becomes `/`."
Confidence: HIGH — Verification: read (slice flow traced for the query-only case).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HTTPS_SCHEME` (const) | 20-23 | REAL | — | Grammar constant, lint-annotated. |
| `HttpResponse` (struct) | 25-36 | REAL | — | status + capped body; "does not follow redirects" policy recorded in the field doc. The cap is enforced at the import layer (`out_cap` vs `io_transfer_bytes`, imports.rs:749-757), not here — accurate per the doc ("bounded … before it ever reaches guest memory" — bounded by the CALLER, verified). |
| `TransportError` (enum) + `Display` + `Error` | 38-59 | REAL | — | Two honest variants; message pinned by test. |
| `HttpTransport` (trait) | 61-71 | REAL | S3 | The "already validated" contract (:63-64) is false for path bytes (F-001). `Send`, blocking — matches a wasmi sync host loop. |
| `NoTransport` + impl | 73-83 | REAL | S4 | Honest refusal, but dead in production — default is `None` (F-002). |
| `ParsedUrl` (struct) | 85-94 | REAL | S4 | `path` doc overstates the `/`-prefix invariant (F-004). |
| `ParsedUrl::parse` | 96-177 | REAL | S3/S4 | Scheme/userinfo/port/IP/DNS-label checks all correct and test-pinned; '@'-anywhere over-broad (F-003); path/query bytes unvalidated (F-001); `port.map(|_| 443)` normalization sane; IPv6-via-brackets refused after the port check (no bypass). |
| `ParsedUrl::allowed_by` | 179-185 | REAL | — | Exact-match, case-insensitive — correct; smuggling test-pinned. |
| `ParsedUrl::url` | 187-190 | REAL | — | Canonical reconstruction; drops the normalized :443 — correct. |
| `mod tests` (4 tests + `ok` helper) | 193-270 | REAL | — | See §5. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `https_only` (:205) | YES | http/ftp/bare-host refused; case normalization + `url()` reconstruction exact. |
| `userinfo_ip_and_ports_refused` (:225) | YES | 2 userinfo forms, IPv4, IPv6, :8080 refused; :443 accepted — the port exception pinned. Would NOT catch F-003 (no path-'@' vector) or F-001 (no dirty-path vector — every URL in the suite has a clean path). |
| `allowlist_is_exact_match` (:237) | YES | The three smuggling shapes + non-listed host all fail, exact host passes — this is the strongest test in the file. |
| `no_transport_is_honestly_unavailable` (:260) | YES | Variant + message pinned. |
All 4 ran and passed in baseline (:4512-4515, inside the plugin-host-wasm 44-test unit run at :4495).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "v1 defines the seam and enforces **everything policy-side** before any transport runs" | http.rs:4-6 | FALSE (narrowly) | Path/query bytes unvalidated (F-001); rate/caps ARE enforced but at the import layer (imports.rs:739-767, verified). |
| "this crate ships only `NoTransport`, which answers every request with the typed `E_TRANSPORT`" | http.rs:7-9 | SHALLOW | True in effect, but the mechanism is `transport: None` (state.rs:24 default); `NoTransport` itself never runs in production (F-002). Every `http_get` DOES observe `E_TRANSPORT` — verified at imports.rs:768-775. |
| "wiring `streaming`'s ureq client into this host is a Lead decision; … the seam stays unplugged" | http.rs:9-13 | REAL | Construction census: no `Some(transport)` outside tests. Honest record. |
| "no userinfo (`user@host` is refused)" | http.rs:98-99 | SHALLOW | '@' refused anywhere, including path/query (F-003) — stricter than documented. |
| "no IP-literal hosts" | http.rs:100 | REAL | Dotted-quad (3-dot all-digit) + bracket forms refused; test-pinned (:231-232). |
| "default port or explicit `:443` only" | http.rs:101 | REAL | :137-143; empty-port (`host:`) also refused (falls into the `p != "443"` arm). |
| "the host must equal an entry" (allowlist) | http.rs:179-180 | REAL | :181-185 + smuggling test. |
| doc 06 §2.2 `http-fetch` capability / ADR-011 references | http.rs:1-2 | UNVERIFIED | doc 06 and ADR-011 are other waves' files; the CODE side of each named rule is verified above. |

## 7. Dead / duplicated / unreachable code
- `NoTransport`: constructed only in its own test (F-002) — dead public API.
- The entire module is unreachable from shipped entry points (no embedder; see Reachability) — test-only execution, like the rest of the crate's seams.
- No duplication found; `ParsedUrl` is this crate's own gate (apps/web's `validate_http_url` is the sibling lane's — same spirit, different crate, no shared code).

## 8. Dependencies & flags
- Depends only on `crate::error::HostError` — zero external deps in this file. No unsafe, no feature gates. Workspace lints respected (`unreachable!` only in the test helper, the sanctioned test-only form).
- The transport trait is `Send` + blocking — consistent with the wasmi host's synchronous dispatch loop (imports.rs `func_wrap`).

## 9. Open questions for the next agent
- imports.rs auditor (A05, part1): `String::from_utf8_lossy(&url_bytes).to_string()` (imports.rs:709) silently replaces invalid UTF-8 with U+FFFD BEFORE parse — a guest's malformed URL is parsed as a different string than it sent. Whose defect is that (read_guest's contract vs parse's input)? Also confirm the `PermissionDenied → EArg` mapping (:721-727) is the doc 06 §5.2 status the examples expect.
- state.rs auditor (A06): F-002's doc fix lands in your file (`default NoTransport` → `default None`).
- Whoever audits the metadata-provider example/conformance (plugins/ tree): does any example actually CALL `http_get` (and thus observe `E_TRANSPORT` in the conformance run), or do the examples avoid the seam? My read of the conformance test names suggests manifest-reporting only — worth pinning whether the http lane has ANY end-to-end execution beyond the security suite's test double.
