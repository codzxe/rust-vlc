# CLAIM AUDIT: docs/adr/ADR-013-va-api-hw-decoder.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R3; replaces a prior audit written at the wrong path audit/docs/adr/… missing the `docs/` segment) |
| Claims extracted | 14 |
| REAL 7 / TRUE-at-time-stale 3 / TRUE-but-gate-violated 1 / CONTRADICTED-BY-IMPLEMENTATION 1 / REAL-UNPROVEN 2 | |
| Internal contradictions found | 2 (vs feature-matrix notes + tree) |

## 1. Blunt verdict
An honest, unusually self-aware proposal ADR ("this file does NOT decide anything") with
verified cross-references — and a Context section now doubly stale: (1) "Zero adapters have
landed" was true on 2026-11-09 and is false in this tree (3,734 LOC of VA-API adapter code
landed 2026-11-10, bridge registered with no feature gate); (2) its own precondition
"Decision due before any VA-API adapter code lands" was not honored — code landed while the
ADR is still proposed. Worse for the doc's substrate analysis: the landed implementation is
exactly the "not viable" option (c) — direct VA-API, hand-rolled FFI, no crate imported.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: proposed ... does NOT decide anything" | :3-4 | REAL | corroborated: KNOWN-LIMITS.md:27-28 "draft — decision pending"; 14-risk-register.md:92-93 "admission pending Lead"; matrix notes "[proposed]" |
| 2 | "Decision due before any VA-API adapter code lands" | :5-6 | TRUE-but-gate-violated | hw/vaapi/ = 3,734 LOC landed 2026-11-10 per matrix note (feature-matrix.toml:416 "hw lanes landed ... VA-API (Linux) compile-verified + failure-path runtime-tested ... activation [PROPOSAL] pending ADR-013"); ADR still proposed |
| 3 | "Zero adapters have landed (matrix rows platform-hwdecode-*: not_started on every OS)" | :44-45 | TRUE-at-time, FALSE-now | code: crates/codecs/src/video/h264/hw/vaapi/{mod,session,ffi,dl,library,device}.rs; `b.push(Arc::new(vaapi::VaApiBridge));` hw/mod.rs:456 `#[cfg(target_os = "linux")]` — no feature gate |
| 4 | "so on Linux today H.264/HEVC files do not play" | :45-46 | TRUE-at-time, half-stale | H.264 half superseded: KNOWN-LIMITS.md:26 w27 software landing (rust_h264 admitted, matrix codec-video-h264-software implemented); HEVC still parser-only |
| 5 | "Decoder impls fail configure with the typed h264.hw.required / hevc.hw.required Unsupported" | :42-43 | REAL | decode.rs:13, :182, :274; h264/mod.rs:13; hw/mod.rs:54 — all cite the typed refusal |
| 6 | "VLC 3.0.23 ships vaapi/vdpau_avcodec modules" | :49-51 | REAL | platform-linux.md:176 exact dump citation (vaapi, vaapi_drm, vaapi_filters, glconv_vaapi_* + vdpau family) |
| 7 | access-path table: libva 0.1.5 dlopen/bindgen; libva-sys 0.1.2; "va 0.0.1 false friend: Vapory" | :59-63 | REAL | h264-software-decoder-audits.md:23-24, :82-84, :194-195 — byte-matched |
| 8 | "matrix rows codec-video-h264/-hevc (partial on seam-only evidence)" | :18-20 | REAL | feature-matrix.toml:411 `status = { windows = "partial", linux = "partial", macos = "partial", web = "not_started" }` (hevc same shape) |
| 9 | "Nothing here authorizes importing either crate today" | :143-144 | REAL | rg 'libva' Cargo.lock crates/codecs/Cargo.toml -> 0 hits |
| 10 | "Direct VA-API without a crate. Not viable" (option c) | :123-125 | CONTRADICTED-BY-IMPLEMENTATION | landed lane IS crate-less direct FFI: dl.rs:1-7 "no link-time dependency ... typed LibraryUnavailable", hand-written ffi.rs (25,304 B), library.rs symbol typing |
| 11 | R-01/R-02 exposure 9 | :46, :29-31 | REAL | 14-risk-register.md:92-93 "**9**" both rows |
| 12 | SV-28 "VA-API E2 classification = open Lead question" | :23-24 | REAL | feature-matrix.toml:2425 note "VA-API E2 classification = open Lead question (SV-28)" |
| 13 | AGENTS §3.2/§3.3 dependency + unsafe-adapter rules | :9-10, :25 | REAL | AGENTS.md:33 "No unsafe code in application or core crates. Platform adapters need an" (rule exists) |
| 14 | Sign-off gate: admission record R1-R10, deny.toml same-change | :164-173 | REAL-UNPROVEN | not triggered (no admission); unverifiable future event |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "Zero adapters have landed" (:44) | matrix note :416 "hw lanes landed (2026-11-10, issue #41)" + 3,734 LOC in hw/vaapi/ | the code exists; the ADR Context is stale by one wave |
| "Decision due before any VA-API adapter code lands" (:5) | code landed 2026-11-10, ADR still proposed | both true as statements; the sequencing rule was broken by the project, recorded in the matrix note itself |

## 4. Fabrication / inflation indicators
None — the doc is anti-inflation (repeatedly denies itself authority). The defects are
staleness + a broken self-gate, not invention.

## 5. Findings (doc-level)
F-001 [S3] :5-7 "Decision due before any VA-API adapter code lands" — VaApiBridge landed
2026-11-10 (hw/mod.rs:453-456, cfg linux, no feature flag) with the ADR still proposed; the
release lane for Linux compiles the adapter in. Code-before-decision, recorded nowhere in
this ADR. Confidence HIGH — read.
F-002 [S3] :44-45 "Zero adapters have landed" cites matrix rows platform-hwdecode-* as proof,
but those rows are themselves stale (still `not_started` all-OS + `evidence = ""` at
feature-matrix.toml:2423/2436 while sibling row :416 records the landing) — the ADR's
evidence chain rotted underneath it.
F-003 [S4] :45-46 "H.264/HEVC files do not play" on Linux — H.264 half superseded by the
w27 software landing (2026-11-15); no addendum here.
F-004 [S4] :123-125 option (c) "not viable" — the shipped lane implements exactly that
option (crate-less dlopen FFI); the ADR's a/b substrate question was mooted without
amendment.

## 6. Reachability
N/A (decision document). The adapter code it gates IS reachable in the Linux release build
(h264 hw/mod.rs bridge list, cfg(target_os="linux") — no feature gate found in
crates/codecs/Cargo.toml).
Commands: rg across feature-matrix.toml / risk register / records; sed -n matrix rows;
wc -l hw/vaapi; rg libva in Cargo.lock (0 hits); Read hw/vaapi/dl.rs:1-40, hw/mod.rs.
