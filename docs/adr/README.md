# ADR register — index of architecture decision records

| Field | Value |
|---|---|
| Purpose | Card 8.G1 (issue #58): "ADR register current: every docs/adr/ entry listed in an index with status; stale ADRs marked superseded (never deleted)" |
| Compiled | 2026-11-10, wave 24 lane E, against tree `c339ee8` (origin/main) |
| Rule | Statuses are quoted from each ADR's own `Status:` line — this register never re-decides. Drift observed at compile time is recorded in the Notes column with a next step; changing a status is a Lead action (AGENTS.md §3.6/§3.8). ADRs are append-only history: superseding happens by a status edit **in the ADR file itself**, never by deletion. |

## Register

| ADR | File | Decision (title abridged) | Status (own words) | Date | Notes |
|---|---|---|---|---|---|
| ADR-001 | `ADR-001-hardware-decode.md` | Hardware video decoding — hw-first for H.264/HEVC, software-first for everything else | accepted | 2026-10-19 (Phase 0 gate) | Implementation Phase 7 landed compile-verified in wave 23 (VT/MF/VA-API lanes, issue #41, commit `67a139c`); matrix rows `platform-hwdecode-*` remain `not_started` — flip is lane P8-2/P8-6 business, not this register's. |
| ADR-002 | `ADR-002-wasm-runtime-wasmi.md` | WASM extension runtime — wasmi | accepted | 2026-10-19 (Phase 0 gate) | Wasmi is a workspace dependency (`crates/plugin-host-wasm`); the runtime is exercised by the plugin conformance kit (issue #46, wave 23). |
| ADR-002-evidence | `ADR-002-evidence.md` | WASM runtime candidate table — **evidence pack, NOT a decision** | evidence pack (feeds ADR-002) | 2026-10-19 | Companion evidence for ADR-002's decision; deliberately carries no `Status:` decision line. Listed here so the numbering is not mistaken for a second ADR-002 decision. Never to be "superseded" — it is input, not output. |
| ADR-003 | `ADR-003-video-in-webview.md` | Video-in-webview presentation — per-OS strategy with prototype gates | accepted (per-OS final confirmation gated on Phase 2 prototypes) | 2026-10-19 (Phase 0 gate) | Prototype evidence: `docs/prototypes/video-in-webview.md` (B-protocol proven at protocol/virtual-time level; per-OS browser measurement still PENDING — headless sandbox, no webview runtime). The gated confirmation therefore remains open; see ADR-004's per-OS PENDING verdicts. |
| ADR-004 | `ADR-004-os-webview.md` | The OS-webview exception — tauri/wry as the audited UI shell (E1) | accepted; **per-OS webview verdicts PENDING** | 2026-10-22 (card 3.DOC-1) | No tauri binary has ever been built or launched on any OS (`apps/desktop/tauri/INFRA-BLOCKED.md`, issue #4, R-04). The `desktop` package that ships in release artifacts is the std-only composition-root skeleton, not the webview shell. |
| ADR-005 | `ADR-005-audio-output.md` | Audio output backends — adapter seam in audio-output (L4), OS impls in platform-* (L5), admission gated | accepted for the adapter-seam architecture; **real-backend admission PENDING** | 2026-10-22 (card 3.DOC-1) | cpal remains NOT admitted (Lead-gated conditional-E2; `docs/dependencies/records/cpal-0.18.2.md`); every platform still refuses real output through a typed marker. |
| ADR-006 | `ADR-006-license-mit.md` | Repository license — MIT | accepted (user decision) | 2026-10-19 | Supersedes the open question in doc 11 §8.2 and closes risk R-05. |
| ADR-010 | `ADR-010-codec-patent-shipping.md` | Codec patent & redistribution policy — the shipping matrix | **proposed** (draft brief; decision deferred to user + counsel) | drafted 2026-11-09 (issue #24) | Decision due **before any binary-distribution plan**; hard trigger = Phase-8 packaging cards 8.E1–8.E3. Still proposed at this register's compile date — the release checklist (docs/release-checklist.md) carries it as a blocking sign-off row. |
| ADR-011 | `ADR-011-tls-https.md` | TLS/HTTPS strategy — ureq+rustls(ring) ratified as the E2-class exception | accepted | 2026-11-09 (activates issue #39) | Real TLS round-trip needs a networked lane (sandbox has no egress); out-of-band measurement recorded in `docs/dependencies/records/ureq-3.5.0.md` R10. |
| ADR-012 | `ADR-012-bitmap-subtitles.md` | Bitmap + closed-caption subtitle strategy (N-wave unlock) | accepted (Lead signature) | 2026-10-24 | Renderer overlay contract still missing (R-27) — PGS/DVB parse landed (matrix `implemented`), compositing proof pending. |
| ADR-013 | `ADR-013-va-api-hw-decoder.md` | VA-API hardware decode — the Linux lane | **proposed** (draft; decision pending) | drafted 2026-11-09 (issue #2) | **Drift flag (honest):** the ADR says decision is due *before any VA-API adapter code lands*, yet the wave-23 lane landed VA-API hw-decode internals (issue #41, card 7.B1, commits `3b82b4b`/`67a139c`, compile-verified only — `crates/codecs/src/video/h264/hw/vaapi/`). The ADR has not been accepted or amended since. Next step (Lead): either ratify ADR-013 with the landed lane as its evidence or record the deviation. This register does not decide. |

## Numbering gaps (honest, not filled by this register)

- **ADR-007, ADR-008, ADR-009 do not exist.** The sequence jumps ADR-006 → ADR-010.
- **ADR-008 is referenced as a future owner of two scopes** but was never opened:
  - optical-disc scope — referenced by `docs/formats/disc-dvd-video.md` ("ADR-008 is not [open]") and matrix rows `platform-disc-dvd`/`-bluray`/`-cdda` (`blocked_reason`/`notes`), doc 06 §2;
  - WebCodecs/web hardware decode — referenced by `docs/formats/video-h264.md` and `video-hevc.md` ("WebCodecs is ADR-008 territory") and matrix row `platform-web-webcodecs`.
  These references describe a *planned* ADR; none of them becomes true by citation. Opening (or renumbering) ADR-008 is a Lead decision.
- **ADR-009 is referenced once** as "crate registry, ADR-009" (`docs/09-web-wasm-target.md` header table) and likewise was never opened.
- ADR-002 has two files by design (decision + evidence pack), see the register row above.

## Maintenance rule

When an ADR's status changes, edit that ADR's `Status:` line **and** this
register in the same commit. When an ADR supersedes another, the superseded
ADR keeps its file with a `Superseded by ADR-NNN` banner added at the top —
deletion is forbidden (append-only history, AGENTS.md §4 discipline).
