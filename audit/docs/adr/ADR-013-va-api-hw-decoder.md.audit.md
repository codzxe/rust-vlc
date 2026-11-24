# CLAIM AUDIT: docs/adr/ADR-013-va-api-hw-decoder.md (L1-184)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 / 0 / 0 / 0 / 0 / 1 / 2 |
| Internal contradictions found | 2 (both cross-file) |

## 1. Blunt verdict
The ADR's own rule — "Decision due **before any VA-API adapter code lands**" — has been violated by its own project: the adapter lane EXISTS in-tree (crates/codecs/src/video/h264/hw/vaapi/: hand-rolled FFI + runtime dlopen of libva.so.2, session/pool/capability modules, ~1500+ LOC, compile-verified per README #41) while this ADR's status header still reads **proposed — decision pending** and the substrate decision (libva vs libva-sys vs hand-rolled) it reserves for the Lead has already been resolved in code ("the no-new-dependency reading: hand-rolled FFI"). The adapter module's own header discloses the tension honestly ("ADR-013 **[PROPOSAL]** is the pending Lead decision record for this lane's activation"), which makes this governance debt rather than deception — but the decision-gate ordering the ADR defines is not being followed. Additionally, the matrix rows this ADR cites (platform-hwdecode-* = not_started, "zero adapters exist") are now stale in the understating direction, with the row notes frozen at the ADR's draft date.

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: proposed (drafted 2026-11-09…) decision due before any VA-API adapter code lands" | :3-6 | REAL (as written) / VIOLATED in practice | crates/codecs/src/video/h264/hw/vaapi/mod.rs:1-7 documents the landed lane with "ADR-013 [PROPOSAL] is the pending Lead decision record for this lane's activation". Code exists; decision still pending. |
| 2 | "Zero adapters have landed (matrix rows platform-hwdecode-*: not_started on every OS)" | Context | FALSE (now) | The vaapi + videotoolbox (+ Media Foundation per README #41) adapter dirs exist in-tree; matrix row platform-hwdecode-h264 still says not_started with evidence="" (feature-matrix.toml:2427-2433) — the row was true at draft date (2026-11-09), stale now. |
| 3 | H.264/HEVC seams "fail configure with the typed h264.hw.required… honest by design" | Context | REAL | 5.F2/5.G2 seam pattern (typed refusals) consistent with the no-GPU sandbox and the capability module's HwAvailability::Unavailable. |
| 4 | "What exists today is only the seam" (parser + hw seams, no adapters) | :Context | FALSE (superseded) | Same as #2 — adapters landed post-draft; the ADR text is a dated snapshot not updated. |
| 5 | Substrate options libva 0.1.5 / libva-sys 0.1.2 "both remain unadmitted" | :Evidence | REAL (recorded) / superseded in code | hw/vaapi/mod.rs:25-33: "both remain unadmitted… This lane implements the no-new-dependency reading: hand-rolled FFI" — the Lead-facing substrate decision was pre-empted by the no-dep reading (defensible: no dependency admission needed), but the ADR remains unaccepted. |
| 6 | "compile-verified only; runtime hardware behavior NOT [verified]" (adapter lane label) | code header | REAL | hw/vaapi mod.rs:43-51 records the honest compile-only label and the failing probe on this host. |
| 7 | VLC baseline "ships vaapi/vdpau_avcodec modules; VA-API covers 8/10/12-bit + 4:2:2/4:4:4 (Intel)" | :Evidence | UNVERIFIED | Inventory citations (platform-linux.md exists); VLC-side facts from the pinned census, not re-derived. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "Decision due before any VA-API adapter code lands"; status proposed | The adapter code is in the tree (hw/vaapi/, compile-verified) | The code is the truth; the gate ordering was inverted. Disclosed in the code header — governance debt, not fraud. |
| "matrix rows platform-hwdecode-*: not_started… zero adapters" | Tree + README (#41 "hw-decode adapter lanes compile-verified") | Rows are stale-understating; same family as matrix rows 187/189 in my chunk audit. |

## 4. Fabrication / inflation indicators
None — understatement everywhere (the project refuses to claim hardware decode it cannot run). The issue is decision-ordering, not truth-bending.
Commands run: ls crates/codecs/src/video/h264/hw/; head hw/vaapi/mod.rs; rg matrix row platform-hwdecode-h264. Exit 0.
Findings: S3: 1 (F-021 adapter code landed while its governing ADR is still proposed — the ADR's own gate ordering inverted); S4: 1 (F-022 ADR body + matrix rows stale post-draft).
