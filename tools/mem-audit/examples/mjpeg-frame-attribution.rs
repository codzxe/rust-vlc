//! D-1 attribution drilldown (card W5-15, violation register repro):
//! drive ONE real MJPEG frame through `MjpegDecoder::decode` N times
//! under a counting global allocator, printing the per-iteration
//! allocation deltas — the ground truth behind the audit's
//! `calls_per_pkt` column for the video limb.
//!
//! Usage: `cargo run -p lumen-mem-audit --example mjpeg-frame-attribution -- <file.avi>`
//!
//! What it isolates: the decoder in isolation. The demuxer hands the
//! first video packet; every iteration re-feeds the SAME packet bytes
//! (MJPEG is stateless, the decode context is the real one), so the
//! steady-state per-iteration delta is exactly the decoder's per-frame
//! allocation behavior — the demux/pool side is measured separately by
//! `lumen-mem-audit --demux-only`.
//!
//! The counting allocator here is the same minimal
//! `unsafe impl GlobalAlloc` wrapper the tool itself uses (see
//! src/allocator.rs for the safety argument — there is no safe Rust
//! surface for a global allocator).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink, ThreadBudget};
use media_types::ids::CodecDescriptor;
use media_types::{EngineError, Frame, GenerationId, Packet, TrackId};
use plugin_api::{Access, DemuxEvent, Demuxer};

static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);

struct Counting;

// SAFETY: forwards pointer+layout verbatim to System and counts on
// success only (the tool's allocator.rs argument, verbatim).
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: GlobalAlloc contract on both sides.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: caller-guaranteed provenance.
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

struct NullSink;

impl FrameSink for NullSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        Ok(())
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: mjpeg-frame-attribution <file.avi>");
        std::process::exit(64);
    });
    // Open the AVI through the real access + probe + demuxer path,
    // harvest the first video packet, then decode it repeatedly.
    let uri = media_types::SourceUri::parse(&format!("file://{path}")).unwrap_or_else(|err| {
        eprintln!("error: {err}");
        std::process::exit(65);
    });
    let access = input::FileAccess::new();
    let mut stream = access.open(&uri).unwrap_or_else(|err| {
        eprintln!("error: {err}");
        std::process::exit(65);
    });
    let registry = media_probe::ProbeRegistry::with_builtins();
    let config = media_probe::ProbeConfig::default();
    let report =
        media_probe::probe_stream(&registry, &mut *stream, &config).unwrap_or_else(|err| {
            eprintln!("error: {err}");
            std::process::exit(65);
        });
    if report.format.as_str() != "avi" {
        eprintln!("error: not an AVI");
        std::process::exit(65);
    }
    let mut demuxer =
        demux::avi::AviDemuxer::new(stream).unwrap_or_else(|err| {
            eprintln!("error: {err}");
            std::process::exit(65);
        });
    let tracks = demuxer.start().unwrap_or_else(|err| {
        eprintln!("error: {err}");
        std::process::exit(65);
    });
    let video_track = tracks
        .iter()
        .find(|t| {
            matches!(
                demuxer.stream_format(t.id),
                Some(demux::avi::header::StreamFormat::MotionJpeg { .. })
            )
        })
        .map(|t| t.id);
    let Some(video_track) = video_track else {
        eprintln!("error: no mjpeg video track");
        std::process::exit(65);
    };
    let mut first: Option<Packet> = None;
    while let Ok(event) = demuxer.next_event() {
        match event {
            DemuxEvent::Packet(packet) if packet.track == video_track => {
                first = Some(packet);
                break;
            }
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    let Some(packet) = first else {
        eprintln!("error: no video packet");
        std::process::exit(65);
    };
    // Stamp is Copy: the pts rides every replay identically.
    let pts = packet.pts;
    println!(
        "# frame: {} bytes, dims from strf: {:?}",
        packet.data.len(),
        demuxer.stream_format(video_track)
    );

    // The decoder, configured exactly like the walk does.
    let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
    let descriptor = CodecDescriptor::new("mjpeg").unwrap_or_else(|err| unreachable!("{err}"));
    decoder
        .configure(&DecoderConfig::new(
            descriptor,
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            None,
        ))
        .unwrap_or_else(|err| {
            eprintln!("error: {err}");
            std::process::exit(65);
        });

    const ITERATIONS: usize = 200;
    let mut calls = [0u64; ITERATIONS];
    let mut bytes = [0u64; ITERATIONS];
    let mut decoded: u64 = 0;
    for iteration in 0..ITERATIONS {
        let before_calls = ALLOC_CALLS.load(Ordering::Relaxed);
        let before_bytes = ALLOC_BYTES.load(Ordering::Relaxed);
        // Re-feed the same bytes (MJPEG is stateless; the packet
        // struct is rebuilt per iteration — stack data plus one Arc
        // clone: refcount only, no allocation).
        let replay = Packet {
            track: TrackId::new(0),
            pts,
            dts: packet.dts,
            duration: packet.duration,
            data: packet.data.clone(),
            keyframe: packet.keyframe,
            discontinuity: false,
            generation: GenerationId::FIRST,
        };
        match decoder.decode(replay, &mut NullSink) {
            Ok(DecodeOutcome::Emitted { frames }) => decoded += u64::from(frames),
            Ok(_) => {}
            Err(err) => {
                eprintln!("error: decode failed: {err}");
                std::process::exit(65);
            }
        }
        calls[iteration] = ALLOC_CALLS.load(Ordering::Relaxed) - before_calls;
        bytes[iteration] = ALLOC_BYTES.load(Ordering::Relaxed) - before_bytes;
    }
    let mut histogram: std::collections::BTreeMap<(u64, u64), usize> =
        std::collections::BTreeMap::new();
    for iteration in 0..ITERATIONS {
        *histogram.entry((calls[iteration], bytes[iteration])).or_default() += 1;
    }
    println!("# decoded frames: {decoded}/{ITERATIONS}");
    println!("# per-iteration (calls, bytes) -> count:");
    for ((calls, bytes), count) in &histogram {
        println!("ATTRIB calls={calls} bytes={bytes} iterations={count}");
    }

    // Phase B: the REAL per-frame stream — continue the demux walk,
    // decode every subsequent packet once (varying sizes, real
    // pipeline interleaving), per-packet deltas histogram.
    let mut stream_hist: std::collections::BTreeMap<(u64, u64), usize> =
        std::collections::BTreeMap::new();
    let mut streamed: u64 = 0;
    loop {
        let before_calls = ALLOC_CALLS.load(Ordering::Relaxed);
        let before_bytes = ALLOC_BYTES.load(Ordering::Relaxed);
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                if let Ok(DecodeOutcome::Emitted { frames }) = decoder.decode(packet, &mut NullSink)
                {
                    streamed += u64::from(frames);
                }
            }
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(_) => {}
            Err(err) => {
                eprintln!("error: mid-walk: {err}");
                std::process::exit(65);
            }
        }
        *stream_hist
            .entry(
                (
                    ALLOC_CALLS.load(Ordering::Relaxed) - before_calls,
                    ALLOC_BYTES.load(Ordering::Relaxed) - before_bytes,
                ),
            )
            .or_default() += 1;
    }
    println!("# streamed frames: {streamed}");
    println!("# per-packet (calls, bytes) -> count:");
    for ((calls, bytes), count) in &stream_hist {
        println!("STREAM calls={calls} bytes={bytes} packets={count}");
    }
}
