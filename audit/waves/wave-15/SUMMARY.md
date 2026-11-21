# WAVE 15 — SUMMARY (ui/src + ui config + theme tokens)

Date: 2026-11-21 (session 2) · Agents W15-A01…A10 (every agent required §7.4
re-spawns — 26-file loads exceeded sub-agent limits; all completed across two
passes) · Domain: ui screens/components/state/hooks/bridge, ui scripts/config,
asset groups, theme-tokens crate.
Gate: **PASS — 260/260 assignments audited** (2 lockfile-chunk audits accepted
as proportionate at 25/30 lines; A06's space-separated manifest committed via
the A00 straggler commit), citation spot-checks clean.

## Verdicts (canonicalized)
REAL ~185 · SHALLOW 12 · UNREACHABLE 14 · REAL-UNPROVEN 12+ (many REAL files
are REAL-code/UNREACHABLE-binary given W14's non-compiling GUI crate)

## Findings
**S0:1 S1:0 S2:27 S3:128 S4:297 — total 453**

## The wave's hostile question: is any UI element wired to a real backend?
The UI is REAL, tested, typechecked code wired to a backend that mostly
isn't there:
- **Volume slider is dead chrome**: `audio.set-volume` hits the host's
  unserved arm (dispatch.rs:155) → `state.volume` stays null → slider
  permanently disabled; only DemoTransport serves it (A06).
- **CompressorTool ships 7 fully-live sliders wired to `audio.set-compressor`
  — a command that exists in ZERO Rust files** (A03).
- **All 8 "live" privacy toggles write engine keys with ZERO consumers
  anywhere** (A09); settings never persist (W11).
- **audio.set-eq awaits a nonexistent backend** (W8 pin, re-confirmed);
  `audio.list-devices` has NO runtime handler (W8 class).
- **ProfilesFlyout**: select/create persist to a store with zero consumers,
  with four frame-seeded fixture profiles (Movies/Shows/Music/Work)
  presented as product data (A04).
- **LocalePicker "Interface language" is write-only**: 1 option, zero
  readers, orphaned localization crate (A02).
- And the container truth: **the GUI host crate cannot compile (W14 S0)** —
  so every UI element is REAL-code/UNREACHABLE-binary at HEAD.

## Other notable findings
1. [S0] ui-build.log.txt (baseline evidence) was captured on a DIFFERENT
   tree state — its convert warning names an import graph that doesn't exist
   at HEAD; committed .vitest/output.json proves lead-tree/Nov-11 provenance
   (A02).
2. [S2] tokens.json contrast claims: "tertiary ≥4.5:1 on raised" measures
   4.14:1 dark; "E8E8E8 measured" has 0 corpus hits; "lightness-ranked" has
   39 inversions — recomputed WCAG arithmetic (A01, A10).
3. [S3] stylelint "ZERO inline disables" claim is FALSE: 55 disables in 17
   files (A04).
4. [S3] w26.a14.ts is a dead post-merge staging copy duplicated verbatim in
   messages.ts with no gate comparing definition sites (A07).
5. [S2] VideoSliderRow's React.memo "j11 fix" is false — inline sinks
   re-render every row per drag tick (A08).
6. Cross-file flag: **root package-lock.json committed with unresolved
   merge-conflict markers** (A09 → orchestrator; verify in W17).

## Blocked / UNVERIFIED
- npm lint / lint:css / visual:pr never executed in baseline —
  REQUEST-ORCH-RUN standing.
- Inter font version/axis claims (compressed woff2, no tooling).
