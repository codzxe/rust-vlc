# AUDIT: crates/codecs/src/video/mjpeg/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 146 / 1 harness + 1 sink + 4 tests |
| File verdict | REAL harness (both decode surfaces per call), honest wiring-status docs; harness structurally blind to persistent-decoder state bugs |
| Reachable from shipped entry point? | Not shipped itself (fuzz surface); its target `fuzz_codec_mjpeg` is registered (registry check CLEAN, ran live: 35 targets) but "compile-ready, not built by any gate" (nightly wiring owed). The MjpegDecoder it drives IS shipped (CLI "mjpeg" row); the in-crate tests + tests/fuzz_survival.rs (ran live, 6/6) exercise this harness on stable |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 mjpeg::fuzz` → 4 passed; exit 0; `bash scripts/fuzz_registry_check.sh` → CLEAN (35 targets, corpora present), exit 0; cat fuzz/fuzz_targets/fuzz_codec_mjpeg.rs |

## 1. One-paragraph verdict
Each `fuzz_codec_mjpeg` call runs the input through BOTH surfaces — the
free `decode_image` (full marker→entropy→IDCT→assembly pipeline) and the
codec-api contract path (fresh configure → decode → flush with a
counting sink) — which is more than most harnesses bother with, and the
module docs are scrupulously honest that the libFuzzer campaign itself is
owed nightly wiring (target file exists, calls this harness verbatim, is
not built by any gate). Two structural weaknesses: the contract pass
constructs a FRESH decoder per call, so cross-packet persistent-state
damage (exactly the huffman.rs rebuild-poisoning class found in this
wave) can never be caught by this harness; and the pass-2 early returns
are silent, so if mjpeg configure ever began failing, the contract-path
fuzz coverage would vanish without any signal. The in-crate smoke tests
(4, ran live) plus the committed-corpus survival run (fuzz_survival.rs,
6/6 live) carry the current evidence load.

## 2. What actually works (verified)
— verified by `./target/debug/deps/codecs-bae0f1756cc0a809 mjpeg::fuzz` →
`test result: ok. 4 passed; 0 failed` (empty, SOI-only, marker soup with
absurd lengths + 4 KiB 0xFF tail, truncated valid stream → concealment
path). Registry wiring verified by `bash scripts/fuzz_registry_check.sh`
→ "CLEAN — 35 targets, diff empty, corpora present" (exit 0); the target
shim `fuzz/fuzz_targets/fuzz_codec_mjpeg.rs` calls this harness
verbatim (read).

## 3. Findings

F-001 [S4]
Where: crates/codecs/src/video/mjpeg/fuzz.rs:72-76
Code:
```rust
    let mut decoder = MjpegDecoder::new();
    let config = DecoderConfig::new(descriptor, generation, ThreadBudget::default(), None, None);
    if decoder.configure(&config).is_err() {
        return;
    }
```
What is wrong: (a) a fresh decoder per call means multi-packet state
machines are never fuzzed — the persistent-table poisoning found at
huffman.rs/decoder.rs (W06-A05 huffman audit F-001: failed DHT rebuild
leaves a hybrid table that mis-decodes the NEXT packet) is structurally
unreachable by this harness; no input sequence can cross packets.
(b) the early `return` on configure error is silent — no counter, no
assert, no eprintln: if `MjpegDecoder::configure` ever regressed to
failing with no extra_data, every contract-path fuzz call would quietly
degrade to pass 1 only, and the survival suites would still pass.
What correct looks like: a persistent-decoder mode feeding
sequence-dependent mutations, and a debug_assert/canary that pass 2 ran.
Confidence: HIGH — Verification: read + ran (the state-blindness is
structural; huffman F-001 corroborates the blind spot is real).

F-002 [S4]
Where: crates/codecs/src/video/mjpeg/fuzz.rs:41-54
Code:
```rust
impl FrameSink for CountingSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames = self.frames.saturating_add(1);
        Ok(())
    }
}
```
What is wrong: `frames` is incremented but never read anywhere — the
counter is dead state. Harmless, but it suggests an intended
emission-count check that was never written.
Confidence: HIGH — Verification: read + rg (no reader).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CountingSink | 41-54 | REAL | S4 | F-002 dead counter |
| fuzz_codec_mjpeg | 59-90 | REAL | S4 | both surfaces; F-001 fresh-decoder + silent returns |
| 4 tests | 96-145 | REAL | - | ran live; honest smoke shapes |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 4 | PARTLY (by design) | absence-of-panic contract only; outcome regressions belong to the AC suite (documented at :7-10) |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = one full untrusted-JPEG pass through both decode surfaces" | :5-8 | REAL | read + live run |
| "Never panics, never hangs, never allocates without a ceiling" | :56-58 | REAL-UNPROVEN | held over live corpus runs (fuzz_survival 6/6, alac-style); no libFuzzer campaign exists to sustain it adversarially |
| "the libFuzzer campaign itself is owed CI wiring" | :14-23 | REAL (honest gap) | target exists, registry CLEAN, but no gate builds it (target's own WIRING STATUS comment) |
| "proven ... by the committed-corpus survival run" | :19-23 | REAL | fuzz_survival.rs ran live 6/6 this session |
## 7. Dead / duplicated / unreachable code
CountingSink.frames (F-002). The cargo-fuzz target is intentionally
dormant (wiring owed) — documented, not dead by neglect.
## 8. Dependencies & flags
codec-api + media-types (std-only); libfuzzer-sys only inside the dormant
fuzz/ workspace. No feature gates.
## 9. Open questions for the next agent
- fuzz_loop_stable.rs (referenced by the target's comment, card W1-12):
does IT feed persistent decoders across iterations? If not, the whole
mjpeg fuzz story is single-shot only.
- The hevc/h264/av1 fuzz targets (fuzz_codec_hevc.rs etc.) — same fresh-
decoder shape? (hevc fuzz module exists at
crates/codecs/src/video/hevc/fuzz.rs, not assigned to me.)
