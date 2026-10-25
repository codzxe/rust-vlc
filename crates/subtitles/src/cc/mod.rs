//! Closed captions — CEA-608 (full) + CEA-708 (staged) — decode and
//! render to the compositing contract (card 5.N3, ADR-012 self-build
//! option 1: CC "608 M, then M–L staged").
//!
//! # What this module owns
//!
//! * [`cc_data`] — the ATSC `cc_data()` carriage grammars: the GA94
//!   SEI type-4 form (the H.264 consume seam — the codecs SEI parser
//!   captures type-4 messages intact, read-only), the MPEG-2
//!   picture-`user_data` GA94 form, and SCTE-20 bit-packed carriage.
//! * [`eia608`] — the FULL EIA-608 channel machine: parity, the
//!   field/channel latch, PAC / mid-row / misc / tab control codes,
//!   the 15×32 double-buffered grids, pop-on / roll-up / paint-on.
//! * [`cea708`] — the STAGED CEA-708: DTVCC fragment reassembly, the
//!   service-block layer, the service-1 → 608 compatibility route.
//! * [`render`] — grid snapshot → [`CompositedCue`] (exact monospace
//!   cell metrics over the 704×480 NTSC reference, per-cell colors).
//! * [`fuzz`] — the `fuzz_sub_cc` harness.
//! * this file — [`CcDecoder`] (the stateful front door: triplets +
//!   PTS in, [`CcUpdate`] out, a capped typed damage ledger) and
//!   [`CcCueBuilder`] (update → cues with the end-at-next-event
//!   timing rule).
//!
//! # The honesty table (what is IN, what is recorded OUT)
//!
//! IN (implemented and pinned by tests):
//!
//! | surface | status |
//! |---|---|
//! | GA94 SEI type-4 / MPEG-2 GA94 / SCTE-20 carriage | full |
//! | 608 parity, field/channel latch, CC1–CC4 | full |
//! | 608 PAC (row + attributes + indent), mid-row, tab, misc codes | full |
//! | 608 pop-on double buffer / roll-up window / paint-on | full |
//! | 608 basic + extended charsets | full ([VERIFY] rare slots) |
//! | 708 DTVCC reassembly + service-block walk | full |
//! | 708 service 1 → 608 route | full |
//! | grid → CompositedCue render | full (solid-block glyphs) |
//!
//! OUT (recorded honestly, not interpreted):
//!
//! | surface | status |
//! |---|---|
//! | XDS (field-2 packets with first byte ≤ 0x0F) | skipped, counted |
//! | 708 native window model (DEFINE_WINDOW geometry, pens, colors, per-window addressing, borders, wrap) | walked + op-recorded, never applied |
//! | 708 G0–G3 native glyph planes | byte-walked only |
//! | 608 text mode (T1–T4) rendering | paint-on approximation (documented) |
//! | VBI / line-21 analog extraction | out of v1 scope (ADR-012) |
//!
//! # Timing
//!
//! CC is a streaming protocol with no inherent end times:
//! [`CcCueBuilder::cues`] gives each update's grids the half-open
//! span `[update_pts, next_update_pts)` — closing one nanosecond
//! before the next event, the DVB end-display-time rule — and leaves
//! the last update open-ended (the player's clock closes it; there
//! is no 608/708 page timeout to bound it with, unlike DVB).
//!
//! # Never
//!
//! I/O, unsafe, panics, unbounded allocation (the damage ledger and
//! the DTVCC reassembly buffer are capped — D-2).

pub mod cc_data;
pub mod cea708;
pub mod eia608;
pub mod fuzz;
pub mod render;

use cc_data::{CcDataError, CcDataTriplet};
use cea708::{Cc708ServiceRecord, DtvccReassembler};
use eia608::{Cc608Channel, CcChannel, CcGrid};

/// Damage ledger cap (the SRT/DVB 64–256 event precedent, sized for
/// a few seconds of caption traffic).
pub const MAX_DAMAGE_EVENTS: usize = 256;

/// Typed decode damage (never a panic; the decode continues).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CcDamage {
    /// A pair failed odd parity and was dropped (the spec-conformant
    /// response — ccextractor behavior; ffmpeg processes anyway, a
    /// known divergence).
    Parity {
        /// The channel the pair was latched to.
        channel: CcChannel,
    },
    /// Field-2 XDS traffic (first byte ≤ 0x0F): skipped, counted.
    XdsSkipped,
    /// A pair whose first byte is in the reserved low range on a
    /// caption field (garbage, or an unannounced carriage variant).
    InvalidPair {
        /// The channel.
        channel: CcChannel,
        /// The parity-stripped first byte.
        hi: u8,
    },
    /// A reserved control prefix (`0x10/0x18/0x16/0x1E` family).
    ReservedControl {
        /// The channel.
        channel: CcChannel,
        /// The raw first byte.
        code: u8,
    },
    /// A PAC with the invalid row slot.
    InvalidPac {
        /// The channel.
        channel: CcChannel,
    },
    /// A DTVCC continuation triplet with no active packet.
    DtvContinuationWithoutStart,
    /// A DTVCC size code past the packet cap (defensive — 6-bit
    /// codes cannot exceed it).
    DtvccOverrun {
        /// The computed size.
        size: usize,
    },
    /// A service block cut short by the packet end.
    DtvccTruncatedBlock {
        /// The service.
        service: u8,
        /// The block size the header declared.
        needed: usize,
        /// The bytes actually present.
        got: usize,
    },
    /// A malformed two-byte service escape.
    DtvccBadEscape {
        /// The offending byte.
        got: u8,
    },
    /// A service number outside 1–63 (0, or escape-decoded > 63).
    UnknownService708 {
        /// The offending service number.
        service: u8,
    },
}

impl std::fmt::Display for CcDamage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CcDamage::Parity { channel } => {
                write!(f, "parity failure on {channel:?}, pair dropped")
            }
            CcDamage::XdsSkipped => f.write_str("XDS packet skipped"),
            CcDamage::InvalidPair { channel, hi } => {
                write!(f, "invalid pair (hi {hi:#04x}) on {channel:?}")
            }
            CcDamage::ReservedControl { channel, code } => {
                write!(f, "reserved control code {code:#04x} on {channel:?}")
            }
            CcDamage::InvalidPac { channel } => write!(f, "invalid PAC row slot on {channel:?}"),
            CcDamage::DtvContinuationWithoutStart => {
                f.write_str("DTVCC continuation without an active packet")
            }
            CcDamage::DtvccOverrun { size } => write!(f, "DTVCC packet size {size} past the cap"),
            CcDamage::DtvccTruncatedBlock {
                service,
                needed,
                got,
            } => write!(
                f,
                "service {service} block truncated: needed {needed}, got {got}"
            ),
            CcDamage::DtvccBadEscape { got } => {
                write!(f, "malformed DTVCC service escape {got:#04x}")
            }
            CcDamage::UnknownService708 { service } => {
                write!(f, "unknown CEA-708 service {service}")
            }
        }
    }
}

/// One channel's displayed-grid snapshot at one timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CcCaptionEvent {
    /// The channel whose screen changed.
    pub channel: CcChannel,
    /// The presentation timestamp of the change, ns (`None` before
    /// the first timestamped carriage).
    pub pts_ns: Option<i64>,
    /// The displayed grid.
    pub grid: CcGrid,
}

/// The result of one carriage payload decode.
#[derive(Debug, Clone, Default)]
pub struct CcUpdate {
    /// One event per channel whose displayed grid changed during the
    /// call (the final state, not per-pair deltas — carriage arrives
    /// per field/frame).
    pub events: Vec<CcCaptionEvent>,
    /// The native CEA-708 service blocks seen (recorded, not
    /// interpreted — the module honesty table).
    pub services_708: Vec<Cc708ServiceRecord>,
    /// The damage ledger for this call (capped at
    /// [`MAX_DAMAGE_EVENTS`]).
    pub damage: Vec<CcDamage>,
}

/// One field's 608 state: the active-channel latch plus the two data
/// channels.
#[derive(Debug, Clone, Default)]
struct FieldState {
    active: usize,
    channels: [Cc608Channel; 2],
}

/// The CC decoder: stateful, dirty-world, never panicking.
#[derive(Debug, Clone, Default)]
pub struct CcDecoder {
    fields: [FieldState; 2],
    dtvcc: DtvccReassembler,
    damage: Vec<CcDamage>,
    native_records: Vec<Cc708ServiceRecord>,
}

impl CcDecoder {
    /// A fresh decoder (all channels in pop-on mode, blank grids).
    #[must_use]
    pub fn new() -> CcDecoder {
        CcDecoder::default()
    }

    fn note(&mut self, damage: CcDamage) {
        if self.damage.len() < MAX_DAMAGE_EVENTS {
            self.damage.push(damage);
        }
    }

    /// Decode one carriage payload's triplets at `pts_ns`.
    ///
    /// # Panics
    /// Never (MP §8); damage is typed data.
    #[must_use]
    pub fn decode(&mut self, triplets: &[CcDataTriplet], pts_ns: Option<i64>) -> CcUpdate {
        // per field, per channel
        let mut changed = [[false; 2]; 2];
        for triplet in triplets {
            if !triplet.cc_valid {
                continue;
            }
            match triplet.cc_type {
                0 | 1 => {
                    let field = (triplet.cc_type & 1) as usize;
                    self.decode_608_pair(field, triplet.data, &mut changed);
                }
                2 | 3 => {
                    if let Some(packet) =
                        self.dtvcc
                            .push(triplet.cc_type, triplet.data, &mut self.damage)
                    {
                        self.consume_dtvcc_packet(&packet, &mut changed);
                    }
                }
                _ => {} // cc_type is 2 bits — unreachable by construction
            }
        }
        let mut events = Vec::new();
        for (field, state) in self.fields.iter().enumerate() {
            for (channel, ch) in state.channels.iter().enumerate() {
                if changed[field][channel] {
                    events.push(CcCaptionEvent {
                        channel: CcChannel::new(field as u8, channel == 1),
                        pts_ns,
                        grid: ch.displayed.clone(),
                    });
                }
            }
        }
        CcUpdate {
            events,
            services_708: std::mem::take(&mut self.native_records),
            damage: std::mem::take(&mut self.damage),
        }
    }

    /// The GA94 SEI / MPEG-2 user-data convenience: parse then
    /// decode.
    ///
    /// # Errors
    /// [`CcDataError`] when the carriage grammar itself is broken
    /// (nothing is decoded in that case).
    pub fn decode_ga94(
        &mut self,
        payload: &[u8],
        pts_ns: Option<i64>,
    ) -> Result<CcUpdate, CcDataError> {
        let triplets = cc_data::parse_ga94_sei(payload)?;
        Ok(self.decode(&triplets, pts_ns))
    }

    /// The SCTE-20 convenience: parse then decode.
    ///
    /// # Errors
    /// [`CcDataError`] when the carriage grammar itself is broken.
    pub fn decode_scte20(
        &mut self,
        payload: &[u8],
        pts_ns: Option<i64>,
    ) -> Result<CcUpdate, CcDataError> {
        let triplets = cc_data::parse_scte20(payload)?;
        Ok(self.decode(&triplets, pts_ns))
    }

    /// Drop all state (a seek or stream change).
    pub fn reset(&mut self) {
        self.fields = [FieldState::default(), FieldState::default()];
        self.dtvcc.reset();
        self.damage.clear();
        self.native_records.clear();
    }

    /// One 608 pair: parity, XDS skip, channel latch, dispatch.
    fn decode_608_pair(&mut self, field: usize, data: [u8; 2], changed: &mut [[bool; 2]; 2]) {
        let (hi_raw, lo_raw) = (data[0], data[1]);
        if !eia608::parity_ok(hi_raw) || !eia608::parity_ok(lo_raw) {
            // The pair is dropped with typed damage (spec-conformant).
            let ch = CcChannel::new(field as u8, self.fields[field].active == 1);
            self.note(CcDamage::Parity { channel: ch });
            return;
        }
        let hi = hi_raw & 0x7F;
        let lo = lo_raw & 0x7F;
        if hi <= 0x0F {
            if hi == 0 {
                return; // null padding
            }
            if field == 1 {
                // Field-2 low first bytes are XDS class codes.
                self.note(CcDamage::XdsSkipped);
            } else {
                let ch = CcChannel::new(field as u8, self.fields[field].active == 1);
                self.note(CcDamage::InvalidPair { channel: ch, hi });
            }
            return;
        }
        // The channel latch: codes that carry the channel bit re-latch
        // it; plain text rides the latched channel.
        if (0x10..=0x1F).contains(&hi) || ((0x40..=0x5F).contains(&hi) && lo >= 0x40) {
            self.fields[field].active = usize::from(hi & 0x08 != 0);
        }
        let channel = self.fields[field].active;
        let state = &mut self.fields[field];
        let ch = &mut state.channels[channel];
        let pair_changed = ch.decode_pair(hi, lo);
        let (reserved_control, invalid_pac) = (ch.flags.reserved_control, ch.flags.invalid_pac);
        if pair_changed {
            changed[field][channel] = true;
        }
        // Release the channel borrow before noting damage.
        let channel_id = CcChannel::new(field as u8, channel == 1);
        if reserved_control {
            self.note(CcDamage::ReservedControl {
                channel: channel_id,
                code: hi,
            });
        }
        if invalid_pac {
            self.note(CcDamage::InvalidPac {
                channel: channel_id,
            });
        }
    }

    /// One reassembled DTVCC packet: service walk + records + the
    /// service-1 → 608 route.
    fn consume_dtvcc_packet(&mut self, packet: &[u8], changed: &mut [[bool; 2]; 2]) {
        let blocks = cea708::parse_dtvcc_packet(packet, &mut self.damage);
        for block in &blocks {
            if block.service == 1 {
                // The compatibility service: the payload is the 608
                // byte stream (field 1); parity is re-synthesized
                // (the F1 carriage does not guarantee the bit).
                for pair in block.payload.as_chunks::<2>().0 {
                    let d = [
                        eia608::with_odd_parity(pair[0]),
                        eia608::with_odd_parity(pair[1]),
                    ];
                    self.decode_608_pair(0, d, changed);
                }
            } else {
                let pos = match self
                    .native_records
                    .iter()
                    .position(|r| r.service == block.service)
                {
                    Some(pos) => pos,
                    None => {
                        self.native_records.push(Cc708ServiceRecord {
                            service: block.service,
                            bytes: 0,
                            ops: Vec::new(),
                        });
                        self.native_records.len() - 1
                    }
                };
                cea708::walk_native_block(&block.payload, &mut self.native_records[pos]);
            }
        }
    }
}

/// The cue builder: updates → composited cues with the
/// end-at-next-event rule.
///
/// Timing (module docs): each event's grids span
/// `[event_pts, next_pts)`; `next_pts == None` leaves the last event
/// open-ended (the player's clock closes it).
#[derive(Debug, Clone, Default)]
pub struct CcCueBuilder;

impl CcCueBuilder {
    /// A fresh builder (stateless — the grids carry the state).
    #[must_use]
    pub fn new() -> CcCueBuilder {
        CcCueBuilder
    }

    /// Convert one update into cues. `z_base` orders concurrent
    /// channel events (event order: CC1, CC2, CC3, CC4).
    ///
    /// # Errors
    /// [`CueError`](crate::compositing::CueError) from the bitmap or
    /// span validation.
    pub fn cues(
        &self,
        update: &CcUpdate,
        next_pts_ns: Option<i64>,
        z_base: u16,
    ) -> Result<Vec<crate::compositing::CompositedCue>, crate::compositing::CueError> {
        let mut cues = Vec::new();
        for (i, event) in update.events.iter().enumerate() {
            let Some(rendered) = render::render_grid(&event.grid)? else {
                continue; // a cleared screen renders no cue
            };
            let end = next_pts_ns.map(|next| next.saturating_sub(1));
            cues.push(render::grid_cue(
                &rendered,
                event.pts_ns,
                end,
                z_base.saturating_add(i as u16),
            )?);
        }
        Ok(cues)
    }
}
