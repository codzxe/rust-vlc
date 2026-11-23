# AUDIT: crates/demux/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Lines / functions | 55 / 0 (crate root: docs + 13 `pub mod` declarations) |
| File verdict | REAL (with a stale module map and two house-rule violations in the modules it declares) |
| Reachable from shipped entry point? | YES — the crate is linked into lumen-cli (apps/cli containers + demux_host) and lumen-desktop embeds lumen-cli. Per-module reachability from `lumen open`: wav, aiff, adts, avi, ivf, mkv (matroska), mp4, ogg, ps (mpegps), ts (mpegts), es (flac/h264/hevc/mp2/mp3) — all have registry arms in apps/cli/src/containers/mod.rs:50-75; **`asf` has NO arm and no probe signature → UNREACHABLE**; `error` serves avi. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg scans (house-rule vocabulary, unsafe, module users); cat crates/demux/Cargo.toml; read of signature/mod.rs:130-147 + containers/mod.rs:50-75 (reachability evidence) |

## 1. One-paragraph verdict
The crate root is honest about its architecture (L2, trait seam, no compile-time input edge) and
its two workspace dependencies are internal-only (`media-types`, `plugin-api` — zero external
deps verified in Cargo.toml; zero `unsafe` verified by rg). But the "Module map" documents 7
modules while the file declares 13 — six demuxer families (adts, es, mkv, mp4, ogg, ps) are
invisible in the crate's own map — and the house rule "no `unreachable!` in non-test code" is
violated by ts/mod.rs (two production `unreachable!`, provably unreachable today but
rule-violating), while the ASF header walk carries a latent panic on crafted input (see the
asf/header.rs audit, F-001) that falsifies the "no panic" rule in practice. The root also
silently ships one entire demuxer family (asf) that no shipped entry point can reach.

## 2. What actually works (verified)
- Dependency claims: `cat crates/demux/Cargo.toml` → `[dependencies]` = media-types + plugin-api
  (both path crates); dev-deps testkit + input. Zero crates.io deps — the C5 claim holds.
- No-`unsafe` claim: rg over crates/demux/src → zero unsafe blocks.
- Reachability matrix (verified): 12 of 13 modules reachable from `lumen open` via
  containers/mod.rs arms; asf unreachable (no `AsfSignature` in media-probe
  signature/mod.rs:130-147, no registry arm).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/lib.rs:36-39 (the rule) vs crates/demux/src/ts/mod.rs:369,372 (the violations)
Code (the rule):
```rust
//! Std-only, zero external dependencies (C5); no `unsafe`; no
//! `unwrap`/`expect`/`panic!`/`todo!`/`unreachable!`/`dbg!` in
//! non-test code; bounded memory (MP §12): every read is
//! chunk-bounded and declared sizes are ceiling-checked before they are
//! trusted into an allocation.
```
Code (the violation, ts/mod.rs:367-372, production code — `mod tests` is a separate file):
```rust
                return CodecDescriptor::new(&format!("ts-stream-{other:02x}"))
                    .unwrap_or_else(|_| unreachable!("formatted names are valid"));
            }
        };
        CodecDescriptor::new(name).unwrap_or_else(|_| unreachable!("static names are valid"))
```
What is wrong: two `unreachable!` in non-test code, on a path keyed by a parsed stream-type
byte. I verified they cannot fire today (`CodecDescriptor::new` rejects only empty/control-char
strings — ids.rs:582-591 — and `ts-stream-xx` never matches), so this is a rule violation, not
a live panic; but the crate's own stated invariant is false as written, and the ASF header
walk's crafted-input panic (asf/header.rs audit F-001: unchecked `off as u64 + size` at
header.rs:558) makes the "no panic in non-test code" rule false in practice for this crate.
What correct looks like: propagate the error (the constructors already return Result) and fix
the ASF adds. Confidence: HIGH — Verification: read + rg + ids.rs grammar check.
F-002 [S4]
Where: crates/demux/src/lib.rs:12-33 (module map) vs 43-55 (declarations)
Code (the declarations):
```rust
pub mod adts;
pub mod aiff;
pub mod asf;
pub mod avi;
pub mod error;
pub mod es;
pub mod ivf;
pub mod mkv;
pub mod mp4;
pub mod ogg;
pub mod ps;
pub mod ts;
pub mod wav;
```
What is wrong: the "## Module map" section documents avi, wav, ivf, aiff, ts, asf, error — 7 of
13. Six shipped demuxer families (adts, es, mkv, mp4, ogg, ps) are absent from the crate's own
map, and the doc's "v0 carries the AVI 1.0 demuxer … in [`avi`]" framing predates the other 11.
The Cargo.toml description repeats the AVI-only framing. Stale crate-level documentation.
Confidence: HIGH — Verification: read + diff of the two lists.
F-003 [S4]
Where: crates/demux/src/lib.rs:43-55 (with the reachability matrix)
What is wrong: the root exports `asf` — ~4000+ lines of demuxer (a/b/c cards) with committed
fixtures and its own test suite — that no shipped entry point can reach (no probe signature, no
container arm; rg: zero users outside crates/demux). Shipping dead demuxer surface is a
maintenance and audit-surface cost, and its module doc falsely claims a media-probe signature
exists (see the asf/header.rs audit F-003). Confidence: HIGH — Verification: rg + registry read.

## 4. Per-function table
Not applicable (0 functions; 55 LOC of docs + 13 declarations). The reachability statement
required by §12 is in the header table above.

## 5. Test honesty
No tests in this file. The crate's tests execute via the module files (43 wav, 36 asf, 32 aiff
lib tests + integration suites — all run green in this session).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only, zero external dependencies (C5)" | lib.rs:37 | REAL | Cargo.toml: only path deps |
| "no `unsafe`" | lib.rs:37 | REAL | rg: zero unsafe blocks in crates/demux/src |
| "no `unwrap`/`expect`/`panic!`/…`unreachable!` in non-test code" | lib.rs:37-39 | FALSE | ts/mod.rs:369,372 (`unreachable!`); asf/header.rs:558 latent panic (F-001) |
| "every read is chunk-bounded and declared sizes are ceiling-checked" | lib.rs:39-41 | FALSE-in-one-spot | asf/header.rs:558 adds a declared size unchecked (the F-S1 of the header audit) |
| Module map (7 modules) | lib.rs:12-33 | FALSE | 13 modules declared (F-002) |

## 7. Dead / duplicated / unreachable code
- `demux::asf` — unreachable from shipped entry points (F-003); its `probe`/`EXTENSIONS` have
  zero callers anywhere outside the module itself.
- Nothing else at root level.

## 8. Dependencies & flags
- `media-types` + `plugin-api` (path deps); dev: `testkit`, `input`. Edition 2024. No features
  declared — everything compiles in the release lane (baseline: release build clean).

## 9. Open questions for the next agent
- The `mkv`, `mp4`, `ogg`, `ps`, `ts`, `adts`, `es` modules each deserve their own audit (this
  wave split them across agents); the ts `unreachable!` pair suggests checking those modules
  for the same house-rule pattern (`rg 'unreachable!\(|\.unwrap\(' --glob '!*tests*'`).
- If ASF is wired later, the header walk's unchecked adds must be fixed first (header audit F-001).
