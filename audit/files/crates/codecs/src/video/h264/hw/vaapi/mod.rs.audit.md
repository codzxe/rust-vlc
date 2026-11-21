# AUDIT: crates/codecs/src/video/h264/hw/vaapi/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2026-11-21 |
| Lines / functions | 237 / 11 items in table (discover, VaApiAccelerator + 3 methods + Debug + HwAccelerator impl, VaApiBridge + impl backend_id/open, 2 tests) |
| File verdict | UNREACHABLE (REAL, honestly-labeled adapter glue; the mod gate is `#[cfg(target_os = "linux")]` hw/mod.rs:94-95) |
| Reachable from shipped entry point? | NO. Chain requires `negotiate_lane`/`installed_bridges` (hw/mod.rs:479, :450) or `H264Decoder::from_setting` — `rg -rn "negotiate_lane\|installed_bridges\|from_setting"` over crates/+apps/ → hw/mod.rs internals + tests ONLY; apps/cli/src/pipeline_host.rs:346 dispatches `"h264" \| "avc1"` UNCONDITIONALLY to `H264SwDecoder::new` (the hw lane appears there only in a comment, :342-345). product.toml:168 `hw-decode = false` + :191 `hw_decode_policy = "software-only"`; hw/mod.rs:470-476 documents that under the shipped default "nothing is touched: no device node is scanned, no OS decode library is loaded" — and nothing else ever probes. The lane's tests DO run in baseline: this file's 2 tests ok (test-workspace.log.txt:594-595) + ~25 sibling vaapi tests ok (:572-589) including `probe_on_the_real_host_is_the_typed_sandbox_outcome` (the honest sandbox no-/dev/dri outcome — verified: `ls /dev/dri` → No such file or directory). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | full-file read; `rg -rn "vaapi::discover\|VaApiBridge"` outside the lane (callers: hw/mod.rs discover_lane :527-530 + installed_bridges :453-456 only); `rg -rn "negotiate_lane\|installed_bridges\|from_setting"` (no production callers; pipeline_host mentions from_setting in a comment only); sed hw/mod.rs:88-100 (cfg gating), :448-462 (bridge list), :470-485 (SoftwareOnly touches nothing), :520-545 (per-OS discover_lane arms); `ls /dev/dri` (absent — matches the module doc's sandbox claim); sed docs/feature-matrix.toml:2420-2430 (hw-decode row: not_started, empty evidence); baseline log sed :572-595 (all vaapi tests ok); `wc -l` over vaapi/*.rs (3,734-line lane). |

## 1. One-paragraph verdict
This mod file is the honest face of the Linux VA-API lane — a 4-symbol surface (discover/accelerator/bridge + tests) over five sibling modules — and its module doc is the most candid self-assessment in the repo: runtime-tested paths (the probe's no-device and permission-denied outcomes, the real-libva symbol table, the pure reconstruction functions) are separated by name from the "compile-verified only, runtime hardware behavior NOT exercised — [VERIFY]" decode pipeline, with the admission "No GPU exists in this sandbox; no frame was ever decoded by this lane." The glue itself is clean: an accelerator minted only by a successful probe and deliberately stateless, a bridge that claims exactly its own concrete type via the as_any downcast seam and declines everything else, two meaningful tests (both baseline-ok, one pinning the exact Debug string). The problem is strategic, not local: the whole 3,734-line lane — including a hand-rolled unsafe FFI trio (ffi/dl/library) built against an ADR that is still `[PROPOSAL]` — compiles unconditionally into every Linux binary behind no cargo feature, while no shipped code path can ever reach it (the CLI dispatches the software decoder unconditionally; the policy knob that would probe is accepted by config but wired to nothing). That is dead, unsafe-adjacent weight in the shipped artifact, pending a Lead decision that may replace the whole substrate.

## 2. What actually works (verified)
- Both of this file's tests pass in baseline — test-workspace.log.txt:594-595 (`vaapi::tests::accelerator_is_opaque_but_truthful`, `vaapi::tests::bridge_claims_only_its_own_accelerator`), inside the codecs suite.
- The honest-failure claim — verified live in this sandbox: `/dev/dri` does not exist, and the sibling test `probe_on_the_real_host_is_the_typed_sandbox_outcome` (:585) passes, i.e. the probe's no-device path is the actually-executed path, exactly as the module doc (:43-50) claims.
- The downcast seam discipline: `VaApiBridge::open` (:171-174) uses `as_any().downcast_ref::<VaApiAccelerator>()?` — a foreign accelerator returns None (test proves it with an in-test `OtherBackend`), and the session is minted unopened with the node from the accelerator, matching the "bridge claims exactly its own" contract.
- The stateless-accelerator design (probe terminates its display; the session re-opens) is documented at :110-115 and consistent with the sibling code's bring-up split (probe vs session).
- No unsafe in THIS file (the lane's unsafe lives in dl/ffi/library/session — other agents' files).

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/h264/hw/mod.rs:94-95 (the gate) and crates/codecs/src/video/h264/hw/vaapi/mod.rs:25-41 (the substrate decision)
Code:
```
#[cfg(target_os = "linux")]
pub mod vaapi;
```
(:27-30 of this file:)
```
//! ADR-013's option (a)/(b) named `libva` 0.1.5 / `libva-sys` 0.1.2
//! as substrate candidates — both remain **unadmitted** (the ADR
//! is `[PROPOSAL]`; its sign-off gate is not met). This lane
//! implements the no-new-dependency reading: **hand-rolled FFI
//! declarations** ([`ffi`], mirrored from the public libva 2.22.0
```
What is wrong: a 3,734-line lane (`wc -l` over vaapi/*.rs) — including a hand-rolled, size-pinned unsafe FFI surface and a dlopen loader — compiles into EVERY Linux build with no cargo feature gate (`cfg(target_os)` only), while (a) ADR-013, the decision record that authorizes the lane's substrate, is still `[PROPOSAL]`; (b) no shipped code path reaches it (pipeline_host.rs:346 constructs `H264SwDecoder` unconditionally for h264/avc1; `negotiate_lane`/`installed_bridges`/`from_setting` have zero production callers); (c) even a user who sets `hw_decode_policy = "prefer-hardware"` (accepted by product-config, merge.rs:418-419) changes nothing — nothing in the shipped flow consults the policy.
Why it matters: the checklist's "feature flags gating real code that shipped builds never enable", inverted: real (unsafe-heavy) code with NO flag that shipped builds never execute. Binary size, attack surface (dlopen + hand-rolled ABI structs shipped in the release artifact), and maintenance load — all paid every Linux build for a lane awaiting a decision that, per the module's own doc, may replace the entire ffi/dl/library trio. The code being honest about it does not make it reachable.
What correct looks like: a cargo feature (`h264-vaapi`, default-off) or at minimum a `pub use`-only stub until ADR-013 is ratified, so the shipped Linux binary carries the lane only when a consumer exists; alternatively wire the policy setting to the negotiation so the "opt-in Phase-7 path" comment at pipeline_host.rs:342-345 stops describing a path nothing takes.
Confidence: HIGH — Verification: read + caller sweeps (header); policy-settability verified in product-config merge.rs; UNVERIFIED: actual binary size delta (no builds allowed — REQUEST-ORCH-RUN candidate: `cargo bloat` comparison, non-essential).

### F-002 [S4]
Where: docs/feature-matrix.toml:2427-2428 (row `hw-decode` per-OS)
Code:
```
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
evidence = ""
```
What is wrong: the hw-decode row says `not_started` with EMPTY evidence on Linux, while the Linux lane is implemented, compiled into the binary, and carries ~25 passing tests (baseline :572-589). The row's notes (:2429) say "Linux lane designed — ADR-013 [proposed]" — "designed" understates shipped-and-tested-sans-GPU.
Why it matters: conservative drift is still drift — the matrix is the project's calibration instrument, and a row that ignores 3,734 lines of in-tree code teaches readers the matrix is not maintained against the tree (next stop: the same staleness in the inflating direction). The module doc's own standard ("The feature-matrix row says exactly that", :55) is not met by this row.
What correct looks like: `linux = "partial"` with evidence citing the vaapi lane + the honest "[VERIFY] no GPU decode ever exercised" caveat — matching the HEVC row's pattern (feature-matrix.toml:439 `partial` + notes "hw seam delivered, software decode absent").
Confidence: HIGH — Verification: read (row sed-verified; lane and tests verified above).

### F-003 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/mod.rs:93-105
Code:
```
pub fn discover() -> (
    super::capability::HwAvailability,
    Option<Arc<VaApiAccelerator>>,
) {
    let (availability, minted) = session::probe();
    match minted {
        Some((node, vendor)) => {
            let accel = VaApiAccelerator::new(node, vendor);
            (availability, Some(Arc::new(accel)))
        }
```
What is wrong: the return type permits the illegal state `Some(accelerator)` alongside an `Unavailable` availability — the "mint iff probe succeeded" invariant (:85-92) lives entirely in `session::probe()`'s contract (another file) and this doc comment; `discover` itself never cross-checks.
Why it matters: a future `probe()` refactor that returns `(Unavailable, Some(..))` in an edge case silently mints an accelerator off a device that answered no — the exact "never a guessed success" posture (:14) broken by a type that allows it. Small, but this lane's whole identity is typed honesty.
What correct looks like: `Result<Arc<VaApiAccelerator>, HwAvailability>` (the availability IS the error), or at minimum a `debug_assert!` that `minted.is_some() == availability.is_available()`.
Confidence: HIGH — Verification: read (session::probe's contract not re-audited here — session.rs is W06-A09's chunk; flagged as cross-file dependency).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `discover` | 93-105 | REAL | S4 | F-003 tuple-permits-illegal-state; otherwise a clean mint-iff-successed wrapper |
| `VaApiAccelerator` (struct) | 116-121 | REAL | — | deliberately stateless (node+vendor only); doc explains probe/session split |
| `VaApiAccelerator::new` | 125-127 | REAL | — | pub(crate) mint gate ✓ |
| `VaApiAccelerator::node`/`vendor` | 130-139 | REAL | — | pure accessors, must_use |
| `Debug for VaApiAccelerator` | 142-149 | REAL | — | truthful fields, no pointers; exact output pinned by test |
| `impl HwAccelerator` (backend_id/as_any) | 151-159 | REAL | — | "hw.vaapi" label via HwBackend; as_any is the downcast seam |
| `VaApiBridge` (struct) | 164 | REAL | — | unit struct |
| `impl H264HwBridge::backend_id` | 167-169 | REAL | — | "hw.vaapi" |
| `impl H264HwBridge::open` | 171-174 | REAL | — | downcast-or-None correct; mints unopened session bound to the node |
| `tests::bridge_claims_only_its_own_accelerator` | 181-210 | REAL(test) | — | mine-vs-other with an in-test foreign backend — pins the claim/decline contract |
| `tests::accelerator_is_opaque_but_truthful` | 212-236 | REAL(test) | — | pins backend_id, downcast round-trip, and the exact Debug string |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `bridge_claims_only_its_own_accelerator` | YES | a broken downcast (claiming foreign accelerators) or a None-on-mine regression trips the asserts |
| `accelerator_is_opaque_but_truthful` | YES | backend_id typo, as_any identity bug, or Debug field change all trip (the format assert is exact) |
Both ok in baseline (:594-595). No #[ignore]; no mocks of the subject (the foreign backend is a legitimate test double for the *other* side of the seam). Sibling-lane tests (device/library/session) are other agents' files but were confirmed passing (:572-589).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Runtime-tested here: the probe's no-device / permission-denied paths (this sandbox has no /dev/dri)… that path IS the tested path" | vaapi/mod.rs:43-50 | REAL | `/dev/dri` absent (ls); baseline `probe_on_the_real_host_is_the_typed_sandbox_outcome` + `probe_reports_no_device_node_on_a_dri_less_root` + `probe_reports_permission_denied_on_a_locked_node` all ok (:585-589) |
| "Compile-verified only; runtime hardware behavior NOT exercised — [VERIFY] … No GPU exists in this sandbox; no frame was ever decoded by this lane" | vaapi/mod.rs:51-55 | REAL | consistent with the sandbox (no /dev/dri) and with the matrix row's not_started feature status; nothing in any baseline log shows a vaapi frame decode |
| "The feature-matrix row says exactly that" | vaapi/mod.rs:55 | SHALLOW | the row (feature-matrix.toml:2427-2429) says not_started + empty evidence — it does NOT carry the lane's nuanced compile-verified-only state (F-002) |
| wave-6 hostile question | task prompt | Answered for H.264 in sw.rs.audit.md; this lane confirms the negative space: the H.264 decode the shipped binary performs is software-only; the hw lane is unreachable and its absence of GPU execution is honestly labeled |

## 7. Dead / duplicated / unreachable code
- The entire vaapi lane (this mod + device/dl/ffi/library/session, 3,734 lines) is unreachable from shipped entry points (F-001 chain in header); compiled on Linux targets only, unconditionally.
- No duplication within THIS file; the accelerator/bridge shapes mirror the videotoolbox arm by design (per-OS symmetry).
- `discover()`'s only caller is hw/mod.rs discover_lane (Linux arm) — itself caller-less in production.

## 8. Dependencies & flags
- std (fmt, sync::Arc, any), codec-api `HwAccelerator`, super::{H264HwBridge, H264HwDecoder, capability::HwBackend}, sibling modules. NO cargo feature gate — `#[cfg(target_os = "linux")]` only (F-001). No unsafe in this file; the lane's unsafe (dl/ffi) is other agents' files. Lint-clean per baseline clippy.

## 9. Open questions for the next agent
- session.rs owner (W06-A09): does `probe()` ever return `(Unavailable, Some(..))`? (F-003's invariant is yours to confirm or refute.)
- Whoever owns pipeline_host.rs / the settings wiring: `playback.hw-decode-policy` is user-settable (product-config merge.rs:418-419) but the shipped open path never reads it — a "UI settings wired to nothing" candidate from THIS lane's perspective; file it from the host side.
- hw-decode matrix row (matrix-rows agent): F-002 — bring the Linux cell to `partial` with the compile-verified-only caveat, mirroring the HEVC row's honesty pattern.
