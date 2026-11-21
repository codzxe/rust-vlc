# AUDIT: crates/playlists/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-21 |
| Lines / functions | 24 / 5 config keys ([package]×5, [lints], [dependencies]×1, [dev-dependencies]×0-empty) |
| File verdict | REAL (reachable via streaming, boundaries exact, deps honest — but the manifest's own description is several cards stale and materially understates the crate's untrusted-input parser surface) |
| Reachable from shipped entry point? | YES (call chain: `lumen` bin → lumen_cli (apps/cli/Cargo.toml:62 `streaming`) → streaming (Cargo.toml:42 `playlists = { path = "../playlists" }`) → playlists::mpd — used at streaming/src/dash/abr.rs:32 `use playlists::AdaptationSet;`, dash/sched.rs:30, dash/resolve.rs:2; CLI URL open path routes through streaming at apps/cli/src/demux_host.rs:480-492) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Grep boundaries.toml `[crates.playlists]` → internal_deps [], dev_deps [], external_allowlist ["thiserror"] (:226-231) = exact manifest match; Read crates/streaming/Cargo.toml (:42 playlists dep) + boundaries `[crates.streaming]` internal_deps incl. "playlists" (:215); Grep streaming/src `use playlists` → dash/{abr,sched,resolve}.rs; Grep apps/cli/src `streaming::` → demux_host.rs:383-492; LS crates/playlists/src → 11 modules (m3u, hls, mpd, pls, xspf, cue, probe, types, error, fuzz, lib); Read lib.rs:1-120 (full public surface); Grep `#\[allow\(\|unsafe` in crate → prose only, zero real sites; `sed -n '234,237p'` docs/01-architecture.md (registry row :236); baseline #1/#3/#5/#11 |

## 1. One-paragraph verdict
The manifest's dependency story is exact — boundaries.toml mirrors it perfectly (no internals, thiserror-only external, empty dev-deps with a reasoned comment), thiserror is genuinely used, and the crate is reachable from the shipped CLI through streaming's DASH lane. What fails is the manifest's self-description: it still says "M3U playlist reading/writing (card 4.D1)" while the crate now exports SIX untrusted-input format parsers — M3U, HLS master/media, MPD (DASH manifest, a hand-rolled XML-ish parser), PLS, XSPF (a hand-rolled XML parser with entity/CDATA handling), CUE — plus a bounded directory walker and a fuzz harness (lib.rs:69-119). A manifest that hides five parsers, two of them hand-rolled XML, behind a one-format description misdirects exactly the security review this repo's process is built around. The header's "no internal deps yet / consumer lands with 4.C2's panel work" is also stale: the real consumer is streaming, and doc 01's registry row still lists two deps that exist nowhere.

## 2. What actually works (verified)
— Manifest ↔ boundaries exact two-way match: `[crates.playlists] internal_deps = [], dev_deps = [], external_allowlist = ["thiserror"]` (boundaries.toml:226-231) equals the manifest (no internals, thiserror only, empty dev-deps).
— thiserror is a real, used dependency: 12 `thiserror::Error` derives across error.rs:14/:42, probe.rs:400, xspf.rs:70/:109, pls.rs:63/:88, mpd.rs:541, cue.rs:371.
— Reachability: `lumen` → lumen-cli → streaming (apps/cli/Cargo.toml:62; demux_host.rs:491 `streaming::schemes::SchemeRegistry`) → playlists (streaming/Cargo.toml:42; dash/abr.rs:32, dash/sched.rs:30) — the DASH open path parses through `playlists::mpd`.
— Lint hygiene: zero `#[allow(` and zero unsafe in the crate (Grep → prose mentions only); workspace clippy CLEAN (baseline #5); the crate's suites ran in baseline #3 (1681 passed workspace-wide).
— The empty `[dev-dependencies]` is deliberate and documented (inline splitmix64 fuzz-lite RNG, "matching the repo's no-proptest status quo") — consistent with boundaries' empty dev_deps.

## 3. Findings
### F-001 [S3]
Where: crates/playlists/Cargo.toml:1,12
Code:
```
# crates/playlists — M3U playlist reading/writing (card 4.D1).
...
description = "Lumen playlist support: extended/plain M3U parsing (dirty-world tolerant, D-1/D-2) and canonical writing with pinned roundtrip stability (card 4.D1)."
```
What is wrong: The manifest describes one format (M3U, card 4.D1). The crate actually exports six untrusted-input parsers and more: M3U (`parse`/`write`, m3u.rs), HLS master+media playlists (hls.rs, lib.rs:78-82), MPD/DASH manifests with a hand-rolled parser and depth/byte caps (mpd.rs, lib.rs:84-88, `MAX_MPD_BYTES`/`MAX_MPD_DEPTH`), PLS (pls.rs, lib.rs:89-92), XSPF — a hand-rolled XML subset parser with entities, numeric references, CDATA and a 64-element depth guard (xspf.rs, lib.rs:94-97), CUE sheets (cue.rs, lib.rs:99-104), a probing/directory walker (probe.rs, lib.rs:111-119), and a fuzz module (lib.rs:113). The crate's own lib.rs docs are current (cards 4.D1 + 5.O1a + 5.O1b + 5.O3 named) — only the manifest was never updated.
Why it matters: A crate manifest is the inventory document for dependency and security review. "M3U parsing" invites a line-format review; the reality includes two hand-rolled XML-family parsers (XSPF, MPD) — historically the exact class where entity-expansion, depth, and truncation bugs live. An auditor or Lead triaging by manifest metadata would under-scope the untrusted-input surface by five parsers.
What correct looks like: description = "M3U/M3U8, HLS, PLS, XSPF, CUE playlist and MPD/DASH manifest parsing + canonical M3U writing + bounded directory probing (cards 4.D1, 5.O1a/b, 5.O3, 6.B/6.C models)".
Confidence: HIGH — Verification: read of manifest + lib.rs:69-119 + LS of src (11 modules).

### F-002 [S4]
Where: crates/playlists/Cargo.toml:3-4
Code:
```
# Layer: application-support, no internal deps yet (the queue semantics that
# will consume this crate land with 4.C2's panel work).
```
What is wrong: Both clauses are stale. (a) The crate has had an internal dependent for several waves: streaming (crates/streaming/Cargo.toml:42, boundaries.toml:215 registered) consumes the M3U/HLS/MPD models for its DASH/HLS lanes. (b) The predicted consumer ("queue semantics … 4.C2's panel work") is not what materialized — the panel/queue work is not the dependent; streaming is.
Why it matters: The comment misdirects reachability analysis: a reader concludes this crate is a leaf awaiting UI wiring and deprioritizes it; in reality it sits on the shipped CLI's network-open path (via streaming). Same frozen-at-card-time prose class as audio-output F-001.
What correct looks like: "Consumer: streaming (DASH/HLS models, card 6.B/6.C); the queue/panel wiring remains future work."
Confidence: HIGH — Verification: read + streaming manifest/boundaries citations.

### F-003 [S4]
Where: crates/playlists/Cargo.toml:3 (cross-ref: docs/01-architecture.md:236, boundaries.toml:226-231)
Code:
```
# Layer: application-support, no internal deps yet (the queue semantics that
```
(Cross-referenced row: doc 01:236 `| playlists | crate | L2 | media-types, plugin-api | 4 | playlist formats |`.)
What is wrong: The doc 01 registry row lists internal deps "media-types, plugin-api" — neither is declared in the manifest, and boundaries.toml:228 correctly records `internal_deps = []`. The registry disagrees with BOTH ground-truth artifacts. Third registry-drift instance in this batch (after device-discovery's stale plugin-api edge and platform-linux's three target-state deps): doc 01's crate table is drifting from the manifests with no gate diffing them.
Why it matters: doc 01 is the architecture constitution the boundaries checker is derived from; when its dep table and boundaries.toml disagree, one of them is wrong and nothing says which. Here boundaries+manifest agree and doc 01 is the outlier — but the device-discovery case (this batch, file 4) had the opposite alignment. The registry cannot be trusted without per-row verification.
What correct looks like: doc 01's dep column regenerated from (or cross-checked against) boundaries.toml by an xtask command; until then every row is suspect.
Confidence: HIGH — Verification: read of all three artifacts.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package]` name/version/edition/publish | 8-13 | REAL | — | `playlists` 0.1.0, edition 2024, publish=false — but the description is 4 cards stale (F-001). |
| `[lints] workspace = true` | 15-16 | REAL | — | Binds (baseline #11); zero escape hatches, zero unsafe in crate (Grep → 0). |
| `thiserror = "2"` dep | 19 | REAL | — | Used (12 derives across 7 modules); allowlist verdict `pass` (allowlist.md:66); lock 2.0.21 at floor. Requirement "2" is wider than the audited floor >=2.0.21 (same systemic class as plugin-api F-001, S4 there — noted here, not double-counted). |
| `[dev-dependencies]` (intentionally empty) | 21-23 | REAL | — | Documented rationale (inline splitmix64 RNG, no proptest); matches boundaries dev_deps = [] (:229). |

## 5. Test honesty (if tests exist in/for this file)
No tests in this file. The crate's suites (roundtrip law `parse(write(parse(x))) == parse(x)`, dirty corpus, ffmpeg-generated HLS fixtures, fuzz-lite per lib.rs:25-27, 50-52) ran inside baseline #3. The roundtrip law is a strong oracle — a mutation in write() or parse() breaks it on the corpus. Src auditor to confirm the fuzz-lite harness is deterministic (seeded) rather than vacuous.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "M3U playlist reading/writing (card 4.D1)" | manifest:1,12 | FALSE (stale, under-scoped) | Six parsers + walker exported (lib.rs:69-119) — F-001. |
| "no internal deps yet" | manifest:3 | FALSE (stale) | streaming depends on playlists (streaming/Cargo.toml:42, boundaries:215) — F-002. |
| "Std + thiserror only … no new dependency" | manifest:4-6 | REAL | [dependencies] = thiserror only; used; allowlist pass. |
| Registry row: playlists deps media-types, plugin-api | doc 01:236 | FALSE (phantom deps) | Manifest has no internals; boundaries:228 `internal_deps = []` — F-003. |

## 7. Dead / duplicated / unreachable code
- None in the manifest. The crate is reachable (streaming consumer) — NOT an orphan, unlike four other crates in this batch.

## 8. Dependencies & flags
- External: thiserror "2" (lock 2.0.21, allowlist pass, floor-float note above). Internal: none. Dev: none.
- deny.toml interplay: nothing to govern beyond the license class (MIT/Apache, in allow set).
- Features: none.

## 9. Open questions for the next agent
- Src auditors for xspf.rs and mpd.rs (hand-rolled XML-family parsers): the manifest hid you from triage — check entity/numeric-reference expansion bounds, the 64-element depth guard (lib.rs:62), MAX_MPD_BYTES enforcement, and truncation salvage paths first.
- Registry owner: doc 01's dep column needs a generator or checker (see F-003); three drift instances documented in this batch alone (device-discovery, platform-linux, playlists).
