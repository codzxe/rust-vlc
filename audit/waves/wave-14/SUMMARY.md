# WAVE 14 SUMMARY — apps/desktop-tauri

| Field | Value |
|---|---|
| Agents | A01–A10 all complete |
| Files audited | 13, 100% covered |
| Findings | S0:5 S1:0 S2:4 S3:12 S4:25 |
| Tree purity | clean |

## THE TAURI ANSWER
1. [S0, static proof, 4 independent confirmations] The crate CANNOT
   COMPILE ON ANY HOST: app.rs:301 calls undefined `spawn_session_pump`
   (real name `spawn_event_pump` at :190); no glob imports, no cfg gate,
   lib.rs unconditionally exposes app. ci.yml:731 tauri-check,
   tauri-launch.yml, release.yml:219 all build it => all red on this
   tree. INFRA-BLOCKED.md:4-5 "the crate compiles in its dedicated CI
   lane" is FALSE; the sandbox's gdk-sys failure MASKS the real defect
   (Wave-0's "environment limit" note re-adjudicated as env-hides-repo-
   defect). Green release lane proves nothing (workspace excludes the
   crate).
2. [S0] single_command.rs is a TEXT SCAN (include_str + bracket hunt),
   cannot run anywhere (links the broken lib), and is cited as live
   enforcement — unrunnable test as evidence (W14-A05).
3. [S0] app.rs pump thread dies silently on mutex poison (doc's "app is
   dying anyway" false — tauri catches panics) (W14-A02).
4. [S2] Frame path (when it existed): engine -> FrameTap -> mailbox
   (cap 2, latest-wins = only backpressure) -> 10ms ticker ships
   records.last() -> tauri fetch-token queue (UNBOUNDED — stalled webview
   = OOM-class ~370MB/s) -> eval'd fetch -> JS ArrayBuffer ->
   decodeLuf1Record -> ctx.putImageData (MediaSurface.tsx:227). Pure CPU
   memcpy chain, no texture/GPU, A/V sync by pacer cadence alone
   (W14-A07).
5. [Verified good] capabilities/default.json is genuinely least-privilege
   (core:default + 4 window commands; zero shell/fs/http perms; no
   plugin crates in the 493-package closure); CSP hardened; IPC surface
   = exactly one custom command with PERFECT registration match (W14-A03/
   A09). Icons are honest placeholders (solid #4f8cff, regenerable
   byte-identical via gen-icon.py) — but NOT the product.toml brand
   assets (which are missing — Wave 1's 4 violations).
6. [Contradiction for Wave 19] PARITY-REPORT.md:122-125 "CI billing-
   blocked, every run dies at 0 steps" vs RELEASE.md:145-148 "billing
   unblocked 2026-11-12, workflow executed v0.1.0-v0.1.2" (W14-A04).
7. [S3] Window: first paint 1280x800 (tauri.conf) vs product.toml
   1280x720 — 80px flash every launch; remember_geometry parsed but
   dropped (no persistence exists) (W14-A08).
8. [S3] tauri/wry version claims: tauri 2.12.5 + wry 0.57.2 verified in
   lock; ADR-004 record still says wry 0.57.0 everywhere; Cargo.toml
   ranges ~100x wider than the admitted allowlist (W14-A01/A06).
9. [S4] Lockfile: 493 packages (not the claimed 468), 46 duplicate
   names/52 redundant copies, GUI and CLI compile in two ungated
   dependency universes (32 version-drifted shared names).
