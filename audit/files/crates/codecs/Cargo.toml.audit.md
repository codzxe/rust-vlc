# AUDIT: crates/codecs/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 185 / 11 features + 12 deps + 4 dev-deps |
| File verdict | REAL — disciplined exact-pin dependency manifest with full record chain; 1 stale record citation, stale package description |
| Reachable from shipped entry point? | YES — apps/cli/Cargo.toml:51 `codecs = { path = "../../crates/codecs" }` (default features: all 11 codec gates on) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `test -f` loop over all 11 cited record paths (10 OK / 1 MISSING); rg (web/CLI feature wiring, xtask baseline) |

## 1. One-paragraph verdict
This is what a dependency manifest should look like under a
purity-gated admission policy: every external engine is exact-pinned
(`=0.4.3` … `=1.1.0`), every pin carries a "purity verified in
docs/dependencies/records/*.md BEFORE this edit" citation plus re-trigger
conditions, the risky default features are explicitly off
(rav1d `asm`, ruopus's 23-package ndarray graph, filmcraft-dnx's
`threads`/rayon), and `default` enables the full set so native consumers
resolve one graph. I verified 10 of the 11 cited record files exist. The
defects: the filmcraft-dnx comment cites `filmcraft-dnx-0.1.0.md`, which
does not exist (the real record is `filmcraft-dnx-0.9.0.md`, matching the
`=0.9.0` pin) — a broken link in the admission evidence chain; the
package `description` still names only "MJPEG/PCM/FLAC" for a crate
carrying 11 codec families; and the header's dependency claim includes
`plugin-api`, which is dev-only. Also note the consciously-documented
decision to keep the in-house modules (incl. the unreachable hevc tree)
unconditional in every build, wasm included.

## 2. What actually works (verified)
— verified by `test -f` over all cited record paths: 10/11 exist
(claxon-0.4.3, symphonia-mp3/vorbis/aac/alac-0.6.1, rav1d-1.1.0,
ruopus-0.1.2, rusty_vp9-0.1.3, rust_h264-0.6.1,
h264-software-decoder-audits); the pins' feature discipline
(rav1d :125 `default-features = false, features = ["bitdepth_8",
"bitdepth_16"]`, ruopus :137 `features = ["std"]`,
filmcraft-dnx :158 `default-features = false`) matches each record's
stated binding conditions.

## 3. Findings

F-001 [S3]
Where: crates/codecs/Cargo.toml:146-147 (+ :158)
Code:
```toml
# 6.M-dnx (Lead admission, docs/dependencies/records/filmcraft-dnx-0.1.0.md):
# DNxHD/DNxHR (SMPTE ST 2019-1 VC-3) decode engine. Purity verified
```
What is wrong: the comment cites `docs/dependencies/records/filmcraft-dnx-0.1.0.md`
— that file does NOT exist; the actual record is
`filmcraft-dnx-0.9.0.md` (verified by ls), matching the `=0.9.0` pin at
:158. The admission evidence chain — the manifest's central claim
("Purity verified ... BEFORE this edit") — points at a stale path for the
dnx engine. Anyone following the trail from the Cargo.toml hits a dead
link, and a future auditor checking "which record governs this pin" gets
a version (0.1.0) that contradicts the pin (0.9.0).
Why it matters: the repo's whole dependency-admission discipline rests on
these citations being resolvable; the xtask feature-audit already runs
R4 missing-path checks (baseline: 2 violations elsewhere) and this path
is one comment-scan away from being a third.
What correct looks like: cite `filmcraft-dnx-0.9.0.md`.
Confidence: HIGH — Verification: `test -f` (MISSING) + `ls
docs/dependencies/records/`.

F-002 [S4]
Where: crates/codecs/Cargo.toml:36
Code:
```toml
description = "Lumen builtin software codecs: baseline-JPEG/MJPEG video decoder, unpacked-PCM audio decoder and FLAC audio decoder (claxon engine) behind the codec-api contract (doc 01 §2.5, cards 2.B4/2.B5/5.A1)."
```
What is wrong: the crate description names three codecs (mjpeg, pcm,
flac) and three cards (2.B4/2.B5/5.A1); the crate now ships eleven codec
families behind default-on features (aac, alac, av1, dnx, flac, h264-sw,
mp2, mp3, opus, vorbis, vp9 — :47-59). Publishing metadata (cargo info /
crate listings) would materially understate the surface.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/codecs/Cargo.toml:2 and :5-6 vs :184
Code:
```toml
# doc 01 §2.5 registry row: L3, deps codec-api + plugin-api + media-types).
...
plugin-api = { path = "../plugin-api" }
```
What is wrong: the header states the crate's dependencies are
"codec-api + plugin-api + media-types", but `plugin-api` is a
DEV-dependency only (:176-185 `[dev-dependencies]`); the real
`[dependencies]` are codec-api + media-types + the optional engines. The
header even argues why the narrower list is "legal" against
boundaries.toml — while naming a dep that isn't one. Stale card-header
comment.
Confidence: HIGH — Verification: read.

## 4. Per-function table (config keys)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| default features (11 gates) | 47-59 | REAL | - | full set; CLI/web resolve as documented |
| codec-aac/-alac/-flac/-mp3/-opus/-vorbis | 60-80 | REAL | - | symphonia family, exact pins, records exist |
| codec-av1 (rav1d) | 62-66, 125 | REAL | - | asm-off pin matches record; wasm never enables (apps/web/Cargo.toml:113-121) |
| codec-dnx (filmcraft-dnx) | 146-158 | REAL (stale comment) | S3 | F-001 record path |
| codec-h264-sw (rust_h264) | 159-174 | REAL | - | record + audits exist; in-house bridge documented |
| codec-vp9 (rusty_vp9) | 138-145 | REAL | - | empty default features claim consistent |
| [dependencies] block | 83-174 | REAL | - | all optional engines exact-pinned |
| [dev-dependencies] | 176-185 | REAL | - | input/demux/plugin-api/testkit, downward edges |
| description | 36 | STALE | S4 | F-002 |
| header comment | 1-30 | STALE (partial) | S4 | F-003; also documents the conscious choice to keep in-house modules (hevc incl.) unconditional — feeds nal.rs F-003 dead-code finding |

## 5. Test honesty
N/A (manifest).
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Purity verified in docs/dependencies/records/<engine>.md BEFORE this edit" (×11) | Cargo.toml comments | REAL for 10/11 | test -f loop: 10 OK, filmcraft-dnx-0.1.0.md MISSING (F-001) |
| "`default` enabling the full set so every existing consumer ... resolves the exact SAME dependency graph" | :21-24 | REAL | CLI uses default features (no default-features=false in apps/cli/Cargo.toml:51) |
| "rav1d does not compile for wasm32 ... wasm builds simply never enable it" | :62-65 | REAL | apps/web/Cargo.toml:113-121 excludes codec-av1 from every wasm feature set |
| "the gates only ever REMOVE code" | :23-24 | REAL | optional = true everywhere; in-house modules ungated |
| "deps codec-api + plugin-api + media-types" | :2 | FALSE (stale) | plugin-api is dev-only (:184) |
## 7. Dead / duplicated / unreachable code
Conscious ungated shipping of in-house modules (hevc tree — 1517-line
parser + 765-line decoder + hw seam — compiled into wasm and native
binaries with no caller; see nal.rs/decode.rs audits). The comment at
:25-30 documents this as a deliberate tree-shake tradeoff.
## 8. Dependencies & flags
See table. All versions exact-pinned; optional engines behind codec-*
features; no build.rs; workspace lints inherited.
## 9. Open questions for the next agent
- dnx/mod.rs (file 20 of my list): does the wrapper really decode with
  single-threaded semantics (threads off) as the record requires?
- Should the xtask feature-audit R4 scan cover Cargo.toml COMMENT paths
  (not just code citations)? It missed F-001's dead link.
