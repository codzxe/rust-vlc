//! `Cues` parsing → the seek index (card 5.D1).
//!
//! One `CuePoint` per seek target: `CueTime` (absolute scale ticks)
//! plus one `CueTrackPositions` per track (the container byte offset
//! of the holding cluster, relative to the `Segment` payload start —
//! `CueClusterPosition`; `CueRelativePosition` narrows inside it).
//!
//! # Bounds
//!
//! The index is memory-bounded (MP §12): a `Cues` element declaring
//! more than [`MAX_CUE_ENTRIES`] entries is the typed
//! [`crate::mkv::error::cue_table_too_large`] refusal and the demuxer
//! degrades to `SeekSupport::SlowLinear` — never an unbounded
//! allocation.

use media_types::EngineError;

use crate::mkv::ebml::{self, EbmlReader, ElementHeader};
use crate::mkv::error;

/// The cue-entry ceiling (MP §12 index memory; ~1 h at one cue per
/// 5 s cluster is 720 entries — three orders of headroom).
pub const MAX_CUE_ENTRIES: usize = 65_536;

/// One parsed seek target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CuePoint {
    /// The target's absolute timestamp (scale ticks).
    pub time_ticks: i64,
    /// The cluster byte offset (relative to the Segment payload).
    pub cluster_pos: u64,
    /// The in-cluster byte offset when `CueRelativePosition` declared
    /// one (rare; 0 = cluster start).
    pub relative_pos: u64,
    /// The track the entry indexes (multi-track cues repeat targets;
    /// the demuxer keeps the *earliest* cluster per time).
    pub track: u64,
}

/// The parsed index (time-ascending — muxers write ascending, and the
/// parser enforces nothing about order; the seek binary-searches).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CueIndex {
    /// The seek targets.
    pub points: Vec<CuePoint>,
}

impl CueIndex {
    /// The cue point at or before `ticks` (the keyframe-safe landing).
    pub fn landing_before(&self, ticks: i64) -> Option<&CuePoint> {
        // Binary search: last point with time <= ticks.
        let mut lo = 0usize;
        let mut hi = self.points.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.points[mid].time_ticks <= ticks {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo == 0 {
            return None;
        }
        Some(&self.points[lo - 1])
    }
}

/// Parse the `Cues` element (all `CuePoint` children).
///
/// # Errors
/// [`error::cue_table_too_large`] past the ceiling;
/// [`error::bad_structure`] for a point without a usable
/// `CueTrackPositions`; [`error::truncated`] when cut by the stream.
pub fn parse_cues(
    reader: &mut EbmlReader,
    cues: &ElementHeader,
    bound: u64,
) -> Result<CueIndex, EngineError> {
    let end = cues.declared_end().unwrap_or(bound).min(bound);
    let mut index = CueIndex::default();
    let mut pos = cues.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        if child.id == ebml::ID_CUE_POINT {
            if index.points.len() >= MAX_CUE_ENTRIES {
                return Err(error::cue_table_too_large(
                    MAX_CUE_ENTRIES as u64,
                    MAX_CUE_ENTRIES as u64 + 1,
                ));
            }
            if let Some(point) = parse_cue_point(reader, &child, child_end)? {
                index.points.push(point);
            }
        }
        pos = child_end;
    }
    index.points.sort_by_key(|p| p.time_ticks);
    Ok(index)
}

/// One `CuePoint`: `CueTime` + the first `CueTrackPositions`.
/// `Ok(None)` = a degenerate point (skip — the others survive).
fn parse_cue_point(
    reader: &mut EbmlReader,
    point: &ElementHeader,
    end: u64,
) -> Result<Option<CuePoint>, EngineError> {
    let mut time: Option<i64> = None;
    let mut positions: Option<(u64, u64, u64)> = None;
    let mut pos = point.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        match child.id {
            ebml::ID_CUE_TIME => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                time = ebml::uint_from_be(&bytes).map(|v| v as i64);
            }
            ebml::ID_CUE_TRACK_POSITIONS => {
                positions = parse_cue_track_positions(reader, &child, child_end)?;
            }
            _ => {}
        }
        pos = child_end;
    }
    Ok(match (time, positions) {
        (Some(time_ticks), Some((track, cluster_pos, relative_pos))) => Some(CuePoint {
            time_ticks,
            cluster_pos,
            relative_pos,
            track,
        }),
        _ => None,
    })
}

/// One `CueTrackPositions`: track + cluster (and optional relative)
/// offsets.
fn parse_cue_track_positions(
    reader: &mut EbmlReader,
    positions: &ElementHeader,
    end: u64,
) -> Result<Option<(u64, u64, u64)>, EngineError> {
    let mut track = None;
    let mut cluster = None;
    let mut relative = 0u64;
    let mut pos = positions.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        if len > 8 {
            return Err(error::bad_structure(child.data_pos));
        }
        let read_uint = |reader: &mut EbmlReader| -> Result<Option<u64>, EngineError> {
            let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
            Ok(ebml::uint_from_be(&bytes))
        };
        match child.id {
            ebml::ID_CUE_TRACK => track = read_uint(reader)?,
            ebml::ID_CUE_CLUSTER_POSITION => cluster = read_uint(reader)?,
            ebml::ID_CUE_RELATIVE_POSITION => relative = read_uint(reader)?.unwrap_or(0),
            _ => {}
        }
        pos = child_end;
    }
    Ok(match (track, cluster) {
        (Some(track), Some(cluster)) => Some((track, cluster, relative)),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    #[test]
    fn landing_before_is_floor_binary_search() {
        let mut index = CueIndex::default();
        for t in [0i64, 100, 200, 300] {
            index.points.push(CuePoint {
                time_ticks: t,
                cluster_pos: t as u64 * 10,
                relative_pos: 0,
                track: 1,
            });
        }
        assert_eq!(index.landing_before(-1), None);
        assert_eq!(index.landing_before(0).must().time_ticks, 0);
        assert_eq!(index.landing_before(150).must().time_ticks, 100);
        assert_eq!(index.landing_before(300).must().time_ticks, 300);
        assert_eq!(index.landing_before(999).must().time_ticks, 300);
    }
}
