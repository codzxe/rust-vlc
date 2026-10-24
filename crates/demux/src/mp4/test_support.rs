//! In-crate byte builders for the MP4 tests (card 5.C1).
//!
//! Public like [`crate::wav::test_support`] on purpose: the
//! integration suites + fuzz harness need deterministic shapes. This
//! is test-support surface, not player surface — nothing in the
//! demuxer itself uses it.
//!
//! The builders emit the layouts ffmpeg's muxers produce (two-pass
//! offset patching, one-chunk-per-sample stsc, mvex/trex defaults)
//! so the fixtures exercise the same paths the corpus does.

use std::sync::Arc;

use plugin_api::SourceStream;

/// One sample spec for the builders.
#[derive(Clone)]
pub struct SampleSpec {
    pub bytes: Vec<u8>,
    pub dur: u64,
    pub sync: bool,
    pub cts: i64,
}

/// One track spec for the builders.
#[derive(Clone)]
pub struct TrackSpec {
    pub handler: [u8; 4],
    pub entry: [u8; 4],
    pub timescale: u32,
    pub samples: Vec<SampleSpec>,
    /// Optional first `elst` entry: (segment_duration, media_time
    /// with -1 meaning empty edit) — in *movie* timescale units for
    /// the duration, media ticks for media_time.
    pub edit: Option<(u64, i64)>,
    /// When set, the trak carries `tref/chap` referencing this track
    /// id (the chapter-reference shape).
    pub chap_ref: Option<u32>,
}

/// A plain box: `size kind payload`.
fn boxb(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 + payload.len());
    v.extend_from_slice(&((8 + payload.len()) as u32).to_be_bytes());
    v.extend_from_slice(kind);
    v.extend_from_slice(payload);
    v
}

/// A full box: `size kind version+flags payload`.
fn fullbox(kind: &[u8; 4], version: u8, payload: &[u8]) -> Vec<u8> {
    fullbox_f(kind, version, 0, payload)
}

/// A full box with explicit 24-bit flags.
fn fullbox_f(kind: &[u8; 4], version: u8, flags: u32, payload: &[u8]) -> Vec<u8> {
    let mut head = vec![
        version,
        (flags >> 16) as u8,
        (flags >> 8) as u8,
        flags as u8,
    ];
    head.extend_from_slice(payload);
    boxb(kind, &head)
}

fn u32b(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

fn ftyp(major: &[u8; 4]) -> Vec<u8> {
    // ftyp is a plain box (no version/flags): major + minor + compat.
    let mut body = Vec::new();
    body.extend_from_slice(major);
    body.extend_from_slice(&u32b(0));
    for brand in [major, b"isom", b"iso2", b"mp41", b"mp42"] {
        body.extend_from_slice(brand);
    }
    boxb(b"ftyp", &body)
}

fn mvhd(timescale: u32, duration: u64) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&u32b(0)); // creation
    b.extend_from_slice(&u32b(0)); // modification
    b.extend_from_slice(&u32b(timescale));
    b.extend_from_slice(&u32b(duration as u32));
    b.extend_from_slice(&u32b(0x0001_0000)); // rate 16.16
    b.extend_from_slice(&[0, 1]); // volume
    b.extend_from_slice(&[0u8; 10]); // reserved
    b.extend_from_slice(&[0u8; 36]); // matrix
    b.extend_from_slice(&[0u8; 24]); // predefined
    b.extend_from_slice(&u32b(0xFFFF_FFFF)); // next_track_id
    fullbox(b"mvhd", 0, &b)
}

fn tkhd(track_id: u32, duration: u64) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&u32b(0)); // creation
    b.extend_from_slice(&u32b(0)); // modification
    b.extend_from_slice(&u32b(track_id));
    b.extend_from_slice(&u32b(0)); // reserved
    b.extend_from_slice(&u32b(duration as u32));
    b.extend_from_slice(&[0u8; 8]); // reserved
    b.extend_from_slice(&[0u8; 4]); // layer + alternate
    b.extend_from_slice(&[0, 0]); // volume
    b.extend_from_slice(&[0u8; 2]); // reserved
    b.extend_from_slice(&[0u8; 36]); // matrix
    b.extend_from_slice(&u32b(0)); // width 16.16
    b.extend_from_slice(&u32b(0)); // height
    // flags: enabled | in_movie (a zero-flags tkhd is a disabled
    // track — damage, not the demuxer's default state).
    let mut head = vec![0u8, 0, 0, 3];
    head.extend_from_slice(&b);
    boxb(b"tkhd", &head)
}

fn mdhd(timescale: u32, duration: u64) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&u32b(0));
    b.extend_from_slice(&u32b(0));
    b.extend_from_slice(&u32b(timescale));
    b.extend_from_slice(&u32b(duration as u32));
    b.extend_from_slice(&u32b(0x55C4)); // "und"
    b.extend_from_slice(&u32b(0));
    fullbox(b"mdhd", 0, &b)
}

fn hdlr(handler: &[u8; 4]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&u32b(0)); // predefined
    b.extend_from_slice(handler);
    b.extend_from_slice(&[0u8; 12]); // reserved
    b.extend_from_slice(&[0]); // name
    fullbox(b"hdlr", 0, &b)
}

/// One stsd entry (audio-shaped 28-byte payload — the parser only
/// needs the entry's size and kind; the codec cards own the fields).
fn stsd_entry(entry: &[u8; 4]) -> Vec<u8> {
    let mut b = vec![0u8; 28];
    b[8..10].copy_from_slice(&2u16.to_be_bytes()); // channels
    b[10..12].copy_from_slice(&16u16.to_be_bytes()); // samplesize
    b[16..20].copy_from_slice(&44_100u32.to_be_bytes()); // 16.16 rate
    boxb(entry, &b)
}

/// The trak of one track; `offsets` patches stco (None for fragment
/// tracks with empty tables).
fn trak(spec: &TrackSpec, track_id: u32, offsets: Option<&[u64]>) -> Vec<u8> {
    let mut t = Vec::new();
    t.extend_from_slice(&tkhd(track_id, spec.samples.iter().map(|s| s.dur).sum()));
    if let Some(chap) = spec.chap_ref {
        let mut body = Vec::new();
        body.extend_from_slice(&u32b(chap));
        t.extend_from_slice(&boxb(b"tref", &boxb(b"chap", &body)));
    }
    if let Some((seg_dur, media_time)) = spec.edit {
        let mut e = Vec::new();
        e.extend_from_slice(&u32b(1)); // entry_count
        e.extend_from_slice(&u32b(seg_dur as u32));
        e.extend_from_slice(&media_time.to_be_bytes()[4..8]);
        e.extend_from_slice(&u32b(0x0001_0000)); // rate
        t.extend_from_slice(&boxb(b"edts", &fullbox(b"elst", 0, &e)));
    }
    let mut mdia = Vec::new();
    mdia.extend_from_slice(&mdhd(
        spec.timescale,
        spec.samples.iter().map(|s| s.dur).sum(),
    ));
    mdia.extend_from_slice(&hdlr(&spec.handler));
    let mut stbl = Vec::new();
    let mut stsd_body = Vec::new();
    stsd_body.extend_from_slice(&u32b(1)); // entry_count
    stsd_body.extend_from_slice(&stsd_entry(&spec.entry));
    stbl.extend_from_slice(&fullbox(b"stsd", 0, &stsd_body));
    if let Some(offsets) = offsets {
        // stts: one run per distinct delta.
        let mut stts = Vec::new();
        stts.extend_from_slice(&u32b(1)); // patched below per-run
        let mut runs: Vec<(u64, u64)> = Vec::new();
        for s in &spec.samples {
            match runs.last_mut() {
                Some((count, delta)) if *delta == s.dur => *count += 1,
                _ => runs.push((1, s.dur)),
            }
        }
        stts.clear();
        stts.extend_from_slice(&u32b(runs.len() as u32));
        for (count, delta) in &runs {
            stts.extend_from_slice(&u32b(*count as u32));
            stts.extend_from_slice(&u32b(*delta as u32));
        }
        stbl.extend_from_slice(&fullbox(b"stts", 0, &stts));
        let mut stsc = Vec::new();
        stsc.extend_from_slice(&u32b(1));
        stsc.extend_from_slice(&u32b(1));
        stsc.extend_from_slice(&u32b(1));
        stsc.extend_from_slice(&u32b(1));
        stbl.extend_from_slice(&fullbox(b"stsc", 0, &stsc));
        let mut stsz = Vec::new();
        stsz.extend_from_slice(&u32b(0)); // fixed = 0
        stsz.extend_from_slice(&u32b(spec.samples.len() as u32));
        for s in &spec.samples {
            stsz.extend_from_slice(&u32b(s.bytes.len() as u32));
        }
        stbl.extend_from_slice(&fullbox(b"stsz", 0, &stsz));
        let mut stco = Vec::new();
        stco.extend_from_slice(&u32b(offsets.len() as u32));
        for o in offsets {
            stco.extend_from_slice(&u32b(*o as u32));
        }
        stbl.extend_from_slice(&fullbox(b"stco", 0, &stco));
        let syncs: Vec<usize> = spec
            .samples
            .iter()
            .enumerate()
            .filter(|(_, s)| s.sync)
            .map(|(i, _)| i + 1)
            .collect();
        if !syncs.is_empty() && syncs.len() < spec.samples.len() {
            let mut stss = Vec::new();
            stss.extend_from_slice(&u32b(syncs.len() as u32));
            for s in &syncs {
                stss.extend_from_slice(&u32b(*s as u32));
            }
            stbl.extend_from_slice(&fullbox(b"stss", 0, &stss));
        }
        if spec.samples.iter().any(|s| s.cts != 0) {
            let mut ctts = Vec::new();
            ctts.extend_from_slice(&u32b(spec.samples.len() as u32));
            for s in &spec.samples {
                ctts.extend_from_slice(&u32b(1));
                ctts.extend_from_slice(&u32b(s.cts as u32));
            }
            stbl.extend_from_slice(&fullbox(b"ctts", 0, &ctts));
        }
    } else {
        // Empty tables (fragment track).
        stbl.extend_from_slice(&fullbox(b"stts", 0, &u32b(0)));
        stbl.extend_from_slice(&fullbox(b"stsc", 0, &u32b(0)));
        stbl.extend_from_slice(&fullbox(b"stsz", 0, &[0u8; 8]));
        stbl.extend_from_slice(&fullbox(b"stco", 0, &u32b(0)));
    }
    let mut minf = Vec::new();
    minf.extend_from_slice(&fullbox(b"vmhd", 0, &[0u8; 8]));
    minf.extend_from_slice(&boxb(b"stbl", &stbl));
    mdia.extend_from_slice(&boxb(b"minf", &minf));
    t.extend_from_slice(&boxb(b"mdia", &mdia));
    boxb(b"trak", &t)
}

/// The `udta/meta/ilst/covr/data` box ladder for an attached
/// picture (the iTunes cover shape ffmpeg surfaces as an attached-pic
/// stream; `wkt` 13 = JPEG, 14 = PNG).
fn udta_cover(wkt: u32, payload: &[u8]) -> Vec<u8> {
    let mut data_body = Vec::new();
    data_body.extend_from_slice(&u32b(wkt)); // well-known type
    data_body.extend_from_slice(&u32b(0)); // locale
    data_body.extend_from_slice(payload);
    let covr = boxb(b"covr", &boxb(b"data", &data_body));
    let ilst = boxb(b"ilst", &covr);
    let mut hdlr_body = Vec::new();
    hdlr_body.extend_from_slice(&u32b(0)); // predefined
    hdlr_body.extend_from_slice(b"mdir"); // iTunes metadata handler
    hdlr_body.extend_from_slice(&[0u8; 12]); // reserved
    hdlr_body.extend_from_slice(&[0]); // name
    let mut meta = Vec::new();
    meta.extend_from_slice(&[0, 0, 0, 0]); // version/flags (FullBox)
    meta.extend_from_slice(&fullbox(b"hdlr", 0, &hdlr_body));
    meta.extend_from_slice(&ilst);
    boxb(b"udta", &boxb(b"meta", &meta))
}

/// The moov body (children) over the tracks (offsets None for
/// fragment tracks).
fn moov_body(tracks: &[TrackSpec], offsets: Option<&[Vec<u64>]>) -> Vec<u8> {
    let mut m = Vec::new();
    m.extend_from_slice(&mvhd(1000, 0));
    for (i, spec) in tracks.iter().enumerate() {
        let offs = offsets.map(|o| o[i].as_slice());
        m.extend_from_slice(&trak(spec, i as u32 + 1, offs));
    }
    m
}

/// The moov body plus trailing `udta` children (the cover hang).
fn moov_body_with_udta(tracks: &[TrackSpec], offsets: Option<&[Vec<u64>]>, udta: &[u8]) -> Vec<u8> {
    let mut m = moov_body(tracks, offsets);
    m.extend_from_slice(udta);
    m
}

/// The whole moov box.
fn moov(tracks: &[TrackSpec], offsets: Option<&[Vec<u64>]>) -> Vec<u8> {
    boxb(b"moov", &moov_body(tracks, offsets))
}

/// Build a progressive MP4: `ftyp + moov + mdat`, one chunk per
/// sample, offsets patched in a second pass.
#[must_use]
pub fn progressive(tracks: &[TrackSpec], major: &[u8; 4]) -> Arc<[u8]> {
    // Pass 1: measure with zero offsets (same byte lengths).
    let zero: Vec<Vec<u64>> = tracks.iter().map(|t| vec![0u64; t.samples.len()]).collect();
    let mut file = Vec::new();
    file.extend_from_slice(&ftyp(major));
    let moov1 = moov(tracks, Some(&zero));
    file.extend_from_slice(&moov1);
    let mdat_payload_start = file.len() as u64 + 8;
    // Pass 2: real offsets.
    let mut offsets: Vec<Vec<u64>> = Vec::new();
    let mut at = mdat_payload_start;
    for t in tracks {
        let mut list = Vec::new();
        for s in &t.samples {
            list.push(at);
            at += s.bytes.len() as u64;
        }
        offsets.push(list);
    }
    let mut file = Vec::new();
    file.extend_from_slice(&ftyp(major));
    file.extend_from_slice(&moov(tracks, Some(&offsets)));
    let mut mdat = Vec::new();
    for t in tracks {
        for s in &t.samples {
            mdat.extend_from_slice(&s.bytes);
        }
    }
    file.extend_from_slice(&boxb(b"mdat", &mdat));
    Arc::from(file)
}

/// [`progressive`] with an iTunes cover-art `udta` spliced into the
/// moov (the attached-pic shape of corpus 0029-cover-audio.m4a).
#[must_use]
pub fn progressive_with_cover(
    tracks: &[TrackSpec],
    major: &[u8; 4],
    wkt: u32,
    cover: &[u8],
) -> Arc<[u8]> {
    let udta = udta_cover(wkt, cover);
    // Pass 1: measure with zero offsets (same byte lengths).
    let zero: Vec<Vec<u64>> = tracks.iter().map(|t| vec![0u64; t.samples.len()]).collect();
    let mut file = Vec::new();
    file.extend_from_slice(&ftyp(major));
    file.extend_from_slice(&boxb(
        b"moov",
        &moov_body_with_udta(tracks, Some(&zero), &udta),
    ));
    let mdat_payload_start = file.len() as u64 + 8;
    // Pass 2: real offsets.
    let mut offsets: Vec<Vec<u64>> = Vec::new();
    let mut at = mdat_payload_start;
    for t in tracks {
        let mut list = Vec::new();
        for s in &t.samples {
            list.push(at);
            at += s.bytes.len() as u64;
        }
        offsets.push(list);
    }
    let mut file = Vec::new();
    file.extend_from_slice(&ftyp(major));
    file.extend_from_slice(&boxb(
        b"moov",
        &moov_body_with_udta(tracks, Some(&offsets), &udta),
    ));
    let mut mdat = Vec::new();
    for t in tracks {
        for s in &t.samples {
            mdat.extend_from_slice(&s.bytes);
        }
    }
    file.extend_from_slice(&boxb(b"mdat", &mdat));
    Arc::from(file)
}

/// Build a fragmented MP4: `ftyp + moov(mvex/trex, empty tables) +
/// (moof+mdat)*` with per-fragment trun runs (sizes, durations and
/// flags explicit; defaults ride trex).
#[must_use]
pub fn fragmented(tracks: &[TrackSpec], frags: usize, per_frag: usize) -> Arc<[u8]> {
    let mut file = Vec::new();
    file.extend_from_slice(&ftyp(b"isom"));
    let mut m = moov_body(tracks, None);
    let mut mvex = Vec::new();
    for (i, t) in tracks.iter().enumerate() {
        let mut trex = Vec::new();
        trex.extend_from_slice(&u32b(i as u32 + 1)); // track_id
        trex.extend_from_slice(&u32b(1)); // default_sample_desc
        trex.extend_from_slice(&u32b(1024)); // default_duration
        trex.extend_from_slice(&u32b(0)); // default_size (variable)
        trex.extend_from_slice(&u32b(0)); // default_flags (sync)
        mvex.extend_from_slice(&fullbox(b"trex", 0, &trex));
        let _ = t;
    }
    m.extend_from_slice(&boxb(b"mvex", &mvex));
    file.extend_from_slice(&boxb(b"moov", &m));
    let mut dts: Vec<u64> = vec![0; tracks.len()];
    for f in 0..frags {
        // Build the mdat payload first (sizes fixed).
        let mut mdat = Vec::new();
        let mut sizes: Vec<Vec<u32>> = Vec::new();
        for t in tracks {
            let mut list = Vec::new();
            for i in 0..per_frag {
                let idx = f * per_frag + i;
                let bytes = t
                    .samples
                    .get(idx)
                    .map_or(vec![0xAA; 4], |s| s.bytes.clone());
                list.push(bytes.len() as u32);
                mdat.extend_from_slice(&bytes);
            }
            sizes.push(list);
        }
        // The moof box: mfhd + one traf per track. The trun
        // data-offset needs the final moof length, so build the
        // children once, wrap, and compute.
        let mut moof2 = Vec::new();
        moof2.extend_from_slice(&fullbox(b"mfhd", 0, &u32b(f as u32 + 1)));
        for (ti, t) in tracks.iter().enumerate() {
            let mut traf = Vec::new();
            let mut tfhd_body = Vec::new();
            tfhd_body.extend_from_slice(&u32b(ti as u32 + 1));
            traf.extend_from_slice(&fullbox_f(b"tfhd", 0, 0x020_000, &tfhd_body));
            traf.extend_from_slice(&fullbox(b"tfdt", 0, &u32b(dts[ti] as u32)));
            let mut trun = Vec::new();
            trun.extend_from_slice(&u32b(per_frag as u32));
            trun.extend_from_slice(&u32b(0)); // placeholder data_offset
            for (i, &size) in sizes[ti].iter().enumerate() {
                let idx = f * per_frag + i;
                let s = t.samples.get(idx);
                trun.extend_from_slice(&u32b(s.map_or(1024, |s| s.dur as u32)));
                trun.extend_from_slice(&u32b(size));
                let sync = s.is_some_and(|s| s.sync);
                trun.extend_from_slice(&u32b(if sync { 0 } else { 0x8000 }));
                trun.extend_from_slice(&u32b(s.map_or(0, |s| s.cts as u32)));
            }
            traf.extend_from_slice(&fullbox_f(b"trun", 0, 0x000_0F01, &trun));
            moof2.extend_from_slice(&boxb(b"traf", &traf));
        }
        let moof_box = boxb(b"moof", &moof2);
        // Per-track sample start offsets inside this fragment's mdat
        // (the trun data_offset is per-track: track ti's samples sit
        // after every earlier track's in the shared mdat).
        let mut track_start: Vec<usize> = Vec::with_capacity(tracks.len());
        let mut acc = 0usize;
        for list in &sizes {
            track_start.push(acc);
            acc += list.iter().map(|&s| s as usize).sum::<usize>();
        }
        // Rebuild once with the real per-track data_offsets (byte
        // lengths are identical — only field values change).
        let mut moof3 = Vec::new();
        moof3.extend_from_slice(&fullbox(b"mfhd", 0, &u32b(f as u32 + 1)));
        for (ti, t) in tracks.iter().enumerate() {
            let data_offset = moof_box.len() as u32 + 8 + track_start[ti] as u32;
            let mut traf = Vec::new();
            let mut tfhd_body = Vec::new();
            tfhd_body.extend_from_slice(&u32b(ti as u32 + 1));
            traf.extend_from_slice(&fullbox_f(b"tfhd", 0, 0x020_000, &tfhd_body));
            traf.extend_from_slice(&fullbox(b"tfdt", 0, &u32b(dts[ti] as u32)));
            let mut trun = Vec::new();
            trun.extend_from_slice(&u32b(per_frag as u32));
            trun.extend_from_slice(&u32b(data_offset));
            for (i, &size) in sizes[ti].iter().enumerate() {
                let idx = f * per_frag + i;
                let s = t.samples.get(idx);
                trun.extend_from_slice(&u32b(s.map_or(1024, |s| s.dur as u32)));
                trun.extend_from_slice(&u32b(size));
                let sync = s.is_some_and(|s| s.sync);
                trun.extend_from_slice(&u32b(if sync { 0 } else { 0x8000 }));
                trun.extend_from_slice(&u32b(s.map_or(0, |s| s.cts as u32)));
            }
            traf.extend_from_slice(&fullbox_f(b"trun", 0, 0x000_0F01, &trun));
            moof3.extend_from_slice(&boxb(b"traf", &traf));
        }
        file.extend_from_slice(&boxb(b"moof", &moof3));
        file.extend_from_slice(&boxb(b"mdat", &mdat));
        for (ti, t) in tracks.iter().enumerate() {
            for i in 0..per_frag {
                let idx = f * per_frag + i;
                dts[ti] += t.samples.get(idx).map_or(1024, |s| s.dur);
            }
        }
    }
    Arc::from(file)
}

/// A chapter track spec (QuickTime text samples).
#[must_use]
pub fn chapter_track(timescale: u32, titles: &[(u64, &str)]) -> TrackSpec {
    TrackSpec {
        handler: *b"text",
        entry: *b"text",
        timescale,
        edit: None,
        chap_ref: None,
        samples: titles
            .iter()
            .map(|(dur, title)| {
                let mut bytes = Vec::new();
                bytes.extend_from_slice(&(title.len() as u16).to_be_bytes());
                bytes.extend_from_slice(title.as_bytes());
                SampleSpec {
                    bytes,
                    dur: *dur,
                    sync: true,
                    cts: 0,
                }
            })
            .collect(),
    }
}

/// The `tref/chap` reference bytes for a chapter track id (kept
/// public for tests that splice their own shapes).
#[must_use]
pub fn chap_ref(track_id: u32) -> Vec<u8> {
    boxb(b"tref", &boxb(b"chap", &u32b(track_id)))
}

/// [`progressive`] with the first track carrying `tref/chap` → the
/// chapter track (track id 2): the common single-video + chapter
/// shape.
#[must_use]
pub fn progressive_with_chapters(main: &TrackSpec, chapter: &TrackSpec) -> Arc<[u8]> {
    let main = TrackSpec {
        chap_ref: Some(2),
        ..main.clone()
    };
    progressive(&[main, chapter.clone()], b"isom")
}

/// A boxed in-memory [`SourceStream`] (the crate's shared seam).
///
/// # Errors
/// Propagates the avi test seam's (never-failing) URI parse.
pub fn mem_stream(
    bytes: impl Into<Arc<[u8]>>,
) -> Result<Box<dyn SourceStream>, media_types::EngineError> {
    crate::avi::test_support::mem_stream(bytes)
}
