# AUDIT: crates/streaming/src/auth.rs#part1 (lines 1-539)
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-21 |
| Chunk boundary | part1 = **lines 1-539** (module docs, `Credentials`, credential-store seam, scheme parsers, digest math, `AuthFetch` construction + `next_nc`/`next_cnonce` + `answer`). Boundary taken at the end of `answer` (539) to keep whole functions — 19 lines past the strict equal split (1-520, which would cut `answer` mid-signature); part2 (W10-A01) starts at `follow` (541). I read 541-770 for context (follow/fetch/classify) but did not audit it. Tests live at 923+ (part2/3 ranges); assessed for part1 coverage only. |
| Lines / functions | 539 / 20 (see §4) |
| File verdict | UNREACHABLE (real, security-conscientious, RFC-vector-pinned — but zero production callers) |
| Reachable from shipped entry point? | **NO.** `rg -n "AuthFetch\|CredentialStore\|InMemoryCredentialStore\|basic_authorization" crates/ apps/ -g '!crates/streaming/src/auth.rs'` → hits only: doc comments (client.rs:221, remote_playlist.rs:37/104, fetch.rs:527) and test files (`crates/streaming/tests/auth_e2e.rs`, `auth_scheme_gate.rs`, in-file `mod tests`). No app or production module constructs an `AuthFetch`; the CLI's URL lane (`apps/cli/src/demux_host.rs:491`) goes straight to `SchemeRegistry::open_http` with no auth wrapper. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "^(pub )?(fn\|struct\|enum\|impl\|mod)" crates/streaming/src/auth.rs` (outline); Read 1-770 (part1 + context); `sed -n '412,421p;127,129p;381,400p;218,225p'` (quote verification); `rg -n "AuthFetch\|CredentialStore" crates/ apps/` (reachability); `rg -n "AuthFetch" crates/streaming/src/schemes.rs crates/streaming/src/client.rs` (production wiring: none); `sed -n '184,303p' crates/streaming/src/fetch.rs` (Url validation: control bytes+spaces rejected at :209/:307 — but `"`/`\`/`,` NOT rejected → F-001); `rg -n "auth" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` + `sed -n '5161,5180p'` (11/11 unit + 8/8 e2e + 2/2 scheme-gate tests green); `sed -n '923,1041p;1456,1486p' crates/streaming/src/auth.rs` (test honesty). |

## 1. One-paragraph verdict
This is the best-engineered file so far in my list: redaction-by-construction with `Debug`/`Display` impls that cannot format the password, an RFC 2617 §3.5 known-answer digest vector, RFC 1321/4648 vector tests for the hand-rolled MD5/base64, a challenge-first flow that withholds credentials until a 401 (pinned by e2e over a real loopback socket, `unchallenged_server_never_receives_credentials` green), and a same-origin-only Authorization attachment policy. But the module's own AUD-15 security bar — "a server-controlled value can neither terminate its own quoted field nor smuggle a separator" — is enforced for username/realm/nonce/opaque and **not for the request URI**, which is interpolated raw into the `Authorization: Digest` header (F-001, S2: `"` and `,` are legal in URL paths and pass every validation layer I traced). Add a case-sensitive credential-store key (F-002) and the wave's standing verdict: none of this runs in the shipped binary — no production code wraps a fetch in `AuthFetch`.

## 2. What actually works (verified)
— `auth::tests` 11/11 `ok` incl. `digest_rfc2617_known_vector` (Mufasa → `6629fae49393a05397450978507c4ef1`), `md5_rfc1321_vectors`, `base64_rfc4648_vectors`, `redaction_never_exposes_passwords`, `store_lookup_is_origin_keyed` — `orchrun-test-workspace-nofailfast.log.txt:4944-4954`.
— `tests/auth_e2e.rs` 8/8 `ok` over `MockClient` (real socket): `basic_challenge_round_trip`, `digest_challenge_round_trip`, `basic/digest_wrong_password_is_typed_auth_failure`, `credential_store_feeds_the_challenge_answer`, `url_userinfo_is_accepted_stripped_and_used`, `unchallenged_server_never_receives_credentials`, `auth_across_redirect_chain_answers_at_final_hop` — same log :5161-5172.
— `tests/auth_scheme_gate.rs` 2/2 `ok` (downgrade refusal / same-scheme follow) — same log :5175-5180.
— Digest math hand-checked against RFC 7616 §3.4: HA1 = MD5(user:realm:pass), HA2 = MD5(method:uri), response = MD5(HA1:nonce:nc:cnonce:qop:HA2) with the RFC 2617 legacy no-qop shape — matches the known vector.

## 3. Findings

### F-001 [S2]
Where: crates/streaming/src/auth.rs:381-391 and 412-414
Code:
```
    // The wire form of every quoted parameter: escaped for the
    // quoted-string grammar (the digest itself hashes the *raw*
    // values — escaping is a wire-format concern only, and the
    // server's quoted-string parser recovers the raw value back).
    let username = escape_quoted_string(credentials.username())?;
    let realm = escape_quoted_string(&challenge.realm)?;
    let nonce = escape_quoted_string(&challenge.nonce)?;
```
```
    let mut header = format!(
        "Digest username=\"{username}\", realm=\"{realm}\", nonce=\"{nonce}\", uri=\"{uri}\""
    );
```
What is wrong: `uri` is interpolated into the `Authorization` header **without** `escape_quoted_string`, unlike every other quoted parameter. The URI is not guaranteed clean: `Url::parse`/`join` (fetch.rs:209, :307) reject only control bytes and spaces — `"` (field terminator), `\` (escape smuggle) and `,` (parameter separator) are all legal and survive into `path_and_query`, and the header-value validation (fetch.rs:424) also only blocks control bytes. A request URL or a server `Location:` hop like `/a",qop=auth-int` produces `uri="/a",qop=auth-int` — a genuine auth-param injection into the `Authorization` header value.
Why it matters: This is precisely the attack class the module documents itself as defending against (AUD-15a/15b, doc at 356-360: values can "neither break out of its quoted field nor smuggle" separators — the doc's list quietly omits uri). The digest `response` is computed over the raw uri and the server unescapes quoted strings, so escaping uri is lossless and safe — the omission has no excuse. A hostile server on the redirect chain controls this value; a benign server with a quote-bearing path gets a grammar-broken header and a failed auth. Security module, security invariant, one parameter missed.
What correct looks like: `let uri = escape_quoted_string(uri)?;` alongside the other parameters (control-byte refusal included), and a test with `"`/`,`/`\` in the uri mirroring `digest_parameters_are_escaped_and_control_bytes_refused` (which today passes a benign `/dir/index.html` — the gap is untested, verified at auth.rs:1456-1474).
Confidence: HIGH — Verification: read + traced every validation layer (fetch.rs:209/:307/:424 — none rejects `"`/`,`/`\`); the escaping test read (uri gap confirmed).

### F-002 [S3]
Where: crates/streaming/src/auth.rs:127-129 (with fetch.rs:219 vs 277)
Code:
```
fn origin_key(url: &Url) -> String {
    format!("{}://{}", url.scheme, url.authority())
}
```
What is wrong: The credential-store key is case-sensitive in the host: `Url::parse` lowercases only the scheme (fetch.rs:219), the host is stored verbatim (fetch.rs:277). `InMemoryCredentialStore::put` under `http://example.com/…` does not match a later lookup of `http://Example.com/…` — while the same module's origin comparisons (`follow`'s `allowed_host` check at 592, the challenge-target check at 680) use `eq_ignore_ascii_case`. Hosts are case-insensitive per RFC 3986 §3.2.2.
Why it matters: A playlist or redirect that varies host case silently loses stored credentials — the 401 comes back with no answer and no diagnostic, while the same two URLs are treated as same-origin everywhere else in this file. The store's own test (`store_lookup_is_origin_keyed`) only pins the path/query exclusion, not case.
What correct looks like: `origin_key` formats `url.host.to_ascii_lowercase()` (authority() already carries the lowercase scheme's port semantics).
Confidence: HIGH — Verification: read (fetch.rs host handling at 219/277; both compare sites in auth.rs use case-insensitive matching).

### F-003 [S4]
Where: crates/streaming/src/auth.rs:218-224
Code:
```
    let (scheme, rest) = value.trim().split_once(' ')?;
    if scheme.eq_ignore_ascii_case("basic") {
        let realm = parse_auth_params(rest)
            .into_iter()
            .find(|(name, _)| name == "realm")
            .map(|(_, value)| value);
        return Some(Challenge::Basic { realm });
    }
```
What is wrong: Only the first scheme token of a `WWW-Authenticate` value is considered. A server offering `Digest …, Basic realm="x"` in one value (or multiple header lines, of which `HttpResponse::header` returns the first) answers with whichever scheme happens to be named first; there is no preference for the strongest supported scheme. A tab separator (`Digest\trealm=…`, legal OWS per RFC 9110) fails `split_once(' ')` → `None` → typed `ProtocolViolation` for a well-formed challenge.
Why it matters: Interop edge: servers listing multiple challenges get an arbitrary pick; tab-separated challenges are refused despite being grammar-legal. No security impact (worst case: Basic chosen over Digest, or a typed refusal).
What correct looks like: Split the value into challenge candidates (quote-aware comma split), pick the strongest supported (Digest > Basic), and accept SP or HTAB after the scheme token.
Confidence: MEDIUM — Verification: read (single-value handling traced; `HttpResponse::header` behavior taken from fetch.rs docs — first match — not re-verified line-by-line, part2's range).
### F-004 [S4]
Where: crates/streaming/src/auth.rs:392-400
Code:
```
    let qop = if challenge
        .qop
        .iter()
        .any(|token| token.eq_ignore_ascii_case("auth"))
    {
        Some("auth")
    } else {
        None
    };
```
What is wrong: A challenge offering `qop="auth-int"` only (no `auth` token) silently falls back to the legacy no-qop response shape (no `qop`/`nc`/`cnonce` fields in the header) instead of a typed refusal — the module's standard posture for unsupported-but-named capabilities (cf. the SHA-256 algorithm refusal at 374-380).
Why it matters: A server that requires qop (RFC 7616 makes qop mandatory-to-implement) will reject the legacy response — the user sees `AuthFailed` with no signal that the cause is an unsupported qop. The module doc says "qop=auth; auth-int needs entity hashing that v0 has no method for" but does not say the fallback is silent.
What correct looks like: If `challenge.qop` is non-empty and contains no `auth` token, refuse typed (`ProtocolViolation`: "digest challenge offers no supported qop").
Confidence: HIGH — Verification: read (qop empty → `unwrap_or_default()` at 242 → `any` false → None).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-38 | REAL | — | Challenge-first, precedence, [PROPOSAL]/[VERIFY] markers honest; redaction claim pinned by test (`redaction_never_exposes_passwords` green). |
| `Credentials` + accessors | 54-78 | REAL | — | Password accessor is pub (needed by digest math); redaction lives in the fmt impls. |
| `From<Userinfo>` | 80-84 | REAL | — | URL userinfo → credentials. |
| `Debug`/`Display for Credentials` | 88-102 | REAL | — | `"[redacted]"` placeholder; Display names user only. |
| `CredentialStore` (trait) | 116-123 | REAL | — | Policy seam; persistence [PROPOSAL] documented. |
| `origin_key` | 127-129 | REAL | S3 (F-002) | Case-sensitive host key; path/query excluded ✓. |
| `InMemoryCredentialStore::{new,lock,lookup,put}` + `Debug` | 135-171 | REAL | — | Poisoned-mutex `into_inner` (documented house pattern); Debug shows count only. |
| `basic_authorization` | 178-181 | REAL | — | RFC 7617; base64 of `user:pass` — no escaping needed (base64 alphabet safe). |
| `Challenge` / `DigestChallenge` | 184-210 | REAL | — | pub(crate) model, all fields consumed. |
| `parse_www_authenticate` | 217-252 | REAL | S4 (F-003) | Basic/Digest case-insensitive; Digest requires realm+nonce (`get(...)?` → None → refusal); unknown scheme → None. First-scheme-wins; SP-only separator. |
| `parse_auth_params` | 257-303 | REAL | — | Quote-aware, backslash escapes, names lowercased, malformed tails skipped-never-fatal (server text) — hand-traced on the RFC 2617 example; unterminated quoted string tolerated. |
| `DigestInputs` | 307-327 | REAL | — | The many-arg collapse; all fields used. |
| `digest_response` | 305-349 | REAL | — | RFC 7616 MD5 qop=auth + RFC 2617 legacy shapes; pinned by known vector (§2). |
| `digest_authorization` | 366-423 | REAL | S2 (F-001), S4 (F-004) | Algorithm≠MD5 (incl. MD5-sess) typed refusal ✓; escapes 4 of 5 quoted params — uri missed (F-001); qop silent legacy fallback (F-004). |
| `escape_quoted_string` | 439-453 | REAL | — | Control bytes (<0x20, 0x7F) typed refusal; `\`+`"` escaped; static error detail (never the bytes). Correct per its contract — the defect is its non-use for uri. |
| `AuthFetch` (struct) | 463-470 | REAL | — | Arc<dyn HttpFetch> + Option<Credentials>/store + AtomicU32 nc counter. |
| `AuthFetch::new` / `with_credentials` / `with_store` | 475-496 | REAL | — | Builder; no production caller anywhere (header). |
| `next_nc` | 499-502 | REAL | — | 8 hex digits, fetch_add, starts at 1. |
| `next_cnonce` | 507-514 | REAL | — | MD5(nonce:uri:nanos:counter); entropy honestly marked [VERIFY]; pre-epoch clock → 0 (harmless). |
| `answer` | 519-539 | REAL | — | Conservative-transport fallback: strips existing Authorization headers (retain), one Basic retry, second 401 → typed AuthFailed. No redirect re-follow on this path (documented VLC-style fallback). |

## 5. Test honesty
Tests for part1 functions live at 923+ (part2/3 chunks) and in `tests/auth_e2e.rs`; all green in baseline (orchrun log :4944-4954, :5161-5180).
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `md5_rfc1321_vectors` / `base64_rfc4648_vectors` | YES | Six/seven standard vectors + round-trip + invalid-input refusal — any MD5/base64 regression flips them. |
| `digest_rfc2617_known_vector` | YES | Known-answer test against RFC 2617 §3.5's worked example (both qop shapes). |
| `digest_authorization_carries_the_vector_response` | YES (partial) | Pins the header form + response — but passes a clean uri, so F-001's injection is NOT covered (verified :1456-1474). |
| `digest_parameters_are_escaped_and_control_bytes_refused` | YES for realm/opaque/nonce | Concrete `"`/`\` payloads asserted escaped in place + control-byte refusals; uri absent from the payload set. |
| `redaction_never_exposes_passwords` | YES | Asserts Debug/Display outputs never contain the password. |
| `store_lookup_is_origin_keyed` | YES (partial) | Pins path/query exclusion; does NOT pin host-case (F-002 untested). |
| e2e suite (8 + 2) | YES | Real-socket round trips: challenge-first withholding, wrong-password typed failure, redirect-chain answer at final hop, downgrade refusal. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Basic (RFC 7617) or Digest MD5 (RFC 7616)" challenge answering | auth.rs:10-11 | REAL | e2e `basic/digest_challenge_round_trip` green (:5164, :5169). |
| "a crafted realm/nonce/opaque/username can neither break out of its quoted field nor smuggle CR/LF" | auth.rs:358-360 | REAL for those four, **SHALLOW for the header as a whole** | F-001: uri (fifth quoted param) is unescaped and injectable. |
| "the password never reaches a logged URL string" (userinfo) | auth.rs:17-19 | REAL | `url_userinfo_is_accepted_stripped_and_used` green (:5172); fetch.rs:188-194 Display contract. |
| "SHA-256 challenges are a typed refusal" | auth.rs:26-27 | REAL | auth.rs:374-380 read; `algorithm` mismatch → ProtocolViolation. |
| "entropy adequacy is [VERIFY]" (cnonce) | auth.rs:28-30 | REAL (honest marker) | next_cnonce is clock+counter — predictability acknowledged, not hidden. |

## 7. Dead / duplicated / unreachable code
- Entire module production-unreachable (header rg). `InMemoryCredentialStore`, `with_store`, `with_credentials`, `answer`'s fallback lane exist only for tests + future composition.
- No duplication found; no dead items in range.

## 8. Dependencies & flags
- std-only (HashMap/Mutex/atomics/SystemTime); `crate::error::{AuthSchemeKind, NetworkError}`, `crate::fetch` types. No feature gates, no unsafe, no unwrap/expect (poisoned-lock handled by `into_inner`). MD5/base64 hand-rolled (part2 range) — consistent with the no-new-deps rule; vector-pinned.

## 9. Open questions for the next agent
- part2 (W10-A01, lines 541-1041): `follow`'s visited-list is `Vec<String>` with linear `contains` — O(hops²) at max_redirects scale, and the canonical form `next.to_string()` — does Url's Display include userinfo (leak path into the loop-detection error?)? Check `RedirectLoop`'s payload.
- Whoever audits `schemes.rs`: confirm `SchemeRegistry::open_http` really is the only shipped fetch path and that composing `AuthFetch` there is a documented future card (the CLI's http lane currently sends NO credentials at all — a URL with userinfo on the shipped lane: does it 401-refuse or silently strip?).
- F-001 fix must add uri to the escape set AND a test with `"`/`,` payloads; the existing e2e MockServer could pin it end-to-end.
