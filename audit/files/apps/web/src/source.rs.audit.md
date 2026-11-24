# AUDIT: apps/web/src/source.rs
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2025-06-27 |
| Lines / functions | 277 / 12 non-test + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web lane): worker.js postMessage `loadBlob` -> protocol.rs `HostToWorker::LoadBlob` -> host.rs:1026 `self.registry.load(&name, bytes)`; `session.open` url `blob:<name>` -> open.rs:144-178 `registry.open` -> `BlobSource`. Native release lane (lumen-cli/desktop): NO — apps/web is the wasm crate. Baseline: lumen_web 111 tests ok incl. this file's 5 (cargo-test-workspace.txt:3435-3439) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg "BlobRegistry|LoadBlob" apps/web/src` (call chain); `rg "\.read\(" crates/demux/src` -> demuxers use read_into/read_exact wrappers only; `sed -n '200,240p' crates/media-probe/src/lib.rs` (probe read bounded by max_lookahead, ceiling 1 MiB config.rs:23); `rg "loadBlob" ui/` -> only ui/e2e/web/site/harness.js (names "fixture"/"garbage"); baseline + rc-smoke citations |

## 1. One-paragraph verdict
A real, correct, in-memory byte source for the browser's File/Blob lane: bounded registry (16 entries, replacement never grows), Arc-shared bytes, honest short-read/EOF semantics, positional seeks with typed out-of-range refusals, and a borrow-based `read_into`. It is wired into the shipped web lane (LoadBlob message + blob: URI open). One real defect: `load` accepts names that `open` can never serve (any name failing the `blob:` URI grammar — e.g. containing a space), and `open` then reports the FALSE error "no blob named X is loaded" for a blob that IS loaded and was acked `blobLoaded` to the page. Two latent hazards: no read-budget parity with the native lane (native refuses >64 MiB reads typed, web serves them), and `self.pos + len` can wrap on wasm32 (usize=u32) for pathological len — currently unreachable because the probe caps reads at the 1 MiB lookahead ceiling and demuxers use `read_into`.

## 2. What actually works (verified)
— Baseline cargo-test-workspace.txt:3435-3439: `source::tests::{blob_source_read_into_fills_caller_memory, blob_source_reads_seeks_and_ends, blob_uri_parses, registry_is_bounded, registry_resolves_replaces_and_refuses}` — 5/5 ok, inside the lumen_web run (111 passed).
— Call chain verified by rg: host.rs:1026 `HostToWorker::LoadBlob { name, bytes } => match self.registry.load(&name, bytes)`; open.rs:144/:178 resolves `blob:` URIs against the registry with the "send loadBlob first" diagnostic. The e2e harness drives the whole lane in a real browser (ui/e2e/web/site/harness.js:188 loads "fixture", opens `blob:fixture`).
— Seek/EOF/read_into semantics checked against the SourceStream contract (plugin-api/src/access.rs:51): short reads are data, `Ok(0)` only at EOS (tests L193-196), `read_into(&mut [])` is Ok(0) without moving position (L218-219), seek to exactly EOF allowed, seek past EOF typed refusal at PipelineStage::Open (L205-206).

## 3. Findings
F-001 [S3]
Where: apps/web/src/source.rs:71-78 and 90-101
Code:
```
pub fn load(&mut self, name: &str, bytes: Vec<u8>) -> Result<u64, RegistryError> {
    if !self.blobs.contains_key(name) && self.blobs.len() >= MAX_BLOBS {
        return Err(RegistryError::Full);
    }
```
and
```
uri: SourceUri::parse(&format!("blob:{name}")).map_err(|_| RegistryError::Unknown {
    name: name.to_string(),
})?,
```
What is wrong: `load` performs no name validation — only the entry bound. `open` validates the name for the first time, when it builds the `blob:{name}` URI, and maps a parse failure to `RegistryError::Unknown` — whose Display (L45) is "no blob named `{name}` is loaded". So a page that sends `loadBlob { name: "my movie.avi", ... }` receives a SUCCESSFUL `blobLoaded` ack (host.rs:1026 -> registry.load -> Ok), and every subsequent `session.open` of `blob:my movie.avi` fails with the false claim that the blob was never loaded. The `blob_uri_parses` test (L265-276) pins the L0 grammar refusal for `BlobSource::new` only — the registry's load/open pair never round-trips an invalid name, so the inconsistency is invisible to the suite. Why it matters: the loadBlob message is the public web ABI for user-selected files (doc 09 §6.1/§9.2), and natural media filenames are exactly the space-bearing strings the L0 grammar rejects; an embedder following the ack gets an error message that misdirects debugging ("send loadBlob first" — it was sent). Currently latent (in-tree caller ui/e2e harness uses "fixture"/"garbage"; ui/src sends no loadBlob). What correct looks like: validate the name in `load` (one `SourceUri::parse` check) and reject at load time with a distinct typed error; keep `Unknown` for genuine misses. Confidence: HIGH — Verification: read + rg (callers enumerated).

F-002 [S4]
Where: apps/web/src/source.rs:142-147
Code:
```
fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
    let end = (self.pos + len).min(self.bytes.len());
    let out = self.bytes[self.pos..end].to_vec();
```
What is wrong: Two divergences from the native lane's bounded reader. (1) No budget: crates/input/src/buffered.rs:202-203 refuses any `read(want)` over `MAX_READ_BYTES` (64 MiB) with a typed `ResourceLimit` error; this web twin serves any length silently (allocation is incidentally bounded by the blob size via the `.min`). (2) `self.pos + len` is unchecked usize arithmetic: on wasm32 (usize = u32) a `len` near `u32::MAX` wraps, and a wrapped `end < self.pos` makes `self.bytes[self.pos..end]` panic. Reachability today: the probe's `fill_window` asks for at most `max_lookahead` (ceiling 1 MiB, media-probe/src/config.rs:23) and both demuxers read via `read_into`/`read_exact` wrappers (rg over crates/demux/src: no variable-length `.read(` in production) — so no current caller can trigger it. Why it matters: the same bytes are demuxed on the web lane as on native; a future caller (or a new demuxer using `read`) inherits a silent wrap-to-panic on wasm32 that native tests can never catch. What correct looks like: `let end = self.pos.saturating_add(len).min(self.bytes.len())` plus the native `MAX_READ_BYTES` parity check. Confidence: HIGH — Verification: read + rg (caller survey); UNVERIFIED by execution (no wasm32 panic harness available in budget).

F-003 [S4]
Where: apps/web/src/source.rs:158-160
Code:
```
let target: i64 = match origin {
    SeekOrigin::Start(offset) => offset as i64,
```
What is wrong: `offset as i64` silently truncates offsets above `i64::MAX` (u64 -> i64 cast wraps negative), which is then refused by the `target < 0` guard — the correct OUTCOME, but achieved by relying on wrap-around rather than an explicit range check. Native twin (crates/input/src/seekable.rs:74-80 `resolve_origin`) documents and rejects "before-start and overflowing positions with typed errors" deliberately. What correct looks like: `i64::try_from(offset).map_err(...)` — same refusal, no cast subtlety. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory, file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_BLOBS` | 29 | REAL | — | 16; bound tested |
| `RegistryError` + Display | 33-48 | REAL | S3 | F-001: `Unknown` doubles as the invalid-name error with a false message |
| `BlobRegistry::new` | 60-64 | REAL | — | |
| `BlobRegistry::load` | 71-78 | REAL | S3 | F-001: no name validation; bound logic correct (replacement never grows) |
| `BlobRegistry::size_of` | 82-84 | REAL | — | |
| `BlobRegistry::open` | 90-101 | REAL | S3 | F-001: URI-parse failure misreported as Unknown |
| `BlobSource::new` | 120-126 | REAL | — | typed EngineError on bad names (tested) |
| `SourceStream::uri/size/supports_seek` | 130-140 | REAL | — | size always known, always seekable |
| `SourceStream::read` | 142-147 | REAL | S4 | F-002: unchecked add, no budget parity |
| `SourceStream::read_into` | 149-155 | REAL | — | partial fill at EOS, count returned; no overflow (out is caller-owned) |
| `SourceStream::seek` | 157-173 | REAL | S4 | F-003 cast; range check + typed refusal correct |
| `SourceStream::position` | 175-177 | REAL | — | |
| 5 tests | 187-276 | REAL | — | honest; gap: no load/open round-trip of an invalid name (F-001 blind spot) |

## 5. Test honesty (if tests exist)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| blob_source_reads_seeks_and_ends | YES | exact bytes, EOS zero, both seek origins, typed out-of-range with stage assert |
| blob_source_read_into_fills_caller_memory | YES | exact fill counts incl. zero-length and partial-at-EOS |
| registry_resolves_replaces_and_refuse | YES | replace shrinks reported size; Unknown message asserted |
| registry_is_bounded | YES | 16 ok / 17th refused / replace-at-bound ok |
| blob_uri_parses | YES (partially blind) | pins L0 space refusal for `BlobSource::new` — but never exercises `load`+`open` with a bad name (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a later same-name load REPLACES the bytes" | L13-14 | REAL | load L71-77 + test L234-235 |
| "at most MAX_BLOBS entries … typed refusal, never an unbounded map" | L12,26-29 | REAL | L72-74 + registry_is_bounded test |
| "zero copies per read beyond the demuxer's own buffer fill" | L9-10 | REAL (for the demux path) | demuxers use read_into (rg); `read()`'s to_vec is used by the probe window only (≤1 MiB) |
| "Bytes are retained for the session's life" | L15-16 | REAL | no eviction API exists; replacement only |

## 7. Dead / duplicated / unreachable code
None dead. `BlobSource::new` is production (registry.open constructs via struct literal, tests via new) — slight asymmetry: `open` builds the struct directly, duplicating `new`'s URI construction (S4-level duplication, folded into F-001's fix).

## 8. Dependencies & flags
media-types (EngineError/PipelineStage/SourceUri), plugin-api (SourceStream/SeekOrigin/SourcePosition), std BTreeMap/Arc. No feature gates. No unsafe. wasm32-safe except F-002's usize arithmetic.

## 9. Open questions for the next agent
- Does the UI product surface (ui/src) plan to use loadBlob for local files? If yes, F-001 becomes user-visible on day one — fix before that lane ships.
