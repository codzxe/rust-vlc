//! FX4-A05 — the audio path's end-to-end observability (FIX-PLAN §3
//! F1.1's test lane): the decoded PCM walking the shipped chain —
//! `open_audio_session` → `AudioLimb::stage` (the engine's mix + the
//! runtime master chain) → the null ring → the pump's device renders —
//! lands in the counting null ledger (FX3-A05's `SinkStats`), and the
//! live controls FX4 wires into the session (volume / mute / balance /
//! EQ) are observable in the rendered samples themselves.
//!
//! Two depths, split honestly:
//!
//! - **Session level** — the deepest lane that can OBSERVE both the
//!   discard ledger and the rendered samples: the real
//!   `open_audio_session` construction (the host's own open path),
//!   decoder-output-shaped PCM frames staged through the real limb,
//!   the pump's own render loop, the null device's own buffer. The AVI
//!   fixture's PCM bytes are read from the fixture file itself, so
//!   the asserted totals are the media's own facts, never hand-picked
//!   constants. The host-level lane cannot observe either (the
//!   `CliHost` keeps its `NullDevice` private and opens capture-less,
//!   and the pump renders into pump-local buffers) — the deepest
//!   feasible integration stops before the demuxer/decoder stages,
//!   which the neighboring suites already pin end to end.
//! - **Host level** — the wire verbs `audio.set-volume`/`set-mute`
//!   ride the real `lumen_dispatch` on a real `CliHost` session
//!   (FX4-A02's routing — the refusal this wave replaces), and the
//!   session plays its media to the exact end through the live
//!   controls.
//!
//! The controls seam is coded against the wave's frozen contract
//! (FX4-A01): a public live-control entry named `audio_controls` on
//! the pipeline host's audio session machinery, driving the same
//! runtime master chain the engine-side tests (fx3_a03) pinned.

use audio_engine::Volume;
use audio_output::null::NullDevice;
use commands::catalog::types::SourceSpec;
use commands::envelope::{CommandOutcome, RequestId};
use commands::{Command, CommandResult, PlaybackState, WireRequest};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::{CliHost, RUN_PUMPS};
use lumen_cli::pipeline_host::{open_audio_session, AudioSession, RENDER_QUANTUM};
use media_clock::clock::PlaybackClock;
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::ids::GenerationId;
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, Timestamp};
use testkit::Must;

/// The fixture rate for the plain tone lanes (the first vertical's own
/// rate).
const RATE: u32 = 44_100;
/// The EQ lane's rate (mirrors fx3_a03's own EQ lane).
const EQ_RATE: u32 = 48_000;
/// Every session here is stereo (the fixture's and the lanes' shape).
const CHANNELS: u32 = 2;

/// The audio-bearing fixture every neighboring suite uses (the
/// pipeline-accounting AC's own parent): MJPEG + PCM, 2.000 s, 320x240
/// @ 30 fps, stereo s16le 44.1 kHz.
fn fixture_path() -> String {
    format!(
        "{}/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Open the real audio session (the host's own open path — the exact
/// `open_audio_session` call host.rs runs when a media's audio track
/// survives selection).
fn stereo_session(rate: u32) -> AudioSession {
    open_audio_session(CHANNELS, rate, 0).must()
}

/// One decoder-output-shaped frame: stereo s16le interleaved (the PCM
/// lane's decode output — the shape `AudioLimb::stage` receives from
/// the engine's consumer).
fn stereo_s16_frame(pairs: &[(i16, i16)], pts_ticks: i64, rate: u32) -> AudioFrame {
    let mut bytes = Vec::new();
    for (l, r) in pairs {
        bytes.extend_from_slice(&l.to_le_bytes());
        bytes.extend_from_slice(&r.to_le_bytes());
    }
    AudioFrame {
        format: AudioFormat {
            sample: SampleFormat::S16,
            channels: CHANNELS,
            layout: AudioLayout::Interleaved,
            rate,
        },
        samples: SharedBytes::from_vec(bytes),
        pts: Stamp::Present(Timestamp::from_parts(pts_ticks, 1, rate).must()),
        duration: None,
        generation: GenerationId::FIRST,
    }
}

/// An integer-period stereo tone pair (deterministic f32 sines
/// quantized to s16) — the fx3_a03 generator convention, kept
/// phase-exact for the settled RMS probes.
fn tone_pairs(n: usize, freq: f32, amp_l: f32, amp_r: f32, rate: u32) -> Vec<(i16, i16)> {
    (0..n)
        .map(|i| {
            let phase = 2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32;
            (
                (amp_l * phase.sin() * 32_767.0) as i16,
                (amp_r * phase.sin() * 32_767.0) as i16,
            )
        })
        .collect()
}

/// Render exactly `frames` frames from the open device — the pump's
/// own render step (host.rs's `advance_time`): the caller's buffer is
/// what the device served, which is where the controls become
/// observable in samples.
fn render_frames(device: &mut NullDevice, frames: usize, channels: u32) -> Vec<f32> {
    let mut buffer = vec![0.0_f32; frames * channels as usize];
    match device.render(&mut buffer) {
        Ok(_) => buffer,
        Err(error) => unreachable!("device render failed: {error}"),
    }
}

/// One channel's plane from interleaved rendered samples.
fn plane(samples: &[f32], channel: usize) -> Vec<f32> {
    samples.iter().skip(channel).step_by(2).copied().collect()
}

/// RMS of the settled tail (transients discarded — the fx3_a03 probe).
fn settled_rms(samples: &[f32]) -> f32 {
    let tail = &samples[samples.len() / 2..];
    (tail.iter().map(|s| s * s).sum::<f32>() / tail.len() as f32).sqrt()
}

// ---------------------------------------------------------------------------
// The fixture's own PCM facts (read from the file — the test oracle)
// ---------------------------------------------------------------------------

/// Walk one RIFF level (descending LISTs), keeping `(fourcc, payload
/// range)` pairs in stream order.
fn walk_riff(
    data: &[u8],
    start: usize,
    end: usize,
    out: &mut Vec<([u8; 4], std::ops::Range<usize>)>,
) {
    let mut pos = start;
    while pos + 8 <= end {
        let id = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
        let size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
            as usize;
        let body = pos + 8;
        if id == *b"LIST" {
            walk_riff(data, body + 4, (body + size).min(end), out);
        } else {
            out.push((id, body..body + size));
        }
        pos = body + size + (size & 1);
    }
}

/// The fixture's own PCM facts, read from the file: the audio stream's
/// WAVEFORMATEX (channels, rate) and every audio payload chunk, in
/// order. The end-to-end test's asserted totals are derived from THESE
/// — the media's facts, never hand-picked constants.
fn fixture_pcm_facts() -> (u32, u32, Vec<Vec<u8>>) {
    let data = std::fs::read(fixture_path()).must();
    let mut chunks: Vec<([u8; 4], std::ops::Range<usize>)> = Vec::new();
    walk_riff(&data, 12, data.len(), &mut chunks);
    let mut channels = 0_u32;
    let mut rate = 0_u32;
    let mut audio_stream = false;
    let mut payloads: Vec<Vec<u8>> = Vec::new();
    for (id, range) in &chunks {
        let payload = &data[range.clone()];
        if *id == *b"strh" {
            audio_stream = payload.starts_with(b"auds");
        } else if *id == *b"strf" && audio_stream && channels == 0 {
            channels = u16::from_le_bytes([payload[2], payload[3]]) as u32;
            rate = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
        } else if id[2..4] == *b"wb" {
            payloads.push(payload.to_vec());
        }
    }
    (channels, rate, payloads)
}

// ---------------------------------------------------------------------------
// 1 — end-to-end audio to counting null stats
// ---------------------------------------------------------------------------

/// THE end-to-end lane: the fixture's own PCM packets walk the real
/// session machinery to completion — the real `open_audio_session`, the
/// real limb staging (process → mix → the runtime master chain → stage
/// D → the ring), the pump's own quantum render loop plus the
/// end-of-media tail drain — and the null backend's ledger counts the
/// media's PCM facts exactly: whole frames, f32 sample slots, bytes,
/// render buffers, and the data/silence split. The ledger agrees with
/// the pre-existing device counter (FX3-A05's own property), and the
/// counter feeds the session clock — media time reaches the duration
/// through the counting null path alone.
#[test]
fn fx4_a05_end_to_end_audio_lands_in_the_counting_null_stats() {
    let (channels, rate, chunks) = fixture_pcm_facts();
    assert_eq!(
        (channels, rate),
        (CHANNELS, RATE),
        "the fixture's own PCM facts"
    );
    assert_eq!(chunks.len(), 60, "60 PCM packets — the fixture's shape");

    // The host's own open path for an audio-bearing media.
    let mut session = open_audio_session(channels, rate, 0).must();

    // The decode-ahead regime: stage every PCM packet (decoder-output
    // shape) through the real limb — process, mix, the identity master
    // chain, stage D, into the ring.
    let mut pts_ticks = 0_i64;
    let mut staged_frames = 0_u64;
    for payload in &chunks {
        let frame = stereo_s16_frame_from(payload, channels, rate, pts_ticks);
        session.limb.stage(&frame).must();
        let frames = payload.len() as u64 / u64::from(channels * 2);
        staged_frames += frames;
        pts_ticks += frames as i64;
    }
    assert_eq!(
        session.limb.written_frames(),
        staged_frames,
        "the ring holds every staged frame"
    );

    // The pump's own drive (host.rs's advance_time): quantum renders
    // while the ring holds a full quantum...
    let mut pumped = 0_u64;
    while session.device.buffered_frames() >= RENDER_QUANTUM {
        render_frames(&mut session.device, RENDER_QUANTUM, channels);
        pumped += RENDER_QUANTUM as u64;
    }
    // ...then the end-of-media tail (drain_tail_at_end): the residue,
    // never a stranded sub-quantum remainder.
    while session.device.buffered_frames() > 0 {
        let take = session.device.buffered_frames().min(RENDER_QUANTUM);
        render_frames(&mut session.device, take, channels);
        pumped += take as u64;
    }
    assert_eq!(
        pumped, staged_frames,
        "the device consumed everything staged"
    );

    // THE ledger (FX3-A05's SinkStats): the media's PCM facts, counted.
    let stats = session.device.stats();
    let frames = stats.frames();
    assert_eq!(frames, staged_frames, "every staged frame was consumed");
    assert_eq!(
        frames, 88_200,
        "the 2.000 s duration at 44 100 Hz (the fixture's own arithmetic)"
    );
    assert!(frames.is_multiple_of(RENDER_QUANTUM as u64));
    assert_eq!(
        stats.buffers(),
        frames / RENDER_QUANTUM as u64,
        "one render per quantum"
    );
    assert_eq!(stats.buffers(), 60, "the fixture's 60 quanta");
    assert_eq!(
        stats.samples(),
        frames * u64::from(channels),
        "f32 slots = frames × channels"
    );
    assert_eq!(
        stats.bytes(),
        stats.samples() * 4,
        "interleaved f32 LE = samples × 4"
    );
    assert_eq!(
        stats.silence_frames(),
        0,
        "a fed ring renders no underrun silence"
    );
    assert_eq!(stats.data_frames(), frames, "the whole run is data");
    assert_eq!(
        session.device.sample_count(),
        frames,
        "the ledger and the device counter agree"
    );

    // The counter is the clock's truth: media time reached the duration
    // through the counting null path alone.
    let duration_ns = frames * 1_000_000_000 / u64::from(rate);
    assert_eq!(
        session.clock.snapshot().position.ticks,
        duration_ns as i64,
        "the audio-master clock reads the drained ledger"
    );
}

/// One raw PCM payload straight into the decoder-output shape (the
/// fixture's packets are already stereo s16le interleaved).
fn stereo_s16_frame_from(payload: &[u8], channels: u32, rate: u32, pts_frames: i64) -> AudioFrame {
    AudioFrame {
        format: AudioFormat {
            sample: SampleFormat::S16,
            channels,
            layout: AudioLayout::Interleaved,
            rate,
        },
        samples: SharedBytes::from_vec(payload.to_vec()),
        pts: Stamp::Present(Timestamp::from_parts(pts_frames, 1, rate).must()),
        duration: None,
        generation: GenerationId::FIRST,
    }
}

// ---------------------------------------------------------------------------
// 2/3/4/5 — the controls observable in the rendered samples
// ---------------------------------------------------------------------------

/// Volume, live on the session: unity renders the s16 quantum's exact
/// f32, the seam's 0.5 halves every rendered sample exactly, the
/// getter reads the live value back, and returning to unity restores
/// the rendered bit stream.
#[test]
fn fx4_a05_volume_is_observable_in_rendered_samples() {
    let mut session = stereo_session(RATE);
    let frame = stereo_s16_frame(&[(16384, -16384)], 0, RATE);

    session.limb.stage(&frame).must();
    let unity = render_frames(&mut session.device, 1, CHANNELS);
    assert_eq!(unity, vec![0.5, -0.5]);

    // The controls seam (FX4-A01's live surface — FX4-A02's verb
    // target): volume set between quanta on the SAME session.
    session
        .limb
        .audio_controls()
        .set_volume(Volume::from_linear(0.5).must());
    assert_eq!(session.limb.audio_controls().volume().linear(), 0.5);

    session.limb.stage(&frame).must();
    let halved = render_frames(&mut session.device, 1, CHANNELS);
    assert_eq!(
        halved,
        vec![0.25, -0.25],
        "every rendered sample halves exactly"
    );

    // Back to unity: the rendered bit stream is restored.
    session.limb.audio_controls().set_volume(Volume::UNITY);
    session.limb.stage(&frame).must();
    let restored = render_frames(&mut session.device, 1, CHANNELS);
    assert_eq!(restored, unity, "UNITY restores the exact prior samples");
}

/// Mute, live on the session: every rendered sample is exactly zero
/// while the frame count is conserved (mute is silence, never a skip),
/// the ledger still counts the muted render as DATA (FX3-A05's split
/// is the starved subset, not the muted subset), and unmuting restores
/// the exact prior samples (a stateless ×0).
#[test]
fn fx4_a05_mute_is_observable_as_rendered_silence() {
    let mut session = stereo_session(RATE);
    let pairs = tone_pairs(4410, 1000.0, 0.8, 0.8, RATE);
    let frame = stereo_s16_frame(&pairs, 0, RATE);

    session.limb.stage(&frame).must();
    let audible = render_frames(&mut session.device, pairs.len(), CHANNELS);
    assert!(
        audible.iter().any(|sample| *sample != 0.0),
        "the lane starts audible"
    );

    session.limb.audio_controls().set_muted(true);
    assert!(session.limb.audio_controls().muted());

    session.limb.stage(&frame).must();
    let muted = render_frames(&mut session.device, pairs.len(), CHANNELS);
    assert_eq!(
        muted.len(),
        pairs.len() * 2,
        "mute emits silence, never a shorter quantum"
    );
    assert!(
        muted.iter().all(|sample| *sample == 0.0),
        "every rendered sample is exactly zero"
    );

    session.limb.audio_controls().set_muted(false);
    assert!(!session.limb.audio_controls().muted());
    session.limb.stage(&frame).must();
    let restored = render_frames(&mut session.device, pairs.len(), CHANNELS);
    assert_eq!(restored, audible, "unmute restores the exact prior samples");

    // The ledger's honest split: the muted render is data (rendered
    // silence from live content), never underrun silence.
    let stats = session.device.stats();
    assert_eq!(
        stats.frames(),
        (pairs.len() * 3) as u64,
        "three staged quanta, three rendered"
    );
    assert_eq!(stats.silence_frames(), 0, "mute is not an underrun");
    assert_eq!(stats.data_frames(), stats.frames());
}

/// A band cut attenuates a band-centered tone at the session level —
/// the fx3_a03_eq3_attenuates_sine_at_band_center mirror through the
/// whole session path: the 1 kHz sine (the mid peak's exact center)
/// rides stage → ring → device render, and the settled amplitude
/// carries the −12 dB mid cut (the reference is the same session's
/// uncontrolled render, so s16 quantization cancels in the ratio).
#[test]
fn fx4_a05_eq_band_cut_attenuates_a_band_centered_tone() {
    let mut session = stereo_session(EQ_RATE);
    let pairs = tone_pairs(24_000, 1000.0, 0.4, 0.4, EQ_RATE);
    let frame = stereo_s16_frame(&pairs, 0, EQ_RATE);

    session.limb.stage(&frame).must();
    let reference = render_frames(&mut session.device, pairs.len(), CHANNELS);

    session
        .limb
        .audio_controls()
        .set_eq(0.0, -12.0, 0.0, Volume::UNITY)
        .must();
    assert_eq!(
        session.limb.audio_controls().eq_bands(),
        Some((0.0, -12.0, 0.0, Volume::UNITY))
    );

    session.limb.stage(&frame).must();
    let filtered = render_frames(&mut session.device, pairs.len(), CHANNELS);

    let want = 10.0_f32.powf(-12.0 / 20.0);
    for (channel, name) in [(0_usize, "L"), (1_usize, "R")] {
        let ratio =
            settled_rms(&plane(&filtered, channel)) / settled_rms(&plane(&reference, channel));
        assert!(
            (ratio - want).abs() / want < 0.05,
            "{name} ratio {ratio} vs expected {want}"
        );
    }
}

/// Balance, live on the session: a hard-left position keeps L by the
/// constant-power pan law (amplitude / √2) and silences R — the
/// fx3_a03 balance mirror through the whole session path.
#[test]
fn fx4_a05_balance_shifts_rendered_l_r_energy() {
    let mut session = stereo_session(RATE);
    let pairs = tone_pairs(4410, 1000.0, 0.8, 0.2, RATE);
    let frame = stereo_s16_frame(&pairs, 0, RATE);

    session.limb.audio_controls().set_balance(-1.0).must();
    assert_eq!(session.limb.audio_controls().balance(), Some(-1.0));

    session.limb.stage(&frame).must();
    let hard_left = render_frames(&mut session.device, pairs.len(), CHANNELS);
    let l = settled_rms(&plane(&hard_left, 0));
    let r = settled_rms(&plane(&hard_left, 1));
    assert!(
        (l - 0.8 / std::f32::consts::SQRT_2).abs() < 1e-3,
        "L kept by the pan law: {l}"
    );
    assert!(r < 1e-6, "hard left silences R: {r}");
}

// ---------------------------------------------------------------------------
// 6 — the host verbs route for real (the wire lane)
// ---------------------------------------------------------------------------

/// Open the fixture through the real dispatch and settle (the
/// pipeline-accounting harness's own open step).
fn dispatch_open(host: &mut CliHost) {
    let response = lumen_dispatch(
        host,
        &WireRequest {
            request_id: RequestId(1),
            command: Command::SessionOpen {
                source: SourceSpec::File {
                    path: fixture_path().into(),
                },
                options: Default::default(),
            },
        },
    );
    match response.result {
        Ok(_) => {}
        Err(error) => unreachable!("open failed: {error}"),
    }
    host.settle();
}

/// The wire lane (FX4-A02's routing): `audio.set-volume` and
/// `audio.set-mute` answer the catalog's own payloads on the real
/// dispatch — the `unsupported-for-host` refusal this wave replaces
/// is gone — and the session stays healthy through the live-control
/// verbs, playing its media to the exact end on the audio-master
/// clock (the counting null path).
#[test]
fn fx4_a05_host_audio_verbs_route_to_the_live_controls() {
    let mut host = CliHost::new();
    dispatch_open(&mut host);

    let response = lumen_dispatch(
        &mut host,
        &WireRequest {
            request_id: RequestId(2),
            command: Command::AudioSetVolume { volume: 0.5 },
        },
    );
    match response.result {
        Ok(CommandOutcome::Completed(CommandResult::AudioVolume { volume })) => {
            assert_eq!(volume, 0.5, "the verb answers the applied volume");
        }
        other => unreachable!("audio.set-volume must route for real: {other:?}"),
    }

    let response = lumen_dispatch(
        &mut host,
        &WireRequest {
            request_id: RequestId(3),
            command: Command::AudioSetMute { muted: true },
        },
    );
    match response.result {
        Ok(CommandOutcome::Completed(CommandResult::AppliedBool { value })) => {
            assert!(value, "mute on answers true");
        }
        other => unreachable!("audio.set-mute must route for real: {other:?}"),
    }

    let response = lumen_dispatch(
        &mut host,
        &WireRequest {
            request_id: RequestId(4),
            command: Command::AudioSetMute { muted: false },
        },
    );
    match response.result {
        Ok(CommandOutcome::Completed(CommandResult::AppliedBool { value })) => {
            assert!(!value, "mute off answers false");
        }
        other => unreachable!("audio.set-mute (off) must route for real: {other:?}"),
    }

    // The session stays healthy through the live-control verbs: the
    // media still plays to its exact end.
    host.advance(RUN_PUMPS as u32);
    assert_eq!(host.state(), PlaybackState::Ended);
    assert_eq!(host.position_us(), 2_000_000, "the 2.000 s duration");
}
