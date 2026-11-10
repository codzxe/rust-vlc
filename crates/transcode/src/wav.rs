//! The RIFF/WAVE PCM writer (card 7.E1) — the v1 "encoder": a
//! streaming, size-patching serializer for the one container this
//! crate honestly writes.
//!
//! # What it is (and is not)
//!
//! This is a **container writer**, not a codec [`Encoder`] — WAV PCM
//! is uncompressed serialization, not bitstream compression, and
//! implementing codec-api's packet-emitting `Encoder` here would
//! fabricate a PCM-packet vocabulary no consumer exists for (MP §4:
//! no invented capabilities). The codec-encoder seam joins with the
//! 7.E3 card `[PROPOSAL: the encode stage becomes a factory-built
//! codec-api Encoder between decode and the sinks — the same
//! DecoderFactory shape the decode stage already carries]`.
//!
//! # The byte format written (public-specification clean-room)
//!
//! The canonical 44-byte header, then payload bytes verbatim:
//!
//! ```text
//! "RIFF" <u32 riff_size = file_size − 8> "WAVE"
//! "fmt " <u32 16> <u16 tag> <u16 channels> <u32 rate>
//!        <u32 byte_rate> <u16 block_align> <u16 bits>
//! "data" <u32 data_size>
//! ```
//!
//! `tag` is `1` for integer PCM (u8/s16/s24/s32) and `3` for IEEE
//! float (f32/f64) — the two families the `fmt ` parser on the read
//! side ([`demux::wav::format`]) accepts as plain PCM. The header is
//! written **lazily, on the first payload write**, so the decode
//! path can learn channels/rate from the first decoded frame; an
//! empty job still finalizes to a structurally valid 44-byte WAV
//! with `data` size zero (sizes tell the truth about nothing).
//!
//! # Bounded resources (MP §12)
//!
//! The writer holds no payload buffer at all: payload slices stream
//! straight through to the [`WriteSeek`] target (the conversion
//! scratch lives in the decode sink adapter, bounded by one frame).
//! The two arithmetic ceilings are typed, never wrapped:
//!
//! * `rate × block_align` must fit the `fmt ` chunk's 32-bit
//!   `byte_rate` ([`wav_byte_rate_overflow`]);
//! * total `data` bytes must stay addressable by a classic RIFF
//!   (not RF64) `u32` size field — `u32::MAX − 36` bytes of payload
//!   ([`wav_size_overflow`]; RF64 is the later remux card's shape).
//!
//! Whole-frame discipline: every payload handed to
//! [`write_pcm`](WavPcmWriter::write_pcm) must be a whole number of
//! sample frames — a torn tail would make the file lie about its
//! own sample count ([`wav_partial_frame`]).

use std::io::{Seek, SeekFrom, Write};

use demux::wav::format::{MAX_CHANNELS, PcmSample};
use media_types::EngineError;

use crate::error::{
    sink_seek_io, sink_write_io, wav_byte_rate_overflow, wav_channels_out_of_range,
    wav_partial_frame, wav_rate_zero, wav_size_overflow,
};
use crate::graph::WriteSeek;

/// `wFormatTag` for integer PCM (the RIFF spec's `WAVE_FORMAT_PCM`).
const TAG_PCM: u16 = 1;
/// `wFormatTag` for IEEE float PCM (`WAVE_FORMAT_IEEE_FLOAT`).
const TAG_IEEE_FLOAT: u16 = 3;
/// The canonical header the writer emits: 12-byte RIFF head + 8-byte
/// `fmt ` head + 16-byte fmt body + 8-byte `data` head.
const HEADER_BYTES: usize = 44;
/// The RIFF size fields are `u32`; with the fixed 36 bytes of head
/// after `RIFF <size>`, payload space is `u32::MAX − 36`.
const MAX_DATA_BYTES: u64 = u32::MAX as u64 - 36;

/// The streaming WAVE PCM writer.
///
/// Build with [`WavPcmWriter::new`] (which validates the facts the
/// header will carry), feed whole sample-frame payload slices, then
/// [`finalize`](WavPcmWriter::finalize) to patch the `RIFF`/`data`
/// sizes and recover the underlying sink. Not `Clone`: one writer
/// owns one output.
pub(crate) struct WavPcmWriter {
    out: Box<dyn WriteSeek>,
    /// The PCM representation the `fmt ` chunk declares.
    sample: PcmSample,
    channels: u16,
    rate: u32,
    /// Whether the 44-byte header has been written (first payload).
    header_written: bool,
    /// Payload bytes written so far (the future `data` size field).
    data_bytes: u64,
}

impl WavPcmWriter {
    /// Validate the header facts and assemble the writer.
    ///
    /// # Errors
    /// Typed at stage `Transcode` (module docs' two ceilings, plus
    /// the channel/rate vocabulary): [`wav_channels_out_of_range`]
    /// for channels outside `1..=MAX_CHANNELS` (the demux layer's
    /// own ceiling, reused — one accepted-set), [`wav_rate_zero`]
    /// for a zero rate, [`wav_byte_rate_overflow`] when
    /// `rate × block_align` does not fit `u32`.
    pub(crate) fn new(
        out: Box<dyn WriteSeek>,
        sample: PcmSample,
        channels: u16,
        rate: u32,
    ) -> Result<WavPcmWriter, EngineError> {
        if channels == 0 || channels > MAX_CHANNELS {
            return Err(wav_channels_out_of_range(u32::from(channels)));
        }
        if rate == 0 {
            return Err(wav_rate_zero());
        }
        let block_align = u32::from(channels) * u32::from(sample.bytes_per_sample());
        let byte_rate = u64::from(rate) * u64::from(block_align);
        if byte_rate > u64::from(u32::MAX) {
            return Err(wav_byte_rate_overflow());
        }
        Ok(WavPcmWriter {
            out,
            sample,
            channels,
            rate,
            header_written: false,
            data_bytes: 0,
        })
    }

    /// Stream one payload slice of whole sample frames to the sink.
    ///
    /// Writes the header first (lazily), checks the whole-frame and
    /// RIFF-size contracts, then hands the bytes to the sink
    /// verbatim. Returns the payload bytes written (the caller's
    /// accounting denominator).
    ///
    /// # Errors
    /// [`wav_partial_frame`] for a torn tail, [`wav_size_overflow`]
    /// past the RIFF ceiling, [`sink_write_io`] when the sink fails.
    pub(crate) fn write_pcm(&mut self, payload: &[u8]) -> Result<u64, EngineError> {
        self.ensure_room(payload.len() as u64)?;
        if !self.header_written {
            self.write_header()?;
            self.header_written = true;
        }
        if payload.is_empty() {
            return Ok(0);
        }
        write_all(&mut self.out, payload)?;
        self.data_bytes = self.data_bytes.saturating_add(payload.len() as u64);
        Ok(payload.len() as u64)
    }

    /// The whole-frame + RIFF-ceiling guard, separated so the ceiling
    /// arithmetic is testable without materializing 4 GiB (module
    /// docs; the same function `write_pcm` runs before every write).
    ///
    /// # Errors
    /// [`wav_partial_frame`] / [`wav_size_overflow`] per module docs.
    pub(crate) fn ensure_room(&mut self, add: u64) -> Result<(), EngineError> {
        // `new` validated channels >= 1 and the width >= 1, so the
        // frame size is never zero (no division hazard).
        let frame = u64::from(self.channels) * u64::from(self.sample.bytes_per_sample());
        if !add.is_multiple_of(frame) {
            return Err(wav_partial_frame());
        }
        let after = self.data_bytes.saturating_add(add);
        if after > MAX_DATA_BYTES {
            return Err(wav_size_overflow(after.saturating_add(HEADER_BYTES as u64)));
        }
        Ok(())
    }

    /// Patch the `RIFF` and `data` size fields and hand the sink back.
    ///
    /// Idempotent-shaped by consumption: takes `self`, so a writer
    /// finalizes exactly once. A finalized file's sizes tell the
    /// truth about exactly the bytes that made it — including zero.
    ///
    /// # Errors
    /// [`sink_seek_io`] / [`sink_write_io`] when the sink fails at
    /// patch time (the sizes may then be stale — the job's outcome
    /// carries the failure, no validity is claimed).
    pub(crate) fn finalize(mut self) -> Result<Box<dyn WriteSeek>, EngineError> {
        if !self.header_written {
            self.write_header()?;
            self.header_written = true;
        }
        seek(&mut self.out, SeekFrom::Start(4))?;
        let riff_size = u32::try_from(self.data_bytes.saturating_add(36));
        let data_size = u32::try_from(self.data_bytes);
        let (riff_size, data_size) = match (riff_size, data_size) {
            (Ok(r), Ok(d)) => (r, d),
            // `ensure_room` already refused everything past the
            // ceiling; reaching here means arithmetic was bypassed —
            // the typed answer, never a wrapped size.
            _ => return Err(wav_size_overflow(self.data_bytes)),
        };
        write_all(&mut self.out, &riff_size.to_le_bytes())?;
        seek(&mut self.out, SeekFrom::Start(40))?;
        write_all(&mut self.out, &data_size.to_le_bytes())?;
        self.out.flush().map_err(sink_write_io)?;
        Ok(self.out)
    }

    /// Payload bytes written so far (the truth `finalize` will patch).
    #[cfg(test)]
    pub(crate) fn data_bytes(&self) -> u64 {
        self.data_bytes
    }

    /// Emit the canonical 44-byte header with placeholder sizes.
    fn write_header(&mut self) -> Result<(), EngineError> {
        let tag = match self.sample {
            PcmSample::U8 | PcmSample::S16Le | PcmSample::S24Le | PcmSample::S32Le => TAG_PCM,
            PcmSample::F32Le | PcmSample::F64Le => TAG_IEEE_FLOAT,
        };
        let width = u32::from(self.sample.bytes_per_sample());
        let block_align = u32::from(self.channels) * width;
        let byte_rate = u64::from(self.rate) * u64::from(block_align);
        let mut header = [0u8; HEADER_BYTES];
        header[0..4].copy_from_slice(b"RIFF");
        header[8..12].copy_from_slice(b"WAVE");
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&16u32.to_le_bytes());
        header[20..22].copy_from_slice(&tag.to_le_bytes());
        header[22..24].copy_from_slice(&self.channels.to_le_bytes());
        header[24..28].copy_from_slice(&self.rate.to_le_bytes());
        header[28..32].copy_from_slice(&(byte_rate as u32).to_le_bytes());
        header[32..34].copy_from_slice(&(block_align as u16).to_le_bytes());
        header[34..36].copy_from_slice(&(width as u16 * 8).to_le_bytes());
        header[36..40].copy_from_slice(b"data");
        // Sizes patched at finalize; zero is the honest placeholder.
        write_all(&mut self.out, &header)?;
        Ok(())
    }
}

/// `write_all` with the typed sink-error fold.
fn write_all(out: &mut Box<dyn WriteSeek>, bytes: &[u8]) -> Result<(), EngineError> {
    out.write_all(bytes).map_err(sink_write_io)
}

/// `seek` with the typed sink-error fold.
fn seek(out: &mut Box<dyn WriteSeek>, to: SeekFrom) -> Result<u64, EngineError> {
    out.seek(to).map_err(sink_seek_io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Must, assert_err};

    /// A `Write + Seek` sink whose bytes land in a buffer the test
    /// can read after `finalize` hands the box back (a `Box<dyn
    /// WriteSeek>` cannot be read; the shared buffer is the seam).
    type SharedBuf = std::sync::Arc<std::sync::Mutex<Vec<u8>>>;

    struct SharedCursor {
        buf: SharedBuf,
        pos: usize,
    }

    impl SharedCursor {
        fn sink() -> (Box<dyn WriteSeek>, SharedBuf) {
            let buf: SharedBuf = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            (
                Box::new(SharedCursor {
                    buf: buf.clone(),
                    pos: 0,
                }),
                buf,
            )
        }
    }

    impl Write for SharedCursor {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let mut guard = self.buf.lock().unwrap_or_else(|p| p.into_inner());
            if self.pos + bytes.len() > guard.len() {
                guard.resize(self.pos + bytes.len(), 0);
            }
            guard[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
            self.pos += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Seek for SharedCursor {
        fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
            match to {
                SeekFrom::Start(n) => {
                    self.pos = n as usize;
                    Ok(n)
                }
                SeekFrom::Current(n) => {
                    self.pos = usize::try_from(self.pos as i64 + n).unwrap_or(self.pos);
                    Ok(self.pos as u64)
                }
                SeekFrom::End(n) => {
                    let len = self.buf.lock().unwrap_or_else(|p| p.into_inner()).len() as i64;
                    self.pos = usize::try_from(len + n).unwrap_or(self.pos);
                    Ok(self.pos as u64)
                }
            }
        }
    }

    /// A Write+Seek sink that discards bytes but counts them — the
    /// size-ceiling test's vehicle (4 GiB of writes without 4 GiB of
    /// memory; `Seek` reports the virtual position).
    struct DiscardSink {
        pos: u64,
    }

    impl Write for DiscardSink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.pos = self.pos.saturating_add(buf.len() as u64);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Seek for DiscardSink {
        fn seek(&mut self, _to: SeekFrom) -> std::io::Result<u64> {
            Ok(self.pos)
        }
    }

    fn u16at(file: &[u8], at: usize) -> u16 {
        u16::from_le_bytes(file[at..at + 2].try_into().must())
    }

    fn u32at(file: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(file[at..at + 4].try_into().must())
    }

    /// Read the sink's bytes back through the twin `Arc` (finalize
    /// wrote through the shared buffer; the returned box is dropped).
    fn file_of(twin: &SharedBuf) -> Vec<u8> {
        twin.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    #[test]
    fn header_is_the_canonical_44_bytes_per_sample_kind() {
        for (sample, tag, bits) in [
            (PcmSample::U8, 1u16, 8u16),
            (PcmSample::S16Le, 1, 16),
            (PcmSample::S24Le, 1, 24),
            (PcmSample::S32Le, 1, 32),
            (PcmSample::F32Le, 3, 32),
            (PcmSample::F64Le, 3, 64),
        ] {
            let (sink, twin) = SharedCursor::sink();
            let mut w = WavPcmWriter::new(sink, sample, 2, 48_000).must();
            // Two sample frames (stereo x the sample width).
            let payload = vec![0x5Au8; 4 * usize::from(sample.bytes_per_sample())];
            w.write_pcm(&payload).must();
            drop(w.finalize().must());
            let file = file_of(&twin);
            assert_eq!(file.len(), 44 + payload.len(), "{sample:?}");
            assert_eq!(&file[0..4], b"RIFF", "{sample:?}");
            assert_eq!(&file[8..12], b"WAVE", "{sample:?}");
            assert_eq!(&file[12..16], b"fmt ", "{sample:?}");
            assert_eq!(u32at(&file, 16), 16, "{sample:?}");
            assert_eq!(u16at(&file, 20), tag, "{sample:?}");
            assert_eq!(u16at(&file, 22), 2, "{sample:?}");
            assert_eq!(u32at(&file, 24), 48_000, "{sample:?}");
            let width = u32::from(sample.bytes_per_sample());
            assert_eq!(u32at(&file, 28), 48_000 * 2 * width, "{sample:?}");
            assert_eq!(u16at(&file, 32), (2 * width) as u16, "{sample:?}");
            assert_eq!(u16at(&file, 34), bits, "{sample:?}");
            assert_eq!(&file[36..40], b"data", "{sample:?}");
            assert_eq!(u32at(&file, 40), payload.len() as u32, "{sample:?}");
            assert_eq!(u32at(&file, 4), (36 + payload.len()) as u32, "{sample:?}");
            assert_eq!(&file[44..], payload.as_slice(), "{sample:?}");
        }
    }

    #[test]
    fn lazy_header_means_empty_jobs_write_only_the_header() {
        let (sink, twin) = SharedCursor::sink();
        let w = WavPcmWriter::new(sink, PcmSample::S16Le, 1, 8_000).must();
        drop(w.finalize().must());
        let file = file_of(&twin);
        assert_eq!(file.len(), 44);
        assert_eq!(u32at(&file, 40), 0);
        assert_eq!(u32at(&file, 4), 36);
    }

    #[test]
    fn sizes_patch_to_the_truth_about_what_was_written() {
        let (sink, twin) = SharedCursor::sink();
        let mut w = WavPcmWriter::new(sink, PcmSample::S24Le, 2, 48_000).must();
        // Two writes: 3 frames then 2 frames (6 bytes/frame stereo).
        w.write_pcm(&[0xAB; 18]).must();
        w.write_pcm(&[0xCD; 12]).must();
        assert_eq!(w.data_bytes(), 30);
        drop(w.finalize().must());
        let file = file_of(&twin);
        assert_eq!(u32at(&file, 40), 30);
        assert_eq!(u32at(&file, 4), 66);
        assert_eq!(&file[44..62], &[0xAB; 18]);
        assert_eq!(&file[62..74], &[0xCD; 12]);
    }

    #[test]
    fn torn_tails_are_refused_before_any_byte_moves() {
        let (sink, twin) = SharedCursor::sink();
        let mut w = WavPcmWriter::new(sink, PcmSample::S16Le, 2, 44_100).must();
        let err = assert_err(w.write_pcm(&[0; 5]), "half a sample frame");
        assert_eq!(err.context.code, "transcode.wav.partial-frame");
        // Nothing was written: the header is still lazy, sizes zero.
        drop(w.finalize().must());
        let file = file_of(&twin);
        assert_eq!(u32at(&file, 40), 0);
        assert_eq!(&file[44..], &[]);
    }

    #[test]
    fn fact_validation_refuses_the_impossible_headers() {
        let zero_channels = assert_err(
            WavPcmWriter::new(SharedCursor::sink().0, PcmSample::S16Le, 0, 44_100),
            "no channels",
        );
        assert_eq!(
            zero_channels.context.code,
            "transcode.wav.channels-out-of-range"
        );
        let over_channels = assert_err(
            WavPcmWriter::new(
                SharedCursor::sink().0,
                PcmSample::S16Le,
                MAX_CHANNELS + 1,
                44_100,
            ),
            "past the channel ceiling",
        );
        assert_eq!(
            over_channels.context.code,
            "transcode.wav.channels-out-of-range"
        );
        let zero_rate = assert_err(
            WavPcmWriter::new(SharedCursor::sink().0, PcmSample::S16Le, 2, 0),
            "no rate",
        );
        assert_eq!(zero_rate.context.code, "transcode.wav.rate-zero");
        // 64 channels x 8 bytes x 9 MHz exceeds the u32 byte_rate
        // (512 x 9_000_000 = 4.61e9 > 4.29e9; the 192 kHz tone of the
        // fixtures cannot reach it — the ceiling needs an extreme rate).
        let byte_rate = assert_err(
            WavPcmWriter::new(
                SharedCursor::sink().0,
                PcmSample::F64Le,
                MAX_CHANNELS,
                9_000_000,
            ),
            "byte rate past u32",
        );
        assert_eq!(byte_rate.context.code, "transcode.wav.byte-rate-overflow");
    }

    #[test]
    fn riff_ceiling_is_typed_not_wrapped() {
        // The guard arithmetic, driven directly (the same call
        // `write_pcm` makes — no 4 GiB materialized). u8 mono: the
        // 1-byte frame makes every byte count a whole frame, so the
        // size ceiling is the only refusal in play.
        let (sink, _) = SharedCursor::sink();
        let mut w = WavPcmWriter::new(sink, PcmSample::U8, 1, 8_000).must();
        let err = assert_err(w.ensure_room(u32::MAX as u64), "past the RIFF ceiling");
        assert_eq!(err.context.code, "transcode.wav.size-overflow");
        // Exactly at the ceiling is legal (u32::MAX - 36 payload).
        w.ensure_room(u32::MAX as u64 - 36).must();
        // And end-to-end: a discard sink, 8 MiB chunks until the
        // ceiling crosses — the typed refusal, sizes never wrap.
        let mut big =
            WavPcmWriter::new(Box::new(DiscardSink { pos: 0 }), PcmSample::S16Le, 1, 8_000).must();
        let chunk = vec![0u8; 8 * 1024 * 1024];
        let mut written = 0u64;
        let refusal = loop {
            match big.write_pcm(chunk.as_slice()) {
                Ok(_) => written += chunk.len() as u64,
                Err(err) => break err,
            }
        };
        assert_eq!(refusal.context.code, "transcode.wav.size-overflow");
        assert!(
            written <= u32::MAX as u64 - 36,
            "{written} past the ceiling"
        );
        assert!(
            written >= u32::MAX as u64 - 36 - chunk.len() as u64,
            "refused within one chunk of the ceiling, wrote {written}"
        );
    }

    #[test]
    fn sink_failures_fold_typed_with_the_os_error_kept() {
        struct FailingSink;
        impl Write for FailingSink {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("no space"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl Seek for FailingSink {
            fn seek(&mut self, _to: SeekFrom) -> std::io::Result<u64> {
                Ok(0)
            }
        }
        let mut w = WavPcmWriter::new(Box::new(FailingSink), PcmSample::S16Le, 1, 8_000).must();
        let err = assert_err(w.write_pcm(&[0; 2]), "sink refuses");
        assert_eq!(err.context.code, "transcode.sink.write-io");
        assert!(std::error::Error::source(&err).is_some(), "OS error kept");
    }

    #[test]
    fn seek_and_write_paths_agree_on_position_semantics() {
        // finalize seeks to offset 4 and 40 and patches both u32s;
        // a sink whose seek is Start-only still lands both fields.
        let (sink, twin) = SharedCursor::sink();
        let mut w = WavPcmWriter::new(sink, PcmSample::U8, 1, 8_000).must();
        w.write_pcm(&[7, 8, 9]).must();
        drop(w.finalize().must());
        let guard = twin.lock().unwrap_or_else(|p| p.into_inner());
        assert_eq!(u32at(&guard, 40), 3);
        assert_eq!(u32at(&guard, 4), 39);
        assert_eq!(&guard[44..], &[7, 8, 9]);
    }
}
