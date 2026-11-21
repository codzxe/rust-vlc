# AUDIT: apps/web/src/source.rs
| Field | Value |
|---|---|
| Auditor | W12-A01 |
| Date | 2026-11-21 |
| Lines / functions | 277 / 16 (1 const, 1 enum + Display, 1 registry impl of 4 methods, 1 source impl of 8 trait methods + constructor, 5 tests) |
| File verdict | REAL (executed on the REAL first-vertical fixture in baseline via the host open path) |
| Reachable from shipped entry point? | NO from `lumen`/`lumen-tauri` (lumen-web unshipped — see mock_fetch.rs audit). YES within the web lane, executed: `host.rs:1026` (`LoadBlob` → `registry.load`) and `open.rs:226` (`registry.open` → `BlobSource` → probe → AVI demuxer) — the baseline-passing `host::tests::pump_slices_play_the_fixture_to_its_end` (orchrun log :3119) loads `generated/valid/avi_mjpeg_pcm.avi` through exactly this path (host.rs:1999-2008), and 5/5 unit tests pass (:3195-3199). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `Read` full file; `Grep BlobRegistry\|BlobSource\|LoadBlob` (consumers: host.rs:108/789/836/1026/1494, open.rs:35/144/214/226, lib.rs:76 re-export, protocol.rs:69/265); `sed -n` host.rs:1026-1037 (LoadBlob handler — no size gate), open.rs:155-235 (blob URL resolution — the production gate that masks F-002's lying error), abi.rs:91/156 (MAX_INPUT_BYTES = 32 MiB — the real byte bound, lives in another layer); read of the callers' read patterns: media-probe/src/lib.rs:215-233 (`fill_window`, `want ≤ max`), demux/src/avi/riff.rs:268-278 (`read_into` with `step ≤ READ_STEP_MAX`), plugin-api/src/access.rs:43-69 (trait contract — `len` unbounded); baseline log greps :3119, :3195-3199. |

## 1. One-paragraph verdict
A small, genuinely exercised file: the blob registry and its seekable shared-memory source are the ONLY input lane the web host actually opens media through, and the baseline proves it end-to-end on the real AVI fixture (LoadBlob → blob: URL → probe → demux → play to end). The seek semantics match the input crate's (one-past-end legal, before-start and past-end typed, saturating for `Current`), `read_into` is overflow-proof by construction, and the registry bound is honest about replacement-not-growth. Two defects survive scrutiny: `read()` does plain `pos + len` — a usize overflow on the plugin-facing trait seam that panics the slice on wasm32 (the crate's real target) with any caller-supplied huge length, where the codebase's own convention (`saturating_add`, used 40 lines down in `seek`) is the one-word fix; and `load()` accepts names the `blob:` URI grammar can never express — a user's file called "my movie.avi" acks `BlobLoaded` and then refuses at open with a URI error, while a direct `registry.open` of the same name answers a flat lie ("no blob named … is loaded").

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt:3119` → `host::tests::pump_slices_play_the_fixture_to_its_end ... ok`: the real fixture enters through `registry.load` (host.rs:1026, fed by host.rs:1999-2002) and is demuxed through `BlobSource` (open.rs:226) — this file's registry and stream run in the lane proof, not just unit tests.
— verified by baseline `:3195-3199` → 5/5 `source::tests` pass (trait contract incl. short-read/EOS/zero-at-EOS, read_into partial fills and zero-length, registry resolve/replace/refuse, 16-entry bound with replacement-at-bound, URI grammar pin).
— verified by read: seek semantics are the input crate's — `target < 0 || target > len` typed refusal with the same `Inaccessible/NotFound` kind the native ranged lane uses for `input.seek.overflow` (ranged.rs:161-167), one-past-end legal, `Current` via `saturating_add` (:160), and `Start(offset as i64)` truncation for offset > i64::MAX lands in the `target < 0` refusal (benign: out of range either way).
— verified by read: `read_into` (:149-155) cannot overflow — `pos ≤ bytes.len() ≤ isize::MAX` and `out.len() ≤ isize::MAX`, sum < usize::MAX on both 32- and 64-bit targets; zero-length buffer is a zero-copy no-op (test :218).
— verified by read: the "shared, reference-counted storage" claim (:9) is real — `Arc<[u8]>` per blob, `Arc::clone` on open (:98), no byte copy until the demuxer's own `read_into` fill (the AVI demuxer uses only `read_into`, riff.rs:272/329/362 — verified).

## 3. Findings

### F-001 [S3]
Where: apps/web/src/source.rs:142-147
Code:
```
    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let end = (self.pos + len).min(self.bytes.len());
        let out = self.bytes[self.pos..end].to_vec();
```
What is wrong: `self.pos + len` is plain usize addition. On overflow it panics in debug and WRAPS in release; a wrapped sum smaller than `self.pos` makes `end < self.pos` and `&self.bytes[self.pos..end]` panics ("slice index starts at … but ends at …"). Concrete wasm32 case (this crate's real target, `usize = u32`): `pos = 5`, `len = 0xFFFF_FFFF` → sum wraps to 4 → `bytes[5..4]` → panic. The trait is the PUBLIC plugin seam (`plugin_api::SourceStream::read`, access.rs:51) whose contract places no bound on `len` — the probe's own defensive-clamp comment (media-probe lib.rs:231-233) shows the codebase expects sources to survive contract-violating traffic.
Why it matters: the in-tree callers are safe today (verified: the probe asks `want ≤ max` in `fill_window`, the AVI demuxer uses only `read_into` with `step ≤ READ_STEP_MAX`), but any demuxer plugin (the wasmi lane this wave exists for) passing a raw parsed u32/u64 size — the obvious first implementation — kills the whole Worker with a panic instead of answering the typed `EngineError` the trait promises. The fix is one word and already used 17 lines below in `seek`'s `Current` arm (:160, `saturating_add`).
What correct looks like: `let end = self.pos.saturating_add(len).min(self.bytes.len());` (then `end ≥ pos` always, `Ok(empty)` only at EOS — contract preserved).
Confidence: HIGH — Verification: read (arithmetic traced on the u32 wrap case); no run (panic path needs a hostile caller; no in-tree caller reaches it — caller survey above).

### F-002 [S3]
Where: apps/web/src/source.rs:71-78 vs :90-101 (production gate: open.rs:163-179)
Code:
```
    pub fn load(&mut self, name: &str, bytes: Vec<u8>) -> Result<u64, RegistryError> {
        if !self.blobs.contains_key(name) && self.blobs.len() >= MAX_BLOBS {
            return Err(RegistryError::Full);
        }
        let size = bytes.len() as u64;
        self.blobs.insert(name.to_string(), Arc::from(bytes));
        Ok(size)
    }
```
(+ :94-97:)
```
        Ok(BlobSource {
            uri: SourceUri::parse(&format!("blob:{name}")).map_err(|_| RegistryError::Unknown {
                name: name.to_string(),
            })?,
```
What is wrong: `load` validates NOTHING about the name; `open` requires the name to survive `SourceUri::parse("blob:{name}")`. The unit test itself pins that a space fails the grammar (`uri.control_chars`, :271-275). So `load("my movie.avi", bytes)` → `Ok` → host acks `BlobLoaded` success (host.rs:1027-1029) → a later `open` of `blob:my movie.avi` refuses at `SourceUri::parse` (open.rs:170-175, "the blob URL was rejected"). And a direct `registry.open("my movie.avi")` answers `RegistryError::Unknown` — "no blob named `my movie.avi` is loaded" — which is false: it IS loaded; the name is merely unparseable.
Why it matters: spaces are the norm in real media filenames ("My Movie (2024) 1080p.mkv"); a page that passes `file.name` through as the blob name gets a success ack followed by an open refusal, and the registry slot is consumed by an eternally unopenable entry. The production URL gate (open.rs) at least refuses with an honest sentence; the registry's own error is a lie about state.
What correct looks like: `load` refuses (typed, e.g. a new `RegistryError::InvalidName`) when `SourceUri::parse(&format!("blob:{name}"))` fails — fail at the boundary where the bad data enters, and keep `Unknown` for genuinely-absent names.
Confidence: HIGH — Verification: read of load/open/parse chain + the file's own test :264-276; the success-then-refuse flow traced through host.rs:1026-1037 and open.rs:163-179; not executed end-to-end (no test loads a spacey name through the host — gap noted in §5).

### F-003 [S4]
Where: apps/web/src/source.rs:26-29
Code:
```
/// The registry bound (bounded memory, MP §11 — a host that loads
/// more distinct blobs than this gets the typed refusal, never an
/// unbounded map).
pub const MAX_BLOBS: usize = 16;
```
What is wrong: the bound is ENTRY COUNT, not memory. Nothing in this file limits blob bytes; the real byte ceiling (32 MiB per message, `abi.rs:91 MAX_INPUT_BYTES`, enforced at abi.rs:156) lives two layers away and is not referenced here. Registry memory is therefore bounded at ~16 × 32 MiB only by cross-layer coincidence, and a non-ABI embedder (test, future host) gets no byte bound at all.
Why it matters: the "bounded memory, MP §11" sentence reads as a property of THIS layer; it is a property of the ABI lane. If the ABI bound moves (or a second entry path appears), the claim silently rots.
What correct looks like: either a byte budget checked in `load` (`sum + bytes.len() > BLOB_MEMORY_BUDGET → typed refusal`) or a doc sentence naming the ABI bound as the real ceiling.
Confidence: HIGH — Verification: read of this file + abi.rs:91/156 + host.rs:1026-1037 (handler adds no size gate).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_BLOBS` (const) | 26-29 | REAL | S4 | Entry-count bound only (F-003). 16 is a product choice; replacement-at-bound pinned by test. |
| `RegistryError` (enum) + `Display` | 31-48 | REAL | S4 | `Unknown`'s sentence is state-false for URI-unparseable names (F-002). |
| `BlobRegistry` (struct) | 50-55 | REAL | — | `BTreeMap<String, Arc<[u8]>>` — deterministic order, refcounted bytes. |
| `BlobRegistry::new` | 57-64 | REAL | — | Default delegate. |
| `BlobRegistry::load` | 66-78 | REAL | S3 | No name validation (F-002); bound check correct (replacement never grows); `Arc::from(Vec)` moves. |
| `BlobRegistry::size_of` | 80-84 | REAL | — | Used by open.rs:168 (the resolve gate). |
| `BlobRegistry::open` | 86-101 | REAL | S3 | URI-parse failure mislabeled `Unknown` (F-002); `Arc::clone` — zero-copy handoff. |
| `BlobSource` (struct) | 104-111 | REAL | — | `pos: usize` — on wasm32 caps blobs at 4 GiB, consistent with the ABI's 32 MiB. |
| `BlobSource::new` | 113-127 | REAL | — | Typed URI refusal (test-pinned); used by tests; registry is the production entry. |
| `SourceStream::uri` | 130-132 | REAL | — | Trivial. |
| `SourceStream::size` | 134-136 | REAL | — | Always `Some` — in-memory truth. |
| `SourceStream::supports_seek` | 138-140 | REAL | — | Always true — honest (full random access). |
| `SourceStream::read` | 142-147 | REAL | S3 | Overflow→slice-panic path on hostile `len` (F-001); otherwise correct short-read/EOS semantics. |
| `SourceStream::read_into` | 149-155 | REAL | — | Overflow-proof by construction; zero-copy fill; partial at EOS; the demuxer's actual path. |
| `SourceStream::seek` | 157-173 | REAL | — | Saturating `Current`; typed out-of-range (kind matches native lane's convention); one-past-end legal; `as i64` truncation lands in the refusal (benign). |
| `SourceStream::position` | 175-177 | REAL | — | Trivial. |
| `mod tests` (5 tests) | 180-277 | REAL | — | See §5. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `blob_source_reads_seeks_and_ends` (:187) | YES | Short-read-is-data, zero-only-at-EOS, both seek origins, typed out-of-range with stage pin. Would NOT catch F-001 (no test asks for a near-usize::MAX length — the max asked is 100). |
| `blob_source_read_into_fills_caller_memory` (:211) | YES | Exact fills, zero-length no-op, partial-at-EOS with position pinned. |
| `registry_resolves_replaces_and_refuses` (:228) | YES | Replace-doesn't-grow and the unknown-name refusal pinned. |
| `registry_is_bounded` (:244) | YES | 16 fit, 17th refused, replacement-at-bound works. |
| `blob_uri_parses` (:264) | YES | Scheme round-trip + the space-name grammar refusal pinned — the test PROVES the grammar rejects the names `load` accepts (F-002's root) but never closes the loop through `load` itself. |
All 5 ran and passed in baseline (:3195-3199). Gap: no test drives `load("bad name")` → `open` — F-002's success-then-refuse flow is unpinned.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "bytes arrive through the port as `HostToWorker::LoadBlob` messages and are served back … through `SourceStream`" | source.rs:7-9 | REAL | host.rs:1026 → open.rs:226 chain; executed on the real fixture (baseline :3119). |
| "zero copies per read beyond the demuxer's own buffer fill" | source.rs:9-10 | REAL | `Arc` handoff at open; demuxer path is `read_into` (copy-into-caller only, verified riff.rs uses). `read()`'s `to_vec` is the convenience lane, not the demuxer's. |
| "a later same-name load REPLACES the bytes" | source.rs:12-14 | REAL | :72-76; test :233-235. |
| "Bytes are retained for the session's life" | source.rs:15-17 | REAL | No eviction API exists; BTreeMap lives in the host state. |
| "bounded (at most `MAX_BLOBS` entries … never an unbounded map)" | source.rs:12, :26-28 | SHALLOW | True for the MAP; "bounded memory, MP §11" is really the ABI lane's 32 MiB message bound, not this layer's (F-003). |
| "an `EngineError` when the name does not form a valid `blob:` URI" (BlobSource::new) | source.rs:117-119 | REAL | Test-pinned (:271-275). |

## 7. Dead / duplicated / unreachable code
- No dead code. `BlobSource::new` is test-facing (registry is the production entry) — documented as such (:114-115).
- `lib.rs:76` re-exports `BlobSource` publicly — no in-crate production consumer outside tests; API surface for embedders, acceptable.

## 8. Dependencies & flags
- `std` (BTreeMap, Arc) + `media_types` + `plugin_api` — all declared crate deps. No unsafe, no feature gates, no external crates. Workspace lints respected (no unwrap/expect/panic; `unreachable!` appears only in a test's match arm :251, which is the sanctioned test-only use).

## 9. Open questions for the next agent
- host.rs auditors: does anything cap `LoadBlob` name length (a 32 MiB name would itself be a message-sized DoS)? I found no name-length check at host.rs:1026 or here.
- open.rs auditor (A04): the `strip_prefix("blob:")` name extraction (:166, :223) and `SourceUri::parse` are the gate that makes F-002's lying registry error invisible in production — if open.rs ever calls `registry.open` before the parse gate, the false "not loaded" surfaces.
- plugin-api access.rs auditors (A03/A07): the trait contract's silence on `len` bounds is what makes F-001 a seam risk — if the contract gains a "len is caller-budgeted, sources must not panic" sentence, this file's `read` violates it as written.
