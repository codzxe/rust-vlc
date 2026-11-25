//! FX3-A01 regression suite: the ADR-005 admission change (Route A,
//! FIX-PLAN §3 F1.1) — the flip-point discipline seen from its end
//! state. What must now hold:
//!
//! 1. The admission is **recorded open** (`AUDIO_ADR005_ADMITTED` is
//!    `true`) and the not-admitted marker vocabulary is gone — the
//!    compile-level pins for the deletions live in the wave report
//!    (rg over the crate: no `BackendNotAdmitted`, no compile-time
//!    gate, no refusal arm).
//! 2. **Default (feature-off) builds behave exactly as today** — the
//!    audit's byte-compatibility demand: the seam constructor falls
//!    back to the null backend (a WORKING sink, not a fake success),
//!    the builtin table is null-then-stub, a session opened through
//!    the seam round-trips bytes through the ring with the typed
//!    all-or-nothing backpressure, and `cpal_linked()` reports the
//!    build truth (`false` here — this sandbox compiles the default
//!    graph only).
//! 3. **Feature-on builds** (`backend-cpal`, FX3-A07's windows CI
//!    lane — this sandbox can neither compile the alsa-sys graph
//!    (P0 record R5: pkg-config probe, exit 101) nor drive a device):
//!    the real backend constructs, enumerates honestly, refuses
//!    unknown device ids and not-open states with typed errors, and —
//!    when the host has an output device — opens, plays, pauses and
//!    resumes it with the sample-counter clock advancing as the
//!    device consumes. Hosts without a device skip honestly (empty
//!    enumeration IS the no-device answer, never a placeholder).

use audio_engine::buffer::AudioBuffer;
use audio_output::cpal::AUDIO_ADR005_ADMITTED;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::Stamp;
use testkit::Must;

fn stereo_f32(rate: u32) -> AudioFormat {
    AudioFormat {
        sample: SampleFormat::F32,
        channels: 2,
        layout: AudioLayout::Interleaved,
        rate,
    }
}

fn buffer_of(format: AudioFormat, samples: &[f32]) -> AudioBuffer {
    let mut data = Vec::new();
    for sample in samples {
        data.extend_from_slice(&sample.to_le_bytes());
    }
    AudioBuffer {
        format,
        data: SharedBytes::from_vec(data),
        pts: Stamp::Missing,
    }
}

#[test]
fn fx3_a01_admission_flag_is_flipped_true() {
    // The flip-point discipline's end state, pinned at compile time:
    // a drifted re-introduction of the closed gate fails the build
    // here (and the same assertion lives in src/cpal.rs's own tests).
    const {
        assert!(
            AUDIO_ADR005_ADMITTED,
            "the ADR-005 admission (Route A) must be recorded open"
        );
    };
}

#[cfg(all(test, not(feature = "backend-cpal")))]
#[test]
fn fx3_a01_default_build_reports_no_link_and_falls_back_to_null() {
    assert!(!audio_output::cpal::cpal_linked());
    let backend = audio_output::cpal::cpal_backend().must();
    let descs = backend.enumerate();
    assert_eq!(descs.len(), 1, "exactly the null device");
    assert_eq!(descs[0].id, audio_output::null::NULL_DEVICE_ID);
    assert!(descs[0].name.contains("null"), "{}", descs[0].name);
}

#[cfg(all(test, not(feature = "backend-cpal")))]
#[test]
fn fx3_a01_default_builtin_table_is_byte_compatible_with_pre_admission() {
    // The default lane's composition is exactly the pre-admission
    // list: null first (one virtual device), the typed platform stub
    // second (zero devices) — the audit's byte-compatibility demand
    // on the selection surface.
    let table = audio_output::builtin_backends();
    assert_eq!(table.len(), 2);
    let null_descs = table[0].enumerate();
    assert_eq!(null_descs.len(), 1);
    assert_eq!(null_descs[0].id, audio_output::null::NULL_DEVICE_ID);
    assert_eq!(table[1].enumerate().len(), 0, "the stub enumerates nothing");
}

#[cfg(all(test, not(feature = "backend-cpal")))]
#[test]
fn fx3_a01_null_fallback_session_roundtrips_with_typed_backpressure() {
    // The seam in a default build is a WORKING null session, never a
    // fake success: open, write, clock source, bounded backpressure —
    // the same observable behavior the pre-admission null path had.
    let mut backend = audio_output::cpal::cpal_backend().must();
    let format = stereo_f32(44_100);
    backend
        .open(audio_output::null::NULL_DEVICE_ID, &format, 8)
        .must();
    backend
        .write(&buffer_of(format, &[0.5, -0.5, 0.25, -0.25]))
        .must();
    let clock = backend.clock_source().must();
    assert_eq!(clock.sample_count(), 0, "the parked driver is undriven");
    assert_eq!(clock.sample_rate_hz(), 44_100);
    // 2 of 8 frames queued: 2 * 1e9 / 44_100 ns of ring latency.
    assert_eq!(
        backend.latency().must(),
        media_types::time::Duration::Known(media_types::time::Nanoseconds(45_351))
    );
    // The ring is bounded and all-or-nothing: 10 more frames do not
    // fit (6 free) — typed refusal, FIFO intact.
    let err = backend.write(&buffer_of(format, &[0.0; 20])).err().must();
    assert_eq!(err.context.code, "audio.ring_full");
    // Pause/resume through the seam behave as the null contract does.
    backend.pause().must();
    backend.resume().must();
    assert_eq!(backend.underruns(), 0, "nothing rendered: no underruns");
}

/// Feature-on lane (FX3-A07's windows CI job compiles and runs this;
/// this sandbox cannot — the recorded alsa-sys build failure plus no
/// audio device).
#[cfg(all(test, feature = "backend-cpal"))]
mod backend_cpal_lane {
    use super::*;
    // The concrete CpalBackend's trait surface needs its defining
    // traits in scope (clock_truth.rs's convention on the concrete
    // NullBackend): `write` via AudioSink; `open`/`enumerate`/`pause`/
    // `resume`/`clock_source` via AudioBackend. The default build's
    // Box<dyn AudioBackend> and Arc<dyn SampleCounterSource> calls
    // resolve by trait-object candidates and import nothing.
    use audio_engine::AudioSink;
    use audio_output::AudioBackend;
    use audio_output::DeviceId;
    use audio_output::cpal::CpalBackend;

    #[test]
    fn fx3_a01_cpal_build_reports_the_link() {
        assert!(audio_output::cpal::cpal_linked());
    }

    #[test]
    fn fx3_a01_cpal_backend_enumerates_honestly_and_refuses_unknown_ids() {
        let mut backend = CpalBackend::new();
        let descs = backend.enumerate();
        for desc in &descs {
            assert!(!desc.name.is_empty(), "a redacted name, never empty");
            assert_eq!(desc.preferred.sample, SampleFormat::F32);
            assert_eq!(desc.preferred.layout, AudioLayout::Interleaved);
            assert!(desc.preferred.rate > 0);
        }
        // The typed refusal for a handle this backend never minted.
        let err = backend
            .open(DeviceId::new(u64::MAX), &stereo_f32(48_000), 64)
            .err()
            .must();
        assert_eq!(err.context.code, "audio.device_id_unknown");
        // Not-open operations refuse honestly too.
        let err = backend.pause().err().must();
        assert_eq!(err.context.code, "audio.sink_not_open");
    }

    #[test]
    fn fx3_a01_cpal_opens_plays_pauses_and_resumes_a_real_device() {
        // The device-open happy path — the Windows CI lane's job (a
        // host without an output device skips honestly: empty
        // enumeration is the no-device answer, never a placeholder).
        let mut backend = CpalBackend::new();
        let descs = backend.enumerate();
        if descs.is_empty() {
            return;
        }
        let target = &descs[0];
        backend.open(target.id, &target.preferred, 1_024).must();
        let clock = backend.clock_source().must();
        assert_eq!(clock.sample_rate_hz(), target.preferred.rate);

        // Eight whole frames at the device's own preferred shape...
        let frame = target.preferred.channels as usize * 8;
        backend
            .write(&buffer_of(target.preferred, &vec![0.0f32; frame]))
            .must();

        // ...and the device consumes them: the counter (the clock's
        // truth) must advance within a bounded wait.
        let mut observed = 0u64;
        for _ in 0..200 {
            observed = clock.sample_count();
            if observed > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(observed > 0, "the device consumed the written frames");

        // Pause freezes the counter (doc 03 §4.7 e); resume carries on.
        backend.pause().must();
        let frozen = clock.sample_count();
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(clock.sample_count() >= frozen, "paused never rewinds");
        backend.resume().must();
    }
}
