# CLAIM AUDIT: docs/dependencies/preaudit-foundation.md
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 / 0 / 0 / 0 / 0 / 0 / 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A Phase-0 candidate survey whose structural predictions all cashed out in
the tree: every `conditional`/`exception` crate it flagged now has the full
record it promised (cpal-0.18.2.md, wgpu-30.0.1.md, tauri-wry-2.x.md exist),
cpal remains unadmitted exactly as its conditional verdict implied
(KNOWN-LIMITS §4: "cpal is NOT admitted"), tauri/wry live only inside the
workspace-EXCLUDED subcrate under ADR-004's envelope (subcrate lock:
tauri 2.12.5, wry 0.57.2, objc2 0.6.5, windows 0.62.2 — the last two
matching the preaudit's surveyed versions exactly), and the allowlist.md was
built from this doc's quick-pass rows (allowlist.md:24-41 cites "inherited
preaudit-foundation"). The version/license table itself is network data
fetched 2026-10-19 — not re-verifiable from this sandbox, and honestly
marked ([VERIFY-P1] on the toml license null). The doc's self-classification
("quick pass ... full R1–R10 records happen at Phase 1") is accurate — the
later records exist.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Method: docs/dependencies/audit-method.md (quick pass ...)" | :3-5 | REAL | audit-method.md exists; the quick-pass vocabulary matches its §3 M1-M2 |
| 2 | cpal 0.18.2 "conditional (E2 ...)" | table | REAL | cpal NOT in root Cargo.lock (rg -> 0); records/cpal-0.18.2.md exists; ADR-005 exists; KNOWN-LIMITS §4 confirms not-admitted |
| 3 | wgpu 30.0.1 "conditional (E2 graphics-stack)" | table | REAL | wgpu NOT in root Cargo.lock; records/wgpu-30.0.1.md exists (record for the excluded-subcrate closure) |
| 4 | tauri 2.12.4 / wry 0.57.0 "exception-ADR-004 ... consumed ONLY under ADR-004" | table + §2 | REAL | apps/desktop-tauri/Cargo.lock: tauri 2.12.5 (:3688), wry 0.57.2 (:4961) — in the EXCLUDED subcrate only; ADR-004-os-webview.md exists; records/tauri-wry-2.x.md exists |
| 5 | windows 0.62.2 / objc2 0.6.5 metadata-generated E2 canonical | table | REAL | subcrate lock: windows 0.62.2 (:4654), objc2 0.6.5 (:2139) — exact match to the surveyed versions |
| 6 | Version/license table (serde 1.0.229, serde_json 1.0.154, toml 1.1.8, thiserror 2.0.21, schemars 1.2.3, tracing 0.1.44 ...) fetched 2026-10-19 | table | UNVERIFIED | network facts; the root-lock subset (serde/serde_json/toml/thiserror/schemars) matches allowlist.md:24-28 exactly, corroborating the survey; tracing/tracing-subscriber never entered the lock (candidate, not admission) |
| 7 | "toml ... version endpoint returned null [VERIFY-P1: license ... confirm at pin]" | table | UNVERIFIED (network) / REAL (marker) | the [VERIFY-P1] marker is present; allowlist.md:26 says the marker was "closed by this card's API check + green cargo-deny licenses run" — chain consistent |
| 8 | "Not pre-audited here ... symphonia, mp4parse ... truehdd — those ... get full R1–R10 records at their phase" | §2 | REAL | records now exist: symphonia-{mp3,vorbis,aac,alac}-0.6.1.md, truehd-0.7.2.md — the promised phase records landed |

## 3. Contradictions
None. The one drift: tauri/wry surveyed at 2.12.4/0.57.0, shipped at
2.12.5/0.57.2 — normal patch drift between survey (2026-10-19) and
admission, and the tauri-wry-2.x.md record owns the admitted versions.

## 4. Fabrication / inflation indicators
LOW. The doc labels its own depth honestly ("pre-audit level, not
R1–R10"), marks the toml license anomaly [VERIFY-P1], and its "pass" rows
were later inherited by allowlist.md with [VERIFY] tags rather than silent
upgrades. The E2-class framing (OS-service bindings allowed, third-party C
forbidden) matches doc 11 §1's actual policy text.

## 5. Chain integrity
- allowlist.md's §3 rows cite this file as the inheritance source — the
  Phase-0 → Phase-1 audit chain is intact and traceable.
- Raw JSON preservation claim (tmp/preaudit2/, gitignored) — by-design
  uncommitted; .gitignore:40 covers tmp/ (verified in the audit-method audit).
