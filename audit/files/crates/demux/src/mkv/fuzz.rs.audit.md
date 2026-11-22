# AUDIT: crates/demux/src/mkv/fuzz.rs

| Field | Value |
|---|---|
| Auditor | W05-A04 |
| Date | 2026-11-17 |
| Lines / functions | 84 / 1 harness fn + 1 const + 2 tests |
| File verdict | REAL-UNPROVEN (genuine survival harness, correctly looped and budgeted; never executed against a valid MKV in any run I can cite — the stable-runnable inputs never pass the magic check, and the nightly campaign is admitted NOT WIRED) |
| Reachable from shipped entry point? | NO — `lumen` CLI never calls it. Callers: the two `#[cfg(test)]` tests in this file, and `fuzz/fuzz_targets/fuzz_demux_mkv.rs:18-20` (a dev-only cargo-fuzz package, not part of the release build). `pub mod fuzz;` at mkv/mod.rs:46 is unconditional, so the code ships in the library but is dead weight in the binary |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | Read (full file); Read of `fuzz/fuzz_targets/fuzz_demux_mkv.rs` (21 lines); `ls fuzz/fuzz_targets/`, `ls fuzz/corpus/fuzz_demux_mkv/` (3 seeds: 26 KB mkv×2, 27 KB webm); `sed -n '1,40p' scripts/fuzz_registry_check.sh`, `sed -n '36,80p' fuzz/README.md`; Grep for callers of `fuzz_demux_mkv` and `mod fuzz` declarations; Read of `avi/test_support.rs:1-40`. No cargo (adaptation); the W00 baseline ran this module's two tests inside the workspace suite |

## 1. One-paragraph verdict

The harness itself is real and correctly shaped: one call = construct → `start` → drain events under a 200,000-event budget, all outcomes swallowed except `EndOfStream`/error. The wiring claims around it are mostly honest — the cargo-fuzz shim exists and calls this function, the registry gate script cross-checks it, and three documents say plainly the campaign is "SCAFFOLDED, NOT WIRED". The defect is the middle claim: "the harness logic is meanwhile **proven** on stable by the deterministic sweeps below" (`:16-17`) — those sweeps are 52 inputs (empty, a 4-byte magic, two 4 KB constant buffers, 48 patterned ≤ 48-byte buffers), and by trace, **not one of them passes the EBML magic check** (`header.rs:277-281` requires `1A 45 DF A3`) except the 4-byte magic input, which dies at the size vint. The `start()` path — the "full header/cluster/block walk, which is where untrusted bytes land" (`:17-19`) — is never entered by any stable-runnable input to this harness. The only inputs that would reach it (the 3 corpus seeds) are usable solely by the unwired nightly campaign. Today this fuzz harness has processed zero valid Matroska bytes.

## 2. What actually works (verified)

- The drain loop is correct: budget checked before each event (`:50-52`), all three non-terminal `DemuxEvent` kinds consumed (`:55-57`), `EndOfStream`/`Err` break (`:58`). No path panics on its own; `mem_stream`/`MkvDemuxer::new`/`start` failures return early (`:39-47`) — verified by reading the loop.
- The cargo-fuzz shim exists and calls exactly this function: `fuzz/fuzz_targets/fuzz_demux_mkv.rs:18-20` — `fuzz_target!(|data: &[u8]| { demux::mkv::fuzz::fuzz_demux_mkv(data); });`.
- The registry is gate-checked: `scripts/fuzz_registry_check.sh` requires [[bin]] names ≡ target files ≡ README rows ≡ in-crate harness fns, plus a non-empty seed corpus; `fuzz/corpus/fuzz_demux_mkv/` carries 3 seeds (`seed_mkv_flip1.mkv`, `seed_mkv_h264_aac.mkv`, `seed_webm_vp9_opus.webm`).
- The wiring status is honestly stated in three places: this file (`:12-19`), the shim (`:7-10`), and `fuzz/README.md:17` "Status: SCAFFOLDED, NOT WIRED — one Lead decision outstanding".
- `FUZZ_EVENT_BUDGET = 200_000` is documented with a rationale (`:31-32`).

## 3. Findings

### F-001 [S3] "Proven on stable by the deterministic sweeps" — the sweeps never get past the 4-byte magic; zero valid-MKV inputs run on stable
Where: crates/demux/src/mkv/fuzz.rs:16-19 (claim) and 67-83 (the sweeps)
Code (fuzz.rs:16-19):
```rust
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_demux_mkv.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's integration corpora.
```
Code (fuzz.rs:73-82):
```rust
    fn hostile_wide_inputs_survive() {
        // All-0xFF (EBML maximal varint false positives) and
        // all-0x00 (zero-length ID/size fields).
        fuzz_demux_mkv(&vec![0xff; 4096]);
        fuzz_demux_mkv(&vec![0x00; 4096]);
        for length in 1..=48usize {
            let data: Vec<u8> = (0..length).map(|i| (i * 53 % 256) as u8).collect();
```
What is wrong: of the 52 stable-runnable inputs, 51 begin with bytes ≠ `1A 45 DF A3` and die at `parse_ebml_header`'s magic check (`header.rs:277-281`, `error::not_matroska`); the one that matches (`b"\x1A\x45\xDF\xA3"`, `:70`) dies at the size vint (`read_prefix_at` returns 4 bytes → `parse_vint_size` NeedMore → `truncated`, `header.rs:282-285` + `ebml.rs:458-465`). Every sweep therefore returns at `:45-47` before any header/Info/Tracks/cluster parsing. The patterned inputs' first byte is `(0*53)%256 = 0` — all fail the magic. The 3 corpus seeds that would drive the full walk are only consumed by the nightly campaign, which `fuzz/README.md:17` says is not wired.
Why it matters: the doc's "proven" gives the harness a stable-runnable pedigree it does not have; a reader concludes the header/cluster/block walk survives hostile bytes on stable, when no stable run has ever entered it via this harness. (The integration corpora do test the demuxer with real bytes — but through their own fixtures, not this harness, and not fuzzed.)
What correct looks like: add the corpus seeds (or a minimal synthetic valid-MKV builder) to the stable sweep, or delete the word "proven" until the campaign is wired.
Confidence: HIGH — Verification: read (full static trace of every input through the magic check; runtime UNVERIFIED, no cargo).

### F-002 [S4] The two tests assert nothing — an empty harness body would pass them
Where: crates/demux/src/mkv/fuzz.rs:67-83
Code (fuzz.rs:67-71):
```rust
    #[test]
    fn empty_and_marker_inputs_survive() {
        fuzz_demux_mkv(&[]);
        fuzz_demux_mkv(b"\x1A\x45\xDF\xA3");
    }
```
What is wrong: neither test contains an `assert!`. They verify only "does not panic", which is the stated harness contract — but a mutation that replaces the loop body with `return` (budget removed, events never drained) keeps both tests green. They cannot detect harness regressions, only harness panics.
Why it matters: test-hygiene (§3.4): the tests distinguish a working harness from a gutted one only by accident of a panic; the budget and event-handling logic are untested.
What correct looks like: assert something observable (e.g., expose a counters struct from the harness: events drained, budget hit) or state the tautology in the test name.
Confidence: HIGH — Verification: read (mutation reasoning, UNVERIFIED at runtime).

### F-003 [S4] Dead code in the shipped library: unconditional `pub mod fuzz` with no production caller
Where: crates/demux/src/mkv/mod.rs:46; fuzz.rs:38
Code (mod.rs:46):
```rust
pub mod fuzz;
```
What is wrong: the module compiles into every build of the demux crate, including the release binary, but its only callers are `#[cfg(test)]` tests and the separate dev-only fuzz package. Its `crate::avi::test_support::mem_stream` dependency likewise rides the library (that one is documented as deliberate — `avi/test_support.rs:1-10` "public on purpose … the cargo-fuzz target … needs a `SourceStream`").
Why it matters: shipped surface that exists for dev tooling; small, but it is dead weight in the player binary unless stripped, and every `pub` item here is API surface to maintain.
What correct looks like: `#[cfg(any(test, feature = "fuzz"))]` gating, or accept and document it as library-surface fuzz API (the current docs half-do this).
Confidence: HIGH — Verification: read + grep (no production caller exists).

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FUZZ_EVENT_BUDGET` = 200_000 | 31-33 | REAL | — | Documented rationale; enforced at `:50-52` before each event (no off-by-one: max 200k `next_event` calls) |
| `fuzz_demux_mkv` | 35-61 | REAL | — | Loop correct, early returns on construct/start failure per contract; **F-001** (stable inputs never reach `start()`'s walk); **F-003** (no production caller); hang-freedom relies on demuxer ceilings it does not itself verify (scoped honestly at `:24-25`) |
| test `empty_and_marker_inputs_survive` | 67-71 | REAL (runs) | S4 | **F-002**: no assertion; inputs die at magic/size-vint |
| test `hostile_wide_inputs_survive` | 73-83 | REAL (runs) | S4 | **F-001/F-002**: 50 inputs, none passes the magic check; no assertion |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `empty_and_marker_inputs_survive` | NO (except on panic) | No assertion; a gutted harness body passes. Fails only if the demuxer panics on 2 trivial inputs — which never reach the parser |
| `hostile_wide_inputs_survive` | NO (except on panic) | Same; additionally none of its 50 inputs passes the EBML magic check, so the parser proper is never entered |

Runtime: UNVERIFIED directly (no cargo); the W00 baseline ran the workspace suite (1751 pass / 2 fail / 25 ignored) containing these two tests.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_demux_mkv.rs`" | fuzz.rs:14-15 | REAL | File exists; shim `:18-20` calls this harness |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps below" | fuzz.rs:16-17 | FALSE (overstated) | F-001: all 52 sweep inputs die at the magic check or size vint; no stable run enters the walk |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:5-8 | REAL | Loop + budget + early returns match the contract |
| "this pass covers the full header/cluster/block walk, which is where untrusted bytes land" | fuzz.rs:17-19 | REAL (capability) / SHALLOW (coverage) | The harness would cover it with valid-magic input; no stable-runnable input does |
| "Status: SCAFFOLDED, NOT WIRED" | fuzz/README.md:17 | REAL | README's own admission; corroborates F-001 |
| Registry row `fuzz_demux_mkv` → `demux::mkv::fuzz::fuzz_demux_mkv` | fuzz/README.md targets table | REAL | Shim + gate script (`scripts/fuzz_registry_check.sh` sets 1≡2≡3≡4 + corpus non-empty; 3 mkv seeds present) |

## 7. Dead / duplicated / unreachable code

- `fuzz_demux_mkv` is production-dead (F-003). No duplication found in this file.

## 8. Dependencies & flags

- No feature gates: `pub mod fuzz;` unconditional (mod.rs:46), same pattern in all 12 demux-format modules (avi/wav/aiff/ogg/mkv/mp4/ts/ps/es/ivf/adts/asf all declare unconditional `pub mod fuzz` — a crate-wide pattern, not an mkv accident).
- Dependencies: `plugin_api::{DemuxEvent, Demuxer}`, `super::MkvDemuxer`, `crate::avi::test_support::mem_stream` (documented-public test-support module).

## 9. Open questions for the next agent

- Wave 17 (scripts/CI): does anything in `.github/workflows` or `scripts/quality/16_fuzz.sh`/`25_fuzz_campaign.sh` actually execute a fuzz campaign or only the registry check? My read of `fuzz_registry_check.sh` says registry-only; confirm no CI lane runs `cargo +nightly fuzz run`.
- Whether the "stable-runnable equivalence" section of fuzz/README.md (line 41) lists the mkv harness among the sweeps that run — from this file's evidence, only the AVI harness has a dedicated stable sweep integration (`tests/fuzz_loop_stable.rs:60` uses `fuzz_demux_avi` only; no mkv equivalent).
