# AUDIT: crates/codecs/src/video/h264/hw/vaapi/session.rs#part4
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-21 |
| Lines / functions | ~1600–2134 of 2135 (chunk part4 of 4; part1 ≈1–533 W06-A06, part2 ≈534–1067 W06-A08, part3 ≈1068–1599 W06-A09) / 14 tests + 2 helpers + probe_with tail |
| File verdict | UNREACHABLE (real code, no shipped entry point reaches it; see §Reachability) |
| Reachable from shipped entry point? | NO. Chain: `VaApiSession` ← `VaApiBridge` (`hw/mod.rs:453-460`, `#[cfg(target_os="linux")] installed_bridges()`) ← `H264Decoder::from_setting` (`decode.rs:310`) ← **no production caller** — only `crates/codecs/tests/h264_hw_path.rs:861,897,911,956` call `from_setting`; the shipped lane is `pipeline_host.rs:347` `"h264" \| "avc1" => self.seeded(track, config, H264SwDecoder::new)`; `product.toml:169` `hw-decode = false` + `hw_decode_policy = "software-only"`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "^(pub…fn|impl|struct…)" session.rs` (structure map); `rg -rn "VaApiSession\|from_setting\|negotiate_lane" --type rust crates/ apps/` (reachability, zero production callers); `rg -n "vaapi::session::tests" audit/runs/baseline/test-workspace.log.txt` (14/14 `ok`, lines 579-593); `sed -n` on every quoted range |

## 1. One-paragraph verdict
Part4 is the tail of `probe_with` plus the entire `#[cfg(test)] mod tests`. The tests are the pure-reconstruction layer (ref-list building per §8.2.3 including frame-num wrap, reordering ops, coded-size math, slice/picture parameter packing with hand-computed bitfield goldens) and the honest probe paths (no-device, permission-denied, sandbox reality) — all 14 passed in the baseline workspace run. The file's own header is honest that no test touches a driver and the driver-mediated decode path is compile-only `[VERIFY]`. But the entire lane is dead code in the shipped product: `H264Decoder::from_setting` has zero production callers, the shipped `lumen` CLI dispatches `H264SwDecoder` directly, and `product.toml` keeps `hw-decode = false` / `software-only`. The strongest finding is a test whose name promises the parameter-set refusal while the exercised path never reaches it.

## 2. What actually works (verified)
— All 14 tests: `rg -n "vaapi::session::tests" audit/runs/baseline/test-workspace.log.txt` → 14 `ok` entries (579–593), zero ignored.
— Ref-list math executed on host: `frame_num_wrap_demotes_older_cycle_pictures`, `wrap_encoded_reorder_target_resolves` (real §8.2.3 wrap arithmetic, hand-verified expectations).
— Probe failure paths executed: `probe_reports_no_device_node_on_a_dri_less_root`, `probe_reports_permission_denied_on_a_locked_node` (fixture temp root, `probe_with(&DriScanner::with_root(&root))`).

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:2089-2112
Code:
```
    fn session_configure_without_parameter_sets_is_typed() {
        let mut session = VaApiSession::with_root(
            DriNode {
                name: "renderD128".into(),
```
What is wrong: The test's name promises "configure without parameter sets is typed", but with a temp root containing no `/dev/dri`, `configure` fails at device bring-up before the parameter-set check is ever reached — the test asserts only `code.starts_with("h264.hw.vaapi.")` on whatever first failure occurs. The comment (2098-2101) admits the short-circuit.
Why it matters: Mutation test: make `pick_parameter_sets`/`picture_parameter` panic (`unwrap` on missing SPS) instead of returning a typed error — this test still passes because bring-up fails first. The named property (parameter-set refusal typing) is untested anywhere in the file; coverage claimed by the name does not exist.
What correct looks like: A unit test on `pick_parameter_sets(&H264HwStreamInfo::default())` → `None` (pure function, no device needed), or a session with an injected fake display/library seam so configure reaches the parameter-set branch.
Confidence: HIGH — Verification: read; `rg -n "pick_parameter_sets" session.rs` → defined 1237, exercised by no test in this file.
### F-002 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:2067-2087
Code:
```
        let (availability, minted) = probe();
        match availability {
            HwAvailability::Unavailable { reason, .. } => {
                if !std::path::Path::new("/dev/dri").exists() {
```
What is wrong: On any host WITH `/dev/dri`, every arm of this match accepts: `Unavailable` with any reason passes (the assert is conditional on the path not existing), `Available` passes on non-empty detail. The test is shape-only on hardware hosts; only the no-DRI sandbox gets a real assertion.
Why it matters: The test cannot fail on a GPU CI runner, so it guards nothing where the lane would actually be exercised; its name ("the typed sandbox outcome") oversells a sandbox-specific check.
What correct looks like: `#[cfg]`-split the expectation, or skip the test on DRI-bearing hosts explicitly with a recorded reason rather than silently passing.
Confidence: HIGH — Verification: read.
### F-003 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:1598-1599 (probe_with tail, boundary-straddling with part3/W06-A09)
Code:
```
    let node = node.clone();
    let via = node.name.clone();
```
What is wrong: `via` is cloned from `node` one line before `node` itself is cloned; `via` could be taken from the original before the clone, or `node.name` used directly in the format string at 1603 — one redundant heap clone on the probe path.
Why it matters: Trivial waste on a cold path; mainly a readability smell (two names for one value).
What correct looks like: `let via = node.name.clone(); let node = node.clone();` ordering is unavoidable for the return, but `detail: format!("VA-API {major}.{minor} via {}: {vendor}", node.name)` after the clone avoids the second binding entirely.
Confidence: HIGH — Verification: read.

## 4. Per-function table (part4 range)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `probe_with` (tail) | 1598-1607 | REAL | — | Boundary straddle; bulk (1438–1597) is part3/W06-A09. `Available` minting returns `(node, vendor)` only on success — no accelerator off a failed probe. F-003 applies. |
| `dpb_pic` (helper) | 1621-1629 | REAL | — | Test-only DPB entry constructor. |
| `profile_mapping_covers_the_served_profiles_only` | 1631-1672 | REAL | — | Covers Main/High/Constrained-Baseline/High-10-refused; asserts `va_profile_for` exactly (fn at 123-137, part1's range). Baseline→MAIN mapping is asserted as designed; whether MAIN is the right libva profile for plain baseline is part1's question. |
| `initial_ref_list_orders_short_term_desc_then_long_term_asc` | 1674-1711 | REAL | — | Short-term desc by PicNum, long-term asc by LongTermFrameIdx; flags (`SHORT_TERM_REFERENCE`/`LONG_TERM_REFERENCE`) and `frame_idx` checked. |
| `frame_num_wrap_demotes_older_cycle_pictures` | 1713-1728 | REAL | — | §8.2.3 wrap: PicNum 15−16=−1 demoted below 2; comment honestly records the r3 scenario fix (r2's expected order was unsatisfiable — scenario bug, not code bug). |
| `reorder_ops_reorder_and_reflag` | 1730-1771 | REAL | — | idc 0/1/2 covered; idc 1 re-flags SHORT_TERM→LONG_TERM with wrapped LongTermPicNum (9+4=13 % 16) — non-obvious arithmetic actually asserted. |
| `wrap_encoded_reorder_target_resolves` | 1773-1784 | REAL | — | abs_diff 27 → PicNum 12 across the wrap from CurrPicNum 0. |
| `missing_target_and_mvc_ops_are_typed_errors` | 1786-1808 | REAL | — | MissingTarget{pic_num:2} and MvcViewOp typed refusals; no panics on hostile ops. |
| `coded_size_is_macroblock_math` | 1810-1842 | REAL | — | (3+1)·16=64 × (2+1)·16=48; field doubling to 96; `u32::MAX` width → typed None (no wrap). |
| `slice_parameter_carries_the_parsed_facts` | 1844-1908 | REAL | — | Golden `slice_data_bit_offset == 41` ("8 header bits + 33 RBSP bits") and every field incl. deblocking offsets; override semantics checked second pass. |
| `picture_parameter_packs_the_bitfield_words` | 1910-2008 | REAL | — | seq_fields/pic_fields bitfield goldens computed by hand (0x1|0x10|0x40|(1<<8)|(2<<12)); `va_reserved == [0;8]` ABI rule; comment records the r2 golden's own bug (1993-1994) — honest. |
| `probe_reports_no_device_node_on_a_dri_less_root` | 2010-2032 | REAL | — | Fixture root without dri → `NoDeviceNode`, `minted.is_none()`. Executed in baseline. |
| `probe_reports_permission_denied_on_a_locked_node` | 2034-2065 | REAL | — | chmod-000 renderD128 → `PermissionDenied`; skipped under root via `euid_is_root` (unprivileged-only truth, comment says CI note). Executed in baseline. |
| `probe_on_the_real_host_is_the_typed_sandbox_outcome` | 2067-2087 | SHALLOW | S4 | See F-002: real assertion only on no-DRI hosts; shape-only elsewhere. |
| `session_configure_without_parameter_sets_is_typed` | 2089-2112 | SHALLOW | S3 | See F-001: exercises bring-up failure, never the named parameter-set refusal. |
| `va_api_lane_backend_label_is_stable` | 2114-2121 | REAL | — | Trivial label contract check; correct and content-sensitive. |
| `euid_is_root` (helper) | 2123-2133 | REAL | — | Parses `/proc/self/status` Uid line; Linux-only, same as the whole module (`#[cfg(target_os="linux")]` at hw/mod.rs:94). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 12 of 14 | YES | Hand-computed goldens (bitfields, wrap arithmetic, byte offsets) or executed fixture probes with typed-equality asserts. |
| `session_configure_without_parameter_sets_is_typed` | NO (for its named property) | Bring-up short-circuits before the parameter-set logic; only the "some h264.hw.vaapi.* code" property is guarded (F-001). |
| `probe_on_the_real_host_is_the_typed_sandbox_outcome` | NO on GPU hosts | Any well-formed outcome passes; only the sandbox arm asserts (F-002). |
| Run status | 14/14 `ok`, baseline `test-workspace.log.txt:579-593` | None ignored. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NO test here touches a driver … the driver-mediated path is [VERIFY] compile-only" | session.rs:1609-1613 | REAL | The test module contains zero driver-mediated calls; all FFI-touching tests stop at `probe`'s no-device/permission failures. |
| "Runtime-tested here: the probe's no-device / permission-denied paths … every pure reconstruction function" | vaapi/mod.rs:30-36 | REAL | Baseline lines 579-593; tests at 2010-2065 construct exactly those paths. |
| hw lane reachable when policy enabled | product.toml:169 `hw-decode = false`, `hw_decode_policy = "software-only"` | FALSE (today) | `H264Decoder::from_setting` has no production caller (`rg` over crates/ apps/ → tests only). The policy knob exists in product-config (model.rs:137, merge.rs:418-419) but nothing in the shipped pipeline consults `HwDecodePolicy` to build a decoder. |

## 7. Dead / duplicated / unreachable code
- **The entire lane is UNREACHABLE from shipped entry points**: `lumen` CLI (`pipeline_host.rs:347` sw-lane only) and `lumen-tauri` (→ `desktop` → `lumen-cli` lib, same host). Compiled (Linux), tested for failure paths, but no production caller constructs `H264Decoder`/`VaApiBridge`/`negotiate_lane`.
- Cross-part: `installed_bridges()` (hw/mod.rs:450) and `negotiate_lane()` (hw/mod.rs:483) are `pub` with no production callers — same verdict family.

## 8. Dependencies & flags
- Module compiled only on Linux: `#[cfg(target_os = "linux")]` (hw/mod.rs:94). Release workflow builds Linux/macOS/Windows lanes (release.yml:193, 474, 627, 763) — the Linux artifact carries this code, dead at runtime under the shipped product config.
- No Cargo feature gates the hw modules (in-house, unconditional per codecs/Cargo.toml comment); `codec-h264-sw` gates only `rust_h264` (sw path).

## 9. Open questions for the next agent
- W06-A09 (part3): `probe_with` maps `DriDiscovery::Unreadable` to `NoDeviceNode` (1453-1464) — vocabulary lacks an Io variant; confirm whether any consumer distinguishes "absent" from "unreadable" (information loss, likely S4).
- Whoever audits `crates/codecs/tests/h264_hw_path.rs`: does any test there cover `pick_parameter_sets` returning None → typed configure refusal (the gap F-001 identifies)?
