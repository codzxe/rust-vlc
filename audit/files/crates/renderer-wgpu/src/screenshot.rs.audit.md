# AUDIT: crates/renderer-wgpu/src/screenshot.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2025-06-27 |
| Lines / functions | 245 / 4 fns + 5 tests |
| File verdict | REAL — correct, deterministic, 10 tests executed across two binaries |
| Reachable from shipped entry point? | YES (evidence surface): lumen-cli host.rs:430 `screenshot_png` → pipeline_host.rs:909-918 → renderer readback + this `encode_rgba`; also apps/desktop/src/frame_tap.rs. NOT reachable as the user-facing `video.take-screenshot` command — that command has no host wiring (see F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/renderer_wgpu-cea255c0e8d5c422 screenshot` -> 5/5 ok; `./target/debug/deps/png_hash-cee1d8862e16caad` -> 5/5 ok; rg wiring analysis; sed verification |

## 1. One-paragraph verdict
A genuinely minimal, correct PNG writer: 8-bit RGBA, filter 0, stored (uncompressed)
deflate blocks with correct zlib framing (0x78 0x01 passes the %31 check, LEN/NLEN
complement, BFINAL on the last block, empty-block degenerate case, Adler-32),
per-chunk CRC-32 (bitwise reflected 0xEDB88320), IHDR→sRGB→IDAT→IEND ordering —
all spec-conformant as written, and byte-stable by construction (no compressor
heuristics), which the determinism test pins. I executed both test binaries:
the 5 unit tests (including the classic CRC check value 0xCBF43926 and the
4-block split arithmetic 2+4·5+200050+4) and the 5 png_hash acceptance tests,
all green. Allocation is bounded by the input buffer (checked-multiply with
u64::MAX saturation refusing absurd extents). The two findings are peripheral:
the user-facing `video.take-screenshot` command is catalog-declared with result
"accepted" but no shipped host implements the capture (the honest gap is
documented in playback-engine/src/snapshot.rs, not in the catalog), and
`encode_rgba` silently accepts buffers longer than needed.

## 2. What actually works (verified)
— `./target/debug/deps/renderer_wgpu-cea255c0e8d5c422 screenshot` -> 5/5:
known_crc32_and_adler32_vectors, encoded_stream_has_the_png_shape,
encoding_is_byte_stable_across_calls, size_mismatches_are_typed_errors,
large_streams_split_into_stored_blocks — all ok.
— `./target/debug/deps/png_hash-cee1d8862e16caad` -> 5/5 (whole render→PNG
chain, structurally decoded in-test).
— Wiring proof: rg `encode_rgba` -> apps/cli/src/pipeline_host.rs:918,
renderer-wgpu/tests/png_hash.rs, lib.rs docs.

## 3. Findings
F-001 [S3]
Where: crates/commands/src/catalog/spec.rs:395-402 (declaration);
apps/cli/src/dispatch.rs (absence); crates/playback-engine/src/snapshot.rs:29-36 (the honest gap)
Code:
        CommandSpec {
            id: CommandId::VideoTakeScreenshot,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("path: Option<PathBuf>"),
            result: PayloadDesc::Type("accepted"),
        },
What is wrong: the command catalog declares `video.take-screenshot` (class A,
result "accepted") and the control-protocol policy lists it, but no shipped
host dispatches it — rg over apps/cli/src finds zero take-screenshot handling;
the capture execution is the documented "structural gap" in
playback-engine/src/snapshot.rs ("Execution — reading pixels back from the
presenting renderer — is the structural gap recorded below"). This file's
encoder is real and reachable, but only through the headless evidence surface
(host.screenshot_png) and the desktop frame tap. Why it matters: a control
client reading the catalog believes screenshots work; the planning half
(path/format policy) exists, the capture does not. What correct looks like:
either wire the command to the existing readback+encode path or mark the
command unimplemented in the catalog.
Confidence: HIGH — Verification: rg dispatch/catalog/snapshot + read.
F-002 [S4]
Where: crates/renderer-wgpu/src/screenshot.rs:35-44
Code:
    let expected = u64::from(extent.width)
        .checked_mul(u64::from(extent.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .unwrap_or(u64::MAX);
    if (rgba.len() as u64) < expected {
What is wrong: only the SHORTER-than-needed case is refused; a longer buffer is
accepted and its tail silently ignored (no error, no length echo). Lenient
contract, documented as "shorter than" in the doc comment — but a readback
returning stride-padded rows would silently produce a correct-looking PNG with
garbage rows instead of a typed error. What correct looks like: exact-length
requirement (the readback path this serves produces tight buffers).
Confidence: MEDIUM — Verification: read (no padded-readback caller executed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| encode_rgba | 28-92 | REAL | S4 | spec-correct; longer-buffer leniency (F-002); bounded alloc |
| push_chunk | 95-103 | REAL | — | length+type+data+CRC |
| crc32 | 106-119 | REAL | — | bitwise, vector-pinned 0xCBF43926 |
| adler32 | 122-131 | REAL | — | per-byte modulo (slow, documented non-hot-path) |
| tests (5) | 138-244 | REAL | — | executed (§2) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| known_crc32_and_adler32_vectors | Yes | external check values (CRC-32 check value, Adler "Wikipedia") |
| encoded_stream_has_the_png_shape | Yes | exact byte offsets for signature/IHDR/sRGB/IEND |
| encoding_is_byte_stable_across_calls | Yes | determinism pin |
| size_mismatches_are_typed_errors | Yes | exact error values |
| large_streams_split_into_stored_blocks | Yes | IDAT length arithmetic 200_076 — catches block-split regressions |
Plus png_hash (5 tests, separate file): structural PNG decode + committed
sha256 with documented anti-tautology chain (pattern code-specified,
value-transparency asserted, in-test decoder).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "byte-stable by construction (no compressor heuristics, no entropy coding)" | L5-7 | REAL | stored blocks only; determinism test executed |
| "The stream declares itself sRGB (the `sRGB` chunk, perceptual intent)" | L8-10 | REAL | chunk emitted before IDAT (spec order), test asserts offset |
| "~140 lines, no dependencies" | L13-14 | REAL | std-only, file is 245 lines total incl. 110 lines of tests |

## 7. Dead / duplicated / unreachable code
None dead. The encoder is live via the evidence surface; the interactive
command that would be its natural consumer is unwired (F-001).

## 8. Dependencies & flags
crate error/params types only; std-only. No unsafe, no features. renderer-wgpu
is a dependency of apps/cli + apps/desktop (Cargo.toml proof in run log).

## 9. Open questions for the next agent
- Whoever audits apps/cli/dispatch.rs or the commands catalog: how does the
  host answer an incoming `video.take-screenshot` today (typed not-wired, or
  silence)? That determines whether F-001 is an honest refusal or a lie.
- The desktop frame_tap's final-frame.png flow: does the tauri build (which
  fails in this sandbox at gdk-sys) exercise encode_rgba at runtime?
  UNVERIFIED (environment limit, per baseline).
