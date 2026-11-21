# AUDIT: crates/media-types/src/error.rs
| Field | Value |
|---|---|
| Auditor | W02-A08 |
| Date | 2026-11-21 |
| Lines / functions | 538 / 21 (incl. 6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `playback-engine` `EngineEventKind::Fatal` → `crates/playback-engine/src/event.rs:541` `error.message_summary()` → `kind_summary`/`Display` in this file; baseline run output shows the rendered string) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | `rg -F -c '<Variant>'` over apps/crates/tools for every enum variant (usage counts, dead-variant proof); `rg 'ErrorContext::new'` (263 hits/119 files — taxonomy is the real error spine); `rg -oN 'ErrorContext::new\("([^"]*)"' -r '$1'` + shape filter (all 141 unique literals conform, only test's "Bad Code" fails); `rg -n 'recovery' product.toml` (no match — refutes doc comment); `rg -n 'Decoder \{ codec' docs/01-architecture.md` (spec line 1275); `sed -n` verification of every quote below |

## 1. One-paragraph verdict
This file is the genuine engine-wide error taxonomy: 263 `ErrorContext::new` call sites across 119 files, all 7 kinds and 13 of 14 stages minted by production code, `Display`/`message_summary` executed in the baseline MP4 smoke run (`event: error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)` — audit/runs/baseline/mp4-smoke-open-session.txt, INDEX.md §"interactive open"). The design is disciplined: structured payloads only, sniffed bytes capped at 16, source chain delegated to `std::error::Error`, constructor contract enforced by `assert` on a `&'static str`. The defects are drift, not fakeness: five taxonomy variants are minted by no code on earth (locked/auth/disc/WASM-memory/GPU-adapter), the `Decoder` kind silently dropped doc 01 §3.4's `codec` payload so decoder failures cannot name their codec, and the module doc points at `product.toml` recovery budgets that do not exist in `product.toml`.

## 2. What actually works (verified)
— `kind_summary` + `message_summary` executed in baseline: audit/runs/baseline/mp4-smoke-open-session.txt prints `event: error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)`; the message tail `unsupported in h264 (h264.sw.unsupported)` is exactly `format!("{} ({})", kind_summary(&kind), code)` with `FormatInfo { name: Some("h264") }` (lines 305-307, 315-329). Verified by reading + baseline cite.
— Sniffed-byte cap: `media-probe/src/error.rs:26` constructs `sniffed: window[..window.len().min(16)].to_vec()` — bounded at both construction and Display (line 322). Verified by `rg -n "sniffed" crates/media-probe/src/error.rs`.
— `ErrorContext::new` shape guard: all 141 unique repo literals extracted via `rg -oN 'ErrorContext::new\("([^"]*)"' -r '$1'` pass the lowercase-dotted shape; the only violator is this file's own intentional test literal `"Bad Code"` (line 535). Verified by extraction + regex filter.
— `std::error::Error::source` delegation (lines 399-405) — the `with_source` chain root survives to standard tooling; `Display` never prints `source`, so underlying `io::Error` text (potentially containing paths) cannot leak into the no-secrets rendering. Verified by reading.

## 3. Findings
### F-001 [S3]
Where: crates/media-types/src/error.rs:103-110 (plus 183 and 151)
Code:
```
    /// Another process holds an exclusive lock.
    Locked,
    /// The source demands authentication the engine does not hold.
    AuthenticationRequired,
    /// The physical disc is missing from the drive.
    DiscMissing,
```
(and line 183 `    WasmMemory,` / line 151 `    Adapter,`)
What is wrong: Five taxonomy variants — `AccessDenied::Locked`, `AccessDenied::AuthenticationRequired`, `AccessDenied::DiscMissing`, `LimitKind::WasmMemory`, `DeviceSlot::Adapter` — are constructed by ZERO production or test code anywhere in apps/, crates/, tools/ (`rg -F` count 0 outside this file; inside this file only the `DiscMissing` in the display test, line 436). Doc 01 §3.4's "representative combinations" table promises exactly these: "Open | Inaccessible | … missing disc", "Plugin | ResourceLimit | WASM extension exceeded memory/CPU budget", "Present | Device | GPU/adapter lost" (docs/01-architecture.md:1290-1296), "each becomes a fixture in `testkit`" — no such fixture exists (`crates/media-types/tests/` contains no error fixtures).
Why it matters: The taxonomy advertises diagnosability (locked files, auth-required streams, missing discs, WASM memory ceilings, GPU adapter loss) that the engine can never report; a locked file or auth demand will surface as some other kind (or a raw `Inaccessible::PermissionDenied`), and doc 01's fixture claim is unbacked. `LimitKind::WasmMemory` at least has a product.toml knob (`max_wasm_extension_memory_mib`, product.toml:208) waiting for Phase 7; the other four have nothing.
What correct looks like: Either mint these variants at their real sites (`input`/`streaming` mapping lock/auth io errors to `Locked`/`AuthenticationRequired`, disc probe to `DiscMissing`, renderer-wgpu adapter-loss to `DeviceSlot::Adapter`) or delete them and shrink doc 01's table to what exists.
Confidence: HIGH — Verification: read + `rg -F -c` across apps/crates/tools (counts pasted in log).

### F-002 [S3]
Where: crates/media-types/src/error.rs:201-202 (spec: docs/01-architecture.md:1275)
Code:
```
    /// A codec failed to decode its bitstream.
    Decoder { cause: DecodeCause },
```
What is wrong: Doc 01 §3.4's normative enum for this exact type is `Decoder { codec: CodecDescriptor, cause: DecodeCause }` (docs/01-architecture.md:1275). The `codec` payload was dropped; `kind_summary` renders only "decoder: bitstream error" / "decoder: hardware decoder lost" / "decoder: invalid state transition" (lines 361-365) with no codec identity.
Why it matters: A decoder failure cannot say which codec failed from the structured kind; the only codec identity lives in the free-form `context.code` string minted per site. Doc-code drift on the error surface the whole repo is told (doc 01 §3.4) to build against.
What correct looks like: Add `codec: CodecDescriptor` (or `codec: &'static str`) to the variant and include it in `kind_summary`'s Decoder arm, or amend doc 01 §3.4 to the shipped shape.
Confidence: HIGH — Verification: read (doc line + code line both quoted verbatim above).

### F-003 [S3]
Where: crates/media-types/src/error.rs:9-11 (and 167-169)
Code:
```
/// The pipeline stage that produced an error (doc 01 §3.4). Fourteen
/// stages — the same names the boundary rules (doc 01 §1.1) and the
/// recovery budgets (product.toml) speak.
```
(and lines 167-169: "The limits themselves live in `product.toml` (MP §5/§12); this names the axis.")
What is wrong: `product.toml` (244 lines, greped for `recovery|probe|memory|cpu|queue|wasm|session`) contains NO recovery budget keys at all — `rg -n 'recovery' product.toml` exits 1 (no match). The actual recovery budgets are hard-coded in `crates/playback-engine/src/recovery.rs` ("A bounded retry budget", line 125). For `LimitKind`, product.toml carries only `probe_max_bytes` (205), `queue_packets_audio/video` (203-204) and `max_wasm_extension_memory_mib` (208); there is no product.toml `memory`, `cpu` or recovery-attempt budget. The `LimitKind` doc even contradicts itself: `SessionGenerations` is documented "structural, not tunable" (line 184) while the enclosing doc says the limits "live in product.toml".
Why it matters: Two doc comments in the canonical types file direct readers to a configuration surface that does not exist; doc 01 §3.4 repeats the false claim ("each stage has a configurable recovery budget (product.toml — MP §5)"). Tuning recovery via config is impossible today — the budgets are compiled in.
What correct looks like: Point the docs at the real homes (`recovery.rs` constants; product.toml only for probe/queue/wasm knobs) or move the budgets into product.toml as claimed.
Confidence: HIGH — Verification: read + `rg` on product.toml (zero matches) and recovery.rs.

### F-004 [S4]
Where: crates/media-types/src/error.rs:233-239
Code:
```
    /// # Panics (constructor contract)
    /// This is the crate's one constructor-contract panic: a malformed
    /// *code* is a programming error, not runtime data. The workspace
    /// lint set denies `panic!` in production code, so this uses `assert`
    /// on an invariant that only Lead-reviewed call sites can reach —
    /// every in-crate call site passes a literal.
```
What is wrong: "Every in-crate call site passes a literal" is false as stated: in-crate sites `crates/media-types/src/time.rs:715`, `ids.rs:137`, `ids.rs:196` pass a `code` variable (each bound only to literals, so the invariant holds transitively), and 88 out-of-crate sites pass `&'static str` parameters. No static tool validates error-code literals — `xtask lint-literals` covers only identity/url/abs-path/hex-color classes (tools/xtask/src/cmd/lint_literals.rs:13-18) — so a future uppercase typo in an untested error path panics exactly when that error fires.
Why it matters: The stated mitigation ("only Lead-reviewed call sites") is process assertion, not mechanism; the doc overstates the guarantee that makes the panic safe.
What correct looks like: Rephrase the doc to the real invariant (the `&'static str` parameter admits only compile-time literals; 141/141 current literals conform) and/or extend `lint-literals` with an `error-code` class enforcing the regex statically.
Confidence: HIGH — Verification: read + `rg -n 'ErrorContext::new'` pass-through sites + literal extraction (all 141 conform today).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum PipelineStage` (14 variants) | 13-42 | REAL | — | All 14 variants minted somewhere in apps/crates/tools (rg counts: Open 68, Decode 116, Demux 89 … Transcode 8). Matches doc 01 §3.4 list exactly. |
| `PipelineStage::as_str` | 46-63 | REAL | — | Straight match; verified against test + wire `fatal_details` (event.rs:730). No panic paths. |
| `impl Display for PipelineStage` | 66-70 | REAL | — | Delegates to `as_str`. |
| `struct FormatInfo` | 74-81 | REAL | — | 113 uses repo-wide; `sniffed` bounded to 16 at construction (media-probe/src/error.rs:26). |
| `enum CorruptDetail` (5) | 84-96 | REAL | — | All variants minted (InvalidStructure 59 files, Truncated 53, Timing 7, BadIndex 2, ChecksumMismatch 4). |
| `enum AccessDenied` (5) | 99-111 | REAL | S3 | 3 of 5 variants never minted — F-001. |
| `enum NetworkCause` (5) | 115-128 | REAL | — | All minted (streaming crate; ConnectionLost 13, ReconnectExhausted 10, Tls 5, Dns 4, Timeout 5). |
| `enum DecodeCause` (3) | 131-141 | REAL | — | All minted (BadState 21 files, Bitstream 10, HardwareLost 4). |
| `enum DeviceSlot` (3) | 144-152 | REAL | S3 | `Adapter` never minted — F-001. |
| `enum DeviceCause` (4) | 155-165 | REAL | — | All minted. |
| `enum LimitKind` (7) | 170-186 | REAL | S3 | `WasmMemory` never minted (Phase 7, wasm-extensions=false); doc-claim drift — F-001/F-003. |
| `enum ErrorKind` (7 kinds) | 189-214 | REAL | S3 | All 7 minted (Unsupported 136 files … Device 13). `Decoder` lost doc 01's `codec` payload — F-002. |
| `struct ErrorContext` | 222-225 | REAL | — | Single `code: String` field; Hash+Eq derived. |
| `ErrorContext::new` | 240-252 | REAL | S4 | `assert!` panic on malformed literal (documented contract, `#[allow(clippy::panic)]`); all 141 current literals conform; doc wording overstated — F-004. |
| `struct EngineError` | 258-264 | REAL | — | Fields public; no `kind()` accessor needed. |
| `EngineError::new` | 268-275 | REAL | — | Trivial constructor; no panic paths. |
| `EngineError::with_source` | 280-286 | REAL | — | Replaces (not chains) source — documented L0 semantics. |
| `impl Display for EngineError` | 289-297 | REAL | — | Structured data only; never prints `source` (no-secrets rule upheld by construction); no panic paths. |
| `EngineError::message_summary` | 305-307 | REAL | — | Production caller playback-engine/src/event.rs:541 (wire PlaybackError); executed in baseline smoke (INDEX.md §"interactive open"). |
| `kind_summary` (private) | 313-397 | REAL | — | Pure formatting; sniffed cap at 16 (line 322); executed in baseline (`unsupported in h264`); no panics, no ignored params. |
| `impl Error for EngineError::source` | 399-405 | REAL | — | Correct double-deref cast to `&(dyn Error + 'static)`. |

## 5. Test honesty (in-file tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `all_seven_kinds_display_structured_summaries` (414) | YES | Exact-string assertions on all 7 kind summaries; any wording/format mutation fails. |
| `display_is_stage_bracketed_with_code` (474) | YES | Full `to_string()` equality including `[audio-output]` prefix and code suffix. |
| `sniffed_bytes_are_capped_at_sixteen_in_display` (490) | YES | 4096-byte sniffed input; if the `min(16)` cap (line 322) were removed, `"ab "` appears 4095 times and `<= 16` fails. (Minor: asserts `count() <= 16`, actual capped count is 15 — slack of one, still catches mutation.) |
| `source_chain_delegates_to_std` (508) | YES (weakly) | Only asserts `source().is_some()`; would catch a None-returning impl but not a mis-chained one. |
| `stage_names_are_stable_lowercase` (523) | PARTIAL | Spot-checks 3 of 14 stages; a rename of e.g. `AudioFilter`'s string to "audiofilter" passes. Low stakes (full list verified by this audit instead). |
| `context_codes_enforce_their_shape` (530) | YES | `catch_unwind` proves the guard fires on "Bad Code"; also asserts the accepted shape round-trips. (Noise: the intentional panic prints to stderr during the run.) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "kinds are exactly the seven MP names: unsupported, corrupt, inaccessible, network, decoder, device, resource limit" | docs/01-architecture.md:1262; MASTER-PROMPT.md:203; error.rs:2-3 | REAL | ErrorKind 190-214 has exactly these 7; all minted by production code (rg counts in §4). |
| "the recovery budgets (product.toml) speak [the stage names]" | error.rs:9-11 (citing doc 01 §3.4) | FALSE | `rg -n 'recovery' product.toml` → 0 matches; budgets hard-coded in playback-engine/src/recovery.rs:125. F-003. |
| "The limits themselves live in product.toml (MP §5/§12)" | error.rs:167-169 | FALSE (partial) | product.toml has probe/queue/wasm knobs only (203-208); no memory/cpu/recovery keys. F-003. |
| `Decoder { codec: CodecDescriptor, cause: DecodeCause }` | docs/01-architecture.md:1275 | FALSE (shape) | Code is `Decoder { cause: DecodeCause }` (error.rs:202). F-002. |
| "each [representative combination] becomes a fixture in testkit" | docs/01-architecture.md:1289-1296 | FALSE | No error fixtures in crates/media-types/tests/; 3 of the 9 table combos need variants that are never minted (F-001). |
| "Display never embeds credentials, cookies, auth headers or full user paths" | error.rs:4-6; doc 01 §3.4 no-secrets rule | REAL | Display (289-297) renders only stage + structured kind summary + code; `source` never printed; URIs only via `ids.rs::SourceUri::redacted` (ids.rs:243, adversarially unit-tested per its docs). |
| Unsupported media surfaces as a typed error with known-format info | doc 01 §3.4 / MP §8 | REAL | baseline mp4-smoke-open-session.txt: `event: error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)`. (The exit-code-0 half of the "never reported as success" rule is a CLI defect owned by apps/cli — INDEX.md standing defects.) |

## 7. Dead / duplicated / unreachable code
Dead taxonomy variants (never constructed anywhere): `AccessDenied::Locked` (105), `AccessDenied::AuthenticationRequired` (107), `AccessDenied::DiscMissing` (109), `LimitKind::WasmMemory` (183), `DeviceSlot::Adapter` (151) — F-001. No duplicated logic; no `#[allow]` besides the single documented `clippy::panic` (239); no unreachable functions.

## 8. Dependencies & flags
Crate has no external dependencies (only `std`). Reachability chain: apps/cli/Cargo.toml:57 (`media-types = { path = ... }`), crates/playback-engine/Cargo.toml:38, crates/demux/Cargo.toml:29. No feature gates in this file. Built clean in baseline run #1 (`cargo build --release --locked --workspace` exit 0, INDEX.md row 1); this crate's tests ran in runs #3/#4 (workspace pass).

## 9. Open questions for the next agent
- Who SHOULD mint `AccessDenied::Locked` — check `crates/input` error mapping for Windows sharing-violation / POSIX advisory-lock handling; if locked files map to `PermissionDenied`, that's the concrete drift site.
- `crates/commands` (next file in my queue) — does its `ErrorCode` enum mirror these 7 kinds 1:1, and does any code there re-derive kind strings instead of using `message_summary`?
- Does any UI string table key on `context.code` (compatibility-surface claim, error.rs:220-221)? If ui/ hard-codes a code that later gets renamed, nothing detects it.
