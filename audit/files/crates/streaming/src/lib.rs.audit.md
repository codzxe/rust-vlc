# AUDIT: crates/streaming/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-21 |
| Lines / functions | 40 / 0 (crate root: module docs + 13 `pub mod` declarations) |
| File verdict | REAL (stale self-description) |
| Reachable from shipped entry point? | YES — it is the crate root of a crate the shipped `lumen` binary links (apps/cli demux_host.rs uses `streaming::fetch`, `streaming::ranged`, `streaming::schemes`; baseline loopback e2e orchrun-test-workspace-nofailfast.log.txt:3039). But only 5 of its 13 modules are reachable from the shipped lane: `fetch`, `client` (via `SchemeRegistry::new` → `UreqClient`, schemes.rs:238), `ranged`, `schemes`, `error`. UNREACHABLE: `auth`, `buffer`, `clock`, `dash`, `hls`, `reconnect`, `remote_playlist` (no production constructor/caller — verified per-file in my other audits) and `mockhttp` (testkit; production-tree references are all `#[cfg(test)]`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `rg -o "streaming::[a-z_]+"` over apps/cli (reachable subtree: fetch/mockhttp/ranged/schemes — mockhttp only in `#[cfg(test)]` at demux_host.rs:881); `rg -n "with_retrying_fetch"` (schemes.rs tests only — the retry layer is NOT in the shipped lane); `rg -ln "clock::"` (buffer/reconnect/hls-live/schemes — the schemes use is the unwired `with_retrying_fetch`); `sed -n` on lib.rs and Cargo.toml quotes (verbatim). |

## 1. One-paragraph verdict
The crate root is a 40-line module map whose prose is two revisions stale: it calls the crate a "scaffold" whose scope is "the port only" while declaring 13 modules — three of them parenthetically marked "landed" inside the very sentence that says everything else "lands in later cards" — and it asserts "This crate is std-only" over a Cargo.toml listing four dependencies including the external ureq. The async claim is true (no async runtime anywhere in the crate) and the bounded-memory claim is real (verified in client.rs). The actionable truth this file obscures: of the whole 13-module network layer, only the plain-HTTP progressive lane (fetch→client→ranged→schemes) is wired into the shipped binary; auth, HLS, DASH, reconnect, buffering, remote playlists, and the retry layer are tested library islands, and the mock HTTP server ships in the production binary unused.

## 2. What actually works (verified)
— verified by baseline run `orchrun-test-workspace-nofailfast.log.txt:3039` (`http_url_opens_over_a_real_loopback_socket … ok`): the crate root's reachable subtree (fetch/ranged/schemes/client) executes in the shipped binary over a real socket.
— verified by read: "no async runtime may appear in engine crates" holds — no async/await/tokio tokens in the streaming crate (checked across my seven audited files); "Responses are whole bodies under an explicit memory budget … a hostile server advertising more fails typed" holds in the transport (client.rs:288-300 advertised check, 304-317 bounded read — my client.rs#part1 audit).

## 3. Findings

### F-001 [S4]
Where: crates/streaming/src/lib.rs:3-10
Code:
```
//! Scope of this scaffold: the **sync HTTP(S) fetch port**
//! ([`fetch::HttpFetch`]), the typed network error vocabulary
//! ([`error::NetworkError`]), and a std-only mock HTTP/1.1 testkit
//! ([`mockhttp`]). Everything else network — the real HTTP client
...
//! and RTSP (6.G) — lands in later cards on top of this port.
```
What is wrong: the crate self-describes as a scaffold whose scope is "the port only", in the same breath marking the scheme registry, HLS, and DASH as "landed" — the sentence contradicts itself and the 13 `pub mod` declarations below it (10 modules beyond the claimed scope, including the landed-marked ones plus auth, buffer, reconnect, remote_playlist, clock, client).
Why it matters: the crate root is the map every new reader (and auditor) starts from; it misstates what exists and what is wired, which is exactly the "what really opens a URL" question this wave was asked.
What correct looks like: a current scope paragraph naming all 13 modules with their wiring status (5 shipped-lane, 8 unwired).
Confidence: HIGH — Verification: read + per-module reachability greps (header).

### F-002 [S4]
Where: crates/streaming/src/lib.rs:14-16 (mirrored at Cargo.toml:10-11)
Code:
```
//! in engine crates (docs/01-architecture.md §4.2). This crate is
//! std-only and blocking; deadlines are plain [`std::time::Duration`]s
```
What is wrong: "This crate is std-only" is false — the crate depends on `media-types`, `plugin-api`, `playlists`, and the external `ureq` (Cargo.toml:34-52). The Cargo.toml header repeats the claim ("C5 discipline: std-only, zero external dependencies") directly above a dependency table that disproves it; only the ureq admission note (43-52) documents the exception.
Why it matters: dependency-discipline claims that the manifest itself contradicts are exactly the drift a hostile audit exists to catch; a reader trusting the header would believe the network layer has no external code (it carries ureq + rustls + ring C/asm).
What correct looks like: "std-only except the admitted ureq pin (record R10)" in both sites, or drop the claim.
Confidence: HIGH — Verification: read of both files.

### F-003 [S4]
Where: crates/streaming/src/lib.rs:36 (`pub mod mockhttp;`)
Code:
```
pub mod mockhttp;
```
What is wrong: the mock HTTP/1.1 server + client testkit is a public production module — compiled into the shipped release binary — whose every in-tree consumer is `#[cfg(test)]` (demux_host.rs:881 CLI tests; streaming crate's own test modules). No production path constructs a MockServer.
Why it matters: test tooling (a socket-listening server, however loopback-bound and std-only) ships unused in the release binary; binary size and attack-surface hygiene, plus a standing temptation for production code to reach for the mock.
What correct looks like: `#[cfg(any(test, feature = "testkit"))] pub mod mockhttp;` so release builds drop it.
Confidence: HIGH — Verification: `rg -ln "mockhttp"` — all non-test hits are doc comments or the declaration itself.

## 4. Per-function table (module declarations — the only items)
| Module | Line | Reachable from shipped lane? | Notes |
|---|---|---|---|
| auth | 28 | NO | no production `AuthFetch` constructor (auth.rs#part2 audit) |
| buffer | 29 | NO | `BufferModel::new` tests-only (buffer.rs#part1 audit) |
| client | 30 | YES | `SchemeRegistry::new` → `UreqClient` (schemes.rs:238) |
| clock | 31 | NO | used only by unwired lanes (buffer/reconnect/hls-live/`with_retrying_fetch`) |
| dash | 32 | NO | no caller outside tests (mpd.rs#part4 audit) |
| error | 33 | YES | typed vocabulary used by the whole subtree |
| fetch | 34 | YES | port types; `Url::parse` on the open path |
| hls | 35 | NO | no `hls::` reference in any app |
| mockhttp | 36 | NO (testkit in prod build) | F-003 |
| ranged | 37 | YES | `RangedResource::open` on the open path (demux_host.rs:493-498) |
| reconnect | 38 | NO | `with_retrying_fetch` callers are schemes.rs tests (958, 1015) |
| remote_playlist | 39 | NO | `open_remote_playlist` callers are its own tests |
| schemes | 40 | YES | `SchemeRegistry::new().open_http` (demux_host.rs:491) |

## 5. Test honesty
No tests in this file. The crate's tests run in baseline (streaming suites in orchrun-test-workspace-nofailfast.log.txt:4944-4972 region) — they exercise library code regardless of wiring; passing tests prove the islands work, not that the shipped binary reaches them.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Scope of this scaffold: the port only … everything else lands in later cards" | lib.rs:3-10 | FALSE (stale) | F-001: 13 modules, three marked landed in the same sentence |
| "This crate is std-only" | lib.rs:14-15; Cargo.toml:10-11 | FALSE | F-002: ureq + 3 internal deps |
| "no async runtime may appear in engine crates" | lib.rs:13-14 | REAL | no async tokens across the audited files |
| "a hostile server advertising more fails typed … instead of allocating" | lib.rs:20-24 | REAL | client.rs:288-317 (my part1 audit) |
| streaming crate = layer L2 of doc 01 §2.4 | lib.rs:1 | REAL | architecture gate knows the crate (baseline gate-check_architecture: lumen-cli→streaming edge undeclared — registration debt) |

## 7. Dead / duplicated / unreachable code
8 of 13 modules are production-unreachable (see table); `mockhttp` additionally ships in the binary (F-003). No duplication in this file.

## 8. Dependencies & flags
Crate deps: media-types, plugin-api, playlists (internal), ureq =3.5.0 (external, pinned, rustls+webpki, default-features=false). No `[features]` section — nothing can be compiled out; everything ships. Dev-deps: testkit, demux, input, codecs (dev-only, correctly separated).

## 9. Open questions for the next agent
- Wave gate: the reachable-subtree map above (5/13 modules) is the evidence base for every network matrix row — rows citing hls/dash/auth/reconnect/buffer code as "implemented/partial" should carry an explicit "library landed, no shipped wiring" qualifier, consistent with my per-file audits.
- Whoever owns schemes.rs (not in my list): confirm `SchemeRegistry::new()` is the ONLY production constructor (my greps say yes) — the retry layer's absence from the shipped lane is a matrix-level truth (`network-reconnect-buffering` not_started is accurate).
