# AUDIT: crates/demux/src/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-21 |
| Lines / functions | 292 / 18 (1 const, 1 private `mint`, 14 public constructors, 3 tests) |
| File verdict | REAL (with dead members — see F-001) |
| Reachable from shipped entry point? | YES (used constructors mint on the AVI open path: `apps/cli/src/containers/mod.rs:54` `"avi"` → `avi/mod.rs` call sites below; the shipped AVI lane is the proven first vertical) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:3 |
| Commands run | Per-constructor caller census via `rg` over crates/demux/src + apps/cli/src (results: 12/14 constructors have 1-4 minting sites; `payload_too_large` 0 sites — doc admits; `seek_unindexed` 0 sites — doc does not); `rg -n "unseekable" crates/demux/src/avi` → riff.rs:231 rejects unseekable sources at construction; `rg -n "not_started\|demux\..-not-started"` across sibling error modules (all 9 prefix, avi does not); baseline grep for the 3 tests (log 1675-1677); `sed -n` verification of every quote. |

## 1. One-paragraph verdict
Clean, small, honest-by-mostly: every constructor mints a media-types
`EngineError` at stage Demux with a stable dotted code and no secrets; the
test pins all 14 codes, the stage tag, the Display shape, and the sniff-cap,
and all 3 tests pass in the baseline
(`orchrun-test-workspace-nofailfast.log.txt:1675-1677`). Two real defects:
`seek_unindexed` is dead code whose doc presents an unreachable condition as
live (unseekable sources are rejected at construction, so the no-idx1-plus-
unseekable conjunction can never arise), and the module itself is the AVI
error module (13 of 14 codes are `demux.avi.*`) living at the crate root under
a layer-wide name, with one code (`demux.not-started`) violating the prefix
convention all nine sibling modules follow. `payload_too_large` is also
unminted, but its doc says so explicitly — deliberate vocabulary, not a lie.

## 2. What actually works (verified)
— 3/3 tests pass: baseline log 1675-1677 (`error::tests::*`).
— Constructor caller census (verified by rg): not_avi → avi/mod.rs:1118,1121;
   no_header → 1227; truncated → 472,814,1116,1143; bad_structure →
   mod.rs:677, index.rs:733; bad_stream_params → header.rs:268,290,570;
   walk_resync_failed → mod.rs:816; no_supported_streams → 1243;
   index_too_large → index.rs:328; no_movi → 443,447,1237; not_started →
   922,1035; seek_out_of_range → 1041,1044,1050,1052; scan_too_slow →
   index.rs:706. Twelve of fourteen constructors are live on the shipped AVI
   path.
— No-secrets rule enforced and tested: `not_avi_sniff_capped…` fails if the
   Display ever renders more than 16 sniffed bytes (290).

## 3. Findings
### F-001 [S3]
Where: crates/demux/src/error.rs:195-204
Code:
```
/// A seek was asked of a container with no `idx1` **and** a source that
/// cannot be positionally scanned (unseekable stream): the linear
/// fallback is impossible (doc 03 §4.3(b) `SeekSupport::SlowLinear`
/// needs a seekable source).
pub fn seek_unindexed() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.avi.seek.unindexed",
    )
}
```
What is wrong: Zero minting sites in the workspace (rg: only its own test at
error.rs:245 constructs it). And the documented condition is structurally
unreachable: unseekable sources are rejected at `AviDemuxer` construction —
`crates/demux/src/avi/riff.rs:231` returns `unsupported("demux.source.unseekable")`
— pinned by `avi/tests.rs:2206-2258` (`unusable_sources_are_rejected_at_construction`).
So no `idx1` + unseekable can never coexist inside a started AVI demuxer.
Unlike `payload_too_large` (whose doc admits "without a minting site",
135-140), this doc presents a live refusal path that does not exist.
Why it matters: dead public API whose doc misleads — a maintainer wiring a
seek path will reach for this constructor believing the demuxer can hit the
condition; an auditor reading the code list overestimates the typed-refusal
surface. The test at 245 cements the dead code as "covered".
What correct looks like: delete it, or add the same honest "no minting site;
kept for X" note `payload_too_large` carries — or actually mint it if some
future source type (streaming) makes the condition real.
Confidence: HIGH — Verification: read + repo-wide caller census + construction-gate traced (riff.rs:231, tests.rs:2244).
### F-002 [S4]
Where: crates/demux/src/error.rs:1-11 (with lib.rs:31-33)
Code:
```
//! Typed error constructors for the demux layer (card 2.B2).
//!
//! The error *taxonomy* is media-types' seven kinds (doc 01 §3.4) — this
```
What is wrong: The module is presented as the demux LAYER's error module, but
13 of its 14 codes are `demux.avi.*` — it is the AVI demuxer's error module
(card 2.B2 was the AVI card) parked at the crate root. Every sibling demuxer
grew its own module (asf/error.rs, adts/error.rs, wav/error.rs, aiff/error.rs,
ogg/error.rs, es/error.rs, ps/error.rs, ts/mod.rs, mkv/error.rs, mp4/error.rs
— all with their own `mint`).
Why it matters: naming that lies — a reader expects `crate::error` to be the
shared seam and instead finds AVI-only vocabulary; the lib.rs module map
(31-33) reinforces the wrong scope.
What correct looks like: move it to `avi/error.rs` (the sibling pattern) and
let `crate::error` either disappear or hold only genuinely shared codes.
Confidence: HIGH — Verification: read + sibling-module census (10 error modules enumerated).
### F-003 [S4]
Where: crates/demux/src/error.rs:182-184
Code:
```
pub fn not_started() -> EngineError {
    mint(ErrorKind::Unsupported { format: None }, "demux.not-started")
}
```
What is wrong: The only un-prefixed code in the file. All nine sibling
modules prefix theirs (`demux.wav.not-started` at wav/error.rs:131-134,
`demux.asf.not-started`, mkv/error.rs:173, mp4/error.rs:142, ps/error.rs:103,
es/error.rs:33, ogg/error.rs:138, ts/mod.rs:169). AVI — the one demuxer using
the crate-root module — is the outlier.
Why it matters: consumers matching `demux.<container>.*` to route errors
mis-route AVI's not-started; the taxonomy is inconsistent for the same
operational condition across containers.
What correct looks like: `demux.avi.not-started` (breaking change — do it
while the error surface is still internal), or a genuinely shared
`demux.not-started` all modules use.
Confidence: HIGH — Verification: read + code census across all sibling error modules.
### F-004 [S4]
Where: crates/demux/src/error.rs:135-150
Code:
```
/// … so this constructor is currently part of the error
/// vocabulary without a minting site; it documents the ceiling's typed
/// flavor for callers that choose to fail fast.
pub fn payload_too_large(budget: u64, attempted: u64) -> EngineError {
```
What is wrong: Documented-dead public constructor (zero minting sites —
verified). Honest, but the pattern propagated: wav/error.rs:151-153 and
aiff/error.rs:139-141 copy it verbatim citing "the `avi::payload_too_large`
precedent — the typed flavor is kept for callers that choose to fail fast" —
three parallel dead constructors across the crate (ps/error.rs:91 is the one
that actually mints, at ps/mod.rs:893).
Why it matters: vocabulary that can never fire is coverage-shaped noise: each
has a test row asserting its code (error.rs:236; wav/error.rs:180;
aiff/error.rs:171), inflating the "typed refusal surface" an auditor counts.
What correct looks like: mint it or drop it — and if kept, one shared
constructor at the crate root rather than three copies.
Confidence: HIGH — Verification: read + caller census incl. wav/aiff/ps modules.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `const STAGE` | 24 | REAL | — | PipelineStage::Demux, per doc 03 §4.3(d). |
| `fn mint` | 27-29 | REAL | — | The single construction seam; no paths/URIs/payloads. |
| `not_avi` | 34-44 | REAL | — | Sniffed bytes into FormatInfo — file content by design, capped at 16 by taxonomy Display (tested at 285-291); module doc's "never … file content" (line 15-16) overstates — next sentence carves the magic-bytes exception. |
| `no_header` | 49-56 | REAL | — | Corrupt/InvalidStructure, no offset (structural absence). Minted at avi/mod.rs:1227. |
| `truncated` | 62-69 | REAL | — | expected/found u64s — byte counts only. 4 mint sites. |
| `bad_structure` | 74-83 | REAL | — | 2 mint sites. |
| `bad_stream_params` | 89-98 | REAL | — | 3 mint sites (header.rs). |
| `walk_resync_failed` | 100-118 | REAL | — | Doc honestly records the W2-07→W3-07 history and the old silent-EOS behavior it replaced. 1 mint site (mod.rs:816 — pinned by my avi part4 audit). |
| `no_supported_streams` | 123-133 | REAL | — | Unsupported with format name "avi"; empty sniffed vec. 1 mint site. |
| `payload_too_large` | 141-150 | REAL (dead) | S4 | F-004: honest unminted vocabulary; pattern copied into wav/aiff. |
| `index_too_large` | 154-163 | REAL | — | Minted at index.rs:328 (the idx1 ceiling — real). |
| `no_movi` | 169-176 | REAL | — | 3 mint sites. |
| `not_started` | 182-184 | REAL | S4 | F-003: un-prefixed code outlier. |
| `seek_out_of_range` | 188-193 | REAL | — | 4 mint sites. |
| `seek_unindexed` | 199-204 | DEAD | S3 | F-001: zero mint sites; condition unreachable by construction gate. |
| `scan_too_slow` | 212-221 | REAL | — | CpuTime LimitKind with an honest [PROPOSAL] for a ScanBudget kind; minted at index.rs:706. |
| test `errors_carry_demux_stage_and_stable_codes` | 227-257 | REAL | — | All 14 codes + stage + Display prefix/suffix shape. |
| test `kinds_map_to_the_documented_taxonomy` | 259-282 | REAL | — | Kind structure equality for 3 representative constructors. |
| test `not_avi_sniff_capped_at_sixteen_bytes_by_display` | 284-291 | REAL | — | Mutation-sensitive cap check (64 in → ≤16 rendered). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_stable_codes | YES | Exact code strings ×14 + `[demux] ` prefix + ` (code)` suffix; any rename or stage change fails. |
| kinds_map_to_the_documented_taxonomy | YES | Structural equality on kind + detail fields (expected/found, budget/attempted). |
| not_avi_sniff_capped… | YES | Uncapped Display would render 32+ "ab" pairs; assert ≤16 fails. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Typed error constructors for the demux layer" (layer-wide scope) | error.rs:1 + lib.rs:31-33 | FALSE (scope) | F-002: AVI-only vocabulary; 10 sibling error modules exist. |
| "Constructors take byte counts and offsets only — never paths, URIs or file content" | error.rs:15-16 | PARTLY-FALSE | not_avi embeds sniffed file bytes (34-44); the exception is self-documented on the next lines and Display-capped (tested) — the absolute "never" is wrong, the policy is right. |
| seek_unindexed's condition | error.rs:195-198 | FALSE (reachability) | F-001: riff.rs:231 rejects unseekable sources before any seek can run. |
| payload_too_large "without a minting site" | error.rs:138-140 | REAL | Caller census: zero sites — the doc is accurate. |

## 7. Dead / duplicated / unreachable code
- `seek_unindexed` — dead (F-001).
- `payload_too_large` — dead-by-design, duplicated into wav/aiff (F-004).
- `mint` duplicated 11× across the crate's error modules (each sibling has
  its own) — consistent house pattern, not a finding on this file alone.

## 8. Dependencies & flags
No feature gates; depends only on media-types' public error taxonomy
(boundaries respected — the doc's claim "never redefines" holds: every kind
used is media_types'). No unsafe. cfg(test) mod only for tests. Shipped via
the AVI lane.

## 9. Open questions for the next agent
- The `demux.not-started` vs `demux.<container>.not-started` split (F-003):
  does any consumer (playback-engine error routing, CLI display) actually
  string-match the prefix? If yes, AVI's outlier is a live routing bug, not
  hygiene — promote to S3.
- wav/error.rs and aiff/error.rs auditors: confirm whether YOUR
  payload_too_large is minted (the precedent you copied is not).
