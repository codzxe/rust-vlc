# AUDIT: crates/demux/src/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Lines / functions | 293 / 18 (15 production incl. private `mint`, 3 tests) |
| File verdict | REAL (2 of 15 constructors are dead vocabulary) |
| Reachable from shipped entry point? | YES — apps/cli containers/mod.rs:55 `"avi" => avi::open` → `AviDemuxer::new`/`parse_container` → `error::*` at avi/mod.rs:1116-1243, header.rs:268/290/570/522+, riff.rs:331, index.rs:328/706/733 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `cargo test -p demux --lib error::tests` (exit 0; error::tests 3/3 ok among 21 matched); rg caller sweep per constructor across crates/ + apps/ (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
This is the AVI demuxer's error module parked at the crate root under a
"demux layer" name: 13 of 15 constructors mint `demux.avi.*` codes, and
every other container (adts, wav, aiff, ogg, ps, mp4, mkv, asf, es) has its
own error submodule (verified adts/error.rs:114-208 mints `demux.adts.*`).
The live constructors are genuinely wired (call sites listed in §7), the
tests are real and executed, and the no-secrets claim holds (no constructor
takes a path/URI/content; sniffed bytes capped by media-types Display at
media-types/src/error.rs:321-324). Two constructors are dead vocabulary:
`payload_too_large` (doc itself admits "without a minting site") and
`seek_unindexed` (zero callers repo-wide; the case it describes is
structurally foreclosed because `AviDemuxer::new` rejects unseekable
sources). Both are green-checked by a test that mints them directly — the
test proves the constructor works, not that the code path exists.

## 2. What actually works (verified)
— `cargo test -p demux --lib error::tests` → `error::tests::errors_carry_demux_stage_and_stable_codes`, `kinds_map_to_the_documented_taxonomy`, `not_avi_sniff_capped_at_sixteen_bytes_by_display` all ok (3 passed).
— rg sweep: 13/15 constructors have production call sites (§7 table); sticky-error replay clones structured stage/kind/context, no string parsing (avi/mod.rs:170-180).
— Doc claim "seven kinds (doc 01 §3.4)" verified: `ErrorKind` has exactly 7 variants (media-types/src/error.rs:190-214).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/error.rs:135-150
Code: (verbatim)
```
/// A `movi` chunk payload exceeds the payload ceiling (MP §12). The
/// v0 demuxer recovers positionally instead — the oversized chunk is
/// skipped and counted as damage (`chunk-payload-over-ceiling`), never
/// read — so this constructor is currently part of the error
/// vocabulary without a minting site; it documents the ceiling's typed
/// flavor for callers that choose to fail fast.
pub fn payload_too_large(budget: u64, attempted: u64) -> EngineError {
```
What is wrong: zero production callers repo-wide (rg `payload_too_large` over crates/ + apps/: only this file, its test, and the wav/aiff/ps clones). The oversized-chunk path never fails — it recovers via `self.damage.note(size, "chunk-payload-over-ceiling")` (avi/mod.rs:518). Why it matters: the test row `(payload_too_large(100, 200), "demux.avi.payload-too-large")` (error.rs:236) pins a green checkmark onto vocabulary no code path can emit; a consumer matching on this code will never see it. The same dead pattern is copy-paste replicated in wav/error.rs:153 and aiff/error.rs:141 (ps/error.rs:91 is live at ps/mod.rs:893). What correct looks like: delete it, or wire it as the fail-fast option the doc hand-waves at. Confidence: HIGH — Verification: ran (rg sweep).

F-002 [S3]
Where: crates/demux/src/error.rs:195-204
Code: (verbatim)
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
What is wrong: zero callers anywhere in the repo (rg `seek_unindexed` / `demux.avi.seek.unindexed`: only definition + its own test row at error.rs:245). The documented case cannot arise: `AviDemuxer::new` rejects "unknown length, unseekable, empty" sources (avi/mod.rs:352-361) and `seekability()` therefore only ever reports `Fast{indexed:true}` or `SlowLinear` (avi/mod.rs:956-968, whose comment admits "The constructor only accepts seekable, length-known sources, so the scan fallback is always possible"). Why it matters: dead error vocabulary for a structurally impossible state, indistinguishable in the code table from the live codes. What correct looks like: remove it, or make it the constructor-rejection error if unseekable seek ever becomes reachable. Confidence: HIGH — Verification: ran (rg sweep + read of avi/mod.rs).

F-003 [S3]
Where: crates/demux/src/error.rs:1 and 178-184 (against lib.rs:47 `pub mod error`)
Code: (verbatim)
```
//! Typed error constructors for the demux layer (card 2.B2).
```
```
/// A trait method was called before `start` opened the container (or
/// after a constructor rejection): the demuxer is not in a state that
/// supports the operation. Operational shape, not container data — the
/// caller sees a stable code instead of a panic.
pub fn not_started() -> EngineError {
    mint(ErrorKind::Unsupported { format: None }, "demux.not-started")
}
```
What is wrong: the module claims to be "the demux layer"'s error module but is used exclusively by avi/{mod,header,riff,index,tests}.rs; 13/15 codes are `demux.avi.*`. `not_started` is the one constructor that skips the avi namespace ("demux.not-started" vs adts's "demux.adts.not-started" at adts/error.rs:195-199) yet is called only from avi/mod.rs:922,1035 — an unnamespaced code minted by an avi-only module. Why it matters: names that lie; a consumer cannot infer from the code that this is an AVI refusal, and the crate-root module name invites other demuxers to reuse it (they don't — they all cloned their own). What correct looks like: move to avi/error.rs (matching every sibling container) or rename codes consistently. Confidence: HIGH — Verification: read + rg.

F-004 [S4]
Where: crates/demux/src/error.rs:206-221
Code: (verbatim)
```
/// is the closest L0 vocabulary for a parse-work budget
/// `[PROPOSAL: a dedicated ScanBudget LimitKind is an L0 addition for
/// the Lead if this fires in practice]`.
pub fn scan_too_slow(budget_bytes: u64, attempted_bytes: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::CpuTime,
```
What is wrong: `LimitKind::CpuTime` carrying byte-denominated budget/attempted — a structured-payload unit lie (Display renders bare numbers, media-types/src/error.rs:465 pattern "budget 16777216"). The doc admits the mismatch; no `ScanBudget` kind exists. Live (index.rs:706) so this is a taxonomy wart, not dead code. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/demux/src/error.rs:27-29 (against media-types/src/error.rs:224,240)
Code: (verbatim)
```
/// Mint a stage-`Demux` error with a stable dotted code.
fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}
```
What is wrong: `ErrorContext::new` stores `code: String` (media-types/src/error.rs:224,240) — every mint heap-allocates a copy of a `&'static str`. Trivial per-error cost, but it is pure overhead on every typed failure in the AVI open/seek paths. Confidence: HIGH — Verification: read (media-types/src/error.rs:223-241).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint (private) | 27-29 | REAL | — | Single choke point; String alloc per mint (F-005) |
| not_avi | 34-44 | REAL | — | avi/mod.rs:1118,1121; sniffed bytes copied (cap downstream, tested) |
| no_header | 49-56 | REAL | — | avi/mod.rs:1227 |
| truncated | 62-69 | REAL | — | 20+ sites: riff.rs:331, header.rs:522-793, mod.rs:472,814,1116-1241 |
| bad_structure | 74-83 | REAL | — | mod.rs:677, index.rs:733 |
| bad_stream_params | 89-98 | REAL | — | header.rs:268,290,570 |
| walk_resync_failed | 109-118 | REAL | — | mod.rs:816 |
| no_supported_streams | 123-133 | REAL | — | mod.rs:1243 |
| payload_too_large | 141-150 | UNREACHABLE | S3 | F-001: zero minting sites |
| index_too_large | 154-163 | REAL | — | index.rs:328 (over-cap idx1 fatal — probed EXECUTED in W04-A06 file 3-4) |
| no_movi | 169-176 | REAL | — | mod.rs:443,447,1237; tests.rs:2646 |
| not_started | 182-184 | REAL | — | mod.rs:922,1035; unnamespaced code (F-003) |
| seek_out_of_range | 188-193 | REAL | — | mod.rs:1041-1087 (9 sites) |
| seek_unindexed | 199-204 | UNREACHABLE | S3 | F-002: zero callers, case foreclosed |
| scan_too_slow | 212-221 | REAL | S4 | index.rs:706; unit mismatch F-004 |
| tests: errors_carry_demux_stage_and_stable_codes | 227-257 | REAL | — | Fails on any code/stage/Display regression; also mints the 2 dead codes green (F-001/F-002) |
| tests: kinds_map_to_the_documented_taxonomy | 259-282 | REAL | — | Structural kind assertions |
| tests: not_avi_sniff_capped_at_sixteen_bytes_by_display | 284-291 | REAL | — | Boundary dup of media-types' own cap test; would fail if cap removed |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_stable_codes | YES | Asserts stage, exact code strings, Display prefix/suffix per constructor |
| kinds_map_to_the_documented_taxonomy | YES | assert_eq on kind structure incl. Truncated/ResourceLimit fields |
| not_avi_sniff_capped_at_sixteen_bytes_by_display | YES | 64×0xAB input; >16 "ab" matches fail — but the cap lives in media-types, whose own test (media-types/src/error.rs:490-504) already covers it |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Typed error constructors for the demux layer" | error.rs:1 | FALSE (as scoped) | Only avi/* uses this module; every sibling container has its own error submodule (adts/error.rs:114-208 etc.) — F-003 |
| "taxonomy is media-types' seven kinds" | error.rs:3 | REAL | media-types/src/error.rs:190-214: exactly 7 variants |
| "Constructors take byte counts and offsets only — never paths, URIs or file content" | error.rs:14-16 | REAL | All 15 signatures read; only u64/None/&[u8] sniffed head |
| "Sniffed magic bytes are capped ... 16 bytes of hex" | error.rs:16-17 | REAL | media-types/src/error.rs:321-324 `&info.sniffed[..min(16)]`; boundary test executed |
| payload_too_large "documents the ceiling's typed flavor for callers that choose to fail fast" | error.rs:139-141 | PAPER-ONLY | No such caller exists (F-001) |
| seek_unindexed's unseekable+no-idx1 case | error.rs:195-198 | UNREACHABLE | AviDemuxer::new rejects unseekable sources (avi/mod.rs:352-361); seekability() never reports it (F-002) |

## 7. Dead / duplicated / unreachable code
- DEAD: `payload_too_large` (F-001), `seek_unindexed` (F-002).
- DUPLICATED: the constructor-set shape (not_avi/no_header/truncated/not_started/seek_out_of_range/payload_too_large + cap test) is copy-paste cloned per container: adts/error.rs, wav/error.rs, aiff/error.rs, ogg/error.rs, ps/error.rs (each audited separately by other agents).
- Everything else is live from the AVI open/seek path (call sites in §4).

## 8. Dependencies & flags
- Depends only on media-types (EngineError, ErrorKind, PipelineStage, CorruptDetail, ErrorContext, FormatInfo, LimitKind). No feature gates (`#[cfg(test)]` only). No unsafe.

## 9. Open questions for the next agent
- If L0 ever adds `LimitKind::ScanBudget` (the PROPOSAL in F-004), `scan_too_slow` must be retagged; consumers matching `CpuTime` today would silently break.
- wav/error.rs:153 and aiff/error.rs:141 `payload_too_large` clones: verify whether those are equally unminted in their owner audits (wav confirmed dead by rg here; aiff likewise only self+test hits).
