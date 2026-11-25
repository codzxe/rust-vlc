//! FX2-A01 regression guards — FIX-PLAN F0.2 + F0.3 (W13-A02 F-001
//! and F-002: the four-registry lie + the `seekable` hardcode).
//!
//! # The registry truth (F0.2)
//!
//! `engine-info`'s decoder list must name exactly the families the
//! REAL dispatch table routes at runtime. The audit's disproof: a
//! session whose h264+aac tracks were Playing answered
//! `decoders: mjpeg, pcm_s16le` (host.rs's hand-maintained
//! first-vertical snapshot) while the same binary's
//! `HostDecoderProvider::decoder_for` dispatched 14 families.
//!
//! The dispatch table is a `match` on `track.codec.as_str()`, not
//! data, so the registry lives as `LANDED_DECODER_FAMILIES` in
//! pipeline_host.rs (FX2-A03's export — the same one source the
//! `landed:` refusal string derives from), and this suite checks it
//! against the table ITSELF: `descriptor_is_landed` probes the real
//! `decoder_for` per descriptor (an unroutable descriptor answers
//! the catch-all arm's `cli.codec_unavailable`; a routed one answers
//! anything else — the codec's own `configure` gate refusing a
//! fact-less lane is ROUTING, not refusal). `engine_info()` derives
//! its rows from that registry by code; this suite pins the pinned
//! expectation (below) == the registry == the `engine_info()` answer,
//! so the removed-arm/renamed-arm/renamed-family drift directions
//! fail the build.
//!
//! # The seekable truth (F0.3)
//!
//! `info`'s `seekable` must answer the engine's declared
//! `SeekSupport` — the W13-A02 disproof, re-verified by W20-A02: a
//! crafted FLAC-in-WAV (WAVEFORMATEXTENSIBLE + the FLAC SubFormat
//! GUID + a 34-byte STREAMINFO — the shape `demux::wav::format`
//! parses; FLAC's `samples_per_unit()` is the honest unknown, so
//! `WavDemuxer::seekability()` answers `SeekSupport::None`) ran one
//! session with three contradictions: the engine event
//! `seekable: false`, the host `info` `seekable: yes` (the
//! hardcode), the seek terminal `not-seekable`. Post-fix all three
//! agree. The control lane is the committed PCM WAV fixture (the
//! constant-bit-rate family the demuxer declares
//! `Fast { indexed: false }`): `seekable: yes` must survive the fix
//! — the field answers the engine, it does not blanket-flip.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use commands::catalog::types::TrackKind;
use lumen_cli::host::engine_info;
use lumen_cli::pipeline_host::{LANDED_DECODER_FAMILIES, descriptor_is_landed};
use testkit::Must;

/// Run the binary once with `args` and a piped stdin script; answer
/// (stdout, stderr, exit code) — the command_surface.rs convention.
fn run(args: &[&str], script: &str) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lumen"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .must();
    {
        let stdin = child.stdin.as_mut().must();
        if let Err(err) = stdin.write_all(script.as_bytes()) {
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::BrokenPipe,
                "unexpected stdin write failure: {err}"
            );
        }
    }
    let output = child.wait_with_output().must();
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// A temp dir that removes itself (the fx1_a09/ts.rs suite
/// convention).
struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("lumen-fx2-a01-{tag}-{}", std::process::id())); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
        std::fs::create_dir_all(&dir).must();
        TempDir(dir)
    }
    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).must();
        path
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------------------
// F0.2 — the registry truth, probed against the real dispatch table
// ---------------------------------------------------------------------------

/// The pinned expectation: exactly the 14-family registry
/// (pipeline_host.rs's `LANDED_DECODER_FAMILIES` — one row per codec
/// arm of `HostDecoderProvider::decoder_for`, arm order; `aac-lc` is
/// the `aac|aac-lc` family's registry name, `adpcm`/`pcm` the two
/// prefix-guard arms, the arm aliases `mp1`/`av01`/`avc1` fold into
/// their families' rows). Kept as a literal so the test fails loudly
/// when the dispatch table grows or shrinks — then this pin, the
/// registry and `engine_info()`'s answer must move together.
const EXPECTED_FAMILIES: &[(&str, TrackKind)] = &[
    // arm: "mjpeg"
    ("mjpeg", TrackKind::Video),
    // arm: "flac"
    ("flac", TrackKind::Audio),
    // arm: "mp3" | "mp1"
    ("mp3", TrackKind::Audio),
    // arm: "mp2"
    ("mp2", TrackKind::Audio),
    // arm: "vorbis"
    ("vorbis", TrackKind::Audio),
    // arm: "opus"
    ("opus", TrackKind::Audio),
    // arm: "aac" | "aac-lc"
    ("aac-lc", TrackKind::Audio),
    // arm: "alac"
    ("alac", TrackKind::Audio),
    // arm: `starts_with("adpcm_")` — descriptors like adpcm_ima_wav
    ("adpcm", TrackKind::Audio),
    // arm: "av1" | "av01"
    ("av1", TrackKind::Video),
    // arm: "vp9"
    ("vp9", TrackKind::Video),
    // arm: "dnxhd"
    ("dnxhd", TrackKind::Video),
    // arm: "h264" | "avc1"
    ("h264", TrackKind::Video),
    // arm: `starts_with("pcm_")` — descriptors like pcm_s16le
    ("pcm", TrackKind::Audio),
];

/// engine-info's decoder list is exactly the dispatch table's
/// families: every registry descriptor routes through the REAL table
/// (`descriptor_is_landed` probes `HostDecoderProvider::decoder_for`
/// itself), the audit's executed refusals stay refused (control), the
/// registry equals the pinned expectation, and the `engine_info()`
/// snapshot answers exactly that list with the families' kinds.
/// Pre-fix, the snapshot answered the hand-maintained
/// `mjpeg, pcm_s16le` pair while the same binary dispatched all
/// fourteen.
#[test]
fn fx2_a01_engine_info_lists_the_dispatch_tables_families() {
    for family in LANDED_DECODER_FAMILIES {
        for &descriptor in family.descriptors {
            assert!(
                descriptor_is_landed(descriptor),
                "`{descriptor}` (family {}) must route through the real dispatch table",
                family.name
            );
        }
    }
    // The catch-all arm's control: the audit's executed refusals (the
    // same binary exits 65 `cli.codec_unavailable` on hevc; wave 13
    // lists mpeg2video/mpeg4video refused the same way).
    for refused in ["hevc", "mpeg2video", "mpeg4video"] {
        assert!(
            !descriptor_is_landed(refused),
            "`{refused}` must stay refused by the dispatch table"
        );
    }
    // The pinned expectation == the registry (names, order, count)…
    assert_eq!(
        LANDED_DECODER_FAMILIES
            .iter()
            .map(|family| family.name)
            .collect::<Vec<&str>>(),
        EXPECTED_FAMILIES
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<&str>>(),
        "the pinned registry and the dispatch table's registry must agree"
    );
    // …and engine-info answers exactly it (names + kinds + order).
    let info = engine_info();
    assert_eq!(
        info.decoders.len(),
        EXPECTED_FAMILIES.len(),
        "engine-info must list every dispatch family, no more, no less"
    );
    assert_eq!(
        info.decoders
            .iter()
            .map(|decoder| (decoder.name.as_str(), decoder.kind))
            .collect::<Vec<(&str, TrackKind)>>(),
        EXPECTED_FAMILIES
    );
    assert!(
        info.decoders.iter().all(|decoder| !decoder.hardware),
        "the shipped lane is software-only"
    );
}

/// The CLI surface prints the same truth: `lumen engine-info`'s
/// decoders line names the dispatch table's families in arm order.
#[test]
fn fx2_a01_engine_info_cli_surface_lists_all_dispatch_families() {
    let (out, err, code) = run(&["engine-info"], "");
    assert_eq!(code, 0, "stderr:\n{err}");
    assert!(out.contains("ok: engine"), "{out}");
    assert!(
        out.contains(
            "decoders: mjpeg, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, \
             vp9, dnxhd, h264, pcm"
        ),
        "{out}"
    );
}

// ---------------------------------------------------------------------------
// F0.3 — the seekable truth
// ---------------------------------------------------------------------------

/// The audit's crafted FLAC-in-WAV bytes (audit/verify/
/// make_flac_wav.py, executed twice — W13-A02 and W20-A02): a
/// WAVEFORMATEXTENSIBLE `fmt ` (tag 0xFFFE, 2ch, 48 kHz, 16-bit,
/// cbSize 56, validBits 16, mask 0x3) carrying the
/// KSDATAFORMAT_SUBTYPE_FLAC SubFormat GUID plus the 34-byte FLAC
/// STREAMINFO (4096 min/max blocksize, unknown frame sizes, rate 48
/// kHz / 2ch / 16 bps / 48 000 total, zero MD5), over 4096 junk
/// payload bytes. `demux::wav::format` parses it to
/// `WavCodec::Flac`, whose `samples_per_unit()` is the honest
/// unknown — so `WavDemuxer::seekability()` answers
/// `SeekSupport::None` and the engine reports `seekable: false`.
fn crafted_flac_in_wav() -> Vec<u8> {
    // The 34-byte STREAMINFO.
    let mut streaminfo = Vec::new();
    streaminfo.extend_from_slice(&4096u16.to_be_bytes()); // min blocksize
    streaminfo.extend_from_slice(&4096u16.to_be_bytes()); // max blocksize
    streaminfo.extend_from_slice(&[0u8; 3]); // min framesize (unknown)
    streaminfo.extend_from_slice(&[0u8; 3]); // max framesize (unknown)
    let packed: u64 = (48_000u64 << 44) | (1u64 << 41) | (15u64 << 36) | 48_000;
    streaminfo.extend_from_slice(&packed.to_be_bytes()); // rate/ch/bps/total
    streaminfo.extend_from_slice(&[0u8; 16]); // MD5 (zero)
    assert_eq!(streaminfo.len(), 34);

    // The 74-byte fmt: base WAVEFORMATEX + cbSize 56 + the FLAC GUID
    // + the STREAMINFO.
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&0xFFFEu16.to_le_bytes()); // extensible tag
    fmt.extend_from_slice(&2u16.to_le_bytes()); // channels
    fmt.extend_from_slice(&48_000u32.to_le_bytes()); // samples/sec
    fmt.extend_from_slice(&(48_000u32 * 4).to_le_bytes()); // avg bytes/sec
    fmt.extend_from_slice(&4u16.to_le_bytes()); // block align
    fmt.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    fmt.extend_from_slice(&56u16.to_le_bytes()); // cbSize
    fmt.extend_from_slice(&16u16.to_le_bytes()); // valid bits
    fmt.extend_from_slice(&0x3u32.to_le_bytes()); // channel mask
    fmt.extend_from_slice(&[
        0xAC, 0xF1, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B,
        0x71,
    ]); // KSDATAFORMAT_SUBTYPE_FLAC
    fmt.extend_from_slice(&streaminfo);
    assert_eq!(fmt.len(), 74);

    let data = [0u8; 4096];
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(4 + 8 + fmt.len() as u32 + 8 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    out.extend_from_slice(&fmt);
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// The W13-A02 disproof, flipped: the crafted FLAC-in-WAV run must
/// have the engine's event, the CLI's `info` snapshot and the seek
/// terminal all answering the SAME unseekable truth. Pre-fix the run
/// printed `seekable: false` (engine), `seekable: yes` (host
/// hardcode) and the typed `not-seekable` seek failure — one run,
/// three contradictions (W13-A02 F-001, re-verified W20-A02).
#[test]
fn fx2_a01_info_seekable_answers_the_engine_for_flac_in_wav() {
    let dir = TempDir::new("flacwav");
    let path = dir.write("flac-in-wav.wav", &crafted_flac_in_wav());
    let file = path.to_string_lossy().into_owned();
    let (out, err, code) = run(&["open", &file], "info\nseek 1s\nquit\n");
    assert_eq!(code, 0, "stderr:\n{err}");
    // The engine's own `media.info-ready` event: not seekable…
    assert!(out.contains("seekable: false"), "{out}");
    // …the CLI's `info` snapshot answers the same truth (pre-fix: the
    // hardcoded `seekable: yes`)…
    assert!(out.contains("seekable: no"), "{out}");
    // …and the seek still refuses typed (the honest terminal,
    // unchanged — the source really cannot seek).
    assert!(out.contains("not-seekable"), "{out}");
}

/// The control lane: the committed PCM WAV fixture (the CBR family
/// the demuxer declares `Fast { indexed: false }`) — the engine says
/// seekable and the CLI's `info` agrees. The fix answers the engine;
/// it must not blanket-flip the field.
#[test]
fn fx2_a01_info_seekable_stays_yes_for_the_pcm_wav_family() {
    let file = format!(
        "{}/../../tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav",
        env!("CARGO_MANIFEST_DIR")
    );
    let (out, err, code) = run(&["open", &file], "info\nquit\n");
    assert_eq!(code, 0, "stderr:\n{err}");
    assert!(out.contains("seekable: yes"), "{out}");
}
