# w26.m.t5 — tauri launch-verdict lane (wave 26-M, issue #67)

Proof file for the launch half of the ADR-004 admission
(`apps/desktop-tauri/INFRA-BLOCKED.md` §B.2). This lane does NOT claim
any per-OS verdict — the verdict rows (ADR-004 §"Launch verdicts") are
all PENDING-CI; what landed and is proven HERE is the lane itself:
designed, validated (syntax + structure + pin parity), and honest about
what its future runs will and will not prove.

| | |
|---|---|
| Date | 2026-11-13 |
| Lane | w26.m.t5 (launch-verdict lane), branch `w26.m/t5-launch-verdict` |
| Files | `.github/workflows/tauri-launch.yml` (NEW — the only new workflow); `docs/adr/ADR-004-os-webview.md` (verdict-table section appended); `ui/src/client/tauri-transport.ts` ([VERIFY] marker comment annotated, one sentence — code untouched) |
| Verdict state | Linux PENDING-CI (lane designed; first run pending); macOS/Windows PENDING-CI (best-effort lanes; human confirmation owed) |
| Local execution | none — the lane runs cargo/xvfb in CI by design; discipline for this task forbade local cargo builds/tests, and none were run |

## The lane (what runs, in order)

1. toolchain — dtolnay/rust-toolchain at the repo-pinned SHA + the
   ci.yml-verbatim toolchain-pin assert (file == action == effective
   rustc; the ::error names tauri-launch.yml).
2. build deps — the ci.yml `tauri-check`-proven webkit2gtk-4.1 apt
   line, byte-for-byte, plus the `pkg-config` presence assert.
3. launch env (Linux job only) — `xvfb fonts-liberation fontconfig`;
   NOT build deps; separated into its own step so the proven line stays
   untouched.
4. embedded UI — `npm --prefix ui ci` + `npm --prefix ui run build` +
   the `ui/dist/index.html` assert (tauri-codegen embeds frontendDist
   at COMPILE time — the ordering rule the whole lane hangs on).
5. build — `cargo build --release --locked --manifest-path
   apps/desktop-tauri/Cargo.toml` (the subcrate's own lockfile). NO
   CARGO_TARGET_DIR override: the crate is its own workspace root, so
   artifacts land at `apps/desktop-tauri/target/release/lumen-tauri`
   (the §C recipe's path convention); a missing binary is a loud
   preflight failure, not a silent path guess.
6. launch (the verdict step):
   - Linux: `setsid xvfb-run -a` (software X server, 1920x1080x24) with
     the fixture `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` on
     argv — the app's file-open seam dispatches a real open through the
     single command path and the 40 ms pump emits real engine events on
     `lumen://event`;
   - macOS: direct exec (NOT `open` — that needs an .app bundle; the
     admission build is a bare binary until the bundling half lands) on
     the runner's Aqua session;
   - Windows: `Start-Process` of the bare console-subsystem exe
     (WebView2 ships on the runner image), output redirected.
7. asserts (the verdict semantics below), then artifacts:
   stdout.log / stderr.log / context.txt uploaded per OS with
   `if: always()` (a FAILED launch's stderr is the most valuable
   evidence the lane can produce) and `if-no-files-found: error`.

Smoke-pattern provenance: numbered checks + honest fail counter +
preflight `need()` from `scripts/rc-smoke.sh`; bounded-run exit-code
case analysis from ci.yml's link/exit smoke (which accepts the
apps/desktop skeleton's 86 `WEBVIEW_PENDING` code on a display-less
host — this lane is the one that launches FOR REAL, under xvfb, and
asserts survival instead; 86 is not a `lumen-tauri` exit — its crash
faces are 1 and 101). No `scripts/**` file was added or touched; the
smoke is self-contained in the workflow (scripts/ are other lanes'
property).

## Verdict semantics — what the asserts prove vs what needs humans

The honest automation is the app's OWN receipts (there is no JS driver
in this lane, and none is faked):

- `event emit failed on the shell channel: …` — app.rs
  `forward_pending`'s stderr line, printed for every `lumen://event`
  emit that errors;
- `lumen-tauri: admission shell failed: …` + exit 1 — main.rs's crash
  face when the shell cannot start.

The lane asserts: (1) the process starts and SURVIVES the window
(12 s Linux, 10 s macOS/Windows; 0.5 s liveness polls — an early death
fails regardless of exit code); (2) ZERO failure receipts in the
captured stderr across the run; (3) the exit is harness-initiated (TERM
→ 143) or a clean 0 — never a crash face.

PROVES on a green Linux run (the CI half): GTK/tao/tauri bootstrap
succeeded on a real X server; the `main` webview window exists
(`apply_window_spec` errors out without it); the argv-open dispatch ran
through the single command path; and every emit the dispatch + pump
made on `lumen://event` returned success — no channel-level delivery
failure was recorded while the app lived.

DOES NOT PROVE (the human/GUI half): that the JS side received any
frame, or that the received frame matches `TauriEventFrame` (no JS
observer in the loop); that the UI rendered (no screenshot/driver); that
a user-initiated window close exits 0 (no window manager under xvfb —
the harness TERMs the process group; xvfb-run does not forward signals,
hence the process-group TERM). A green run moves the ADR row to "CI
half proven", not to "verdict".

Known blind spot (recorded, not hidden): a silent web-process crash
AFTER the event traffic ends produces no receipt (no pending events →
no failing emits) and no process death (the main process survives the
web process) — the argv-open burst plus playback events are the
guaranteed traffic, and everything after that is honest silence. A JS
heartbeat is the eventual e2e-lane fix; this lane does not pretend to
have one.

Emit-vs-page-load gray zone: the argv-open burst fires at setup,
possibly before the webview page loads. If WebKitGTK errors on the
early emit, the receipt assert goes red on a run where the app itself
is healthy — that red is CORRECT anyway: events emitted before the UI's
`plugin:event|listen` attaches are lost to the UI, which is exactly a
launch-verdict defect ("file opens" fails in the event-truth sense).
Either outcome of the first run is a real finding, not noise.

## macOS / Windows honesty

BEST-EFFORT-PENDING, stated in the workflow and in the ADR table: a
green runner attempt is NOT a per-OS verdict. A runner session is not a
real desktop — WebView2 runtime versioning, GPU paths, and the
window-server context differ; the receipts asserts run (same strings),
survival is asserted, the Windows exit code after a Force-kill is
recorded but NOT asserted (a Force-killed code is not a signal face;
the deterministic TERM→143 case analysis exists where signals exist:
Linux and macOS).

## The tauri-transport [VERIFY] clearing decision

The marker comment structure made the annotation clean (the marker is a
// comment block above `target: { kind: "Any" }`), so BOTH halves of the
procedure are in place:

- the marker was ANNOTATED, not removed — one sentence added:
  "(CI half: tauri-launch.yml — clears when its no-emit-failure +
  clean-exit proof lands on a real run; the GUI half stays human.)";
- the full clearing procedure lives in the tauri-launch.yml header and
  ADR-004 §"The tauri-transport [VERIFY] clearing rule":
  (a) the lane green on a real run (CI half) →
  (b) a human/GUI driver observes the round-trip on the launched app →
  (c) the marker is deleted in the SAME commit that flips the ADR-004
  verdict row, with the run link as evidence. Until (c) it stands.

## Validation (this file's proof)

- `yaml.safe_load`: PASS — 1 workflow, 3 jobs (linux-launch /
  macos-launch / windows-launch), 29 steps; triggers =
  `workflow_dispatch` ONLY (asserted programmatically; the bare `on:`
  key is PyYAML's YAML-1.1 boolean quirk, the same one every
  GH-standard workflow in this repo carries — Actions accepts it).
- `bash -n` over every bash `run:` block (with `${{ }}` expressions
  neutralized, as the runner resolves them before bash): 13/13 PASS.
- pwsh block (windows launch step): NOT locally checkable (no pwsh in
  this sandbox) — recorded honestly; it follows the release.yml MSI
  lane's pwsh-on-windows precedent.
- Action pins cross-checked against ci.yml's proven pins: checkout,
  setup-node, rust-toolchain, rust-cache, upload-artifact — 5/5
  byte-identical SHAs.
- Artifact-upload posture asserted programmatically: 3/3 jobs upload
  with `if: always()` + `if-no-files-found: error`; `permissions:
  contents: read`; default shell bash with one deliberate
  `shell: pwsh` override.

## Observed CI reality at design time (recorded, owned outside t5)

- ci.yml `tauri-check` (run 43400610890, 2026-11-13) is RED at the
  CLIPPY step: 5 `expect_used`-class denials in `apps/desktop-tauri`
  (e.g. src/app.rs:168's `.expect("product.toml [window] …")`). The
  `cargo test` step on the same run was green — the crate compiles and
  its audit tests pass; only clippy denies remain. This lane runs
  `cargo build` (no clippy), so it is unaffected, but the compile lane
  must go green before any verdict row can flip. Root-caused and
  reported to the wave lead; the fix is not t5's file.

## Next actions

1. Dispatch `tauri-launch.yml` manually (workflow_dispatch) once the
   clippy denials in `apps/desktop-tauri` are fixed (they block
   tauri-check, not this lane's build — but a green compile lane should
   precede a verdict run).
2. Read the Linux run against this file's semantics; move the ADR-004
   Linux row to "CI half proven" with the run link on green, or triage
   the receipts assert per the gray-zone note on red.
3. macOS/Windows rows move only with human confirmation on real
   hardware; the lane's green attempts are evidence, not verdicts.
