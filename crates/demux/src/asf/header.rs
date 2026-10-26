//! The ASF header object walk (card 6.ASF-a): the GUID+size object
//! tree from the Header Object down — File Properties, Stream
//! Properties (the track table + WAVEFORMATEX/VIDEOINFO facts),
//! Codec List, Extended Content Description, a bounded Header
//! Extension child walk — plus the top-level walk that locates the
//! Data Object (packets are ASF-b's; this unit records the location
//! and refuses to parse payload).
//!
//! # Field-map provenance (honest, per the plan's risk register)
//!
//! The ASF Specification text was **not** acquirable this session
//! (download.microsoft.com's recalled URLs 404, the download-center
//! page is bot-blocked 403, archive.org unreachable — see the
//! worklog's [VERIFY-P0] verdict). Every layout below is therefore
//! **byte-verified against ffmpeg 7.1.5 output** (the §1.2 fixture
//! family, sha-pinned in the plan's §4.4 table) and closes each
//! object exactly; the labels mark what is arithmetic-verified vs
//! recalled-but-unexercised:
//!
//! - FPO payload (80 B, closes 24+80=104): FileID(16) · FileSize
//!   QWORD(8) · CreationDate QWORD(8) · **DataPacketsCount QWORD(8)**
//!   (observed `03 00 00 00 00 00 00 00` — the plan's DWORD/QWORD
//!   ambiguity resolves to 8 bytes: the following PlayDuration reads
//!   correctly only at +40) · PlayDuration(8) · SendDuration(8) ·
//!   Preroll(8) · Flags DWORD(4) · Min/MaxPacketSize(4+4) ·
//!   MaxBitrate(4).
//! - SP payload: StreamType GUID(16) · ErrorCorrectionType GUID(16)
//!   · TimeOffset QWORD(8) · TSD length DWORD(4) · ECD length
//!   DWORD(4) · StreamNumber WORD(2) · LanguageIndex WORD(2) ·
//!   **2 bytes at +52 observed zero, semantics unpinned** (exposed
//!   raw as `flags_raw`) · TSD · ECD.
//! - Audio TSD = raw WAVEFORMATEX (tag/ch/rate/avg/align/bits/
//!   cbSize/extra — verified: PCM 18 B, ADPCM-MS 50 B = 18+32,
//!   WMAV2 28 B = 18+10).
//! - Video TSD = a **short form** (the full VIDEOINFOHEADER does not
//!   match the bytes; plan §2.3): width DWORD(4) · height DWORD(4) ·
//!   one byte observed constantly `0x02` (semantics unverified) ·
//!   WORD data size (= the BITMAPINFOHEADER's `biSize` on every
//!   fixture: 40 without extradata, 44 with wmv2's 4-byte tail) ·
//!   BITMAPINFOHEADER · codec extradata. Closes exactly:
//!   11 + data_size = TSD length on all six video fixtures.
//! - Codec List: reserved GUID(16) · count DWORD(4) · entries
//!   { type WORD · name WORD-length (UTF-16 units) · UTF-16LE ·
//!   desc WORD-length · UTF-16LE · info WORD-length (bytes) · bytes }
//!   — closes the 98/100/72/74-byte ffmpeg payloads exactly.
//! - Extended Content Description: count WORD(4) · entries
//!   { name WORD-length (**bytes**) · UTF-16LE · data type WORD ·
//!   value WORD-length (bytes) · value } — closes ffmpeg's 74/144
//!   byte payloads exactly (the plan's §2.3 sketch had the type
//!   before the name; the byte walk wins and is recorded here).
//! - Header Extension: reserved GUID(16) · reserved WORD (observed
//!   6) · data size DWORD · children, walked by size and
//!   **capture-and-ignored** (the plan's recorded v0 posture for the
//!   Extended Stream Properties grammar).
//! - Simple Index (top level, after Data): located and sized only —
//!   entry parsing is ASF-c's.

use media_types::ids::{CodecDescriptor, MediaKind};
use media_types::packet::SharedBytes;

use crate::avi::header::DamageLog;
use crate::avi::riff::SourceReader;

use super::error;
use super::guid::{
    self, CODEC_LIST, DATA, EXTENDED_CONTENT_DESCRIPTION, EXTENDED_STREAM_PROPERTIES,
    FILE_PROPERTIES, Guid, HEADER, HEADER_EXTENSION, STREAM_PROPERTIES, STREAM_TYPE_AUDIO,
    STREAM_TYPE_COMMAND, STREAM_TYPE_VIDEO,
};

/// Maximum Header Object size trusted into memory `[PROPOSAL: 4 MiB —
/// ffmpeg headers are < 1 KiB; real-world metadata headers are KBs;
/// anything larger is corruption or hostile]` (MP §12).
pub const MAX_HEADER_BYTES: u64 = 4 << 20;

/// Maximum header-child objects walked `[PROPOSAL: 4096 — a real
/// header carries < 20; the bound exists so a corrupt size table
/// cannot spin the walk]`.
pub const MAX_HEADER_CHILDREN: usize = 4096;

/// Maximum top-level objects mapped `[PROPOSAL: 4096 — same bound,
/// the Data-Object-trailing index family]`.
pub const MAX_TOP_OBJECTS: usize = 4096;

/// Maximum Header-Extension children counted `[PROPOSAL: 1024]`.
pub const MAX_EXTENSION_CHILDREN: usize = 1024;

/// Maximum Codec List entries `[PROPOSAL: 256 — one per stream plus
/// slack; more is corruption]`.
pub const MAX_CODEC_LIST_ENTRIES: usize = 256;

/// Maximum bytes of one Codec List name/description/info field
/// `[PROPOSAL: 4 KiB strings / 1 KiB info blobs]`.
pub const MAX_CODEC_FIELD_BYTES: usize = 4096;

/// Maximum bytes of one metadata name or value `[PROPOSAL: 4 KiB —
/// titles and credits, not payloads]`.
pub const MAX_METADATA_FIELD_BYTES: usize = 4096;

/// Maximum ECD entries `[PROPOSAL: 1024]`.
pub const MAX_METADATA_ENTRIES: usize = 1024;

/// Maximum type-specific data trusted per stream `[PROPOSAL: 64 KiB —
/// the WAVEFORMATEX cbSize tail is a WORD, so 18+65535 is the
/// format's own ceiling]` (MP §12).
pub const MAX_TSD_BYTES: u64 = 64 * 1024;

/// The File Properties Object facts (all fields raw; derived flags
/// below). Durations are advisory — the plan's flagged decision:
/// **track spans come from ASF-b's packet stamps; ffmpeg's asf muxer
/// pads Play Duration** (measured: the av fixture declares 4.164 s
/// for a 1.128 s file, ffprobe's own duration), so no timing truth
/// is derived from them here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileProperties {
    /// The file's declared identity GUID (ffmpeg writes all zeros —
    /// never an identity oracle, plan §2.2).
    pub file_id: Guid,
    /// Declared total file size in bytes (0 in broadcast files).
    pub file_size: u64,
    /// Creation date, raw (ffmpeg writes the constant
    /// 116_444_736_000_000_000; units unpinned without the spec).
    pub creation_date: u64,
    /// Declared Data Object packet count (QWORD, width pinned this
    /// session; 0 in broadcast files).
    pub data_packets_count: u64,
    /// Play Duration in 100-ns units (advisory; see type docs).
    pub play_duration_100ns: u64,
    /// Send Duration in 100-ns units (advisory; closer to real than
    /// play duration on ffmpeg output — measured 1.064 s vs 1.128 s
    /// real — but still not track timing).
    pub send_duration_100ns: u64,
    /// Preroll in milliseconds (verified behaviorally: payload
    /// timestamps are stored preroll-offset and ffprobe's zero-based
    /// first pts implies subtraction, plan §2.2/§2.4).
    pub preroll_ms: u64,
    /// Raw flags DWORD (bit 0 = broadcast, bit 1 = seekable — both
    /// cross-checked against the §2.5 broadcast fixture: flags 3 vs
    /// the normal files' 2).
    pub flags: u32,
    /// Minimum data packet size in bytes.
    pub min_packet_size: u32,
    /// Maximum data packet size in bytes.
    pub max_packet_size: u32,
    /// Maximum instantaneous bitrate in bits/s.
    pub max_bitrate: u32,
}

impl FileProperties {
    /// Flags bit 0: broadcast (live) file — unknown sizes by design.
    #[must_use]
    pub fn is_broadcast(&self) -> bool {
        self.flags & 0x1 != 0
    }

    /// Flags bit 1: the header declares the file seekable (advisory —
    /// real seekability is the index's, ASF-c's domain).
    #[must_use]
    pub fn declares_seekable(&self) -> bool {
        self.flags & 0x2 != 0
    }
}

/// The WAVEFORMATEX facts harvested from an audio stream's
/// type-specific data (the PCM/ADPCM decoder-config seam — the same
/// fields the WAV `fmt ` parse produces).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaveFormatEx {
    /// `wFormatTag` (0x0001 PCM, 0x0002 MS ADPCM, 0x0161 WMAv2, …).
    pub tag: u16,
    /// `nChannels`.
    pub channels: u16,
    /// `nSamplesPerSec`.
    pub samples_per_sec: u32,
    /// `nAvgBytesPerSec` (advisory CBR arithmetic).
    pub avg_bytes_per_sec: u32,
    /// `nBlockAlign` (the packet framing truth for ADPCM; = payload
    /// size of audio payloads in the Data Object).
    pub block_align: u16,
    /// `wBitsPerSample`.
    pub bits_per_sample: u16,
    /// `cbSize` — extra bytes following the 18-byte base.
    pub cb_size: u16,
}

/// The video facts harvested from a video stream's type-specific
/// data's BITMAPINFOHEADER (the geometry + fourcc truth).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoInfo {
    /// `biWidth` (pixels).
    pub width: u32,
    /// `biHeight` (pixels, container-positive as written).
    pub height: u32,
    /// `biBitCount` (24 on all ffmpeg fixtures).
    pub bit_count: u16,
    /// `biCompression` — the fourcc bytes verbatim (`WMV2`, `MJPG`, …).
    pub compression: [u8; 4],
    /// `biSizeImage` (advisory).
    pub size_image: u32,
    /// The codec extradata length that followed the header
    /// (wmv2 carries 4 bytes; wmv1/msmpeg4v3/mjpeg carry 0 —
    /// measured).
    pub extradata_len: usize,
}

impl VideoInfo {
    /// The fourcc as ASCII (lossy for non-printables — observability
    /// only; the raw bytes are the truth above).
    #[must_use]
    pub fn fourcc(&self) -> String {
        self.compression
            .iter()
            .map(|b| {
                if b.is_ascii_graphic() || *b == b' ' {
                    *b as char
                } else {
                    '.'
                }
            })
            .collect()
    }
}

/// One Stream Properties record — the track-table source. Command/
/// script streams and unknown stream types are skipped during the
/// walk (bounded, damage-noted), so every record here is audio or
/// video.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamRecord {
    /// The SP stream number field (1-based in ffmpeg output; the
    /// packet payload stream ids key off it — ASF-b's join).
    pub stream_number: u16,
    /// The stream language index (references the Language List
    /// object, not parsed in v1 — carried raw, never guessed).
    pub language_index: u16,
    /// The 2 bytes at SP+52 verbatim (observed zero everywhere;
    /// semantics unpinned without the spec — the encryption-gate
    /// input, see `error::encrypted_stream`'s docs).
    pub flags_raw: u16,
    /// Time Offset in 100-ns units (0 on all fixtures).
    pub time_offset_100ns: u64,
    /// The error-correction type GUID (audio-spread observed on
    /// audio, the zero GUID on video).
    pub error_correction_type: Guid,
    /// Audio or video.
    pub kind: MediaKind,
    /// The registry codec descriptor (the §4.5 map: known signals →
    /// vocabulary; unknown → raw passthrough for the typed
    /// codec-unavailable refusal downstream).
    pub codec: CodecDescriptor,
    /// The WAVEFORMATEX facts (audio streams).
    pub wave_format: Option<WaveFormatEx>,
    /// The video facts (video streams).
    pub video_info: Option<VideoInfo>,
    /// The **verbatim type-specific data** — the decoder-config
    /// `extra_data` seam (the WAV `fmt_chunk_bytes` precedent: for
    /// ADPCM streams this whole blob is what
    /// `AdpcmDecoder::configure_wav` parses; for PCM the facts
    /// suffice; for WMA it rides along for the future codec).
    pub tsd_bytes: SharedBytes,
}

/// One Codec List entry (advisory — the SP type-specific data is the
/// decoder-config truth; a disagreement is damage-noted, SP wins).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecEntry {
    /// 1 = video, 2 = audio (observed; 3 = script per the plan's
    /// table, unexercised).
    pub codec_type: u16,
    /// The display name (e.g. `Windows Media Audio V8`).
    pub name: String,
    /// The description (empty on ffmpeg output).
    pub description: String,
    /// The info blob (video: the fourcc ASCII; audio: the 2-byte
    /// format tag — both observed).
    pub info: Vec<u8>,
}

/// One Extended Content Description string pair (data type 0; other
/// types are counted, not guessed into text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataPair {
    /// The name (e.g. `WM/EncodingSettings`, `title`).
    pub name: String,
    /// The UTF-16-decoded value (lossy — advisory display text).
    pub value: String,
}

/// A located top-level object (GUID + offset + declared size).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopObject {
    /// The object GUID.
    pub guid: Guid,
    /// Absolute byte offset.
    pub offset: u64,
    /// Declared size including the 24-byte object header.
    pub size: u64,
}

/// The located Data Object (packets are ASF-b's; this is the
/// location + header facts only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataObjectLocation {
    /// Absolute offset of the Data Object.
    pub offset: u64,
    /// Declared size (a 50-byte placeholder in broadcast files —
    /// they are refused earlier, at the FPO gate).
    pub size: u64,
    /// The declared total data packet count from the Data Object
    /// header (matches the FPO's count on all fixtures — measured 3
    /// on the av fixture).
    pub packet_count: u64,
    /// Whether the reserved bytes matched the observed `01 01`
    /// constant (damage-noted when not; never fatal — the constant's
    /// spec meaning is unverified).
    pub reserved_matches: bool,
}

/// The full header walk's product: everything `start()` harvested.
#[derive(Debug, Clone, PartialEq)]
pub struct AsfHeader {
    /// Where the Header Object started (0 normally; 12 when the
    /// §2.5 broadcast `$H`-family prefix preceded it — those files
    /// are refused at the FPO gate, but the prefix shape is how they
    /// were recognized).
    pub header_offset: u64,
    /// The Header Object's declared size.
    pub header_size: u64,
    /// The declared child count (advisory; the size walk is
    /// authority — a mismatch is damage-noted).
    pub declared_children: u32,
    /// How many children the size walk actually found.
    pub walked_children: usize,
    /// The File Properties facts.
    pub properties: FileProperties,
    /// The audio/video streams, in header order.
    pub streams: Vec<StreamRecord>,
    /// The Codec List entries.
    pub codec_list: Vec<CodecEntry>,
    /// The Extended Content Description string pairs.
    pub metadata: Vec<MetadataPair>,
    /// Header Extension children counted (contents
    /// capture-and-ignored, the recorded v0 posture).
    pub extension_children: usize,
    /// Non-audio/video or unrecognized header objects skipped by
    /// size: (GUID, size).
    pub skipped_objects: Vec<(Guid, u64)>,
    /// The located Data Object.
    pub data: DataObjectLocation,
    /// The top-level object map walked after the header (Data + any
    /// trailing objects, e.g. the Simple Index on video files).
    pub top_objects: Vec<TopObject>,
    /// The header walk's damage notes (advisory disagreements and
    /// skips; observable via the demuxer).
    pub damage: DamageLog,
}

/// Decode a UTF-16LE buffer to a lossy `String`, trailing NULs
/// stripped (ASF length prefixes include the terminator — measured:
/// `"wmv2\0"` at 5 units).
fn utf16_lossy(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    let mut text = String::from_utf16_lossy(&units);
    while text.ends_with('\0') {
        text.pop();
    }
    text
}

/// The §4.5 stream-type-GUID → registry-descriptor map. Known
/// signals map to the vocabulary the host provider speaks
/// (`pipeline_host.rs`'s match arms); unknown signals pass through
/// raw (lowercased fourcc / tag-hex) for the typed
/// codec-unavailable refusal downstream — never a silent guess (the
/// mkv unknown-codec precedent).
fn codec_descriptor(
    stream_type: Guid,
    wfx: Option<&WaveFormatEx>,
    video: Option<&VideoInfo>,
) -> String {
    match stream_type {
        STREAM_TYPE_AUDIO => {
            let Some(wfx) = wfx else {
                return "asf-audio-unknown".to_string();
            };
            match wfx.tag {
                0x0001 => match wfx.bits_per_sample {
                    8 => "pcm_u8".to_string(),
                    16 => "pcm_s16le".to_string(),
                    24 => "pcm_s24le".to_string(),
                    32 => "pcm_s32le".to_string(),
                    // An integer-PCM width outside the registry's
                    // set: raw passthrough, the codec refuses typed.
                    bits => format!("pcm_s{bits}le"),
                },
                0x0003 => match wfx.bits_per_sample {
                    32 => "pcm_f32le".to_string(),
                    64 => "pcm_f64le".to_string(),
                    bits => format!("pcm_f{bits}le"),
                },
                0x0002 => "adpcm_ms".to_string(),
                // `[FORMAT KNOWLEDGE]` — the IMA tag rides the same
                // WAVEFORMATEX family the WAV lane verified.
                0x0011 => "adpcm_ima_wav".to_string(),
                0x0160 => "wmav1".to_string(),
                0x0161 => "wmav2".to_string(),
                0x0162 => "wmapro".to_string(),
                0x0163 => "wmalossless".to_string(),
                tag => format!("asf-audio-tag-{tag:04x}"),
            }
        }
        STREAM_TYPE_VIDEO => {
            let Some(video) = video else {
                return "asf-video-unknown".to_string();
            };
            match &video.compression {
                b"MJPG" => "mjpeg".to_string(),
                b"WMV1" => "wmv1".to_string(),
                b"WMV2" => "wmv2".to_string(),
                b"WMV3" => "wmv3".to_string(),
                b"MP43" => "msmpeg4v3".to_string(),
                b"MP42" => "msmpeg4v2".to_string(),
                // `[FORMAT KNOWLEDGE]` rows of the plan's §4.5 table,
                // unexercised by the ffmpeg fixtures.
                b"WMVA" | b"WMVP" | b"WVP2" | b"VC1\0" | b"vc1 " => "vc1".to_string(),
                fourcc => {
                    let name: String = fourcc
                        .iter()
                        .map(|b| {
                            if b.is_ascii_alphanumeric() {
                                *b as char
                            } else {
                                '.'
                            }
                        })
                        .flat_map(|c| c.to_lowercase())
                        .collect();
                    format!("asf-fourcc-{name}")
                }
            }
        }
        // Command/script and unknown stream types never reach a
        // record (the walk skips them first); the passthrough keeps
        // the map total for future lanes.
        _ => "asf-stream-unknown".to_string(),
    }
}

impl AsfHeader {
    /// Run the full header walk over `reader` (the body of
    /// `AsfDemuxer::start`).
    ///
    /// Reads the Header Object into bounded memory
    /// ([`MAX_HEADER_BYTES`]), walks its children by declared sizes,
    /// then maps the top level to locate the Data Object. Broadcast
    /// files (FPO flags bit 0) are the typed refusal; a missing Data
    /// Object is damage-noted (a header-only file still `start`s with
    /// its track table — the Data Object's absence is ASF-b's packet
    /// concern, recorded honestly).
    ///
    /// # Errors
    /// Typed `demux.asf.*` (see `error.rs`): `not-asf`,
    /// `truncated`, `bad-object-size`, `header-too-large`,
    /// `broadcast-unsupported`, `bad-waveformatex`, `bad-video-info`,
    /// `no-streams`.
    pub fn parse(reader: &mut SourceReader) -> Result<AsfHeader, media_types::EngineError> {
        let mut damage = DamageLog::default();
        let mut skipped: Vec<(Guid, u64)> = Vec::new();

        // --- Locate the Header Object (offset 0, or the §2.5
        // broadcast prefix shape at +12).
        let head = reader.read_prefix_at(16, 0)?;
        if Guid::read(&head, 0) != HEADER {
            let prefixed = reader.read_prefix_at(16, 12)?;
            if Guid::read(&prefixed, 0) == HEADER {
                // The broadcast `$H`-family prefix (bytes
                // `24 48 2f 03 …` measured, plan §2.5) — walk the
                // header so the FPO flags gate can refuse it typed,
                // instead of a bare not-asf that hides the reason.
                Self::parse_at(reader, 12, &mut damage, &mut skipped)
            } else {
                Err(error::not_asf(&head))
            }
        } else {
            Self::parse_at(reader, 0, &mut damage, &mut skipped)
        }
    }

    /// The walk proper, anchored at `header_offset`.
    fn parse_at(
        reader: &mut SourceReader,
        header_offset: u64,
        damage: &mut DamageLog,
        skipped: &mut Vec<(Guid, u64)>,
    ) -> Result<AsfHeader, media_types::EngineError> {
        let stream_len = reader.length();
        // Header Object head: GUID(16) + size QWORD(8) + child count
        // DWORD(4) + 2 reserved bytes (observed `01 02`, semantics
        // unverified) — 30 bytes before the first child.
        let head = reader.read_prefix_at(30, header_offset)?;
        if head.len() < 30 {
            return Err(error::truncated(
                header_offset + 30,
                header_offset + head.len() as u64,
            ));
        }
        let header_size = guid::u64le(&head, 16);
        if header_size > MAX_HEADER_BYTES {
            return Err(error::header_too_large(MAX_HEADER_BYTES, header_size));
        }
        // The header object must at least carry its own 24-byte
        // object header + the child count + the 2 reserved bytes.
        if header_size < 30 {
            return Err(error::bad_object_size(header_offset));
        }
        let declared_children = guid::u32le(&head, 24);

        // The whole header, bounded, into memory (children reference
        // each other positionally; a 4 MiB ceiling is the MP §12
        // bound and the read is prefix-capped).
        let header_end = header_offset + header_size;
        if header_end > stream_len {
            return Err(error::truncated(header_end, stream_len));
        }
        let want = header_size as usize;
        let bytes = reader.read_prefix_at(want, header_offset)?;
        if (bytes.len() as u64) < header_size {
            return Err(error::truncated(
                header_end,
                header_offset + bytes.len() as u64,
            ));
        }

        let mut properties: Option<FileProperties> = None;
        let mut streams: Vec<StreamRecord> = Vec::new();
        let mut codec_list: Vec<CodecEntry> = Vec::new();
        let mut metadata: Vec<MetadataPair> = Vec::new();
        let mut extension_children = 0usize;
        let mut walked = 0usize;

        // Children walk: +30 to header_size, by declared sizes.
        let mut off = 30usize;
        while off + 24 <= header_size as usize {
            if walked >= MAX_HEADER_CHILDREN {
                damage.note(0, "header-children-budget");
                break;
            }
            let child = Guid::read(&bytes, off);
            let size = guid::u64le(&bytes, off + 16);
            if size < 24 {
                return Err(error::bad_object_size(header_offset + off as u64));
            }
            let end = off as u64 + size;
            if end > header_size {
                return Err(error::bad_object_size(header_offset + off as u64));
            }
            let payload = &bytes[off + 24..end as usize];
            walked += 1;
            match child {
                FILE_PROPERTIES => {
                    if properties.is_some() {
                        damage.note(size, "duplicate-file-properties");
                    } else if payload.len() < 80 {
                        return Err(error::truncated(
                            header_offset + off as u64 + 24 + 80,
                            header_offset + off as u64 + 24 + payload.len() as u64,
                        ));
                    } else {
                        properties = Some(Self::parse_file_properties(payload));
                    }
                }
                STREAM_PROPERTIES => {
                    if payload.len() < 54 {
                        return Err(error::truncated(
                            header_offset + off as u64 + 24 + 54,
                            header_offset + off as u64 + 24 + payload.len() as u64,
                        ));
                    }
                    let stream_type = Guid::read(payload, 0);
                    let stream_number = guid::u16le(payload, 48);
                    if stream_type != STREAM_TYPE_AUDIO && stream_type != STREAM_TYPE_VIDEO {
                        // Command/script streams and unknown stream
                        // families: skipped by size, recorded (the
                        // §4.5 command row — the track is filtered
                        // here, never a guessed MediaKind).
                        let reason = if stream_type == STREAM_TYPE_COMMAND {
                            "skipped-command-stream"
                        } else {
                            "skipped-unknown-stream-type"
                        };
                        damage.note(size, reason);
                        skipped.push((stream_type, size));
                    } else if streams.iter().any(|s| s.stream_number == stream_number) {
                        // Duplicates: damage-noted, first wins (the
                        // wav duplicate-fmt rule).
                        damage.note(size, "duplicate-stream-properties");
                    } else {
                        streams.push(Self::parse_stream_properties(
                            payload,
                            header_offset + off as u64 + 24,
                        )?);
                    }
                }
                CODEC_LIST => {
                    let (entries, noted) = Self::parse_codec_list(payload)?;
                    damage.total = damage.total.saturating_add(noted);
                    codec_list.extend(entries);
                }
                EXTENDED_CONTENT_DESCRIPTION => {
                    let (pairs, noted) = Self::parse_extended_content_description(payload)?;
                    damage.total = damage.total.saturating_add(noted);
                    metadata.extend(pairs);
                }
                HEADER_EXTENSION => {
                    let (count, noted) = Self::walk_header_extension(payload)?;
                    extension_children += count;
                    damage.total = damage.total.saturating_add(noted);
                }
                other => {
                    // Known-but-unparsed families (Content
                    // Description's grammar is unverified without the
                    // spec; ffmpeg output carries none) and unknown
                    // GUIDs alike: skip by size, recorded.
                    let reason = if other.known_name().is_some() {
                        "skipped-known-header-object"
                    } else {
                        "skipped-unknown-header-object"
                    };
                    damage.note(size, reason);
                    skipped.push((other, size));
                }
            }
            off = end as usize;
        }
        if off != header_size as usize {
            // A trailing fragment below the 24-byte object floor:
            // damage-noted, the walk's authority was the sizes.
            damage.note(header_size - off as u64, "header-trailing-fragment");
        }
        if declared_children != walked as u32 && declared_children != 0 {
            damage.note(0, "declared-child-count-mismatch");
        }

        let Some(properties) = properties else {
            // An ASF without File Properties cannot even be refused
            // honestly as broadcast — structurally incomplete.
            return Err(error::bad_object_size(header_offset));
        };
        if properties.is_broadcast() {
            return Err(error::broadcast_unsupported());
        }
        if streams.is_empty() {
            return Err(error::no_streams());
        }
        // Codec-list advisory cross-check: the list's info blobs must
        // agree with the SP type-specific data where both name a
        // stream (fourcc for video, tag for audio — measured). A
        // disagreement is damage, the SP stays truth.
        for entry in &codec_list {
            let declared_fourcc = &entry.info[..entry.info.len().min(4)];
            let declared_tag = guid::u16le(&entry.info, 0);
            let agrees = match entry.codec_type {
                1 => streams.iter().any(|s| {
                    s.video_info
                        .is_some_and(|v| &v.compression[..] == declared_fourcc)
                }),
                2 => streams
                    .iter()
                    .any(|s| s.wave_format.is_some_and(|w| w.tag == declared_tag)),
                _ => true,
            };
            if !agrees {
                damage.note(0, "codec-list-disagrees");
            }
        }

        // --- Top-level walk: locate the Data Object (and map any
        // trailing objects, sizes only).
        let (data, top_objects) = Self::walk_top_level(reader, header_end, stream_len, damage)?;

        Ok(AsfHeader {
            header_offset,
            header_size,
            declared_children,
            walked_children: walked,
            properties,
            streams,
            codec_list,
            metadata,
            extension_children,
            skipped_objects: std::mem::take(skipped),
            data,
            top_objects,
            damage: std::mem::take(damage),
        })
    }

    /// The 80-byte FPO payload → facts (layout pinned this session;
    /// see the module docs).
    fn parse_file_properties(payload: &[u8]) -> FileProperties {
        FileProperties {
            file_id: Guid::read(payload, 0),
            file_size: guid::u64le(payload, 16),
            creation_date: guid::u64le(payload, 24),
            data_packets_count: guid::u64le(payload, 32),
            play_duration_100ns: guid::u64le(payload, 40),
            send_duration_100ns: guid::u64le(payload, 48),
            preroll_ms: guid::u64le(payload, 56),
            flags: guid::u32le(payload, 64),
            min_packet_size: guid::u32le(payload, 68),
            max_packet_size: guid::u32le(payload, 72),
            max_bitrate: guid::u32le(payload, 76),
        }
    }

    /// One Stream Properties payload → a `StreamRecord` (audio/video)
    /// — or `Err` for structurally broken TSDs. Non-audio/video
    /// stream types are **not** an error here; the caller skips them
    /// before this runs (see `parse_at`'s call site handling below).
    fn parse_stream_properties(
        payload: &[u8],
        payload_offset: u64,
    ) -> Result<StreamRecord, media_types::EngineError> {
        let stream_type = Guid::read(payload, 0);
        let error_correction_type = Guid::read(payload, 16);
        let time_offset_100ns = guid::u64le(payload, 32);
        let tsd_len = guid::u32le(payload, 40) as u64;
        let ecd_len = guid::u32le(payload, 44) as u64;
        let stream_number = guid::u16le(payload, 48);
        let language_index = guid::u16le(payload, 50);
        let flags_raw = guid::u16le(payload, 52);
        if tsd_len > MAX_TSD_BYTES {
            return Err(error::header_too_large(MAX_TSD_BYTES, tsd_len));
        }
        let tsd = &payload[54.min(payload.len())..];
        let tsd = &tsd[..tsd.len().min(tsd_len as usize)];
        if tsd.len() < tsd_len as usize {
            return Err(error::truncated(
                payload_offset + 54 + tsd_len,
                payload_offset + 54 + tsd.len() as u64,
            ));
        }
        let _ = ecd_len; // Error-correction payloads are ASF-b's parse.

        let (kind, wave_format, video_info) = if stream_type == STREAM_TYPE_AUDIO {
            let wfx = Self::parse_waveformatex(tsd, payload_offset)?;
            (MediaKind::Audio, Some(wfx), None)
        } else {
            let video = Self::parse_video_info(tsd, payload_offset)?;
            (MediaKind::Video, None, Some(video))
        };
        let codec = CodecDescriptor::new(&codec_descriptor(
            stream_type,
            wave_format.as_ref(),
            video_info.as_ref(),
        ))
        .map_err(|_| error::bad_object_size(payload_offset))?;
        Ok(StreamRecord {
            stream_number,
            language_index,
            flags_raw,
            time_offset_100ns,
            error_correction_type,
            kind,
            codec,
            wave_format,
            video_info,
            tsd_bytes: SharedBytes::from_vec(tsd.to_vec()),
        })
    }

    /// The 18-byte WAVEFORMATEX base + cbSize tail (verified on PCM /
    /// ADPCM-MS / WMAv2 / WMAv1 fixtures).
    fn parse_waveformatex(
        tsd: &[u8],
        payload_offset: u64,
    ) -> Result<WaveFormatEx, media_types::EngineError> {
        if tsd.len() < 18 {
            return Err(error::truncated(
                payload_offset + 18,
                payload_offset + tsd.len() as u64,
            ));
        }
        let wfx = WaveFormatEx {
            tag: guid::u16le(tsd, 0),
            channels: guid::u16le(tsd, 2),
            samples_per_sec: guid::u32le(tsd, 4),
            avg_bytes_per_sec: guid::u32le(tsd, 8),
            block_align: guid::u16le(tsd, 12),
            bits_per_sample: guid::u16le(tsd, 14),
            cb_size: guid::u16le(tsd, 16),
        };
        if wfx.channels == 0 || wfx.samples_per_sec == 0 {
            return Err(error::bad_waveformatex(payload_offset));
        }
        let tail = wfx.cb_size as usize;
        if 18 + tail > tsd.len() {
            return Err(error::bad_waveformatex(payload_offset));
        }
        Ok(wfx)
    }

    /// The short VIDEOINFOHEADER-family form (module docs): width(4)
    /// · height(4) · observed-0x02 byte · WORD data size ·
    /// BITMAPINFOHEADER · extradata. Measured `data_size`/`biSize`
    /// pairs: 40/40 (wmv1, msmpeg4v3, mjpeg — no extradata) and 44/44
    /// (wmv2 — 4 bytes of codec extradata); the two fields agree on
    /// every fixture, so `biSize` outside `[40, data_size]` is a
    /// shape this unit never verified and refuses rather than
    /// guesses.
    fn parse_video_info(
        tsd: &[u8],
        payload_offset: u64,
    ) -> Result<VideoInfo, media_types::EngineError> {
        if tsd.len() < 11 {
            return Err(error::bad_video_info(payload_offset));
        }
        let data_size = guid::u16le(tsd, 9) as usize;
        if data_size < 40 {
            return Err(error::bad_video_info(payload_offset));
        }
        let total = 11 + data_size;
        if total > tsd.len() {
            return Err(error::bad_video_info(payload_offset));
        }
        let bmih = &tsd[11..total];
        let bi_size = guid::u32le(bmih, 0);
        if !(40..=data_size as u32).contains(&bi_size) {
            // Only the 40-byte BITMAPINFOHEADER (and its measured
            // 44-byte extradata-carrying extension) is verified;
            // anything else is an unverified shape.
            return Err(error::bad_video_info(payload_offset));
        }
        let mut compression = [0u8; 4];
        compression.copy_from_slice(&bmih[16..20]);
        Ok(VideoInfo {
            width: guid::u32le(bmih, 4),
            height: guid::u32le(bmih, 8),
            bit_count: guid::u16le(bmih, 14),
            compression,
            size_image: guid::u32le(bmih, 20),
            extradata_len: data_size - 40,
        })
    }

    /// The Codec List (grammar closes the ffmpeg payloads exactly;
    /// module docs). Returns the entries and a damage total for the
    /// bounded skips.
    fn parse_codec_list(
        payload: &[u8],
    ) -> Result<(Vec<CodecEntry>, u64), media_types::EngineError> {
        let mut entries = Vec::new();
        if payload.len() < 20 {
            return Ok((entries, 0));
        }
        let count = guid::u32le(payload, 16) as usize;
        let mut off = 20usize;
        let mut noted = 0u64;
        for _ in 0..count.min(MAX_CODEC_LIST_ENTRIES) {
            // type(2) name-len(2) name desc-len(2) desc info-len(2) info
            if off + 2 > payload.len() {
                noted += payload.len() as u64;
                break;
            }
            let codec_type = guid::u16le(payload, off);
            off += 2;
            let mut strings = [String::new(), String::new()];
            let mut overflowed = false;
            for slot in &mut strings {
                if off + 2 > payload.len() {
                    overflowed = true;
                    break;
                }
                let units = guid::u16le(payload, off) as usize;
                off += 2;
                // Display strings: clamped to the field cap and to
                // the object's bytes (advisory text, never a parse
                // failure — the declared length that outruns the
                // object is damage, noted below).
                let byte_len = units
                    .saturating_mul(2)
                    .min(MAX_CODEC_FIELD_BYTES)
                    .min(payload.len() - off);
                *slot = utf16_lossy(&payload[off..off + byte_len]);
                off += byte_len;
                if byte_len < units.saturating_mul(2) {
                    overflowed = true;
                }
            }
            if overflowed {
                noted += payload.len() as u64;
                break;
            }
            if off + 2 > payload.len() {
                return Err(error::bad_object_size(0));
            }
            let info_len = guid::u16le(payload, off) as usize;
            off += 2;
            let info_len = info_len.min(MAX_CODEC_FIELD_BYTES).min(payload.len() - off);
            let info = payload[off..off + info_len].to_vec();
            off += info_len;
            entries.push(CodecEntry {
                codec_type,
                name: std::mem::take(&mut strings[0]),
                description: std::mem::take(&mut strings[1]),
                info,
            });
            if off >= payload.len() {
                break;
            }
        }
        if count > MAX_CODEC_LIST_ENTRIES {
            noted += (count - MAX_CODEC_LIST_ENTRIES) as u64;
        }
        Ok((entries, noted))
    }

    /// The Extended Content Description (module docs for the pinned
    /// field order). String-typed (type 0) pairs are captured; other
    /// types are counted into the damage total — never guessed into
    /// text.
    fn parse_extended_content_description(
        payload: &[u8],
    ) -> Result<(Vec<MetadataPair>, u64), media_types::EngineError> {
        let mut pairs = Vec::new();
        if payload.len() < 2 {
            return Ok((pairs, 0));
        }
        let count = guid::u16le(payload, 0) as usize;
        let mut off = 2usize;
        let mut noted = 0u64;
        for _ in 0..count.min(MAX_METADATA_ENTRIES) {
            // name-len(2) name type(2) value-len(2) value
            if off + 2 > payload.len() {
                return Err(error::bad_object_size(0));
            }
            let name_len = guid::u16le(payload, off) as usize;
            off += 2;
            let name_len = name_len
                .min(MAX_METADATA_FIELD_BYTES)
                .min(payload.len().saturating_sub(off));
            let name = utf16_lossy(&payload[off..off + name_len]);
            off += name_len;
            if off + 4 > payload.len() {
                return Err(error::bad_object_size(0));
            }
            let data_type = guid::u16le(payload, off);
            off += 2;
            let value_len = guid::u16le(payload, off) as usize;
            off += 2;
            let value_len = value_len
                .min(MAX_METADATA_FIELD_BYTES)
                .min(payload.len().saturating_sub(off));
            let value_bytes = &payload[off..off + value_len];
            off += value_len;
            if data_type == 0 {
                pairs.push(MetadataPair {
                    name,
                    value: utf16_lossy(value_bytes),
                });
            } else {
                noted += 1;
            }
            if off >= payload.len() {
                break;
            }
        }
        Ok((pairs, noted))
    }

    /// The Header Extension's bounded child walk: reserved GUID(16) ·
    /// reserved WORD (observed 6) · data size DWORD(4) · children.
    /// Children (Extended Stream Properties in ffmpeg output) are
    /// counted and skipped — capture-and-ignore, the plan's recorded
    /// v0 posture for the ESP grammar (language/name/payload-
    /// extension data: ASF-b+/C-V6's to pin).
    fn walk_header_extension(payload: &[u8]) -> Result<(usize, u64), media_types::EngineError> {
        if payload.len() < 22 {
            return Ok((0, 0));
        }
        let data_size = guid::u32le(payload, 18) as usize;
        let end = 22 + data_size;
        if end > payload.len() {
            return Err(error::bad_object_size(0));
        }
        let mut off = 22usize;
        let mut count = 0usize;
        let mut noted = 0u64;
        while off + 24 <= end && count < MAX_EXTENSION_CHILDREN {
            let size = guid::u64le(payload, off + 16);
            if size < 24 || off as u64 + size > end as u64 {
                noted += (end - off) as u64;
                break;
            }
            if Guid::read(payload, off) != EXTENDED_STREAM_PROPERTIES {
                // Non-ESP children (language lists, payload
                // extension systems the plan flagged): counted as
                // ignored structure, damage-noted.
                noted += 1;
            }
            count += 1;
            off += size as usize;
        }
        Ok((count, noted))
    }

    /// The top-level walk after the Header: locate the Data Object,
    /// then map trailing objects (Simple Index et al.) by size only.
    fn walk_top_level(
        reader: &mut SourceReader,
        header_end: u64,
        stream_len: u64,
        damage: &mut DamageLog,
    ) -> Result<(DataObjectLocation, Vec<TopObject>), media_types::EngineError> {
        let mut objects = Vec::new();
        let mut data: Option<DataObjectLocation> = None;
        let mut pos = header_end;
        let mut guard = 0usize;
        while pos + 24 <= stream_len && guard < MAX_TOP_OBJECTS {
            guard += 1;
            let head = reader.read_prefix_at(24, pos)?;
            if head.len() < 24 {
                return Err(error::truncated(pos + 24, pos + head.len() as u64));
            }
            let guid_at = Guid::read(&head, 0);
            let size = guid::u64le(&head, 16);
            if size < 24 {
                return Err(error::bad_object_size(pos));
            }
            objects.push(TopObject {
                guid: guid_at,
                offset: pos,
                size,
            });
            if guid_at == DATA && data.is_none() {
                // Data Object payload head: FileID(16) + total packet
                // count QWORD(8) + 2 reserved bytes (observed 01 01).
                let body = reader.read_prefix_at(26, pos + 24)?;
                if body.len() < 26 {
                    return Err(error::truncated(pos + 50, pos + 24 + body.len() as u64));
                }
                data = Some(DataObjectLocation {
                    offset: pos,
                    size,
                    packet_count: guid::u64le(&body, 16),
                    reserved_matches: body[24..26] == [0x01, 0x01],
                });
            }
            if pos.checked_add(size).is_none_or(|end| end > stream_len) {
                if guid_at == DATA {
                    // A Data Object whose declared size overruns the
                    // stream: the location is still honest (ASF-b
                    // clamps per-packet), but the trailing-object map
                    // stops here.
                    damage.note(0, "data-size-overrun");
                    break;
                }
                damage.note(size, "top-object-overrun");
                break;
            }
            pos += size;
        }
        let Some(data) = data else {
            // No Data Object at all: recorded honestly — the track
            // table stands, `next_event`'s ASF-b packets would have
            // nothing to walk.
            damage.note(0, "no-data-object");
            return Ok((
                DataObjectLocation {
                    offset: 0,
                    size: 0,
                    packet_count: 0,
                    reserved_matches: false,
                },
                objects,
            ));
        };
        Ok((data, objects))
    }
}
