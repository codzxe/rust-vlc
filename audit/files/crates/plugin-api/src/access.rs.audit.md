# AUDIT: crates/plugin-api/src/access.rs
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-21 |
| Lines / functions | 354 / 12 items (2 traits, 1 default method, 2 types + Display, 1 helper fn, 2 test types, 3 tests) |
| File verdict | REAL (the trait vocabulary of the shipped CLI's open path) — with one PAPER-ONLY enforcement claim in its docs |
| Reachable from shipped entry point? | YES. Chain: `lumen` CLI → `demux_host::open_source` (apps/cli/src/demux_host.rs:391–425) → `input::FileAccess::new().open(&uri)` = `plugin_api::Access::open` (this file, line 130, via `impl Access for FileAccess` input/src/file.rs:181) → `Box<dyn SourceStream>` (FileStream, file.rs:207) → `media_probe::probe_stream(&mut dyn SourceStream)` + demuxer. HTTP lane: `streaming::RangedResource` (ranged.rs:844). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:1 |
| Commands run | rg `impl (Access\|SourceStream) for` repo-wide (30 implementors mapped: input FileStream/FileAccess, streaming RangedResource + 3 HLS streams, apps/web BlobSource/StagedSource, test doubles); rg `plugin-api\|input` apps/cli/Cargo.toml + rg `FileAccess` demux_host.rs (reachability chain); rg `fn read_into` (overriders: file.rs:229, ranged.rs:877, source.rs:149, staged_source.rs:808 — HLS streams use the DEFAULT); rg `ConformanceRunner` repo-wide (docs/06 + docs/01 ONLY — zero code); rg `conformance` crates/plugin-api (none); rg `ADR-008` (no docs/adr/ADR-008 exists; disc-dvd-video.md:52-53 admits "ADR-008 is not [written]"); ls crates/input/src (no disc module); rg `rtsp` streaming/src/lib.rs:10 ("lands in later cards") + schemes.rs (typed refusal); sed file.rs 181–260. |

## 1. One-paragraph verdict
This is the crate where "plugin-api" stops meaning plugins and starts meaning the engine: `Access`/`SourceStream` are the traits the shipped `lumen` CLI's file-open path is written against (verified call chain above), with real implementors in `input`, `streaming`, and `apps/web`. The trait contracts are precise and the tests are honest. Two defects: the `read_into` default method panics if an implementation's `read` returns more bytes than requested — a live exposure, since the three HLS streams use the default — and the doc comment claims a conformance kit "checks" capability honesty that exists only as prose in two design docs, with zero code in the tree.

## 2. What actually works (verified)
- Traits are the shipped vocabulary: `lumen` open → `Access::open` (demux_host.rs:394/422) → `SourceStream` consumed by `probe_stream` (media-probe/src/lib.rs:195) and the demuxer. Baseline #20 (rc-smoke, AVI open lifecycle PASS) exercises this chain end to end through `FileAccess`'s twin path.
- The `read_into` default's happy contract is genuinely tested: `default_read_into_reconstructs_and_tracks_position` (302–353) reconstructs the payload through a 3-byte scratch loop, tracks position, and pins zero-length-buffer semantics.
- `scheme_not_served` (134–145) is used by the real `FileAccess::open` refusal (input/src/file.rs:188) — not dead helper code.
- All three tests assert real behavior and would fail on mutation (§5).

## 3. Findings

### F-001 [S3] — `read_into` default panics on an over-returning `read`
Where: crates/plugin-api/src/access.rs:69-73
Code:
```
    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let chunk = self.read(out.len())?;
        out[..chunk.len()].copy_from_slice(&chunk);
        Ok(chunk.len())
    }
```
What is wrong: The default trusts `read(out.len())` to return at most `out.len()` bytes. An implementation that ignores or mishandles the bound (returns a longer Vec) makes `out[..chunk.len()]` panic ("range end index … out of range") inside library code. This is not hypothetical scaffolding: `HlsVodStream`, `HlsAdaptiveStream`, `HlsLiveStream` (streaming/src/hls/{media,master,live}.rs:401/639/609) implement `SourceStream` WITHOUT overriding `read_into`, so they run this default on the network-streaming lane; the trait is also the declared seam for third-party plugins (doc 06), where a misbehaving guest should be a typed error, never a host panic (doc 06 §7.3's own "never panics" rule).
Why it matters: One buggy HLS read path (or one hostile plugin) turns a contract violation into a panic in the input worker instead of a typed `EngineError` — exactly the failure class the crate's own error philosophy (MP §8) forbids.
What correct looks like: `let n = chunk.len().min(out.len()); out[..n].copy_from_slice(&chunk[..n]); Ok(n)` — or an explicit typed error on over-return, since over-return is itself a contract breach worth surfacing.
Confidence: HIGH — Verification: read + rg of overriders (HLS lanes use the default).

### F-002 [S3] — "the conformance kit checks this honesty" — the kit does not exist
Where: crates/plugin-api/src/access.rs:37-39
Code:
```
    /// Whether positional seeks are served (§2.2 access capability
    /// `supports_seek` — must agree with the manifest claim; the
    /// conformance kit checks this honesty, doc 06 §7.2).
```
What is wrong: Doc 06 §7 says the kit "is implemented in `crates/testkit` (`ConformanceRunner`, doc 01 §2.8)". `rg ConformanceRunner` over the entire repo matches exactly two files: `docs/06-plugin-architecture.md` and `docs/01-architecture.md` — there is no `ConformanceRunner`, no conformance module, no capability-honesty check anywhere in code (crates/testkit/src = alloc_count.rs, faults.rs, lib.rs; crates/plugin-api/tests = doc06_examples.rs, testkit_wiring.rs — neither exercises a manifest-vs-behavior cross-check). The doc comment in this shipped trait file asserts an enforcement mechanism that is prose-only.
Why it matters: A plugin author reading the trait contract believes `supports_seek`/manifest agreement is machine-checked; it is checked by nothing. This is the same evidence-inflation pattern the wave is chartered to expose (docs claiming gates that don't exist), planted inside real code comments.
What correct looks like: Either implement the §7.2 check (testkit runner that opens every manifest-claimed scheme and diffs `supports_seek` against the claim) or change the comment to "must agree with the manifest claim (enforcement planned, doc 06 §7.2 — not yet implemented)".
Confidence: HIGH — Verification: repo-wide rg (two doc files only).

### F-003 [S4] — Module doc names impls and an ADR that do not exist
Where: crates/plugin-api/src/access.rs:3-4
Code:
```
//! `Access` impls live in `input` (file, disc image — ADR-008) and
//! `streaming` (HTTP(S), HLS, DASH, RTSP, with buffering/reconnect
```
What is wrong: (a) No disc-image access impl exists — `crates/input/src/` is buffered/contract/error/file/lib/seekable/uri, and the repo itself admits "ADR-008 is not [written]" (docs/formats/disc-dvd-video.md:52–53, and no docs/adr/ADR-008*.md exists). (b) RTSP has no impl — streaming/src/lib.rs:10 says "RTSP (6.G) — lands in later cards", and schemes.rs refuses `rtsp://` typed. HTTP(S)/HLS/DASH are real (ranged.rs, hls/, dash/).
Why it matters: The comment is the crate map for implementors; two of its six named things are fictional. Doc-geography inflation.
What correct looks like: "`input` (file) and `streaming` (HTTP(S) ranged, HLS, DASH; RTSP pending card 6.G)".
Confidence: HIGH — Verification: ls input/src + rg ADR-008 + rg rtsp.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-7 | SHALLOW | F-003 | Geography inflated (disc image, RTSP). |
| SourceStream::uri | 31 | REAL | — | Trivial accessor. |
| SourceStream::size | 35 | REAL | — | Option-typed absence (MP §8) ✓. |
| SourceStream::supports_seek | 40 | REAL | F-002 | Honesty claim unenforced (kit is docs-only). |
| SourceStream::read | 51 | REAL | — | Contract: Ok(0) only at EOS; short reads are data. |
| SourceStream::read_into (default) | 69-73 | REAL | F-001 | Panic on over-return; used by 3 HLS impls. |
| SourceStream::seek / position | 80 / 84 | REAL | — | Positional-by-definition contract. |
| SeekOrigin (enum) | 88-94 | REAL | — | Start(u64)/Current(i64). |
| SourcePosition (+Display) | 98-105 | REAL | — | Newtype; Display "N bytes". |
| Access::accepts / open | 123 / 130 | REAL | — | Factory split; called by CLI open (demux_host.rs:394). |
| scheme_not_served | 134-145 | REAL | — | Used by FileAccess (file.rs:188). |
| tests: MemSource | 153-211 | REAL | — | Test double; `pos + len` can overflow-panic on `read(usize::MAX)` — test-only, no caller does. |
| tests: MemAccess | 213-229 | REAL | — | — |
| tests: mem_source_reads_seeks_and_errors | 231-276 | REAL | — | Read/seek/EOS/short-read/both error arms. |
| tests: scheme_mismatch_is_a_typed_error | 278-294 | REAL | — | Stage+kind pinned. |
| tests: default_read_into_reconstructs_and_tracks_position | 301-353 | REAL | — | Good coverage of the default's happy contract; cannot see F-001 (needs a misbehaving impl). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| mem_source_reads_seeks_and_errors | YES | Byte-level asserts on read framing, seek targets, EOS-zero, out-of-range and before-start errors. |
| scheme_mismatch_is_a_typed_error | YES | Wrong stage/kind fails it. |
| default_read_into_reconstructs_and_tracks_position | YES | Reconstruction, position tracking, zero-buffer, and single-big-read asserts all bind to the default's behavior. |
| (oversized-read contract) | NO test exists | F-001 — the default's panic path has no adversarial test. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Access impls live in input (file, disc image — ADR-008) and streaming (HTTP(S), HLS, DASH, RTSP…)" | access.rs:3-4 | FALSE (partial) | input has file only; no ADR-008; RTSP unimplemented (F-003). |
| "the conformance kit checks this honesty, doc 06 §7.2" | access.rs:38-39 | PAPER-ONLY | ConformanceRunner exists only in docs/06 + docs/01 (F-002). |
| "conformance kit … implemented in crates/testkit (ConformanceRunner, doc 01 §2.8)" | docs/06:826-827 | PAPER-ONLY | testkit/src = alloc_count, faults, lib — no runner. |
| read_into "the borrow twin of read … minus the per-call Vec allocation" | access.rs:53-63 | REAL | Default allocates once + copies (69–73); FileStream/BlobSource/RangedResource override with direct copies (rg). |

## 7. Dead / duplicated / unreachable code
No dead code in this file. Cross-file note: the trait family is named "plugin-api" but its consumers are the engine's own open path — the crate is the shared pipeline vocabulary (30 implementors, overwhelmingly in-tree), not a plugin-only surface; no wasm plugin implements Access today (plugin-host-wasm has no SourceStream bridge — rg).

## 8. Dependencies & flags
`media-types` only (SourceUri, EngineError, PipelineStage) + std::fmt. No features, no unsafe. Clean layering for an L1 vocabulary crate.

## 9. Open questions for the next agent
- Whoever audits `streaming/src/hls/*`: confirm each HLS `read` honors `len` (F-001's live exposure), and consider overriding `read_into` there anyway.
- Wave gate: fold F-002 into the plugin-domain headline — the conformance kit (doc 06 §7, "mandatory" per MP §6) has zero code; combined with no shipped binary linking plugin-host-wasm (prior Cargo.toml audits), the entire plugin enforcement story is documentation.
