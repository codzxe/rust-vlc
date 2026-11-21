# AUDIT: crates/media-types/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-21 |
| Lines / functions | 29 / 5 manifest keys (package, lints, dev-dep, 2 header claim blocks) |
| File verdict | REAL (the minimal graph it promises is exactly the graph it has; two claim-wording imprecisions, no structural defect) |
| Reachable from shipped entry point? | YES, EXECUTED (call chain: `apps/cli/Cargo.toml:57` `media-types = { path = "../../crates/media-types" }` (and via demux/codecs/playback-engine transitive edges); executed in baseline: every packet/frame in mp4-smoke-open-session.txt and the AVI first-vertical bench is a media-types type) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `sed -n '55,70p' boundaries.toml` ([crates.media-types] exact mirror: internal_deps=[], dev=[testkit], external=[]) · `rg -n 'std::sync\|Mutex\|RwLock\|thread\|io::Read\|TcpStream' crates/media-types/src` (only `use std::sync::Arc;` packet.rs:7) · `rg -n 'pub fn.*Arc<\|pub struct.*Arc' crates/media-types/src` (one public Arc signature: packet.rs:144) · `sed -n '40,50p' crates/media-types/src/packet.rs` (struct field is private) · `head -50 crates/media-types/tests/testkit_wiring.rs` (real asserts) · `rg -n 'fn parse' crates/media-types/src` (Scheme/SourceUri/LanguageTag parse fns, ids.rs:26,132,509) · baseline cites #7/#11 |

## 1. One-paragraph verdict
The smallest manifest in my set and the most honest: it promises "No dependencies. Deliberately boring" and delivers exactly that — no `[dependencies]` section, boundaries.toml:63-68 mirrors it field-for-field, std-only by construction, dev-dep `testkit` genuinely consumed by two test files. The crate-level purity claims mostly survive spot-checks (no locks/threads/sockets in src; `Arc` used but the struct field is private). Two wording defects: the "no threading primitives in the public API" claim is falsified by one public constructor that takes `Arc<Vec<u8>>` head-on (a thread-safe shared-ownership type — by design, but the claim needs scoping), and the direction-boundary claim says testkit_wiring.rs "proves" what it merely demonstrates, leaning on a checker whose baseline run FAILED on a different edge.

## 2. What actually works (verified)
- Zero-dependency claim: no `[dependencies]` section exists; `boundaries.toml:63-68` `[crates.media-types] layer = 0, internal_deps = [], dev_deps = ["testkit"], external_allowlist = []` — exact mirror.
- Std-only claim: trivially true with zero deps (no build.rs either).
- No-I/O / no-locks spot check: `rg 'std::sync|Mutex|RwLock|thread|io::Read|TcpStream'` over src → only `use std::sync::Arc;` (packet.rs:7); `Arc` is shared ownership, not a lock (see F-001 for the public-API nuance).
- dev-dep used: `crates/media-types/tests/testkit_wiring.rs` (imports `testkit::{HardwareClass, Must, assert_contains, assert_err, assert_nanos_near}`) and `shared_bytes_alloc.rs` (the counting-allocator proof referenced by packet.rs docs).
- Test honesty: `testkit_wiring.rs` asserts real semantics — `Timestamp::from_parts(90_000, 1, 90_000)` → `as_nanos` ≈ 1e9 (would fail if time math broke); `Packet::is_stale` across generations (would fail if staleness logic broke). Not tautologies.
- Reachability executed: baseline smoke outputs are full of media-types vocabulary (`TimingDurationKnown { duration: Timestamp(3000000) }`, track kinds, state machine).

## 3. Findings

### F-001 [S4]
Where: crates/media-types/Cargo.toml:8-11 (cross-ref: crates/media-types/src/packet.rs:144)
Code:
```
# The most-reviewed crate in the repo (doc 01 §2.2(a)): data types and pure
# functions only. No I/O, no threading primitives in the public API, no
# UI/GPU/platform types (ARCH-1), no sentinel numbers (MP §8), no
# format-specific parsing (that belongs to demux/codecs/subtitles).
```
and (packet.rs:144)
```
    pub fn from_pool_range(pool: Arc<Vec<u8>>, offset: usize, len: usize) -> SharedBytes {
```
What is wrong: "no threading primitives in the public API" is imprecise: `std::sync::Arc` — a thread-safe atomically-refcounted type from the `std::sync` module — appears in a public function signature (`from_pool_range`). The struct's own `buf: Arc<Vec<u8>>` field is private (verified), but the public constructor hands callers the sync type directly. Arc is shared ownership, not a lock/channel/thread, and doc 01 §3.3 sanctions the design — but the manifest claim as worded is contradicted by the signature.
Why it matters: This crate's entire value proposition is its purity contract (L0 vocabulary, ARCH-1); an auditor verifying the contract by grepping public signatures for `std::sync` types gets a hit and must then adjudicate intent. A contract that needs intent-adjudication is a contract that will be wrongly trusted or wrongly failed.
What correct looks like: Scope the claim: "no locks/channels/threads in the public API; `Arc`-based shared ownership is the sanctioned exception (doc 01 §3.3 SharedBytes)".
Confidence: HIGH — Verification: read (rg of public signatures + field privacy).

### F-002 [S4]
Where: crates/media-types/Cargo.toml:26-28
Code:
```
# testkit is dev-only by contract (MP §3.7; card 1.C3): the direction
# boundary (no production crate depends on it) is enforced by 1.E1's
# check-architecture and proven by tests/testkit_wiring.rs.
```
What is wrong: `tests/testkit_wiring.rs` demonstrates that testkit's helpers compile and work against the vocabulary (a dev-edge wiring proof); it cannot prove "no production crate depends on it" — that is a workspace-graph property only the architecture checker enforces. And that checker is not infallible in this tree: baseline #11 — `cargo xtask check-architecture` FAILED with "1 edge violation: `lumen-cli` depends on `streaming`, not declared in boundaries.toml". The proof hierarchy in the comment inverts reality: the mechanical check is the enforcement, the test is a smoke of the dev edge.
Why it matters: The sentence hands auditors a false sense of mechanical proof. The same pattern (a wiring test cited as boundary proof) appears in plugin-api — whose `testkit_wiring.rs` is the one file that failed `cargo fmt --check` in baseline #7, i.e. the "proof" file is also the tree's hygiene sore.
What correct looks like: "…enforced by check-architecture (baseline #11 shows it catches real violations); tests/testkit_wiring.rs demonstrates the dev-edge works."
Confidence: HIGH — Verification: read + baseline cites #7/#11.

## 4. Per-function table (manifest keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header: registry row (L0, no internal deps) | 3 | REAL | — | boundaries mirror verified |
| header: std-only + future-dep gate claim | 4-6 | REAL | — | zero deps today; allowlist.md referenced |
| header: purity contract | 8-11 | REAL (imprecise) | S4 | Arc in one public signature (F-001); "no sentinel numbers (MP §8)" UNVERIFIED at src level (out of scope here) |
| `[package]` block | 13-18 | REAL | — | description accurate (sources/streams/tracks/packets/frames/time/errors — matches src files) |
| `[lints] workspace = true` | 20-21 | REAL | — | inherits workspace policy |
| "No dependencies" comment | 23 | REAL | — | no [dependencies] section at all |
| dev-dep `testkit` | 25-29 | REAL | — | used by both test files; boundary wording overstated (F-002) |

## 5. Test honesty
`tests/testkit_wiring.rs`: two real tests (vocabulary round-trips, staleness) — mutation reasoning: broken Timestamp/Duration math or GenerationId staleness fails the asserts. `tests/shared_bytes_alloc.rs` is the counting-allocator proof for SharedBytes' no-copy claims — owned by the packet.rs auditor. No tests in this manifest file itself.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "layer L0, no internal deps" | manifest line 3 | REAL | boundaries.toml:63-68; no deps section |
| "Std-only by design" | manifest line 4 | REAL | zero deps, no build.rs |
| "No I/O, no threading primitives in the public API" | manifest lines 9-10 | SHALLOW (imprecise) | Arc<Vec<u8>> in pub fn from_pool_range (packet.rs:144) (F-001) |
| "no format-specific parsing" | manifest lines 10-11 | REAL | src parse fns are URI scheme/language tag vocabulary (ids.rs:26,132,509), not container/codec parsing |
| "direction boundary … proven by tests/testkit_wiring.rs" | manifest lines 26-28 | SHALLOW (wording) | test proves wiring, not the boundary; checker does enforcement (F-002) |

## 7. Dead / duplicated / unreachable code
None. Every manifest key is live; the crate is the most-depended-on in the repo (all L2+ crates and apps depend on it).

## 8. Dependencies & flags
No dependencies, one dev-dependency, no features, no targets. The manifest is the platonic minimum. Note for the root-workspace auditor: this is the shape the other manifests should have had before their comment-narratives grew stale.

## 9. Open questions for the next agent
1. packet.rs auditor (2,000+ lines of the 4,494-LOC crate): the "no sentinel numbers (MP §8)" claim is src-level and unverified here — check constants for magic numbers.
2. Why did `crates/plugin-api/tests/testkit_wiring.rs` (the sibling of this crate's wiring proof) fail `cargo fmt --check` in baseline #7 — one-off dirt or a pattern in "proof" files?
3. ids.rs auditor: `SourceUri::parse` is real URI parsing in the L0 crate — confirm the input crate (L2) doesn't duplicate it (one-parser rule).
