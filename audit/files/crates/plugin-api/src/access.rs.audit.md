# AUDIT: crates/plugin-api/src/access.rs
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-02-14 |
| Lines / functions | 354 / 6 (2 traits, 1 default method, 2 types, 1 helper fn; 3 tests + 2 test impls) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — lumen-cli `demux_host.rs:394/422` `input::FileAccess::new()` → `impl Access for FileAccess` (input/src/file.rs:181) → `impl SourceStream for FileStream` (file.rs:207) → these traits; same traits serve the web lane (BlobSource) and streaming (RangedResource/HLS). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg (impls, callers, read_into production use); sed quote check; baseline citation |

## 1. One-paragraph verdict
A real, load-bearing contract file: the `Access`/`SourceStream` traits every byte source in the tree implements (input file, streaming ranged/HLS ×3, web blob/staged, ~15 test impls), with a genuinely useful allocation-free `read_into` default (card 3.B3) and typed errors at `PipelineStage::Open`. All 3 tests passed in the baseline (:4811-4813) and they test the real default method and the real `scheme_not_served` helper, not mocks. One latent robustness hole: the `read_into` default indexes `out[..chunk.len()]` without clamping, so any `SourceStream` impl whose `read` returns more bytes than requested panics the caller (demuxer buffer fill) — safe today only because every in-tree impl clamps and v1 plugins cannot supply streams.

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt:4811-4813: `access::tests::*` all `ok` (3 tests).
— Trait usage verified: 10+ production `impl SourceStream`/`impl Access` sites (rg list in log); production `read_into` callers include demux/mkv/ebml.rs, demux/avi/riff.rs, input/buffered.rs, streaming/ranged.rs.
— Contract honesty: `RangedResource::read` enforces a `MAX_READ_BYTES` budget with a typed `ResourceLimit` error (streaming/src/ranged.rs:857-869) — the trait's "bounded work per call" doc claim is honored by the flagship network impl.

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/access.rs:69-73
Code:
```
    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let chunk = self.read(out.len())?;
        out[..chunk.len()].copy_from_slice(&chunk);
        Ok(chunk.len())
    }
```
What is wrong: no clamp. The trait contract says `read` returns "up to `len` bytes", but the default trusts it: an impl returning `chunk.len() > out.len()` panics on the slice index (`out[..chunk.len()]`), taking down the calling thread (the demuxer's buffer fill — ebml.rs/riff.rs call this in the shipped CLI path).
Why it matters: the traits live in `plugin-api`, the extension-contract crate — the intended future suppliers of `SourceStream` impls are exactly the untrusted plugin kind. A hostile or buggy future impl turns a contract violation into a panic instead of a typed error. Today every in-tree impl clamps (BlobSource source.rs:143, RangedResource, FileStream), so the hole is latent, not live.
What correct looks like: `let n = chunk.len().min(out.len()); out[..n].copy_from_slice(&chunk[..n]); Ok(n)` — or debug_assert + truncate.
Confidence: HIGH — Verification: read (all in-tree impls checked to clamp; v1 plugin module set has no media functions, plugin-host-wasm/src/api.rs:39-43).

F-002 [S4]
Where: crates/plugin-api/src/access.rs:24-28
Code:
```
/// Called from the input worker thread (doc 01 §4.1); every method
/// must return (bounded work per call — a read of `len` bytes may
/// return fewer, never block forever; network impls use the engine's
/// timeout + reconnect budget and surface
```
What is wrong: the doc promises "network impls use the engine's timeout + reconnect budget" — the trait provides no timeout/budget parameter or hook; the claim is a promissory note about impls, enforced nowhere in this contract. Not false of the current streaming impls, but unenforceable from here (doc-as-contract drift).
Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SourceStream::uri/size/supports_seek/position | 31-35, 40, 84 | REAL | — | Accessors; honest Option<u64> for unknown size. |
| SourceStream::read | 51 | REAL | — | Contract: Ok(0) only at EOS; short reads are data. |
| SourceStream::read_into (default) | 69-73 | REAL | S3 | Alloc-copy default; unclamped index (F-001). |
| SourceStream::seek | 80 | REAL | — | Requires supports_seek agreement (honesty checked by conformance kit per doc). |
| SeekOrigin | 89-94 | REAL | — | Start(u64)/Current(i64); no End — no in-tree consumer needs it. |
| SourcePosition (+Display) | 99-105 | REAL | — | Newtype, "N bytes" display. |
| Access::accepts/open | 123-130 | REAL | — | Factory; candidates/selection is doc 06 §3.2's engine pass. |
| scheme_not_served | 134-145 | REAL | — | Typed Unsupported error naming the scheme. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mem_source_reads_seeks_and_errors | YES | Real trait impl (not a mock of production — the trait IS the unit); asserts reads, both seek origins, EOS-zero, out-of-range and before-start errors. |
| scheme_mismatch_is_a_typed_error | YES | Asserts stage Open + kind Unsupported from the real `scheme_not_served`. |
| default_read_into_reconstructs_and_tracks_position | YES | Drives the REAL default method through varied buffer sizes, position tracking, zero-length buffer, whole-payload borrow read — a mutation of the default body fails it. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Access impls live in `input` (file, disc image) and `streaming` (HTTP(S), HLS, DASH, RTSP...)" | L3-5 | SHALLOW | input: file only (no disc image impl in tree — rg shows FileAccess only); streaming: ranged + HLS yes, DASH/RTSP: none in tree. Forward-looking registry talk. |
| "a zero-length buffer is a zero-length success without I/O" (read_into) | L58-59 | REAL | Default: `read(0)` → empty chunk → copy of nothing; pinned by test L335-340. |
| "errors leave the position unchanged where the source can honor that" | L67-68 | REAL-UNPROVEN | Per-impl property; not asserted by any test in this file (in-tree impls advance only on success — spot-checked BlobSource). |

## 7. Dead / duplicated / unreachable code
None. `SeekOrigin::Current` negative handling is exercised; `SourcePosition` Display has no production consumer found (test/diagnostics only) — kept as the MP §8 newtype discipline.

## 8. Dependencies & flags
media-types only (L1 peer). No features, no unsafe, std-only. Wasm32-clean (used by apps/web).

## 9. Open questions for the next agent
- When `lumen:v1.media` gains functions (api.rs:39-43), can a plugin supply a `SourceStream` to the engine? If yes, F-001 becomes S1 (hostile impl → panic in the input worker) — re-rate then.
- Is a disc-image Access impl (doc claim, L4) tracked in any card? None exists in-tree.
