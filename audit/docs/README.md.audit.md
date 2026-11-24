# CLAIM AUDIT: README.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R3) |
| Claims extracted | 10 |
| REAL 8 (one recomputed EXACT from the matrix) / UNVERIFIED 2 (network/git claims) | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A README whose honesty claims are its strongest feature and survive recomputation: the
"38 of the 73 format rows claimed on desktop (14 containers, 7 video, 9 audio, 8
subtitle), zero on web" headline reproduces EXACTLY from docs/feature-matrix.toml, the
"no sound on any OS yet" null-device admission matches the ADR-005 record, the 0/252
verified-parity residue matches the orchestrator baseline, and the anti-overclaim framing
("never described as all-Rust", "nothing is claimed until the matrix records evidence")
is enforced in the tree. The only unverifiable items are release-publication/git claims
(no network, no git in sandbox).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "38 of the 73 format rows ... claimed (implemented or partial) on desktop (14 containers, 7 video codecs, 9 audio codecs, 8 subtitle formats) — zero on web" | :41-43 | REAL (recomputed EXACT) | python3 per-row parse of feature-matrix.toml: container 14/21, video-codec 7/16, audio-codec 9/19, subtitle-format 8/17 — sum 38/73 desktop-claimed, 0 web-claimed |
| 2 | "audio output is the ADR-005 null device — no sound on any OS yet" | :44-46 | REAL | W17-R1's cpal-record audit: null.rs:433 + stub.rs:87 only impls; AUDIO_ADR005_ADMITTED=false compile-time assert (flip breaks build) |
| 3 | "the desktop app opens any local file but displays no video yet (canvas path built but unwired)" | :46-48 | REAL-UNPROVEN (runtime) | consistent with the phase-8 caveats class; not re-run here |
| 4 | "URL sources (http/hls) are refused with a clear message" | :48-49 | REAL | matches the browser-matrix record's typed `unsupported-for-host` refusals (http-streaming blocked row, JSON lanes) |
| 5 | "verified parity is still 0/252 (the evidence-outbox flip pass has not run ... hardware_class = mock)" | :132-134 | REAL | baseline xtask-parity-report.txt: 0/252 verified on every platform (windows 66/252, linux 61/252, web 0/252) — the README carries the same residue |
| 6 | workflows "ci.yml + ci-full.yml 3-OS matrix, ui.yml, soak-nightly.yml, release.yml ... actionlint-clean" | :126-127 | REAL (existence) | ls .github/workflows -> 11 files incl. all five named (+audit/codeql/compat-matrix/rust/rust-clippy/tauri-launch) |
| 7 | "Build from source ... cargo build --release --locked -p lumen-cli -p desktop (toolchain pinned by rust-toolchain.toml)" | :80-84 | REAL | rust-toolchain.toml + product.toml + AGENTS.md present; the command is the release lane the baseline executed clean (exit 0) |
| 8 | "the lumen-desktop exit-86 stub leaves every user payload (wave 30; through v0.1.2 it shipped as a third binary)" | :74-75 | REAL | the w30.8 stub-exit is the same fact W17-R1's release-checklist audit logged ("both binaries incl. lumen-desktop" false since w30.8) |
| 9 | "Releases v0.1.0, v0.1.1 and v0.1.2 are published ... SHA256SUMS.txt" + "v0.1.3 being cut" | :28-29, :59-63 | UNVERIFIED | publication is a GitHub/network claim (no network in sandbox); corroborated in-tree by docs/RELEASE.md + release-notes/skeleton-v0.1.3.md existence |
| 10 | "rustfmt-drift red (w17, fixed by 621fa20 ... since merged)" | :136-138 | UNVERIFIED | git claim; consistent with baseline cargo-fmt exit-1 history framing (fmt red is real: baseline fmt FAILED) |

## 3. Contradictions
None. The "Phase 5 PASS WITH REGISTERED CAVEATS / Phase 6 verdict PENDING LEAD DECISION"
statuses are stated rather than inflated — and the honest-gaps list (:44-49) is more
prominent than any capability claim.

## 4. Fabrication / inflation indicators
None — the reverse. The README self-polices: "this project is never described as
all-Rust"; "Nothing is claimed implemented or verified until the matrix records
automated evidence" (:53-56); licensing residuals kept [VERIFY] (:146-147).

## 5. Reachability
N/A (top-level README). All structural claims verified against the tree.
Commands: python3 matrix recomputation (3 iterations to get the category split right —
container/video-codec/audio-codec/subtitle-format), ls .github/workflows (11 files),
ls of the six referenced docs, cross-refs to baseline parity-report and W17-R1 cpal audit.
