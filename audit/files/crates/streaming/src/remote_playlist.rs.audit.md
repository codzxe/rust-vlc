# AUDIT: crates/streaming/src/remote_playlist.rs
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Lines / functions | 500 / 6 production items (const, 3 types, len/is_empty, open_remote_playlist, sniff_format) + 9 tests |
| File verdict | REAL (genuine, well-tested network opener) but UNREACHABLE from shipped hosts |
| Reachable from shipped entry point? | NO — `open_remote_playlist` has zero non-test callers repo-wide (proof below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n 'remote_playlist' -g '*.rs' apps crates` (only self + lib.rs + a client.rs doc mention); baseline citation |

## 1. One-paragraph verdict
The module does exactly what its docs promise: fetches M3U/XSPF over the `HttpFetch` port with manual, bounded (5), loop-checked redirect following so entry locations resolve against the FINAL URL; userinfo stripped; https→http downgrade refused through the production gate `client::is_scheme_downgrade` (L150); `file:` redirect targets refused by `Url::join`'s grammar; parse failures folded into a static `ProtocolViolation` that carries no body bytes. All 9 tests run over real loopback sockets and assert exact resolved locations, walked paths, and that a downgrade target is never contacted. The hostile question — "is it reachable from shipped hosts?" — answers NO: the only callers of `open_remote_playlist` in the entire repo are its own `#[cfg(test)]` tests (rg: remote_playlist.rs:265,311,343,363,387,407,434,463,482). apps/cli (the only app depending on `streaming`) uses `fetch`/`ranged`/`schemes` and test-only `mockhttp` — never `remote_playlist`. This is a finished, tested capability wired to nothing.

## 2. What actually works (verified)
— Baseline audit/runs/baseline/cargo-test-workspace.txt:5588-5604: all 9 `remote_playlist::tests::*` ok (inside streaming's 212-passed lib run). Code-level verification of the redirect loop, downgrade gate call, and parse-error folding by direct read (L127-165, L173-178).

## 3. Findings
F-001 [S3]
Where: crates/streaming/src/remote_playlist.rs:114-117 (and repo-wide caller search)
Code:
```
pub fn open_remote_playlist(
    fetch: &dyn HttpFetch,
    url: &str,
) -> Result<RemotePlaylist, NetworkError> {
```
What is wrong: no shipped host calls it. `rg -n 'remote_playlist' -g '*.rs' apps crates` yields only: this file's definition+tests, `lib.rs:39 pub mod remote_playlist;`, and a doc mention in client.rs:222. apps/cli's Cargo.toml depends on `streaming` but its source uses only `fetch`/`ranged`/`schemes` (+ mockhttp under cfg(test)). Why it matters: the module presents itself as the network side of the playlist seam ("card 6.E2") and its tests are strong evidence of a working remote-playlist feature — but no user can trigger it; a feature-matrix row citing remote playlists would be citing dead-in-product code. What correct looks like: a CLI/desktop playlist-open path that calls it, or a doc line marking it unwired. Confidence: HIGH — Verification: ran rg (results above).

F-002 [S4]
Where: crates/streaming/src/remote_playlist.rs:176-208
Code:
```
    let (entries, parse_warnings) = match format {
        RemotePlaylistFormat::M3U => {
            let parsed = playlists::parse_bytes(&response.body).map_err(|_| PARSE_REFUSED)?;
            (
                parsed
                    .playlist
                    .entries
                    .into_iter()
                    .map(|entry| RemoteEntry {
```
What is wrong: the M3U and XSPF arms are 15-line copy-paste twins differing only in the parser call (`parse_bytes` vs `parse_xspf_bytes`); a `map_entries(parsed)` helper (or a parser fn pointer chosen first) would collapse them. Duplication risk: a fix applied to one arm (e.g., a new field) can miss the other. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MAX_PLAYLIST_REDIRECTS | 43-45 | REAL | — | 5, tighter than transport's 10, documented |
| RemotePlaylistFormat | 47-54 | REAL | — | two-variant sniff result |
| RemoteEntry | 56-67 | REAL | — | resolved location + title + duration |
| RemotePlaylist (+len/is_empty) | 69-97 | REAL | — | final-URL base + warning count |
| open_remote_playlist | 99-217 | REAL | S3 | full logic correct; unreachable (F-001) |
| sniff_format | 219-238 | REAL | — | BOM strip, first non-ws byte; HTML→XSPF→typed failure documented as deferred PROPOSAL |
| tests (9) | 240-500 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| m3u_resolves_locations_against_the_playlist_url | YES | exact resolved URLs for relative/../root-relative/absolute + UTF-8 title + durations (incl. -1 marker) |
| xspf_sniffed_and_resolved | YES | sniff + 3 resolved locations + title |
| redirects_followed_with_final_url_base | YES | source_url == /r2, entry against /r2 dir, walked paths == [pl.m3u, r1, r2] |
| redirect_budget_is_bounded_typed | YES | TooManyRedirects{limit:5, followed:5} with server scripted to 8 |
| redirect_loop_is_typed | YES | RedirectLoop on self_loop |
| scheme_downgrade_playlist_redirect_is_refused_typed | YES | CrossSchemeRedirect{hops:1} + code string + target.requests() EMPTY (never contacted) |
| https_playlist_redirect_to_https_target_follows | YES | https→https followed, base is final URL |
| file_scheme_redirect_target_is_refused_typed | YES | file:///etc/passwd Location → ProtocolViolation |
| failures_are_typed | YES | 404 → status_code, non-UTF-8 → ProtocolViolation |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "resolves every entry location with playlists::resolve_uri (the RFC 3986 §5 join...)" | remote_playlist.rs:10-12 | REAL | L185, L201 |
| "Followed manually (bounded by MAX_PLAYLIST_REDIRECTS, loop- and revisit-checked)" | remote_playlist.rs:16-17 | REAL | L125-165 |
| "A scheme downgrade ... is refused with the transport's own typed error" | remote_playlist.rs:20-25 | REAL | L150-152 calls client::is_scheme_downgrade; test at 400-420 |
| "URL userinfo is accepted and stripped" | remote_playlist.rs:35-36 | REAL | L120 `let (mut url, _userinfo) = Url::parse_with_userinfo(url)?;` |
| "matching browser/VLC semantics" (final-URL resolution base) | remote_playlist.rs:19-20 | UNVERIFIED | no browser/VLC executed here; the behavior itself is tested, the equivalence claim is not |
| "no body bytes or URLs ride the error, MP §12" | remote_playlist.rs:112-113 | REAL | PARSE_REFUSED is a static detail const (L173-175) |

## 7. Dead / duplicated / unreachable code
UNREACHABLE (F-001): the module's only consumer is its own test module. Duplication: F-002 (M3U/XSPF arms). No dead items within the file.

## 8. Dependencies & flags
crate::error, crate::fetch, crate::client (is_scheme_downgrade), playlists (parse_bytes/parse_xspf_bytes/resolve_uri). No features. No unsafe. Test module uses mockhttp (real sockets, server side) — the client side is MockClient (see mockhttp audit F-001 for the cross-host-policy divergence; harmless here since no cross-host policy is claimed for playlist opens).

## 9. Open questions for the next agent
- If any feature-matrix row claims "open remote playlist (M3U/XSPF over HTTP)", mark it UNREACHABLE/FALSE-at-product-level citing F-001.
- Entry-level `file:` passthrough inside playlist bodies is documented as deliberate (security review F3, L452-456): whoever audits the security posture should confirm the "opt-in origin policy" fix sketch is tracked, since a remote playlist can currently name local files that a host might open.
