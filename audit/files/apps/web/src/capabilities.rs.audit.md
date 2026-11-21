# AUDIT: apps/web/src/capabilities.rs
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-21 |
| Lines / functions | 440 / 16 (10 impl items + 5 tests + 1 cfg-block) |
| File verdict | UNREACHABLE from shipped entry points (otherwise REAL: executed in baseline tests and, per the committed record, in a real Chromium) |
| Reachable from shipped entry point? | NO — nothing depends on `lumen-web` (rg over apps/desktop-tauri, apps/desktop, apps/cli, apps/control-server Cargo.tomls → zero matches; packaging scripts are desktop-only: deb/rpm/appimage/dmg/msi/targz/zip). YES inside the wasm module: `host.rs:837` builds it, `host.rs:951-958` patches the renderer row, `host.rs:1023` ships it in the `Booted` handshake reply; `open.rs:182-186` cites its rows for typed refusals; `apps/web/worker.js` carries the boot handshake. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rg `capabilities\|HostCapabilities` in apps/web/src → wiring chain above; sed quotes verified; arm-by-arm comparison of `wasm32_builtin_descriptors` (capabilities.rs:193-220) vs `builtin_decoder_for` arms (host.rs:591-677) → names match (mjpeg, pcm_ prefix, adpcm_ prefix, flac, mp3/mp1/mp2, vorbis, aac/aac-lc, alac, opus, vp9, dnxhd); rg `lumen-web` over app Cargo.tomls → none; grep worker.js → boot/capability-snapshot handshake present; baseline: `capabilities::tests::*` 5/5 ok (orchrun-test-workspace-nofailfast.log.txt:3110-3114); `cargo xtask web-budget --check` PASS (INDEX #16 — the gate REBUILDS the wasm module, so the cdylib builds today); size-report.md five real wasm32 builds 2026-11-11; browser-matrix-20261110.md — real Playwright Chromium run, engine boots, WebGL2 negotiated, 10/17 scenarios. |

## 1. One-paragraph verdict
This is the honest-capability machinery the project advertises: a typed blocked-row table whose entries the open path really cites (`open.rs:182-186` refuses `http(s)` with the `http-streaming` row), a single-threading truth rule, and a decoder list that matches the host's actual dispatch table arm-for-arm (verified). It is also genuinely executed: five unit tests green in baseline, the wasm module rebuilds under the web-budget gate, and the committed browser-matrix record shows the snapshot flowing through a real Chromium boot handshake. The catches: nothing ships it — no app depends on `lumen-web` and no packaging lane produces a web artifact, so the whole "honest report" is a report about a product that does not exist; and the descriptor design leaks its lack of kind information into a real bug one file over (`host.rs:728` classifies the video decoders vp9/dnxhd as Audio in codec-carrying builds), plus the usual literal-list drift seams the file's own comments disclaim.

## 2. What actually works (verified)
— Baseline `capabilities::tests::{blocked_row_lookup_answers_by_name, decoder_claims_match_the_profile, doc_09_table_rows_are_typed_data, snapshot_serializes, threading_is_single_on_both_profiles} ... ok` (orchrun-test-workspace-nofailfast.log.txt:3110-3114).
— Blocked-row/open-path consistency: `open.rs:182-186` refuses `http(s)` sources citing capability `http-streaming` with the same card reference as the row at capabilities.rs:272-278 — one source of truth, verified in both directions.
— The wasm module carrying this file builds: baseline INDEX #16 `cargo xtask web-budget --check` PASS (the gate re-measures `docs/web/size-record.toml` rows with a real `cargo build -p lumen-web --target wasm32-unknown-unknown`); size-report.md records five real builds (floor 1,294,311 raw / 300,250 gzip bytes).
— Real-browser execution: docs/web/browser-matrix-20261110.md — chromium lane `playwright-run` (REAL), "the engine core boots, negotiates WebGL2, answers typed refusals", wasm artifact sha256 recorded; wasm artifact byte size matches the size-report floor row exactly.
— rav1d claim verified: size-report.md "rav1d 1.1.0 does not compile for wasm32-unknown-unknown (39 errors, measured 2026-11-10)".

## 3. Findings

### F-001 [S3] (cross-file, root in host.rs) The descriptor list carries no kind, and its one consumer classifies video decoders vp9/dnxhd as Audio
Where: apps/web/src/host.rs:726-733 (defect), consuming `decoder_descriptors` from this file (capabilities.rs:156-161, 217-220)
Code:
```rust
            .map(|name| DecoderInfo {
                kind: if name == "mjpeg" {
                    commands::catalog::types::TrackKind::Video
                } else {
                    commands::catalog::types::TrackKind::Audio
                },
```
What is wrong: `HostCapabilities.decoders` is a flat `Vec<String>` with no kind information, so `engine_info` (the "same truth rule as hardware decode" surface, module doc lines 3-5) guesses kind by name and guesses wrong: with `codec-vp9` or `codec-dnx` enabled, `decoder_descriptors` returns "vp9"/"dnxhd" (video decoders, wired at host.rs:662-677 to `Vp9Decoder`/`DnxDecoder`), and engine_info reports `kind: Audio` for them.
Why it matters: The capability snapshot is the product's honesty surface; an opt-in codec build — the exact configuration this file exists to describe honestly (test doc 352-357: "the snapshot must not claim decoders the module cannot construct, nor hide decoders an opt-in build really carries") — gets wrong metadata in the shipped wire form of `system.get-engine-info`.
What correct looks like: Descriptors as `(name, kind)` pairs (or a `DecoderDescriptor` struct) sourced from the same table the dispatch arms use, so kind is data, not a name heuristic.
Confidence: HIGH — Verification: read both sites; arms and descriptor list compared entry-by-entry.
### F-002 [S4] `decoder_descriptors(Native)` ignores the codec-* features — a native opt-in build hides decoders it really carries
Where: apps/web/src/capabilities.rs:156-161
Code:
```rust
pub fn decoder_descriptors(profile: TargetProfile) -> Vec<String> {
    match profile {
        TargetProfile::Native => vec!["mjpeg".to_string(), "pcm_s16le".to_string()],
        TargetProfile::Wasm32 => wasm32_builtin_descriptors(),
    }
}
```
What is wrong: On native, `builtin_decoder_for`'s per-feature arms (host.rs:617-677: flac, mp3/mp1/mp2, vorbis, aac/aac-lc, alac, opus, vp9, dnxhd) compile under `#[cfg(feature = "codec-*")]` regardless of target, so a native `--features codec-flac` build dispatches flac — but the Native arm of `decoder_descriptors` is hard-coded to mjpeg+pcm and hides it. The module's own rule (test doc 355-357) forbids exactly this hiding; the test only checks the Wasm32 profile (line 362), so the native hole is untested.
Why it matters: Native lumen-web is the lane-proof configuration; its snapshot lies on feature builds. Blast radius limited: native lumen-web is a test lane, not a shipped product (nothing depends on it).
What correct looks like: `TargetProfile::Native => wasm32_builtin_descriptors()` (the same feature-selected list — the cfg split only matters for the codecs dependency, not the reporting), or a native-specific list that also reads the features.
Confidence: HIGH — Verification: read (host.rs arms' cfg conditions traced; they are feature-gated, not target-gated).
### F-003 [S4] "one list, one truth, mirrored by host.rs's dispatch table" — a mirror is two lists; nothing enforces the sync
Where: apps/web/src/capabilities.rs:163-165
Code:
```rust
/// The feature-selected builtin set for a wasm32 build (empty for the
/// shipped decoder-less module; grown per opt-in feature — one list,
/// one truth, mirrored by host.rs's dispatch table).
```
What is wrong: The descriptor list (capabilities.rs:193-220) and the dispatch arms (host.rs:591-677) are two hand-maintained lists kept in step only by developer discipline; no test or compile-time construct ties them. The fallback refusal in host.rs:683-689 interpolates `decoder_descriptors(profile).join(", ")` into "no decoder for `{other}` in this build (landed: …)" — on drift, the typed refusal itself misstates what landed. Today they match (verified arm-by-arm), and the Native hard-coded pair (F-002) is the same pattern the baseline already flagged as a defect class on the CLI's `engine_info` ("hard-coded literal … not derived from live state", INDEX standing defects).
Why it matters: The file's honesty guarantee rests on a comment, not a mechanism.
What correct looks like: One table (name, kind, feature gate) consumed by both the dispatch match and the snapshot — e.g. a `const` registry the arms index into, plus a test asserting every dispatched name appears in the descriptors and vice versa.
Confidence: HIGH — Verification: read + arm-by-arm comparison (currently consistent).
### F-004 [S4] `Threading::as_str` and `CapabilityStatus::as_str` are dead outside their own tests, and their docs claim to be "the wire string"
Where: apps/web/src/capabilities.rs:43-52, 79-89
Code:
```rust
    /// The snapshot's wire string (doc 09 §3.3's vocabulary:
    /// `single` | `pooled(n)`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Threading::Single => "single",
        }
    }
```
What is wrong: The actual wire form is serde-derived (`#[serde(rename_all = "camelCase")]` / `kebab-case` on the serialized snapshot — `snapshot_serializes` asserts `"threading":"single"` from serde, not from `as_str`). Repo-wide, the only callers of these two `as_str` methods are the module's own test (line 348); no production path renders them.
Why it matters: Dead public API whose doc claims the wire role — a maintainer "fixing" the wire by editing `as_str` would change nothing, and the two vocabularies can drift silently (they happen to agree today: kebab-case of `NotStarted` = "not-started" = `as_str`).
What correct looks like: Either delete both methods, or make the serde renames route through them (e.g. serialize via `as_str`), so there is one vocabulary.
Confidence: HIGH — Verification: rg `as_str()` across apps/web — only backend.rs/present.rs/source.rs use *their own* `as_str`s; capabilities' two are test-only.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Threading` (enum) | 35-41 | REAL | — | single variant; honest (pooled not claimed). |
| `Threading::as_str` | 43-52 | REAL | F-004 | test-only caller. |
| `BlockedCapability` (struct) | 56-65 | REAL | — | serialized camelCase; consumed by open.rs typed refusals. |
| `CapabilityStatus` (enum) | 68-77 | REAL | — | Unsupported/Blocked/NotStarted; kebab-case serde matches as_str. |
| `CapabilityStatus::as_str` | 79-89 | REAL | F-004 | test-only caller. |
| `HostCapabilities` (struct) | 93-113 | REAL | — | threading/renderer/audio_output/decoders/blocked; renderer patched at boot (host.rs:951-958). |
| `HostCapabilities::blocked_row` | 115-122 | REAL | — | linear find; used by open.rs:188 and the tests. |
| `TargetProfile` (enum) | 124-134 | REAL | — | Native/Wasm32. |
| `build_profile` | 136-144 | REAL | — | `cfg!(target_arch = "wasm32")`. |
| `decoder_descriptors` | 146-161 | REAL | F-002, F-003 | Native hard-coded pair (honest for default native dispatch — host.rs has no h264 arm — but feature-blind). |
| `wasm32_builtin_descriptors` | 163-225 | REAL | F-003 | cfg-correct (no-features → empty); the `#[cfg_attr(not(any(...)), allow(unused_mut))]` is precise; names match host.rs arms (verified). |
| `capabilities` | 227-334 | REAL | — | 9 base rows + wasm32-conditional decoder row (two cfg'd reasons); rows cite doc 09 and cards; statuses consistent with open.rs refusals (verified for http-streaming, rtsp, filesystem). |
| `tests::threading_is_single_on_both_profiles` | 343-350 | REAL | — | mutation-sensitive for the truth rule. |
| `tests::decoder_claims_match_the_profile` | 358-393 | REAL | — | YES it would fail on list breakage; blind spots: native+features (F-002), kind (F-001). |
| `tests::doc_09_table_rows_are_typed_data` | 397-417 | REAL | — | 4 named rows + non-empty reasons; would fail on removal. |
| `tests::blocked_row_lookup_answers_by_name` | 421-430 | REAL | — | incl. negative lookup. |
| `tests::snapshot_serializes` | 433-439 | REAL | — | pins the serde wire strings. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| threading_is_single_on_both_profiles | YES | asserts enum + wire string on both profiles. |
| decoder_claims_match_the_profile | YES (for what it covers) | exact vec equality on native, empty/first checks on wasm per feature; does NOT cover native+features (F-002) nor kind (F-001). |
| doc_09_table_rows_are_typed_data | YES | named-row presence + non-empty reasons both profiles. |
| blocked_row_lookup_answers_by_name | YES | positive + negative lookup. |
| snapshot_serializes | YES | pins `"threading":"single"` and the decoder-gap row in the JSON. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Nothing here is aspirational: every field names the build's real state" | module doc 3-8 | REAL (with F-001/F-002 exceptions) | rows verified against open.rs refusals and host.rs dispatch; kind metadata wrong for vp9/dnxhd builds (F-001), native feature builds hide decoders (F-002). |
| "the wasm module ships decoder-less" / rav1d not wasm32-buildable | module doc 20-26; size-report.md | REAL | size-report.md 39-error measurement; web-budget gate re-builds the floor (baseline #16 PASS); feature table has no codec-av1. |
| "both profiles' DATA is unit-tested below — the browser runtime itself is `[VERIFY]`" | module doc 27-29 | REAL | 5 tests green (baseline :3110-3114); browser runtime since verified once: browser-matrix-20261110.md chromium playwright-run (10/17 scenarios, engine boots, typed refusals). |
| matrix web column = 0/252 | xtask parity-report (INDEX #9) | consistent | nothing ships the web lane; the parity report's web column counts no shipped capability — matches F-reachability here. |
| `http-streaming` NotStarted "card 7.H4" | row 272-278 + open.rs:182-186 | REAL | the io/ tree (fetch_port, seekability, staged_source) is an unwired model (io/mod.rs:40-41 "models the transport"); the open path refuses http(s) citing this row. |

## 7. Dead / duplicated / unreachable code
- `Threading::as_str`, `CapabilityStatus::as_str` — production-dead (F-004).
- The whole module is unreachable from shipped entry points (no dependent on `lumen-web`; desktop-only packaging) — the wave question "does anything ship apps/web?" answers NO: the wasm artifact exists (real builds, real Chromium record) but no packaging lane, CI release job, or app embeds it. CI's `wasm-build` job builds only the `theme-tokens` canary (ci.yml:548-577), not `lumen-web`.

## 8. Dependencies & flags
- `serde`/`serde_json` (wire family), `testkit` (dev). Feature gates: `builtin-codecs`, `codec-{flac,mp3,vorbis,aac,alac,opus,vp9,dnx}` — all off by default; the shipped wasm floor enables none (decoder-less), consistent with the snapshot. `#[cfg_attr(..., allow(unused_mut))]` at 180-192 is a narrowly-scoped, justified allow (verified: `mut` only needed under external gates).
- Baseline evidence the wasm module builds with this file: INDEX #16 (web-budget --check PASS, which rebuilds the floor row).

## 9. Open questions for the next agent
- A04/A05/A06 (host.rs owners): F-001 lives in your file (engine_info kind heuristic, host.rs:726-733) — please file/merge with your part's findings; also host.rs:719 hard-codes `first-vertical:avi-mjpeg-pcm` features list — same literal-list class.
- Who owns `docs/web/size-record.toml` drift: the record was measured at commit 30153fa (w24/a); web-budget --check passed in baseline at e83e5bd, so the floor row still reproduces — but the per-codec rows are `rebuild = false` and unverified since.
- Is any web deployment lane planned to ship (product.toml `[identity.ids.web]` PWA identity exists, `ui/scripts/web-matrix-run.mjs` stages a site) — if yes, the capability snapshot finally reaches a user; if no, this whole crate is evidence-only.
