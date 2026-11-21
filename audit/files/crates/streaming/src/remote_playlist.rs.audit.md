# AUDIT: crates/streaming/src/remote_playlist.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-21 |
| Lines / functions | 500 / 17 (7 prod items + `len`/`is_empty` + `sniff_format` + 9 tests, see §4) |
| File verdict | UNREACHABLE (real, e2e-tested over sockets — but no production caller; the shipped CLI probes remote URLs as containers instead) |
| Reachable from shipped entry point? | **NO.** `rg -n "open_remote_playlist\|remote_playlist" crates/ apps/ -g '!crates/streaming/src/remote_playlist.rs'` → only a doc comment (client.rs:222) and the module decl (lib.rs:39). The CLI's http(s) lane (`apps/cli/src/demux_host.rs:441-442`, `open_http_source` 478-505) opens every URL as a `RangedResource` and feeds it to `media_probe::probe_stream` + `containers::open` — a remote `.m3u` is probed as a *container* and refused there; this module is never invoked. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "open_remote_playlist\|remote_playlist" crates/ apps/` (reachability); Read (whole file); `sed -n '395,500p' apps/cli/src/demux_host.rs` (CLI URL lane: ranged-bytes only, probe+container dispatch); `rg -n "remote_playlist::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (9/9 ok at :5130-5150); `rg -n "fn parse_bytes\|location line" crates/playlists/src/m3u.rs` + `rg -n "fn resolve_uri" crates/playlists/src/lib.rs` (F-001 chain); `rg -n -i "playlist\|network-" docs/feature-matrix.toml` + `sed -n '1469,1492p'` (matrix rows network-auth/network-remote-playlists = not_started, empty evidence). |

## 1. One-paragraph verdict
A clean, honest module: bounded manual redirect walking (budget 5, revisit detection, https→http downgrade refused with the transport's own typed gate — pinned by a test that proves the downgraded target is never contacted), final-URL-relative entry resolution matching browser/VLC semantics, UTF-8/BOM-aware format sniffing, static error details with no body bytes or URLs riding errors, and 9 socket-level tests all green in the baseline run. Its one real defect: the sniffer has no PLS story — a remote `.pls` (the Shoutcast staple this project's own formats.json tracks) is silently parsed as M3U and yields garbage entries (`[playlist]`, `File1=…` become location lines), where the house style demands a typed refusal (F-001). And the wave's standing verdict applies: no shipped entry point calls any of this — the CLI treats remote playlists as undifferentiated bytes to container-probe.

## 2. What actually works (verified)
— `remote_playlist::tests` 9/9 `ok` (`m3u_resolves_locations_against_the_playlist_url`, `xspf_sniffed_and_resolved`, `redirects_followed_with_final_url_base` — asserts the exact request path chain `/pl.m3u→/r1→/r2`, `redirect_budget_is_bounded_typed` (limit 5), `redirect_loop_is_typed`, `scheme_downgrade_playlist_redirect_is_refused_typed` (+ asserts the http target got ZERO requests), `https_playlist_redirect_to_https_target_follows`, `file_scheme_redirect_target_is_refused_typed`, `failures_are_typed`) — `orchrun-test-workspace-nofailfast.log.txt:5130-5150`, real `MockServer` loopback sockets.
— Redirect semantics read-verified: 301/302/303/307/308 + Location → join; budget checked BEFORE follow; visited-list canonical `to_string()` compare; downgrade gate shared with the transport (`client.rs::is_scheme_downgrade`); a redirect status without Location falls out to `http_status` typed error (167-169).
— Userinfo stripping verified against `Url::parse_with_userinfo` (fetch.rs:199-201) — credentials never reach the resolution base; auth composition delegated to `AuthFetch` (documented at 35-38).

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/remote_playlist.rs:224-238 (with 176-192)
Code:
```
fn sniff_format(body: &[u8]) -> RemotePlaylistFormat {
    let mut bytes = body;
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes = &bytes[3..];
    }
    let starts_with_xml = bytes
        .iter()
        .find(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|byte| *byte == b'<');
```
What is wrong: The sniff is binary — `<` ⇒ XSPF, everything else ⇒ M3U. A PLS body (`[playlist]\nFile1=http://…\nNumberOfEntries=1\nLength1=-1`) contains no `<`, so it is parsed as M3U, and per the M3U grammar ("Everything else is a location line", m3u.rs:21) every PLS directive becomes a playlist *entry*: the caller receives `Ok(RemotePlaylist)` with garbage locations (`[playlist]`, `File1=http://…` resolved as relative URIs against the base). The module's own scope note says "PLS/MPD remote open are not in 6.E2's scope" (34) — but the code neither refuses PLS typed nor detects it; it silently returns nonsense.
Why it matters: PLS is a first-class playlist family in the project's own inventory (formats.json, matrix row `playlist-pls`), and Shoutcast stations commonly serve `.pls` URLs. A consumer of this API (or a future CLI wiring) "opens" the playlist successfully with 4 garbage tracks instead of getting the honest typed refusal every other unsupported shape in this crate produces (cf. the HTML→XSPF typed failure, and `file:` redirect refusal). Silent wrong data beats loud failure only in the attacker's calculus.
What correct looks like: Either sniff `[playlist]` after BOM/whitespace and return a typed `ProtocolViolation` ("PLS not supported in remote open"), or actually delegate to `playlists::parse_pls_bytes` (the parser exists — crates/playlists/src/pls.rs, matrix row cites 46 tests).
Confidence: HIGH — Verification: read (sniff + M3U grammar at m3u.rs:21 + parse flow 176-192); no PLS handling anywhere in this file (rg).

### F-002 [S4]
Where: crates/streaming/src/remote_playlist.rs:1-3 vs docs/feature-matrix.toml:1481-1491
Code:
```
//! Remote playlist open (card 6.E2) — M3U/XSPF over the streaming
//! layer, with relative-URI resolution against the playlist's **final**
//! URL and bounded redirect following.
```
```
id = "network-remote-playlists"
...
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
evidence = ""
```
What is wrong: Matrix bookkeeping contradiction inside the same category: `network-remote-playlists` (this module, fully implemented, 9/9 e2e green) is recorded as not_started with empty evidence, while `network-dash` (also implemented-but-unwired, same crate, same test-suite standard) is recorded "partial" on three platforms citing its library tests as evidence.
Why it matters: The two rows use opposite standards for identical situations — one over-claims platform availability from library tests, the other hides finished library work. Anyone reconciling the matrix against the code gets neither row right; the parity report's 0/252 inherits the noise.
What correct looks like: One standard: either both rows "partial" with library-level evidence strings, or both "not_started" with the library work in `notes`.
Confidence: HIGH — Verification: read (both matrix rows + this module's test results).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-38 | REAL | — | Redirect semantics, HTML [PROPOSAL] deferral, PLS scope-out, auth composition — all verified against code/tests except PLS behavior (F-001 contradicts the scope note's spirit). |
| `MAX_PLAYLIST_REDIRECTS` | 43-45 | REAL | — | 5; pinned by `redirect_budget_is_bounded_typed`. |
| `RemotePlaylistFormat` | 47-54 | REAL | — | Two variants; sniffed only. |
| `RemoteEntry` | 57-67 | REAL | — | location/title/duration; all fields populated from both parsers. |
| `RemotePlaylist` | 70-83 | REAL | — | `parse_warnings` is a COUNT — details dropped (documented "counted, not re-surfaced"); acceptable summary posture. |
| `RemotePlaylist::len` / `is_empty` | 85-96 | REAL | — | `#[must_use]` pair. |
| `open_remote_playlist` | 114-217 | REAL | S3 (F-001) | Userinfo stripped (auth delegated); manual redirect walk (budget/loop/downgrade gates, no cross-host policy — none needed, credential-free); typed statuses; `PARSE_REFUSED` static detail (no bytes/URLs in errors — MP §12 claim verified); both parser branches structurally identical (duplication noted below). |
| `sniff_format` | 224-238 | REAL | S3 (F-001) | UTF-8 BOM strip + whitespace skip + `<` test; no PLS/HTML detection (HTML fails typed via XSPF parse — documented). |
| tests ×9 | 240-500 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `m3u_resolves_locations_against_the_playlist_url` | YES | 4 resolution shapes (relative/`..`/root-relative/absolute) asserted against exact server URLs; UTF-8 title + durations round-tripped; `-1` preserved. |
| `xspf_sniffed_and_resolved` | YES | Exact locations incl. `../` and absolute passthrough; title carried. |
| `redirects_followed_with_final_url_base` | YES | Asserts both the final-URL base AND the request path sequence — a loop that resolved against the original URL fails the second assert. |
| `redirect_budget_is_bounded_typed` | YES | 8-redirect chain must yield `TooManyRedirects{limit:5, followed:5}` — exact match. |
| `redirect_loop_is_typed` | YES | Self-loop → `RedirectLoop` (would hang the test if detection broke). |
| `scheme_downgrade_playlist_redirect_is_refused_typed` | YES | Asserts typed error + code string + that the http target received ZERO requests — proves the gate fires before the hop. |
| `https_playlist_redirect_to_https_target_follows` | YES | Negative space: same-scheme follows and the final https URL is the base. |
| `file_scheme_redirect_target_is_refused_typed` | YES | `file:///etc/passwd` Location → typed refusal, one request. |
| `failures_are_typed` | YES | 404 → `status_code()==Some(404)`; non-UTF-8 body → ProtocolViolation. |
Execution: 9/9 `ok` — orchrun-test-workspace-nofailfast.log.txt:5130-5150. Gap: no test pins a PLS body (F-001 untested), none pins an HTML body's typed failure (the [PROPOSAL] note's promise).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "resolves every entry location … the RFC 3986 §5 join: `../`, `./`, root-relative, absolute, Windows passthrough" | module doc :10-12 | REAL | `m3u_resolves…`/`xspf_sniffed…` assert 3 of 5 shapes; `resolve_uri` is playlists' lib (W10-A10's probe/types range). |
| "a playlist served after `/old/list.m3u → /new/list.m3u` resolves its entries against `/new/`" | :18-20 | REAL | `redirects_followed_with_final_url_base` green. |
| "PLS/MPD remote open are not in 6.E2's scope" | :34 | SHALLOW | Not refused typed either — silently mis-parsed (F-001). |
| "no body bytes or URLs ride the error, MP §12" | :112-113 | REAL | Read: every error variant carries static details/counts/codes only. |
| network-remote-playlists = "not_started", evidence "" | feature-matrix.toml:1486-1490 | Under-claim (F-002) | Code + 9/9 e2e exist; row hides it (product-column accurate: nothing wired). |

## 7. Dead / duplicated / unreachable code
- Production-unreachable (header rg): the only consumer the docs promise ("compose AuthFetch under this opener") doesn't exist.
- Duplication: the M3U and XSPF arms of `open_remote_playlist` (176-208) are structurally identical 15-line blocks differing only in the parse call — one closure over the parser would remove it. (Not filed as a finding; noted for the fix pass.)

## 8. Dependencies & flags
- `crate::error::NetworkError`, `crate::fetch` (HttpFetch/HttpRequest/RedirectMode/Url), `playlists::{parse_bytes, parse_xspf_bytes, resolve_uri}`. std-only otherwise. No feature gates, no unsafe, no unwrap/expect (tests use testkit `Must`). Dev-dep on mockhttp (in-crate testkit, real sockets).

## 9. Open questions for the next agent
- W10-A01 (fetch.rs): confirm `HttpRequest::get`'s default `response_limit` bounds this fetch (module doc says "bounded body" — the bound lives in your file; if the default is unbounded the claim is false).
- Whoever wires the CLI playlist lane: F-001 must be fixed first, or a remote `.pls` open ships garbage entries to users.
- `client.rs` (W10-A04/A05): the doc at :221-222 says the auth follow and this opener share the AUD-07/AUD-14 downgrade gate via `is_scheme_downgrade` — confirm no third manual-follow loop exists without the gate.
