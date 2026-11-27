//! FX9-A03 — the compositing lane's acceptance: subtitle cues reach
//! the USER, as pixels on the presented frames.
//!
//! The defect (FX9-F3.1, REALITY-SCORE rows 057-059/064/068
//! UNREACHABLE): `apps/cli/src/pipeline_host.rs` dropped every
//! `Frame::Subtitle` (`Frame::Subtitle(_) => Ok(())`), sidecar files
//! had no session lane at all, and `accept_video` pushed the
//! converted RGBA frame with zero overlay compositing — the whole
//! compositing contract (crates/subtitles `SubtitleCompositor` /
//! `ComposeTarget` / `CompositedCue`) sat unused. The fix built the
//! lane this suite drives end to end: sidecar loads reduce to cues
//! ([`SubtitleLane::add_sidecar`]), the embedded arm holds A04's
//! overlay payloads ([`SubtitleLane::accept_subtitle_frame`]), and
//! [`SubtitleLane::composite_onto`] blends the cues active at each
//! frame's OWN pts onto the RGBA rows before the scheduler sees the
//! frame — so the presented-frame tap (w31.1) inherits the
//! composited pixels by construction.
//!
//! Oracles, all pipeline-agnostic (ink = a pixel difference from the
//! same frame's no-lane baseline, never a glyph shape; they hold
//! under the placeholder AND the swash glyph pipelines alike):
//!
//! * ink inside the cue window / byte-identical outside it (the
//!   no-cue case leaves the bytes untouched), and the ink confined
//!   to the bottom band (the placement contract);
//! * the sidecar formats the loader dispatches — srt/vtt/ass text
//!   through the real dispatch + the real run, pgs `.sup` and the
//!   vobsub `.idx`+`.sub` pair through the real parsers with bytes
//!   crafted in-test (the crafted display-set and SPU shapes of
//!   crates/subtitles' own suites, pinned against the frame's own
//!   extent so the placement is exact);
//! * the embedded arm: A04's real `PgsSubtitleDecoder` reduces a
//!   crafted display set to a `Frame::Subtitle` overlay payload,
//!   which reconstructs and composites on the lane;
//! * determinism of the composited presented frames;
//! * the verb chain's arrival truth in the `tracks.list` snapshot,
//!   and the typed refusals for bad sidecar inputs.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink, ThreadBudget};
use codecs::subtitle::pgs::{DESCRIPTOR, PgsSubtitleDecoder};
use commands::catalog::types::{SourceSpec, TrackKind, TrackOrigin, TrackSelector};
use commands::envelope::RequestId;
use commands::{Command, CommandOutcome, CommandResult, PlaybackState, WireRequest};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::{CliHost, RUN_PUMPS};
use lumen_cli::pipeline_host::{PresentTap, SubtitleLane};
use media_types::frame::{
    ColorDescription, Frame, FrameStorage, SubtitleCue, SubtitleFrame, VideoFrame,
};
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use media_types::{EngineError, Packet, PipelineStage, TrackId};
use video_engine::sink::PresentRequest;

use testkit::Must;

// -- the shared fixture truth ------------------------------------------------

/// The one cue every text fixture carries, in every format's own
/// spelling: `[00:00:00.500, 00:00:03.000)` — the half-open window
/// the compositing contract judges (`[t0, t1)`), wider than the
/// fixture media's own 2.000 s so every frame from 500 ms on is
/// in-window and every frame before it is out.
const CUE_START_NS: u64 = 500 * 1_000_000;
const CUE_END_NS: u64 = 3_000 * 1_000_000;

/// The presented frames' extent (the AVI fixture's own 320×240).
const FRAME_W: u32 = 320;
const FRAME_H: u32 = 240;
const STRIDE: usize = FRAME_W as usize * 4;

/// The bottom band: everything above this row must stay byte-clean
/// (the fixture's own content there — color bars, the frame counter,
/// the gray ramp's top — ends at row 168; the cue box renders below
/// the bottom anchor's 5 % safe-margin clamp, row ≥ 208).
const TOP_SPLIT_Y: usize = 180;

/// The half-open cue-window rule — the SAME rule the compositing
/// contract's `active_at` applies, against the frame's own pts ns.
fn in_cue_window(pts_ns: u64) -> bool {
    (CUE_START_NS..CUE_END_NS).contains(&pts_ns)
}

/// The MJPEG/PCM AVI (60 frames over 2.000 s, 30 fps — the
/// `pipeline_accounting.rs` / `fx7_a01` oracle fixture).
fn avi_path() -> String {
    format!(
        "{}/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// The stereo PCM WAV (a live session that needs no video limb —
/// the verb chain's fixture, the `fx9_a01` suite's own choice).
fn wav_path() -> String {
    format!(
        "{}/../../tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// One committed text sidecar fixture's absolute path.
fn fixture(name: &str) -> String {
    format!(
        "{}/tests/fixtures/subtitles/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

// -- the harness (the fx7_a01/fx9_a01 in-process host shape) -----------------

/// Dispatch one catalog command through the REAL entry (the same
/// `lumen_dispatch` the machine mode and the desktop shell drive).
fn dispatch(
    host: &mut CliHost,
    command: Command,
) -> Result<CommandOutcome<CommandResult>, commands::CommandError> {
    lumen_dispatch(
        host,
        &WireRequest {
            request_id: RequestId(1),
            command,
        },
    )
    .result
}

/// Open + settle one source (the in-process host harness convention).
fn open(host: &mut CliHost, path: &str) {
    let outcome = dispatch(
        host,
        Command::SessionOpen {
            source: SourceSpec::File { path: path.into() },
            options: Default::default(),
        },
    );
    assert!(outcome.is_ok(), "open failed: {outcome:?}");
    host.settle();
}

/// A temp dir that removes itself (the fx1_a09/fx9_a01 convention).
struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("lumen-fx9-a03-{tag}-{}", std::process::id()));
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

/// One presented frame as the tap saw it: the frame's own pts (ns)
/// and its RGBA rows (empty for non-CPU frames, which the software
/// compositor never produces here).
#[derive(Clone)]
struct TappedFrame {
    pts_ns: Option<u64>,
    rgba: Vec<u8>,
}

/// The presented-frame tap: records every request the renderer took.
/// Kept as a CONCRETE binding (snapshot() needs it); the coercion to
/// `Arc<dyn PresentTap>` happens at the `set_present_tap` call site.
struct FrameTap {
    frames: Mutex<Vec<TappedFrame>>,
}

impl FrameTap {
    fn new() -> FrameTap {
        FrameTap {
            frames: Mutex::new(Vec::new()),
        }
    }

    fn snapshot(&self) -> Vec<TappedFrame> {
        let frames = match self.frames.lock() {
            Ok(frames) => frames,
            Err(poisoned) => poisoned.into_inner(),
        };
        frames.clone()
    }
}

impl PresentTap for FrameTap {
    fn tap_present(&self, request: &PresentRequest, _repeat: bool) -> Result<(), EngineError> {
        let pts_ns = match request.frame.pts {
            Stamp::Present(ts) => ts.as_nanos().ok().map(|ns| ns.0),
            Stamp::Missing => None,
        };
        let rgba = match &request.frame.data {
            FrameStorage::Cpu(bytes) => bytes.as_slice().to_vec(),
            FrameStorage::OpaqueHandle(_) => Vec::new(),
        };
        let mut frames = match self.frames.lock() {
            Ok(frames) => frames,
            Err(poisoned) => poisoned.into_inner(),
        };
        frames.push(TappedFrame { pts_ns, rgba });
        Ok(())
    }
}

/// One whole-media run with the tap bound pre-open (the desktop's
/// bind-before-open shape), optionally loading + selecting one
/// sidecar through the REAL `subtitle.add-file` dispatch. Answers
/// the presented frames keyed by their own pts.
fn presented_frames(sidecar: Option<&str>) -> BTreeMap<u64, Vec<u8>> {
    let tap = Arc::new(FrameTap::new());
    let mut host = CliHost::new();
    host.set_present_tap(tap.clone());
    open(&mut host, &avi_path());
    if let Some(path) = sidecar {
        let add = dispatch(
            &mut host,
            Command::SubtitleAddFile {
                path: path.to_string(),
                encoding: None,
                select: true,
            },
        );
        assert!(add.is_ok(), "the sidecar load refused: {add:?}");
    }
    host.advance(RUN_PUMPS as u32);
    if host.state() != PlaybackState::Ended {
        unreachable!("the run should have ended, state: {:?}", host.state());
    }
    assert_eq!(host.position_us(), 2_000_000, "the whole media plays out");
    let mut buckets: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
    for frame in tap.snapshot() {
        if let (Some(pts_ns), rgba) = (frame.pts_ns, frame.rgba)
            && !rgba.is_empty()
        {
            buckets.insert(pts_ns, rgba);
        }
    }
    assert_eq!(
        buckets.len(),
        60,
        "every decoded frame presented (fx7_a01's 60-frame oracle)"
    );
    let stride = FRAME_W as usize * FRAME_H as usize * 4;
    for (pts, rgba) in &buckets {
        assert_eq!(
            rgba.len(),
            stride,
            "frame at pts {pts} ns carries full RGBA rows"
        );
    }
    buckets
}

/// The first byte at which two equal-length frames differ.
fn first_diff(clean: &[u8], inked: &[u8]) -> Option<usize> {
    clean.iter().zip(inked).position(|(a, b)| a != b)
}

/// The window-edge pair: the first presented frame's pts (0 —
/// before the cue) and the last's (≈1.97 s — inside the cue).
fn window_edges(baseline: &BTreeMap<u64, Vec<u8>>) -> (u64, u64) {
    let before = *baseline.keys().next().must();
    let during = *baseline.keys().next_back().must();
    assert!(
        !in_cue_window(before),
        "the first frame precedes the cue window: {before} ns"
    );
    assert!(
        in_cue_window(during),
        "the last frame sits inside the cue window: {during} ns"
    );
    (before, during)
}

/// One text sidecar's compositing proof: the frame before the cue
/// window is byte-identical to the no-lane baseline, the frame
/// inside carries ink, and the ink stays in the bottom band.
fn assert_sidecar_composites_inside(name: &str) {
    let baseline = presented_frames(None);
    let composited = presented_frames(Some(&fixture(name)));
    let (before, during) = window_edges(&baseline);
    // Outside the cue window: the no-cue case leaves the bytes
    // untouched — full-frame byte identity against the baseline.
    let clean = baseline.get(&before).must();
    let inked = composited.get(&before).must();
    assert_eq!(clean.len(), inked.len(), "{name}: {before} ns");
    assert!(
        first_diff(clean, inked).is_none(),
        "{name}: the frame before the cue window must stay untouched"
    );
    // Inside the cue window: ink present, confined to the band.
    let clean = baseline.get(&during).must();
    let inked = composited.get(&during).must();
    assert_eq!(clean.len(), inked.len(), "{name}: {during} ns");
    assert!(
        first_diff(clean, inked).is_some(),
        "{name}: the frame inside the cue window carries cue ink"
    );
    let split = TOP_SPLIT_Y * STRIDE;
    assert!(
        first_diff(&clean[..split], &inked[..split]).is_none(),
        "{name}: no ink above the bottom band"
    );
    assert!(
        first_diff(&clean[split..], &inked[split..]).is_some(),
        "{name}: the cue's ink landed in the bottom band"
    );
}

// -- the direct-lane crafting vocabulary --------------------------------------

/// The nanosecond base every crafted stamp uses (the vocabulary's
/// own 1/1e9, already reduced).
const NANOS_BASE: TimeBase = TimeBase {
    num: 1,
    den: 1_000_000_000,
};

/// One millisecond stamp.
fn ns_stamp(ms: u64) -> Stamp {
    Stamp::Present(Timestamp::new(
        (ms as i64) * 1_000_000,
        NANOS_BASE,
    ))
}

/// The uniform background the crafted frames carry (dark, far from
/// every ink color the cues paint).
const BG_PIXEL: [u8; 4] = [16, 16, 16, 255];

fn bg_bytes() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(FRAME_W as usize * FRAME_H as usize * 4);
    for _ in 0..FRAME_W * FRAME_H {
        bytes.extend_from_slice(&BG_PIXEL);
    }
    bytes
}

/// A straight-alpha RGBA8 frame at the presented extent.
fn rgba_frame(pts_ms: u64) -> VideoFrame {
    VideoFrame {
        format: media_types::PixelFormat::Rgba8,
        size: (FRAME_W, FRAME_H),
        data: FrameStorage::Cpu(SharedBytes::from_vec(bg_bytes())),
        pts: ns_stamp(pts_ms),
        color: ColorDescription::BT709_LIMITED,
        interlaced: false,
        generation: media_types::GenerationId::FIRST,
    }
}

fn frame_bytes(frame: &VideoFrame) -> Vec<u8> {
    match &frame.data {
        FrameStorage::Cpu(bytes) => bytes.as_slice().to_vec(),
        FrameStorage::OpaqueHandle(_) => Vec::new(),
    }
}

/// The background bytes with one opaque block painted in.
fn with_block(x: u32, y: u32, w: u32, h: u32, pixel: [u8; 4]) -> Vec<u8> {
    let mut bytes = bg_bytes();
    for row in y..y + h {
        for col in x..x + w {
            let offset = ((row * FRAME_W + col) * 4) as usize;
            bytes[offset..offset + 4].copy_from_slice(&pixel);
        }
    }
    bytes
}

// -- the crafted PGS vocabulary (crates/subtitles/src/pgs' own suite) ----------

/// One display-set palette entry: entry 1 = opaque studio white
/// (BT.709 YCbCr 235/128/128 → RGB 255s).
const WHITE_ENTRY: (u8, u8, u8, u8, u8) = (1, 235, 128, 128, 255);

/// The 2×2 all-white object's RLE (the crate's own crafted run —
/// every pixel entry-1).
const RLE_2X2_WHITE: [u8; 8] = [0x00, 0x82, 0x01, 0x00, 0x00, 0x00, 0x82, 0x01];

/// One PGS segment body: presentation composition.
fn pcs_data(state: u8, palette_id: u8, objects: &[(u16, u16, u16)], video: (u16, u16)) -> Vec<u8> {
    let mut data = vec![
        video.0.to_be_bytes()[0],
        video.0.to_be_bytes()[1],
        video.1.to_be_bytes()[0],
        video.1.to_be_bytes()[1],
        0x20, // frame-rate nibble 2 = 24 fps
        0x00,
        0x01, // composition number 1
        state,
        0x00, // no palette update
        palette_id,
        objects.len() as u8,
    ];
    for &(id, x, y) in objects {
        data.extend_from_slice(&id.to_be_bytes());
        data.push(0); // window id
        data.push(0); // flags: not forced, not cropped
        data.extend_from_slice(&x.to_be_bytes());
        data.extend_from_slice(&y.to_be_bytes());
    }
    data
}

/// One PGS segment body: palette definition.
fn pds_data(palette_id: u8, version: u8, entries: &[(u8, u8, u8, u8, u8)]) -> Vec<u8> {
    let mut data = vec![palette_id, version];
    for &(entry_id, y, cr, cb, a) in entries {
        data.extend_from_slice(&[entry_id, y, cr, cb, a]);
    }
    data
}

/// One PGS segment body: a complete single-fragment object
/// definition (the crate's own `ods_first` shape).
fn ods_first_data(id: u16, width: u16, height: u16, rle: &[u8]) -> Vec<u8> {
    let total = 4 + rle.len();
    let mut data = id.to_be_bytes().to_vec();
    data.push(0); // version
    data.push(0x80); // first fragment
    data.push((total >> 16) as u8);
    data.push((total >> 8) as u8);
    data.push(total as u8);
    data.extend_from_slice(&width.to_be_bytes());
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(rle);
    data
}

/// One STRIPPED segment (Matroska `S_HDMV/PGS` framing).
fn seg(kind: u8, data: &[u8]) -> Vec<u8> {
    let mut v = vec![kind];
    v.extend_from_slice(&(data.len() as u16).to_be_bytes());
    v.extend_from_slice(data);
    v
}

/// One SUP-framed segment: the 4-byte 90 kHz PTS the standalone
/// `.sup` carriage stamps before every segment.
fn sup_seg(pts_90khz: u32, kind: u8, data: &[u8]) -> Vec<u8> {
    let mut v = pts_90khz.to_be_bytes().to_vec();
    v.push(kind);
    v.extend_from_slice(&(data.len() as u16).to_be_bytes());
    v.extend_from_slice(data);
    v
}

/// The epoch-start display set: object 1, 2×2 studio white, at
/// `(x, y)` on the given canvas — one set, four segments.
fn display_set(objects: &[(u16, u16, u16)], video: (u16, u16)) -> Vec<u8> {
    [
        seg(0x16, &pcs_data(0x80, 0, objects, video)),
        seg(0x14, &pds_data(0, 0, &[WHITE_ENTRY])),
        seg(0x15, &ods_first_data(1, 2, 2, &RLE_2X2_WHITE)),
        seg(0x80, &[]),
    ]
    .concat()
}

// -- the crafted VobSub vocabulary (crates/subtitles/src/vobsub's own suite) --

/// One SPU packet from field data + commands (the crate's pair-test
/// builder, verbatim: the two DCSQT dates land in chained headers).
#[allow(clippy::too_many_arguments)]
fn build_spu(
    top: &[u8],
    bottom: &[u8],
    x1: u16,
    y1: u16,
    w: u16,
    h: u16,
    start_date: u16,
    end_date: u16,
) -> Vec<u8> {
    let x2 = x1 + w - 1;
    let y2 = y1 + h - 1;
    let mut spu = Vec::new();
    spu.extend_from_slice(&[0, 0]);
    spu.extend_from_slice(&[0, 0]);
    let off1 = spu.len();
    spu.extend_from_slice(top);
    let off2 = spu.len();
    spu.extend_from_slice(bottom);
    let cmds = spu.len();
    // DCSQT 1: date = start_date, chained to DCSQT 2.
    let cmds2 = cmds + 4 + 1 + 3 + 3 + 7 + 5 + 1;
    spu.extend_from_slice(&start_date.to_be_bytes());
    spu.extend_from_slice(&(cmds2 as u16).to_be_bytes());
    spu.push(0x01); // start display at the DCSQT date
    spu.push(0x03);
    spu.extend_from_slice(&[0x01, 0x23]); // colormap [0]=3,[1]=2,[2]=1,[3]=0
    spu.push(0x04);
    spu.extend_from_slice(&[0x0F, 0x00]); // alpha [0]=0,[1]=0,[2]=15,[3]=0
    spu.push(0x05);
    spu.extend_from_slice(&[
        (x1 >> 4) as u8,
        ((x1 & 0xF) << 4) as u8 | (x2 >> 8) as u8,
        (x2 & 0xFF) as u8,
        (y1 >> 4) as u8,
        ((y1 & 0xF) << 4) as u8 | (y2 >> 8) as u8,
        (y2 & 0xFF) as u8,
    ]);
    spu.push(0x06);
    spu.extend_from_slice(&(off1 as u16).to_be_bytes());
    spu.extend_from_slice(&(off2 as u16).to_be_bytes());
    spu.push(0xFF);
    debug_assert_eq!(spu.len(), cmds2, "cmds2 offset arithmetic");
    // DCSQT 2: date = end_date, self-chained (terminates).
    spu.extend_from_slice(&end_date.to_be_bytes());
    spu.extend_from_slice(&(cmds2 as u16).to_be_bytes());
    spu.push(0x02); // stop display at the DCSQT date
    spu.push(0xFF);
    let total = spu.len();
    spu[0..2].copy_from_slice(&(total as u16).to_be_bytes());
    spu[2..4].copy_from_slice(&(cmds as u16).to_be_bytes());
    spu
}

/// Wrap one SPU in a PES 0xBD with substream 0x20 (the .sub file's
/// carriage).
fn wrap_pes(substream: u8, spu: &[u8]) -> Vec<u8> {
    let mut pes = Vec::new();
    pes.extend_from_slice(&[0x00, 0x00, 0x01, 0xBD]);
    let len = 3 + 1 + spu.len();
    pes.extend_from_slice(&(len as u16).to_be_bytes());
    pes.extend_from_slice(&[0x81, 0x00, 0x00]);
    pes.push(substream);
    pes.extend_from_slice(spu);
    pes
}

// -- the tests ----------------------------------------------------------------

/// THE FX9 inversion, present-path half: an SRT sidecar composited
/// through the real run — every presented frame INSIDE the cue
/// window carries ink, every frame OUTSIDE it is byte-identical to
/// the no-lane baseline (the no-cue case leaves the bytes
/// untouched), and the lane never moves the present schedule.
#[test]
fn fx9_a03_srt_sidecar_inks_inside_and_is_absent_outside_the_cue_window() {
    let baseline = presented_frames(None);
    let composited = presented_frames(Some(&fixture("one.srt")));
    assert_eq!(
        baseline.keys().collect::<Vec<_>>(),
        composited.keys().collect::<Vec<_>>(),
        "the sidecar lane never moves the present schedule"
    );
    let (mut outside, mut inside) = (0_usize, 0_usize);
    for (pts, clean) in &baseline {
        let inked = composited.get(pts).must();
        assert_eq!(clean.len(), inked.len(), "frame at pts {pts} ns");
        let differs = first_diff(clean, inked).is_some();
        if in_cue_window(*pts) {
            inside += 1;
            assert!(
                differs,
                "frame at pts {pts} ns is inside [{CUE_START_NS}, {CUE_END_NS}) ns — it must carry ink"
            );
        } else {
            outside += 1;
            assert!(
                !differs,
                "frame at pts {pts} ns is outside the cue window — the no-cue case must leave the bytes untouched"
            );
        }
    }
    assert!(outside >= 10, "the window's outside half exists: {outside}");
    assert!(inside >= 40, "the window's inside half exists: {inside}");
}

/// The placement contract: the SRT cue's ink lands in the BOTTOM
/// band (the default bottom-center anchor pulled inside by the 5 %
/// title-safe clamp), and nothing above the band ever changes.
#[test]
fn fx9_a03_srt_cue_ink_lands_in_the_bottom_band() {
    let baseline = presented_frames(None);
    let composited = presented_frames(Some(&fixture("one.srt")));
    let (_, during) = window_edges(&baseline);
    let clean = baseline.get(&during).must();
    let inked = composited.get(&during).must();
    assert_eq!(clean.len(), inked.len());
    assert!(
        first_diff(clean, inked).is_some(),
        "the frame inside the cue window carries cue ink"
    );
    let split = TOP_SPLIT_Y * STRIDE;
    assert!(
        first_diff(&clean[..split], &inked[..split]).is_none(),
        "no ink above the bottom band (rows 0..{TOP_SPLIT_Y})"
    );
    assert!(
        first_diff(&clean[split..], &inked[split..]).is_some(),
        "the cue's ink landed in the bottom band (rows {TOP_SPLIT_Y}..{FRAME_H})"
    );
}

/// The WebVTT sidecar: header-bearing text cues composite inside
/// the cue window, confined to the bottom band, and the frame
/// before the window stays byte-identical.
#[test]
fn fx9_a03_vtt_sidecar_composites_inside_the_cue_window() {
    assert_sidecar_composites_inside("one.vtt");
}

/// The ASS sidecar: the Default style's dialogue run (half-alpha
/// WHITE — `&H80FFFFFF` in ASS `&HAABBGGRR` order) composites
/// inside the cue window, confined to the bottom band.
#[test]
fn fx9_a03_ass_sidecar_composites_inside_the_cue_window() {
    assert_sidecar_composites_inside("one.ass");
}

/// The verb chain's arrival truth: two sidecar adds through the
/// REAL dispatch, the scale and sync lanes, the selection through
/// `tracks.set-subtitle-track` — every answer the chain's own
/// read-back, and the whole registry + selection visible in ONE
/// `tracks.list` snapshot.
#[test]
fn fx9_a03_verb_chain_reads_back_through_the_track_snapshot() {
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    // Before: the fixture's own audio row, no subtitle rows.
    match dispatch(&mut host, Command::TracksList) {
        Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) => {
            assert!(
                !tracks.iter().any(|row| row.kind == TrackKind::Subtitle),
                "the WAV fixture carries no subtitle rows: {tracks:?}"
            );
        }
        other => unreachable!("tracks.list must complete: {other:?}"),
    }
    // The chain: two adds (the registry), then the scale and sync
    // lanes' own read-backs.
    for name in ["one.vtt", "one.srt"] {
        match dispatch(
            &mut host,
            Command::SubtitleAddFile {
                path: fixture(name),
                encoding: None,
                select: false,
            },
        ) {
            Ok(CommandOutcome::Accepted { session, .. }) => assert!(
                session.is_some(),
                "the {name} add answered inside the session"
            ),
            other => unreachable!("the {name} add must be served: {other:?}"),
        }
    }
    match dispatch(&mut host, Command::SubtitleSetScale { scale: 1.5 }) {
        Ok(CommandOutcome::Completed(CommandResult::AppliedScale { scale })) => {
            assert_eq!(scale, 1.5, "the applied scale is the lane's read-back");
        }
        other => unreachable!("set-scale must complete: {other:?}"),
    }
    match dispatch(&mut host, Command::SubtitleSetSync { delta_ms: 250 }) {
        Ok(CommandOutcome::Completed(CommandResult::AppliedDelayMs { delay_ms })) => {
            assert_eq!(
                delay_ms, 250,
                "the new absolute total is the lane's read-back, never the delta"
            );
        }
        other => unreachable!("set-sync must complete: {other:?}"),
    }
    // The snapshot: both sidecar rows registered, external, unselected.
    let vtt_id = match dispatch(&mut host, Command::TracksList) {
        Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) => {
            let sidecars: Vec<_> = tracks
                .iter()
                .filter(|row| row.kind == TrackKind::Subtitle)
                .collect();
            assert_eq!(
                sidecars.len(),
                2,
                "both sidecar rows are registered: {tracks:?}"
            );
            for row in &sidecars {
                assert_eq!(
                    row.origin,
                    TrackOrigin::ExternalSubtitle,
                    "the added file's row: {row:?}"
                );
                assert!(!row.is_selected, "select: false left it unselected: {row:?}");
            }
            sidecars
                .iter()
                .find(|row| row.name.as_deref() == Some("one.vtt"))
                .must()
                .id
        }
        other => unreachable!("tracks.list must complete: {other:?}"),
    };
    // The selection through the verb → the applied selector answers,
    // the snapshot's flags agree.
    match dispatch(
        &mut host,
        Command::TracksSetSubtitleTrack {
            selector: TrackSelector::Id { id: vtt_id },
        },
    ) {
        Ok(CommandOutcome::Completed(CommandResult::AppliedSelector { selector })) => {
            assert_eq!(
                selector,
                TrackSelector::Id { id: vtt_id },
                "the applied selector is the chain's read-back"
            );
        }
        other => unreachable!("set-subtitle-track must complete: {other:?}"),
    }
    match dispatch(&mut host, Command::TracksList) {
        Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) => {
            for row in tracks.iter().filter(|row| row.kind == TrackKind::Subtitle) {
                assert_eq!(
                    row.is_selected,
                    row.id == vtt_id,
                    "the snapshot holds the selection: {row:?}"
                );
            }
        }
        other => unreachable!("tracks.list must complete: {other:?}"),
    }
    // Deselect → the snapshot's flag agrees.
    match dispatch(
        &mut host,
        Command::TracksSetSubtitleTrack {
            selector: TrackSelector::None,
        },
    ) {
        Ok(CommandOutcome::Completed(CommandResult::AppliedSelector { selector })) => {
            assert_eq!(selector, TrackSelector::None);
        }
        other => unreachable!("the deselect must complete: {other:?}"),
    }
    match dispatch(&mut host, Command::TracksList) {
        Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) => {
            let row = tracks.iter().find(|row| row.id == vtt_id).must();
            assert!(!row.is_selected, "the deselect reached the snapshot's flag");
        }
        other => unreachable!("tracks.list must complete: {other:?}"),
    }
    // The engine's own lane rode the composition (the embedded
    // half's observable — its tracks.changed events).
    assert!(
        host.events().iter().any(|event| {
            matches!(
                event.kind,
                commands::EventKind::TracksChanged {
                    track_kind: commands::catalog::types::TrackChangeKind::Subtitle,
                    ..
                }
            )
        }),
        "the engine's SelectSubtitle lane rode beside every sidecar selection"
    );
}

/// Determinism of the composited present path: two whole-media runs
/// with the same selected sidecar agree on every presented frame's
/// bytes, frame for frame (the `pipeline_accounting.rs` determinism
/// oracle class, now over the composited pixels).
#[test]
fn fx9_a03_composited_presented_frames_are_deterministic() {
    let first = presented_frames(Some(&fixture("one.srt")));
    let second = presented_frames(Some(&fixture("one.srt")));
    assert_eq!(
        first.keys().collect::<Vec<_>>(),
        second.keys().collect::<Vec<_>>(),
        "the same present schedule"
    );
    for (pts, a) in &first {
        let b = second.get(pts).must();
        assert_eq!(a.len(), b.len(), "frame at pts {pts} ns");
        let diff = first_diff(a, b);
        assert!(
            diff.is_none(),
            "two composited runs agree byte-for-byte at pts {pts} ns \
             (first diff at byte {})",
            diff.must()
        );
    }
}

/// The embedded lane (d): A04's REAL `PgsSubtitleDecoder` reduces a
/// crafted display set to a `Frame::Subtitle` overlay payload; the
/// arm's body (`accept_subtitle_frame`) holds it on the lane, the
/// overlay reconstructs through the compositing contract and lands
/// at its authored placement — with the seek boundary clearing the
/// set and a text cue honestly drawing nothing (no producer).
#[test]
fn fx9_a03_embedded_overlay_reconstructs_and_composites() {
    // Object 1, 2×2 studio white, at (10, 20) on the 1920×1080 TS
    // canvas (the crate's own crafted set shape).
    let set_bytes = display_set(&[(1, 10, 20)], (1920, 1080));
    // The real decoder: configured on the demuxer's `pgssub` mint,
    // one packet, then the flush that closes the trailing set.
    let mut decoder = PgsSubtitleDecoder::new();
    let config = DecoderConfig::new(
        media_types::ids::CodecDescriptor::new(DESCRIPTOR).must(),
        media_types::GenerationId::FIRST,
        ThreadBudget::default(),
        None,
        None,
    );
    match decoder.configure(&config) {
        Ok(()) => {}
        Err(error) => unreachable!("configure failed: {error}"),
    }
    let packet = Packet {
        track: TrackId::new(0),
        pts: ns_stamp(500),
        dts: ns_stamp(500),
        duration: None,
        data: SharedBytes::from_vec(set_bytes),
        keyframe: true,
        discontinuity: false,
        generation: media_types::GenerationId::FIRST,
    };
    let mut sink = Sink {
        frames: Vec::new(),
    };
    match decoder.decode(packet, &mut sink) {
        Ok(DecodeOutcome::NeedMoreInput) => {}
        other => unreachable!("the first display set is held, not emitted: {other:?}"),
    }
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("the flush emits the trailing set: {other:?}"),
    }
    assert_eq!(sink.frames.len(), 1);
    let subtitle = match sink.frames.first() {
        Some(Frame::Subtitle(frame)) => frame.clone(),
        other => unreachable!("the flush emitted a subtitle frame: {other:?}"),
    };
    // The overlay payload: the placement recovered from the authored
    // anchor, the pixels straight from the composed image.
    let cue = subtitle.cues.first().must();
    let overlay = match &cue.overlay {
        Some(overlay) => overlay,
        None => unreachable!("a bitmap cue carries its overlay: {cue:?}"),
    };
    assert_eq!(
        (overlay.x, overlay.y),
        (10, 20),
        "the authored placement survived the vocabulary round-trip"
    );
    assert_eq!((overlay.width, overlay.height), (2, 2));
    assert!(
        overlay
            .rgba
            .as_slice()
            .chunks(4)
            .all(|px| px == [255, 255, 255, 255]),
        "every composed pixel is opaque white"
    );
    assert_eq!(cue.span.start, ns_stamp(500));
    assert_eq!(cue.span.end, Stamp::Missing, "the trailing set is open-ended");
    // The arm's body + the compositing seam: ink at the placement,
    // byte-identical rows everywhere else.
    let mut lane = SubtitleLane::new();
    lane.accept_subtitle_frame(&subtitle);
    let mut inside = rgba_frame(600);
    let drawn = lane.composite_onto(&mut inside, Timestamp::new(600_000_000, NANOS_BASE));
    assert_eq!(drawn, 1, "one cue drawn at 600 ms");
    assert_eq!(
        frame_bytes(&inside),
        with_block(10, 20, 2, 2, [255, 255, 255, 255]),
        "the overlay reconstructed at (10, 20) and composited white ink"
    );
    let mut outside = rgba_frame(400);
    let drawn = lane.composite_onto(&mut outside, Timestamp::new(400_000_000, NANOS_BASE));
    assert_eq!(drawn, 0, "the cue is not active at 400 ms");
    assert_eq!(frame_bytes(&outside), bg_bytes(), "no ink outside the cue window");
    // The seek boundary clears the embedded set.
    lane.clear_embedded();
    let mut cleared = rgba_frame(600);
    let drawn = lane.composite_onto(&mut cleared, Timestamp::new(600_000_000, NANOS_BASE));
    assert_eq!(drawn, 0, "the cleared lane draws nothing");
    assert_eq!(frame_bytes(&cleared), bg_bytes());
    // A text cue honestly draws nothing (no in-tree text producer).
    lane.accept_subtitle_frame(&SubtitleFrame {
        cues: vec![SubtitleCue::text(
            TimeSpan {
                start: ns_stamp(500),
                end: ns_stamp(3000),
            },
            "no embedded text producer",
        )],
        pts: ns_stamp(500),
        generation: media_types::GenerationId::FIRST,
    });
    let mut text_only = rgba_frame(600);
    let drawn = lane.composite_onto(&mut text_only, Timestamp::new(600_000_000, NANOS_BASE));
    assert_eq!(drawn, 0, "a text payload without a producer draws nothing");
    assert_eq!(frame_bytes(&text_only), bg_bytes());
}

/// The `.sup` sidecar: the standalone carriage (PTS stamped before
/// every segment) walks through `PgsFraming::detect`, the PGS epoch
/// composes the object, and the cue lands at its authored placement
/// on the frame's own extent — with the read-back's cue count and
/// the byte-identical no-cue case outside the window.
#[test]
fn fx9_a03_pgs_sup_sidecar_composites_the_crafted_display_set() {
    // 45_000 ticks of the 90 kHz carriage = the 500 ms cue start.
    let pts_90khz: u32 = 45_000;
    let sup = [
        sup_seg(pts_90khz, 0x16, &pcs_data(0x80, 0, &[(1, 10, 20)], (320, 240))),
        sup_seg(pts_90khz, 0x14, &pds_data(0, 0, &[WHITE_ENTRY])),
        sup_seg(pts_90khz, 0x15, &ods_first_data(1, 2, 2, &RLE_2X2_WHITE)),
        sup_seg(pts_90khz, 0x80, &[]),
    ]
    .concat();
    let dir = TempDir::new("sup");
    let sup_path = dir.write("one.sup", &sup).display().to_string();
    let mut lane = SubtitleLane::new();
    let sidecar = match lane.add_sidecar(&sup_path, None, true) {
        Ok(sidecar) => sidecar,
        Err(error) => unreachable!("the crafted .sup refused the load: {error}"),
    };
    assert_eq!(sidecar.cues, 1, "one display set, one cue: {sidecar:?}");
    assert_eq!(sidecar.encoding, "binary");
    // Inside the window: the exact authored placement.
    let mut inside = rgba_frame(600);
    let drawn = lane.composite_onto(&mut inside, Timestamp::new(600_000_000, NANOS_BASE));
    assert_eq!(drawn, 1, "the display-set cue is active at 600 ms");
    assert_eq!(
        frame_bytes(&inside),
        with_block(10, 20, 2, 2, [255, 255, 255, 255]),
        "the .sup cue composited its 2×2 white block at (10, 20)"
    );
    // Outside the window: untouched.
    let mut outside = rgba_frame(400);
    let drawn = lane.composite_onto(&mut outside, Timestamp::new(400_000_000, NANOS_BASE));
    assert_eq!(drawn, 0, "the cue starts at 500 ms");
    assert_eq!(frame_bytes(&outside), bg_bytes());
}

/// The VobSub `.idx`+`.sub` pair: the index's own canvas pins the
/// placement, the SPU's crafted RLE crops to the visible run, and
/// the palette's red lands as exactly the authored pixels inside
/// the window.
#[test]
fn fx9_a03_vobsub_idx_sub_pair_sidecar_composites_inside_the_cue_window() {
    // One 4×2 SPU whose visible run crops to a 1×2 red block at
    // (48, 96) — the crate's pair-test shape, at the frame's own
    // extent. start_date 0 → the idx's own 500 ms timestamp is the
    // cue start; end_date 219 → (219·1024)/90 ms later.
    let top = [0x60_u8, 0x00, 0x00];
    let bottom = [0x60_u8, 0x00, 0x00];
    let spu = build_spu(&top, &bottom, 48, 96, 4, 2, 0, 219);
    let sub = wrap_pes(0x20, &spu);
    let idx = concat!(
        "# VobSub index file, v7\n",
        "size: 320x240\n",
        "palette: 000000, ff0000, 00ff00, ffff00, ff0000, ff00ff, ffff00, ffffff,\n",
        "          808080, 404040, 202020, 101010, 080808, 040404, 020202, 010101\n",
        "id: en, index: 0\n",
        "timestamp: 00:00:00:500, filepos: 000000000\n",
    );
    let dir = TempDir::new("vobsub");
    let sub_path = dir.write("one.sub", &sub);
    assert!(sub_path.is_file(), "the pair's .sub half exists");
    let idx_path = dir.write("one.idx", idx.as_bytes()).display().to_string();
    let mut lane = SubtitleLane::new();
    let sidecar = match lane.add_sidecar(&idx_path, None, true) {
        Ok(sidecar) => sidecar,
        Err(error) => unreachable!("the crafted pair refused the load: {error}"),
    };
    assert_eq!(sidecar.cues, 1, "one SPU, one cue: {sidecar:?}");
    assert_eq!(sidecar.encoding, "binary");
    // Inside the window: the 1×2 red block at the authored place.
    let mut inside = rgba_frame(600);
    let drawn = lane.composite_onto(&mut inside, Timestamp::new(600_000_000, NANOS_BASE));
    assert_eq!(drawn, 1, "the SPU cue is active at 600 ms");
    assert_eq!(
        frame_bytes(&inside),
        with_block(48, 96, 1, 2, [255, 0, 0, 255]),
        "the .idx/.sub pair composited its red run at (48, 96)"
    );
    // Outside the window: untouched.
    let mut outside = rgba_frame(400);
    let drawn = lane.composite_onto(&mut outside, Timestamp::new(400_000_000, NANOS_BASE));
    assert_eq!(drawn, 0, "the cue starts at the 500 ms timestamp");
    assert_eq!(frame_bytes(&outside), bg_bytes());
}

/// Bad sidecar inputs refuse TYPED, at every honest boundary of the
/// seam: the unsupported extension, the unreadable file, the
/// format-level refusal (WebVTT's missing header), the empty
/// reduction, and the VobSub pair's missing `.sub` half — and a
/// refused add through the REAL dispatch leaves no sidecar row in
/// the snapshot (the class-A arrival truth).
#[test]
fn fx9_a03_bad_sidecar_inputs_refuse_typed() {
    let dir = TempDir::new("bad");
    let cases: [(&str, Option<&[u8]>, &str); 5] = [
        // Even for an existing file: no format dispatches `.txt`.
        ("one.txt", Some(b"Hello compositing lane"), "cli.subtitle_sidecar_unsupported"),
        // The file cannot be read at all.
        ("missing.srt", None, "cli.subtitle_sidecar_read"),
        // The one fatal WebVTT refusal: the missing header line.
        (
            "bad.vtt",
            Some(b"00:00:00.500 --> 00:00:03.000\nHello compositing lane\n"),
            "cli.subtitle_sidecar_parse",
        ),
        // A parseable SRT whose payload reduced to zero cues.
        (
            "cueless.srt",
            Some(b"just words, no timestamp, no cue\n"),
            "cli.subtitle_sidecar_empty",
        ),
        // The VobSub pair's other half is missing.
        (
            "orphan.idx",
            Some(b"size: 320x240\nid: en, index: 0\ntimestamp: 00:00:00:500, filepos: 000000000\n"),
            "cli.subtitle_sidecar_read",
        ),
    ];
    for (name, bytes, code) in cases {
        let path = match bytes {
            Some(bytes) => dir.write(name, bytes).display().to_string(),
            None => format!("{}/missing.srt", dir.0.display()),
        };
        let mut lane = SubtitleLane::new();
        match lane.add_sidecar(&path, None, true) {
            Err(error) => {
                assert_eq!(error.context.code, code, "{name}: {error}");
                assert_eq!(error.stage, PipelineStage::Demux, "{name}: {error}");
            }
            Ok(_) => unreachable!("{name} must refuse"),
        }
    }
    // The host-level honest observable: the class-A add answers
    // Accepted, but a refused load leaves NO sidecar row.
    let mut host = CliHost::new();
    open(&mut host, &wav_path());
    let bad = dispatch(
        &mut host,
        Command::SubtitleAddFile {
            path: format!("{}/one.txt", dir.0.display()),
            encoding: None,
            select: true,
        },
    );
    match bad {
        Ok(CommandOutcome::Accepted { session, .. }) => assert!(
            session.is_some(),
            "the add answered inside the session"
        ),
        other => unreachable!("the class-A add is served regardless: {other:?}"),
    }
    match dispatch(&mut host, Command::TracksList) {
        Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) => {
            assert!(
                !tracks.iter().any(|row| row.kind == TrackKind::Subtitle),
                "the refused add left no sidecar row: {tracks:?}"
            );
        }
        other => unreachable!("tracks.list must complete: {other:?}"),
    }
}

/// The decode-sink the embedded test drives (the A04 adapter's own
/// delivery vocabulary).
struct Sink {
    frames: Vec<Frame>,
}

impl FrameSink for Sink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        self.frames.push(frame);
        Ok(())
    }
}
