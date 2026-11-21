# AUDIT: crates/streaming/src/schemes.rs#part1 (lines 1-568)
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-11-21 |
| Lines / functions | 568 (of 1137; boundary: tests mod starts 532; `file_uris_normalize_to_local_paths` starts 565, its body continues into part2/A05) / 21 items |
| File verdict | REAL (open_http lane, shipped) / UNREACHABLE (the classification half — classify, split_scheme, classify_file_uri, strip_drive_authority_slash, percent_decode, fuzz entry, AccessKind, SchemeError, into_engine_error) |
| Reachable from shipped entry point? | HALF. YES for `SchemeRegistry::new` + `open_http`: apps/cli/src/demux_host.rs:491 `streaming::schemes::SchemeRegistry::new().open_http(...)` ← demux_host.rs:427 `"http" \| "https" => open_http_source(url, fetch)?` ← `SourceSpec::Url` from parse.rs `open_command`. NO for everything else: `rg "schemes::\|SchemeRegistry::classify\|AccessKind\|SchemeError" crates/ apps/` outside schemes.rs → only crates/streaming/tests/https_activation.rs (in-crate test) and the single demux_host open_http line. The shipped CLI dispatches schemes with its own grammar (parse.rs:502-508 `split_once("://")` + demux_host.rs SourceUri match) and never calls `classify`. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 0 S4: 1 |
| Commands run | `rg -n "classify\|SchemeRegistry\|open_http" apps/cli/src/ crates/input/src/` (CLI has own dispatch; only demux_host:491 uses open_http) · `rg -n "schemes::" crates/ apps/` (https_activation test only) · sed -n 395,435p demux_host.rs + 480,520p parse.rs (the parallel dispatch, quotes) · `rg -n "tls\|verify\|rustls\|redirect" crates/streaming/src/client.rs` (TLS claim has real code: TlsProvider::Rustls, TlsConfig, max_redirects(0)) · sed -n 745,790p schemes.rs (RemoteFileHost test — no userinfo case) · sed -n 4,5p / 32,34p schemes.rs + 502,508p parse.rs (quote verification) · baseline #3 (tests pass) |

## 1. One-paragraph verdict
The file that claims to be "the single point" where every source
reference arrives is, in the shipped binary, only a convenience wrapper:
the CLI calls `SchemeRegistry::new().open_http()` for http(s) sources and
does its OWN scheme dispatch with a different, weaker grammar
(`split_once("://")`), so the entire classification half of this file —
classify, the RFC 8089 file-URI normalizer, the percent-decoder, the
error taxonomy, the fuzz entry — is dead code with an in-crate test
suite and zero production callers. The drift is not hypothetical: the
legal RFC 8089 form `file:/media/a.wav` (which classify_file_uri
explicitly supports and whose "same resource" equivalence the module
docs advertise) is mis-routed by the CLI heuristic to a literal local
file path named `file:/media/a.wav` → file-not-found. Inside the dead
half there is also a real defect against the file's own no-secrets
doctrine: a `file://user:pass@host/…` URI lands its whole userinfo-
bearing authority in the error Display. The reachable half (open_http →
RangedResource over the ureq transport) is real, and the "rustls TLS
always on" claim has actual code behind it in client.rs (cross-checked).

## 2. What actually works (verified)
— Reachable lane: `lumen open http://…` → parse.rs `open_command`
(SourceSpec::Url) → demux_host.rs:427 → `open_http_source` →
demux_host.rs:491 `SchemeRegistry::new().open_http(...)` →
`RangedResource::open` (schemes.rs:343-349). Call chain grepped end to
end.
— TLS claim has code: client.rs:91-93 `TlsProvider::Rustls` +
`tls_config()`, :115 `max_redirects(0)` — the doc's "rustls TLS with
verification always on, bounded redirects" (schemes.rs:233-235) is
backed by real configuration in the transport this registry constructs
(schemes.rs:238 `UreqClient::new()`).
— `Url::parse` scheme-lowercasing claim (schemes.rs:28-29) verified
against fetch.rs:214-221 (`split_once("://")` +
`scheme.to_ascii_lowercase()`).
— classify is pure and total as documented (213-215): no I/O anywhere in
classify/split_scheme/classify_file_uri/percent_decode (read in full);
every arm returns a strategy or a typed refusal.
— UNC refusal logic traced by hand and CORRECT: "file:////server/share"
→ rest "////server/share" → strip "//" → "//server/share" →
split_once('/') → ("", "/server/share") → path "//server/share" →
decoded starts_with("//") → typed UNC refusal (423-428); the 3-slash
form correctly stays a local absolute path.
— Windows drive-form strip (442-454): "/C:/x" → "C:/x" verified against
the byte conditions; single-letter drive-like prefix (`c:`) correctly
stays a local path (376-379).
— In-crate tests pass (baseline #3); https_activation.rs exercises
classify with uppercase scheme, default-port normalization, empty
authority refusal.

## 3. Findings

### F-001 [S2]
Where: crates/streaming/src/schemes.rs:4-5 (the claim), 281-327 (classify), 389-435 (classify_file_uri); the drift site: apps/cli/src/parse.rs:502-508 + apps/cli/src/demux_host.rs:421-435
Code:
```rust
//! Everything the player can be asked to open arrives here first
//! (doc 13 §9 wave 6.A; doc 06 §1.2 "access factory" row):
```
and the shipped reality (parse.rs:505-508):
```rust
    let is_url = source.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
```
What is wrong: The centrality claim is false. The shipped CLI never
calls `SchemeRegistry::classify` — it re-implements URL detection with a
`split_once("://")` heuristic (parse.rs) and scheme matching over
`SourceUri::parse` (demux_host.rs:425-427), a THIRD dispatch grammar.
Consequences: (a) the whole classification half of this file is
UNREACHABLE from the shipped entry point (only
crates/streaming/tests/https_activation.rs exercises it); (b) the two
grammars have already drifted — `classify_file_uri` accepts the legal
RFC 8089 single-slash form `file:/path` (module docs lines 32-34:
"`file:/path` and `file:///path` are the same resource"), but parse.rs's
`is_url` requires `://`, so `lumen open file:/media/a.wav` becomes
`SourceSpec::File { path: "file:/media/a.wav" }` → a literal
file-not-found for a legal URI the registry was built to normalize; (c)
the doc's stated purpose of mirroring input's rules "so Phase-6 dispatch
and `input` cannot drift" (lines 34-36) is defeated — the shipped
dispatch site is not this registry.
Why it matters: This file is the wave's "what really opens a URL"
linchpin; its documentation misdescribes the shipped architecture, its
best-tested code (classification, RFC 8089 normalization, percent-
decoding, the fuzz entry `fuzz_scheme_classify` registered in the fuzz
registry) protects nothing a user can execute, and the divergent live
grammar produces a wrong refusal on a legal input form.
What correct looks like: demux_host/parse.rs route through
`SchemeRegistry::classify` (the doc's own design), or the module doc is
rewritten to say the registry is the streaming-internal dispatch used by
the session layer and NOT the CLI open path.
Confidence: HIGH — Verification: read (all call chains grepped; quotes
verified by sed; both grammars compared line-by-line).

### F-002 [S2]
Where: crates/streaming/src/schemes.rs:389-403 (authority capture) and 189-191 (Display), doc claims at 45-53 and 100-101
Code:
```rust
        let (authority, path) = match after.split_once('/') {
            Some((authority, tail)) => (authority, format!("/{tail}")),
            None => (after, String::new()),
        };
        if authority.is_empty() || authority.eq_ignore_ascii_case("localhost") {
            path
        } else {
            return Err(SchemeError::RemoteFileHost {
                host: authority.to_string(),
            });
```
and the Display that prints it (189-191):
```rust
            SchemeError::RemoteFileHost { host } => {
                format!("file URI names a non-local authority {host:?}")
            }
```
What is wrong: The captured `authority` is whatever precedes the first
`/` — including RFC 3986 userinfo. `file://alice:secret@nas/share`
produces `RemoteFileHost { host: "alice:secret@nas" }`, and the Display
embeds it verbatim: `…non-local authority "alice:secret@nas"…`. The
module's credential-free logging doctrine explicitly promises the
opposite: "SchemeError never embeds the source reference… no URI, no
userinfo, no query" (100-101) and "URI-shaped credentials are refused
before dispatch — the port's Url::parse rejects userinfo outright"
(50-51) — but the `file:` arm never goes through `Url::parse`; it is
hand-parsed here with no userinfo handling. The only test of this
variant (753-761, part2) uses a clean host and asserts the host value —
no userinfo case exists.
Why it matters: Error Displays are log surfaces; a pasted credential-
bearing URL would write the credential to logs, contradicting the
crate's security posture. Today the blast radius is zero only because
the classification half is unreachable (F-001) — the defect ships the
moment anyone wires classify in.
What correct looks like: Strip (and refuse or redact) userinfo before
building the error — e.g. split at '@' and name only the host tail, or
refuse userinfo-bearing file authorities as Malformed with a static
detail, exactly like the http lane does.
Confidence: HIGH — Verification: read (traced the parse; Display and doc
quotes verified by sed; test absence verified by rg for userinfo in
schemes.rs tests).

### F-003 [S4]
Where: crates/streaming/src/schemes.rs:288-293 (the control-bytes refusal) vs 464-501 (percent_decode)
Code:
```rust
        if source.chars().any(|c| c.is_ascii_control()) {
            return Err(SchemeError::Malformed {
                detail: "the source reference contains control bytes",
```
What is wrong: The control-bytes refusal inspects the RAW source, but a
`file:` URI's path is percent-decoded afterwards (418), so
`file:///a%00b.wav` re-introduces a NUL (and any other control byte)
into the decoded path that reaches `AccessKind::LocalFile`. Downstream
`PathBuf`/open handles it as an OS error, so there is no panic — but the
"structurally malformed" guard is bypassable by exactly the encoding it
exists to police.
Why it matters: Cosmetic-to-hygiene: the invariant "control bytes are
refused" does not hold for escaped forms; a NUL-laden PathBuf is a
surprising artifact to hand a composition site.
What correct looks like: Re-run the control-byte check on the decoded
path (or refuse %00-%1F/%7F escapes in percent_decode).
Confidence: HIGH — Verification: read (decode path traced; no post-decode
control check exists between 418 and 434).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-53 | FALSE (centrality claim) | S2 | F-001; normalization/credential-free claims otherwise accurate where checked. |
| imports | 55-68 | REAL | — | All used. |
| `AccessKind` | 74-93 | UNREACHABLE | — | Consumed only by in-crate tests (https_activation.rs). |
| `SchemeError` | 102-131 | UNREACHABLE | S2 | F-002 userinfo-bearing host payload. |
| `SchemeError::code` | 133-141 | UNREACHABLE | — | Three stable codes; tested (part2 codes_are_stable_and_distinct). |
| `SchemeError::into_engine_error` | 148-177 | UNREACHABLE | — | No production caller (rg across repo: only codecs' unrelated same-named method); keeps NetworkError in source chain — correct shape, dead code. |
| `Display for SchemeError` | 180-195 | UNREACHABLE | S2 | F-002 leak surface. |
| `Error for SchemeError` | 197-207 | UNREACHABLE | — | source() only for Malformed — correct. |
| `SchemeRegistry` (struct) | 220-224 | REAL | — | One field (fetch). |
| `Default` | 226-230 | REAL | — | Delegates to new(). |
| `new` | 236-240 | REAL (reachable) | — | demux_host.rs:491; builds UreqClient (TLS claim cross-verified in client.rs). |
| `with_fetch` | 242-246 | REAL | — | Test injection seam; used by https_activation tests. |
| `with_retrying_fetch` | 264-272 | REAL-UNPROVEN | — | RetryFetch composition (reconnect.rs is another agent's); validate error propagated verbatim; no production caller found. |
| `classify` | 281-327 | REAL / UNREACHABLE | S2 | Pure+total verified; http/https arms delegate to Url::parse (scheme case handled both places — verified fetch.rs:219); F-001 unreachability. |
| `open_http` | 343-349 | REAL (reachable) | — | Thin: RangedResource::open over this registry's transport; probe documented as going out here. |
| `split_scheme` | 366-381 | REAL / UNREACHABLE | — | RFC 3986 grammar incl. single-letter drive rule and `name.tar:x` first-segment rule — both documented; traced edge cases (uppercase, unicode first char, `1a:`) — correct. |
| `classify_file_uri` | 389-435 | REAL / UNREACHABLE | S2 | UNC refusal traced CORRECT; F-002 authority capture; empty-path refusals verified. |
| `strip_drive_authority_slash` | 442-454 | REAL / UNREACHABLE | — | Byte conditions verified; `file:///C:` (len==3) yields drive-RELATIVE "C:" — mirrored from input (same-condition claim), edge noted, no finding. |
| `percent_decode` | 464-501 | REAL / UNREACHABLE | S4 | F-003 %00 bypass; otherwise correct hex handling + UTF-8 gate with typed refusal. |
| `fuzz_scheme_classify` | 516-524 | UNREACHABLE | — | Absence-of-panic harness; 64 KiB ceiling (FUZZ_INPUT_LIMIT:530). |
| tests `bare_paths_classify_local` | 545-561 | REAL | — | Would fail if bare-path passthrough broke (asserts verbatim equality). |
| tests `file_uris_normalize_to_local_paths` (starts 565) | 565-568+ | (part2) | — | Body continues in A05's range; boundary noted. |

## 5. Test honesty (tests in 532-568)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| bare_paths_classify_local (545-561) | YES | Mutate split_scheme to treat "c:/…" as scheme → the drive-letter case classifies non-local → unreachable! panic. |
| file_uris_normalize_to_local_paths (565+, body in part2) | YES (per part2's cases visible at 567-570) | Table-driven equality pins ("file:///media/a.wav" → "/media/a.wav" etc.). |
| https_activation.rs (crate test, exercises this part's classify/open_http) | YES | Pins https admission, uppercase scheme, default port 443, empty-authority refusal (sed -n 40,70p read). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Everything the player can be asked to open arrives here first" | schemes.rs:4-5 | FALSE | demux_host.rs:421-435 + parse.rs:502-508 dispatch independently; classify has zero production callers (F-001). |
| "rustls TLS with verification always on, bounded redirects" | schemes.rs:233-235 | REAL | client.rs:91-93 TlsProvider::Rustls + tls_config(); :115 max_redirects(0) (cross-file verified). |
| "Schemes are matched ASCII case-insensitively and lowercased… both here and in Url::parse" | schemes.rs:28-29 | REAL | split_scheme:380 to_ascii_lowercase; fetch.rs:219 same. |
| "`file:/path` and `file:///path` are the same resource" | schemes.rs:32-34 | REAL here / FALSE in shipped CLI | classify_file_uri handles it (404-405); the CLI's parse.rs "://" heuristic does not (F-001 drift). |
| "SchemeError never embeds the source reference… no userinfo" | schemes.rs:100-101 | FALSE | F-002: RemoteFileHost embeds userinfo-bearing authority. |
| "ftp:, rtsp:, data: → typed Unsupported" (HLS/DASH/RTSP refusals, wave question) | schemes.rs:20-24 | REAL (unreachable) | classify:322-324 `other => Unsupported`; https_activation + part2 tests pin rtsp refusal. |

## 7. Dead / duplicated / unreachable code
Dead in shipped builds (zero production callers): classify,
split_scheme, classify_file_uri, strip_drive_authority_slash,
percent_decode, fuzz_scheme_classify, FUZZ_INPUT_LIMIT, AccessKind,
SchemeError (all impls), into_engine_error, with_retrying_fetch.
Duplication: the scheme-dispatch grammar now exists in three places —
schemes.rs (RFC 3986), parse.rs is_url ("//" heuristic), demux_host.rs
(SourceUri scheme match) — with observable behavioral divergence
(F-001's `file:/path` case). This is the exact drift the module doc
claims its mirroring prevents.

## 8. Dependencies & flags
Crate-internal only: client (UreqClient), clock (WallClock), error
(NetworkError), fetch (HttpFetch/Url), ranged (RangedConfig/
RangedResource), reconnect (RetryFetch/RetryPolicy). No feature gates on
any of it. No unsafe. The ureq/rustls externals live behind client.rs
(this file only names them).

## 9. Open questions for the next agent
- A05 (schemes.rs#part2): the tests from 569 on — check whether any
  case pins the `file:/path` single-slash form (if yes, the in-crate
  suite contradicts the shipped CLI even harder); and whether a userinfo
  file-authority test exists (rg says no — F-002 untested).
- Whoever audits apps/cli parse.rs / demux_host.rs (CLI waves): the
  `is_url` "://" heuristic is the live dispatch bug for `file:/path`;
  also `open ftp://…` reaches the CLI's own exit-69 refusal naming
  docs/feature-matrix.toml — confirm that error path (not
  SchemeError::Unsupported) is what users actually see.
- A01 (fetch.rs): confirm `Url::parse` rejects userinfo (the doc claim
  this file leans on at line 50-51) — my grep saw no explicit userinfo
  check at 214-221; verify where that refusal happens.
