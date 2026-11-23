# AUDIT: crates/codecs/src/audio/alac/decoder.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-23 |
| Lines / functions | 884 / 20 (incl. 5 test fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:293-296 `"alac" => self.seeded(track, config, AlacDecoder::new)`; codecs default features include `codec-alac` (crates/codecs/Cargo.toml:47-58), apps/cli/Cargo.toml:51 pulls `codecs` with defaults; also apps/web/src/host.rs (native) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | prebuilt `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::alac::decoder` -> 4 passed; `./target/debug/deps/alac_bitexact-c46b612c74b2627b` -> 1 passed (0.12s); rg over repo + ~/.cargo/registry symphonia sources |

## 1. One-paragraph verdict
This is a REAL symphonia-backed ALAC decoder, not a stub and not hand-rolled entropy decoding. The wrapper owns: magic-cookie parse with typed validation (length/version/depth/channels/frame_length/rate), engine construction, error mapping, 32→16/24/32-bit narrowing, interleaving into a bounded 4-slot SharedBytes pool, and the codec-api Decoder contract. The decode math is `symphonia-codec-alac` 0.6.1 (genuine crates.io Symphonia crate, MPL-2.0, verified in ~/.cargo/registry) invoked via `decode_ref`. The 5-fixture ffmpeg-MD5 differential (tests/alac_bitexact.rs) was EXECUTED by me from the prebuilt binary and passes bit-exactly through the real MP4 demux path. Malformed input is handled with typed errors and explicit bounds checks before every buffer write; I found no panic path on attacker-controlled input.

## 2. What actually works (verified)
- `./target/debug/deps/alac_bitexact-c46b612c74b2627b` -> `test fixtures_bitexact_vs_ffmpeg ... ok. 1 passed` — 5 ffmpeg-generated fixtures (stereo/mono × 16/24-bit + 24-bit white-noise stressor) decode bit-exact vs pinned MD5s through input→demux::mp4→this decoder.
- `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::alac::decoder` -> 4 passed (cookie shapes, typed damage, truncation tripwire, width mapping).
- Baseline corroborates: audit/runs/baseline/cargo-test-workspace.txt:954-970 (bitexact ok, fuzz_survival 3 ok), :445-448 (unit tests ok).
- Engine DoS-gate mirror verified against real engine source: symphonia-codec-alac-0.6.1/src/lib.rs:297-302 `STANDARD_FRAME_SIZE * 16` rejection — wrapper mirrors it at decoder.rs:237.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/alac/decoder.rs:223-228
Code:
```
    if config.bit_depth == 0 || config.bit_depth > 32 {
        return Err(unsupported(
            "alac.config.bit_depth",
            format!("bitDepth {} outside 1..=32", config.bit_depth),
        ));
    }
```
What is wrong: the gate admits every depth in 1..=32, but real ALAC streams are 16/20/24/32; the module's own doc (L116) says "16/20/24/32 in practice". A cookie with bitDepth=5 passes this parse, is handed to the engine, and only fails later as `alac.engine.init` (or, worse, if the engine accepts odd depths, `shift = 32 - bit_depth` narrowing maps a 5-bit stream to S16 with bits dropped — untested territory; engine lib.rs:411 computes `32 - self.config.bit_depth` and I found no engine-side depth whitelist in MagicCookie::read). The wrapper claims to "own the typed gate" (L177-178) but the depth gate is looser than the format. Why it matters: error taxonomy drift and an untested decode regime. What correct looks like: reject depths ∉ {16,20,24,32} with `alac.config.bit_depth`, or document the passthrough deliberately. Confidence: MEDIUM — Verification: read + engine source read; not executed with a depth=5 cookie (marked UNVERIFIED whether engine accepts or rejects it).
F-002 [S4]
Where: crates/codecs/src/audio/alac/decoder.rs:77-79
Code:
```
//! escape width) match as well. The differentials below (stereo 16-bit
//! multi-frame, mono 16-bit, stereo/mono 24-bit — the tail-bits path)
//! are **bit-exact**, so the suspicion is recorded as not reproduced
```
What is wrong: "the differentials below" — there are no differentials below in this file; they live in tests/alac_bitexact.rs (5 fixtures, including a noise fixture this doc omits). A reader auditing this file alone would look for tests that are not there. The claim itself is true (I ran it). Confidence: HIGH — Verification: read + ran.
F-003 [S4]
Where: crates/codecs/src/audio/alac/decoder.rs:435-440
Code:
```
    let need = samples * channels;
    if scratch.len() < need {
        scratch.resize(need, 0);
    }
    let out = &mut scratch[..need];
    decoded.copy_to_slice_interleaved::<i32, _>(&mut *out);
```
What is wrong: `copy_to_slice_interleaved` in symphonia-core-0.6.1/src/audio/util.rs:134-137 `assert!(dst.len() == num_planes * bound.len(), ...)` is a PANIC, not a typed error. The wrapper computes `need` from `decoded.frames()` and `decoded.spec().channels().count()`; if a future engine ever returned a spec whose channel count disagreed with its plane count, this is a process abort rather than the typed-refusal discipline the rest of the file enforces. Currently safe by engine-internal construction (engine builds spec and planes from the same cookie). Confidence: HIGH (assert verified in dep source) — Verification: read.
F-004 [S4]
Where: crates/codecs/src/audio/alac/decoder.rs:755-761
Code:
```
    if slots.len() < SAMPLE_SLOTS {
        slots.push(fresh);
        *next = slots.len() % SAMPLE_SLOTS;
    } else {
        slots[*next] = fresh;
```
What is wrong: eviction `slots[*next] = fresh` silently drops a slot Arc that downstream may still reference (safe — Arc keeps the bytes alive), but the rotation cursor arithmetic (`*next = slots.len() % SAMPLE_SLOTS` after push) and the eviction index are not covered by any test; the in-file tests never exercise `AlacDecoder::decode` on the pooled path at all (only decode_into via the throwaway surface). Pool correctness is proven only indirectly via the integration differential (which does use `decode`). Confidence: MEDIUM — Verification: read; no direct unit test exists.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| skip_cookie_prefixes | 137-153 | REAL | — | Loop provably terminates (each iteration strips ≥12 bytes or hits a strictly smaller `at`); prefix checks bounds-safe (`buf.len() >= 12`) |
| find_alac_atom | 160-172 | REAL | — | Scan capped `min(len-8, 40)`; `at+4..at+8` in bounds; size self-delimit check `at + size == buf.len()` prevents bogus matches |
| parse_alac_config | 183-185 | REAL | — | Thin public wrapper, maps AlacError→EngineError |
| parse_alac_config_inner | 189-250 | REAL | S3 | All fields bounds-checked before read (body is 24/48); see F-001 depth looseness |
| output_sample_format | 256-262 | REAL | — | 0..=16→S16, 17..=24→S24, else S32; unit-tested; exercised by MD5 differential |
| output_format | 266-274 | REAL | — | Interleaved, cookie rate/channels |
| bytes_per | 277-279 | REAL | — | Delegates to SampleFormat |
| max_output_len | 284-287 | REAL | — | frame_length×channels×width; bounded by 65536×8×4=2 MiB cap from config gate |
| engine_for | 310-323 | REAL | — | Builds symphonia engine from stripped cookie body; per-configure cost only |
| map_engine_error | 327-378 | REAL | — | IoError→truncation tripwire incl. `end of bitstream` string match (fragile-but-documented, symphonia verified); non_exhaustive catch-all present |
| decode_into | 392-487 | REAL | S4 | Empty packet typed; zero samples typed; channel mismatch typed; `dst.len() < output_len` typed before writes; narrowing loops write exactly `need` elems via zip (stale slot tail never exposed — caller slices); see F-003 assert |
| write_i24 | 491-502 | REAL | — | Native-endian 24-bit; big-endian branch cfg'd; proven by s24le MD5 rows |
| decode_frame | 513-525 | REAL | — | Free/fuzz surface: parses config + builds throwaway engine per call — documented, not on the hot path |
| AlacDecoder::new | 598-600 | REAL | — | Default |
| AlacDecoder::accepts | 604-606 | REAL | — | `descriptor == "alac"` |
| Decoder::configure | 610-642 | REAL | — | Descriptor gate, mandatory cookie (typed `alac.config.missing`), pool reset; doctest at 539-561 exercises the missing-cookie refusal (baseline :1406 ok) |
| Decoder::decode | 644-700 | REAL | — | Stale-generation drop; empty→NeedMoreInput; pooled fill; pts from samples_out (saturating); duration nanos with rate.max(1); TimeBase::new(1, rate) error mapped; slice 0..exact_len of pooled bytes |
| Decoder::flush | 702-708 | REAL | — | Stateless Drained; BadState if unconfigured — honest |
| pooled_sample_slot | 723-763 | REAL | S4 | Reuse-if-exclusively-owned, else fresh+rotate/evict; fresh `get_mut()` None arm typed BadState (no panic); untested directly (F-004) |
| tests (5) | 765-884 | REAL | — | See §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| cookie_parse_accepts_the_passthrough_shapes | YES | Asserts parsed field values for bare + 12-byte-prefixed shapes and equality across shapes |
| cookie_damage_is_typed_per_rule | YES | Six mutations each assert the exact error code — mutation testing built in |
| truncation_maps_to_the_typed_tripwire | YES | 1-byte packet must produce exactly `alac.frame.truncated` through the real engine |
| output_width_follows_the_bit_depth | YES | Exact tuple equality for 16/20/24/32 |
| tests/alac_bitexact.rs::fixtures_bitexact_vs_ffmpeg (external, this module's real proof) | YES | Pinned ffmpeg MD5s + packet counts + decoded-extent byte lengths + pts monotonicity; any wrapper narrowing/engine drift fails. EXECUTED by me: ok |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "symphonia-codec-alac 0.6.1, exact-pinned ... #![forbid(unsafe_code)] in the crate, MPL-2.0" | decoder.rs:7-13 | REAL | Cargo.toml:115 `symphonia-codec-alac = { version = "=0.6.1", optional = true }`; registry crate Cargo.toml shows MPL-2.0, Symphonia author |
| "The engine ... breaks out of the element loop as soon as all channels are decoded, never reading the trailing 111 END tag" | decoder.rs:81-83 | REAL | Engine lib.rs (decode_inner): `if next_channel >= num_channels { break; }` before END-tag read — verified in dep source |
| "The differentials below ... are bit-exact" | decoder.rs:77-79 | REAL (claim) / misplaced (location) | Differential lives in tests/alac_bitexact.rs; executed, ok — see F-002 |
| "zero per-frame allocation after warm-up" (D-1) | decoder.rs:89-95 | REAL-UNPROVEN | Pool design supports it; no zero-alloc assertion targets ALAC (crates/codecs/tests/zero_alloc_decode.rs — did not verify ALAC inclusion; UNVERIFIED) |
| engine error `io::Error::other("unexpected end of bitstream")` kind `Other` | decoder.rs:331-334 | REAL | symphonia-core BitReader error construction; string-match fallback in map_engine_error is the belt-and-braces path |

## 7. Dead / duplicated / unreachable code
None found. `decode_frame` free surface is used by fuzz/tests (crates/codecs/src/audio/alac/fuzz.rs, tests/alac_fuzz_survival.rs) — not dead. `AlacConfig::rice_pb_mb_kb`/`max_frame_bytes` are diagnostics-only fields (documented as such, L122-127); retained deliberately.

## 8. Dependencies & flags
`codec-alac` feature (crates/codecs/Cargo.toml:61) = dep symphonia-codec-alac + symphonia-core, both `=0.6.1` exact-pinned, both in the crate's DEFAULT feature set (:47-58), which the shipped lumen-cli build enables. symphonia-common/metadata pulled transitively. No unsafe code in this file.

## 9. Open questions for the next agent
- Does the engine accept bitDepth ∉ {16,20,24,32}? (F-001; cheap to test with a hand-built cookie + engine_for.)
- Which registry prints "decoders: mjpeg, pcm_s16le" (baseline mp4-smoke.md:99) while pipeline_host.rs:376 claims 11 landed codecs — the CLI's engine-info lane and the decoder-provider lane disagree; this file is wired in the provider (reachable), so the info lane is the one lying (out of my file scope).
