# WAVE 08 SUMMARY — audio-output, platform backends, filters

| Field | Value |
|---|---|
| Agents | A01–A10 all complete |
| Files audited | 62 files, 62/62 covered |
| Findings | S0:2 S1:3 S2:15+ S3:~50 S4:~90 |
| Citation gate | automated; 0 fabrications |
| Tree purity | clean |

## THE ANSWER: how does a user hear sound?
THEY DON'T. The shipped CLI/desktop render into NullBackend
(pipeline_host.rs:1599) — a discard device. The "real" integrations are:
- WASAPI: real COM FFI but enumeration/notifications ONLY (no
  IAudioClient/IAudioRenderClient) — and platform-windows is an ORPHAN
  crate with zero dependents (W08-A01, S0).
- PulseAudio: real dlopen'd client (20 symbols, 8 signatures executed vs
  installed libpulse 0.24.3) for enumeration/notifications only — no
  output; orphan crate (W08-A03/A10).
- CoreAudio: real FFI, orphan crate; ELEMENT_MAIN=1 contradicts Apple's
  kAudioObjectPropertyElementMaster=0 in every property address (S1)
  (W08-A02).
- cpal.rs: STUB — flipping AUDIO_ADR005_ADMITTED alone breaks the build;
  flip+delete-assert => unreachable!() panic; zero cpal code behind the
  seam (W08-A04).
- device.rs enumerate: zero callers (W08-A09).
Filters: EQ/Gain/Mute/Balance real math, zero consumers (Wave 7 cross-ref);
no video filters found anywhere in this domain (the §8.8 filter question:
video filters are PAPER-ONLY).

## Worst findings
1. [S0] dxgi.rs vtables omit the 4 inherited IDXGIObject slots —
   get_desc1 actually calls EnumOutputs (wrong arity, garbage register);
   GPU seam returns Ok(vec![]) on every real Windows machine with errors
   swallowed (W08-A10, arithmetic proof HIGH confidence).
2. [S0] platform-windows orphan crate: real WASAPI FFI reachable from
   NOTHING (W08-A01).
3. [S1] com.rs PropVariant pinned at 16 bytes; x64 PROPVARIANT is 24 —
   8-byte stack under-write domain under GetValue/PropVariantClear
   (W08-A10).
4. [S1] coreaudio ELEMENT_MAIN=1 vs master=0 (W08-A02); [S1] dxgi
   null-write crash path (W08-A09).
5. [S2 cluster] pulse.rs ownership leaks (ctx/ml/op + dangling callback
   userdata on pump-error), subscribe discards success flag, poll never
   checks context state (W08-A03); pump.rs raw u64 - underflow at
   ADR-005-flip; dialogs.rs probe() missing CoInitializeEx => dialogs can
   never work on real Windows (W08-A05); null.rs written-consumed
   invariant false under underrun (W08-A04); metal.rs "released on Drop"
   with no impl Drop — leaks MTLDevice refs (W08-A09).
6. [SYSTEMIC S2 x6] platform-{linux,windows,macos} are ALL orphan crates
   (zero workspace dependents; release lane never builds them) — ~5k+ LOC
   of real FFI unreachable from every shipped lane (W08-A05/A06/A07/A10).
   Triplicated lib/events/audio files across the three crates.
7. [Test honesty] pcm_pipeline.ts "decoded frames" header FALSE (sine
   pattern, pts always Missing, capture Vec sink) — proves nothing about
   user-audible audio (W08-A08); callback_budget op-proof real, allocation
   structural-only, WCET absent.

## Cross-refs
- Wave 18: matrix rows claiming audio output/device support per platform.
- Wave 11: product.toml audio-output=true vs this reality.
- Wave 19: doc 11's SEC-07 TLS override already proven PAPER-ONLY (W2).
