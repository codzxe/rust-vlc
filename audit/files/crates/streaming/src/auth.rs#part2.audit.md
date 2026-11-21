# AUDIT: crates/streaming/src/auth.rs#part2 (lines 519-1042)
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-21 |
| Chunk boundaries | part2 = 519 (`AuthFetch::answer`) through 1042 (end of `parse_rfc2617_challenge`). Part1 ends with `next_cnonce` (≤514); part3 begins at the `digest_authorization_carries_the_vector_response` doc (1044). Equal-split anchor 521-1041, adjusted ±2 lines to function boundaries. |
| Lines / functions | 524 / 12 (7 prod + 5 tests incl. tests-mod header) |
| File verdict | UNREACHABLE (real, baseline-tested auth code; no shipped entry point constructs it) |
| Reachable from shipped entry point? | NO. `rg "AuthFetch"` outside auth.rs: only doc comments (remote_playlist.rs:37,104; fetch.rs:527) and test files (tests/auth_e2e.rs, tests/auth_scheme_gate.rs). The shipped CLI http lane is `demux_host.rs open_http_source` → `SchemeRegistry::new().open_http` → `UreqClient` (schemes.rs:238) — no `AuthFetch` in the chain. A credential-bearing URL (`http://user:pass@host/x`) is refused at demux_host.rs:485-489 by STRICT `Url::parse` ("userinfo in URL is not supported in v0") before any auth code runs. Executed in baseline as library code: orchrun-test-workspace-nofailfast.log.txt:4944-4951 (`auth::tests::* … ok`) + tests/auth_e2e.rs over real loopback sockets. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `rg -n "^(pub |)fn…" auth.rs` (outline/split); `rg "AuthFetch|CredentialStore|with_credentials"` across repo (test-only constructors); `rg -n "open_remote_playlist"` (all callers inside remote_playlist.rs's own tests — composition surface never used in production); `sed -n` on all quoted ranges (verbatim); baseline log grep for auth tests. |

## 1. One-paragraph verdict
This chunk is the beating heart of the auth layer — the challenge/answer state machine, a manually-followed credential-safe redirect loop with downgrade/cross-host/loop gates, and clean-room MD5/base64 pinned to RFC vectors — and it is honestly tested against the RFC 2617 Mufasa vector over real sockets. But the "same-origin trust boundary" it documents is implemented as same-HOST (port ignored), Basic credentials are never sent preemptively so every authed fetch pays a doubled round trip, and none of it is reachable from the shipped binary: no production code constructs `AuthFetch`, and the CLI's open lane refuses userinfo URLs before auth could engage. The feature matrix says `network-auth` is "not_started" on every platform while `protocol-http-https` notes claim "auth incl. the w30.4 scheme-downgrade refusal" landed — the matrix contradicts itself in both directions.

## 2. What actually works (verified)
— verified by baseline run `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4944-4951`: `auth::tests::md5_rfc1321_vectors`, `base64_rfc4648_vectors`, `digest_rfc2617_known_vector`, `parse_rfc2617_challenge`, `cross_host_hop_never_receives_credentials`, `downgrade_hop_never_receives_credentials`, `conservative_transport_gets_basic_fallback` all pass — the MD5 core is pinned to RFC 1321 §A.5 and the digest computation to RFC 2617 §3.5's published vector (`6629fae49393a05397450978507c4ef1`).
— verified by read: `follow` re-attaches `Authorization` only on host-matching hops (591-597), refuses https→http downgrades unconditionally before host policy (624-626, shared `client::is_scheme_downgrade`), bounds the chain (603-608), and detects loops (632-636).

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/auth.rs:591-593 and 680-682 (contract stated at 545-550)
Code:
```
if let Some((value, allowed_host)) = &authorization
    && url.host.eq_ignore_ascii_case(allowed_host)
{
...
// A cross-origin challenge target never receives
// these credentials (same-origin trust boundary).
if !final_url.host.eq_ignore_ascii_case(&original_url.host) {
```
What is wrong: the documented boundary is "same-origin" (545-550: "a cross-origin hop never receives these credentials… same-origin hops are the curl/VLC default trust boundary"), but the check compares HOST only. Origin per RFC 6454 = scheme + host + **port**. Scheme is guarded separately (downgrade refusal), but a redirect from `http://host:80` to `http://host:8080` (or any port change) re-attaches the Basic/Digest credentials — a different service on the same machine receives them.
Why it matters: the credential-leak posture this code advertises (6.H1) is weaker than claimed; a port-bearing redirect chain on a shared host hands credentials to an unrelated tenant/service. Muted today by unreachability, but this is the landing surface for 6.E.
What correct looks like: compare `Url::authority()` (host+port) — or scheme+authority — instead of bare host, matching the documented origin semantics.
Confidence: HIGH — Verification: read; `Url.host` excludes port (fetch.rs:169-171); the cross-host test (part3, 1401) pins host-mismatch only, no port-mismatch case.

### F-002 [S3]
Where: crates/streaming/src/auth.rs:666-669
Code:
```
// Challenge-first probe: credentials withheld until the
// server asks for them, redirects followed manually so the
// challenge's true URL is known.
match self.follow(&request, original_url.clone(), None) {
```
What is wrong: credentials are never sent preemptively — even when explicit credentials exist and the scheme is Basic (where no server state is needed), every authenticated fetch costs a full extra round trip (401 challenge, then retry). Digest requires challenge-first; Basic does not.
Why it matters: for segment-fetched media (HLS/DASH/large files), this doubles request count on every authed source; curl/VLC offer preemptive Basic precisely for this. A deliberate trade documented in a comment, but the cost is unstated and there is no opt-in.
What correct looks like: when `credentials` came from userinfo/explicit config (not a store probe) and the target is https, attach Basic preemptively; keep challenge-first for Digest and for store-sourced credentials.
Confidence: MEDIUM — Verification: read; no execution (auth lane unreachable, so the latency cost is presently theoretical).

### F-003 [S2] (claim defect — matrix self-contradiction on auth status)
Where: docs/feature-matrix.toml:1472-1479 (`network-auth` row) vs docs/feature-matrix.toml:1289 (`protocol-http-https` notes) vs this file
Code:
```
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
evidence = ""
```
What is wrong: the `network-auth` row claims auth is not_started everywhere with empty evidence, while `protocol-http-https`'s notes (line 1289) assert "auth incl. the w30.4 scheme-downgrade refusal" landed — and auth.rs is 1562 lines of real, RFC-vector-pinned, baseline-tested code (Basic + Digest + redaction + scheme-downgrade gate at 624-626).
Why it matters: the matrix contradicts itself about whether HTTP auth exists; neither row states the actionable truth — code exists and is tested, but NO shipped entry point constructs it (see reachability). A capability map that is wrong in both directions (over-claiming in one row, under-claiming in another) cannot be used for planning or verification.
What correct looks like: `network-auth` status = partial/not-wired with evidence citing auth.rs + auth_e2e.rs and an explicit "no shipped entry point constructs AuthFetch; CLI refuses userinfo URLs" note; `protocol-http-https` notes corrected to not claim auth in the shipped lane.
Confidence: HIGH — Verification: read + rg (constructor grep above) + baseline test log.
(For the wave-10 matrix-rows owner: this contradiction belongs in your ledger.)

### F-004 [S4]
Where: crates/streaming/src/auth.rs:916-921
Code:
```
pub(crate) fn md5_hex(input: &[u8]) -> String {
    md5(input)
        .iter()
        .map(|byte| format!("{byte:02x}")
```
What is wrong: 16 heap-allocated intermediate Strings per hash (one `format!` per byte) where a single `String` with push_str of a hex table, or `write!`, allocates once.
Why it matters: hygiene-level (auth headers are tiny and the lane is unreachable), but it is the crate's only hex helper and gets copied.
What correct looks like: build one String with capacity 32 and write nibbles.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: crates/streaming/src/auth.rs:822-825
Code:
```
for &byte in text.as_bytes() {
    if matches!(byte, b'=' | b'\r' | b'\n' | b' ' | b'\t') {
        continue;
    }
```
What is wrong: `base64_decode` skips `=` anywhere in the stream, not just terminal padding, so degenerate inputs like `"Z=g=="` decode as if the stray `=` were absent — beyond RFC 4648's tolerated whitespace.
Why it matters: the decoder is only used by the mock testkit (mockhttp.rs:275, Basic-credential check) and tests — no production impact — but the doc comment (810-811) says "malformed input is never guessed at", and mid-stream `=` skipping is exactly a guess.
What correct looks like: accept `=` only in terminal position (1-2 chars), reject elsewhere.
Confidence: HIGH — Verification: read + rg callers (mockhttp.rs:38,275; tests).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AuthFetch::answer | 519-539 | REAL | — | strips stale Authorization headers, one retry, second 401 → typed AuthFailed; Ok passthrough consistent with port contract (transport classifies non-401 statuses) |
| AuthFetch::follow | 563-644 | REAL (defect) | S3 | budget/loop/cross-host/downgrade gates all real; F-001 host-only match; inherits fetch.rs `join`'s refusal of bare-relative Location (documented v0) |
| HttpFetch for AuthFetch::fetch | 647-743 | REAL | S3 | full state machine: userinfo>explicit>store priority, challenge-first (F-002), cross-host challenge refusal, conservative-transport Basic fallback; method "GET" hard-coded — sound while HttpMethod has one arm |
| Debug for AuthFetch | 749-756 | REAL | — | configuration shape only; no credential material formatted |
| classify_response | 762-776 | REAL | — | mirrors transport classification for challenge-mode responses; 416+range → RangeNotSatisfiable; 1xx/3xx-no-location surfaced |
| BASE64_ALPHABET const | 780-781 | REAL | — | RFC 4648 standard alphabet |
| base64_encode | 785-808 | REAL | — | correct padding, exact capacity (div_ceil(3)*4); pinned by RFC vectors |
| base64_decode | 812-834 | REAL (defect) | S4 | F-005 lenient mid-stream `=`; O(64·n) alphabet scan — fine at auth scale; used by mockhttp + tests only |
| MD5_SHIFTS / MD5_K consts | 839-855 | REAL | — | RFC 1321 §3.4 values, pinned by the vector test |
| md5 | 860-913 | REAL | — | clean-room RFC 1321; padding + LE length + 64 rounds correct; wrapping arithmetic; input.to_vec() copy is inherent to padding |
| md5_hex | 916-921 | REAL (defect) | S4 | F-004 per-byte allocations |
| tests::md5_rfc1321_vectors | 932-948 | REAL | — | 6 standard vectors; load-bearing for the whole digest path |
| tests::base64_rfc4648_vectors | 952-971 | REAL | — | RFC vectors + round-trip + invalid input refusal |
| tests::digest_rfc2617_known_vector | 977-1008 | REAL | — | Mufasa vector incl. legacy no-qop shape; pins digest_response end-to-end |
| tests::parse_rfc2617_challenge | 1012-1042 | REAL | — | pins realm/nonce/opaque/qop parsing, Digest-without-nonce refusal, Negotiate refusal |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| md5_rfc1321_vectors (932) | YES | published RFC digests; any round/shift/constant error fails |
| base64_rfc4648_vectors (952) | YES | all four padding shapes + round-trip + non-alphabet refusal |
| digest_rfc2617_known_vector (977) | YES | the worked RFC example, both qop shapes; pins HA1/HA2/response assembly |
| parse_rfc2617_challenge (1012) | YES | field-level asserts + two refusal shapes |
Mutation check (reasoned): flip one MD5_K entry → md5_rfc1321_vectors fails on every vector; swap `chunks(3)` for `chunks(4)` in base64_encode → RFC vector fails. All four are load-bearing; none are tautologies. The e2e tests over real sockets (auth_e2e.rs, baseline-passed) exercise fetch/follow/answer against a real MockServer including credential-leak scenarios (cross_host/downgrade tests, part3).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a cross-origin hop never receives these credentials … same-origin trust boundary" | auth.rs:545-550, 678-679 | FALSE as stated (same-host, not same-origin) | F-001: port excluded from both checks |
| `network-auth` "not_started" all platforms, empty evidence | feature-matrix.toml:1472-1479 | FALSE vs code reality (code exists, tested) and vs sibling row | F-003; baseline auth tests pass; constructor grep shows no shipped wiring |
| "auth incl. the w30.4 scheme-downgrade refusal" (in the shipped http lane) | feature-matrix.toml:1289 notes on protocol-http-https | REAL code / UNREACHABLE lane | downgrade gate is real (624-626, pinned by downgrade_hop test) but AuthFetch is never constructed in production |
| "Manual following must not become a bypass around the main client's unconditional refusal" | auth.rs:552-558 | REAL | 624-626 shares `client::is_scheme_downgrade` — verified single source, not a mirror |
| "entropy adequacy `[VERIFY]`" for cnonce | auth.rs:505-514 (part1 boundary, quoted at 505-514) | REAL-UNPROVEN, self-flagged | nanos+counter MD5 — flagged by the authors themselves; cnonce need not be secret per RFC 2617 |

## 7. Dead / duplicated / unreachable code
The whole AuthFetch lane is unreachable from shipped entry points (see header) — tested library island. `classify_response` duplicates the transports' status classification by design ("mirrored here for the challenge-mode responses that bypassed it") — tracked, documented duplication.

## 8. Dependencies & flags
std-only (SystemTime, AtomicUsize, sync::Mutex via part1's store). No external crypto — clean-room MD5, self-pinned to RFC vectors. No feature gates. `base64_decode`/`md5` are pub(crate) — reachable only via mockhttp (testkit) inside the crate.

## 9. Open questions for the next agent
- Part3 owner: do the e2e tests (conservative_transport_gets_basic_fallback, 1170+) cover a port-changing same-host hop? My read says no — F-001's gap is untested.
- Wave owner for demux_host/schemes: should `lumen open http://user:pass@host/x` refuse at the strict parse (current behavior, demux_host.rs:485-489) or route through AuthFetch? Today the user-visible answer is a typed refusal — confirm and record in the matrix row.
- Matrix-rows owner: resolve the network-auth vs protocol-http-https contradiction (F-003).
