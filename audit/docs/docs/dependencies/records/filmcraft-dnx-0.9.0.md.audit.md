# CLAIM AUDIT: docs/dependencies/records/filmcraft-dnx-0.9.0.md
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Claims extracted | 24 |
| REAL | 13 (lock/pin/checksum/test/matrix/evidence-file verifications) |
| REAL-UNPROVEN | 5 (evidence-file-only: differential, hostile battery, VTA harness run) |
| UNVERIFIED | 6 (network-dependent: crates.io index, OSV, wasm, downloads count, ffmpeg 7.1.5 presence, publisher) |
| SHALLOW/STUB/FALSE | 0 — 1 drift note (codecs test count 624→773) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This record is the most verifiable kind in the tree: every claim that CAN be checked
against the repo checks out exactly — the lockfile pin, the rayon-free graph, the
tarball sha256 byte-for-byte against the Cargo.lock checksum, the admitted feature
wiring, the 6/6 differential test in the orchestrator baseline, the matrix flip, the
ProRes absence, the ADR-010 table row, and the §D/§H numbers matching the committed
evidence files digit-for-digit. The record is also honest about its own limits (§G
gaps, conditional verdict, "not bit-exact"). The unverifiable residue is genuinely
network-bound (crates.io/OSV) or execution-bound (the ffmpeg differential).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Admission shape: `filmcraft-dnx = { version = \"=0.9.0\", default-features = false }`" | L17-18 | REAL | crates/codecs/Cargo.toml:158 `filmcraft-dnx = { version = "=0.9.0", optional = true, default-features = false }` (optional=true is the feature gate; version + default-features match) |
| 2 | "the pin adds **no `rayon` node** to our lockfile" | L22-23 | REAL | Cargo.lock:217-224 deps: `[ "filmcraft-bitstream", "thiserror" ]` only |
| 3 | "filmcraft-bitstream v0.9.0 ← LEAF. ZERO dependencies." | L67 | REAL | Cargo.lock:211-214 — no `dependencies` key at all |
| 4 | "tarball ... sha256 `a3a59d559caf74b9281578f06015702bd402192198f71f5d41e7c6a9ec3592ab`" | L138-140 | REAL | Cargo.lock:219 checksum = `a3a59d559caf74b9281578f06015702bd402192198f71f5d41e7c6a9ec3592ab` — BYTE-IDENTICAL |
| 5 | "harness at `tools/vta/dnx`, committed" | L61-62 | REAL | ls tools/vta/dnx -> Cargo.toml Cargo.lock README.md src evidence |
| 6 | "Cargo.toml pin (Lead admission): ... `crates/codecs/Cargo.toml`" | L270-271 | REAL | crates/codecs/Cargo.toml:158 |
| 7 | "`cargo test -p codecs` → 643 test cases ... 624 pass, 18 ignored ... 0 fail" | L279-281 | REAL (for its date) / drifted | baseline cargo-test-workspace.txt: codecs now **773 passed / 18 ignored**; 18-ignored matches exactly, totals grew after 2026-11-09 |
| 8 | "`dnx_differential` — **6/6 pass**" | L282-283 | REAL | audit/runs/baseline/cargo-test-workspace.txt:1032-1045: `running 6 tests` -> `test result: ok. 6 passed; 0 failed` (lane_l1..l5 + truncated_real_frame_refuses_typed) |
| 9 | "the `codec-video-prores-dnxhd` row ... is flipped `not_started → partial`" | L284-286 | REAL | docs/feature-matrix.toml:588 `status = { windows = "partial", linux = "partial", macos = "partial", web = "not_started" }` |
| 10 | "ProRes is NOT — a repo-wide grep for prores code under `crates/codecs` returns nothing" | L287-289 | REAL | `rg -i "prores" crates/` -> no files |
| 11 | "**ADR-010:** the \"Software decoders implemented\" table carries the DNxHD/DNxHR (VC-3) row" | L274-276 | REAL | docs/adr/ADR-010-codec-patent-shipping.md:79: `\| DNxHD/DNxHR (VC-3) \| crates/codecs/src/video/dnx/** + filmcraft-dnx =0.9.0 ...` citing this record |
| 12 | "five-lane differential ... every frame within ±2 LSB ... 0.04–6.6 % of samples" | L152-155, L190-196 | REAL-UNPROVEN | tools/vta/dnx/evidence/compare-results.txt matches digit-for-digit (L1 frame2 Cr maxΔ=2 -> lane envelope "2/1/2"; L2 1/1/1, 0.040–0.205%); the run itself cannot be re-executed here (no ffmpeg lane build, cargo-build-budget) |
| 13 | "Delta histogram (L1 frame 0 ...) `{-1: 24848, +1: 29408, +2: 2}`" | L200-201 | UNVERIFIED | histogram detail not in compare-results.txt (only maxΔ/differing per plane); plausibly from harness stdout not committed |
| 14 | "byte-count parity exact on all 5 lanes" | L198-199 | REAL-UNPROVEN | evidence dir has lane-md5s.txt + ffprobe-facts.txt; byte parity itself not re-runnable |
| 15 | "zero-length → `Err(Truncated)`; 64 B garbage-truncation → `Err(Truncated)`; 0xAA fill → `Err(Invalid(...))`" | L227-229 | REAL-UNPROVEN | hostile-results.txt lines match exactly: `hostile zero-length -> Err(Truncated)` etc. |
| 16 | "Sampled off-by-one truncation sweep (256 cut points): **0 panics** (208 Ok / 48 ... 201/55 ... 206/50)" | L235-237 | REAL-UNPROVEN | hostile-results.txt: `sweep: 256 sampled truncation points ..., 0 panics, 208 Ok-frames, 48 typed Err` / `201` / `206/50` — exact |
| 17 | "half-truncated real frame → `Ok(frame)` ... concealment-class note ... flagged for the M-wave" | L231-234 | REAL-UNPROVEN | hostile-results.txt `hostile truncated-frame(1/2) -> Ok(1920x1080 cid 1238 depth 8)`; the honest-flag is the record's own |
| 18 | "0 `unsafe` tokens and 0 `extern \"C\"` across `src/` (rg)" | L54-55 | UNVERIFIED | requires the vendored crate tarball (registry-only; not in-tree) |
| 19 | "`[lints.rust] unsafe_code = \"forbid\"` in the manifest" | L48-49 | UNVERIFIED | manifest not in-tree (registry-only) |
| 20 | "0.9.0 published **2026-10-23** ... **9 downloads** ... not yanked" | L118-121 | UNVERIFIED | crates.io network claims; no network in sandbox |
| 21 | "OSV API ... zero advisories" | L126-128 | UNVERIFIED | network |
| 22 | "our pin 1.99.1 ≥ 1.95 ✓, compiled clean on it" | L119-120 | REAL (toolchain half) | rust-toolchain.toml:9 `channel = "1.99.1"`; compile itself covered by baseline build |
| 23 | "upstream README claims wasm32 ... **not measured by us**" | L91-93 | HONEST | record itself marks it unmeasured |
| 24 | "wasm32 build not measured ... no throughput benchmark ... upstream's oracle-test claims are upstream's" | L255-257 | HONEST | §G self-disclosure |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| — (none internal) | crates/codecs/Cargo.toml:146 comment cites `docs/dependencies/records/filmcraft-dnx-0.1.0.md` | That record file does NOT exist (actual: filmcraft-dnx-0.9.0.md). The broken citation is in the CODE file, not this record; the matrix row (feature-matrix.toml:589) cites the record correctly. Flagged for the codecs Cargo.toml owner. |
| "643 test cases ... 624 pass" (addendum, 2026-11-09) | baseline now: 773 passed / 18 ignored | Both true at their dates; 18-ignored matches; not a contradiction, recorded as drift |

## 4. Fabrication / inflation indicators
None found. Notably the record UNDERSTATES: it refuses to claim bit-exactness, lists
its own harness bugs (L217-222), labels the tolerance verdict "NOT bit-exact" in
bold, and defers the ±2 LSB ratification to the Lead. The evidence numbers in the
committed files match the record's table exactly — no inflation detected.

## 5. Matrix rows
Matrix row `codec-video-prores-dnxhd` (feature-matrix.toml:581-590): statuses
partial/partial/partial/not_started; evidence string cites this record + addendum +
dnx_differential 6/6 — all verified above (claims 8, 9, 10). Row verdict: REAL
(DNxHD half) / honest not_started (ProRes + web).
