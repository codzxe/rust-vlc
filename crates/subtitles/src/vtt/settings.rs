//! Cue settings (card 5.M3): the supported subset maps onto the model's
//! [`Positioning`]; known-but-unsupported settings (`vertical`, `size`)
//! are recorded as typed damage notes, never silently applied, never
//! fatal. Unknown setting names (future WebVTT revisions) are recorded
//! too and skipped, per the spec's forward-compatibility rule.
//!
//! ## Mapping \[PROPOSAL — unified with the SRT/ASS routes at 5.M1\]
//!
//! | WebVTT          | Model                                                    |
//! |-----------------|----------------------------------------------------------|
//! | `line:0%`       | [`LinePlacement::Percent`]                               |
//! | `line:-1`       | [`LinePlacement::LineNumber`] (negative counts from bottom)|
//! | `line:…,start/center/end` | vertical anchor: Top / Middle / Bottom        |
//! | `position:50%`  | [`Positioning::position`]                                |
//! | `position:…,left/center/right/line-left/line-right` | horizontal anchor |
//! | `align:start/center/end/left/right` | horizontal anchor (start=left, end=right) |
//! | `vertical:rl/lr`, `size:n%` | **unsupported** — typed note, not applied      |
//!
//! Per the spec, a duplicate setting name is ignored after its first
//! valid occurrence; settings apply in encounter order so a later
//! `align` can override a `position:…,X` suffix's horizontal choice.

use crate::model::position::{
    HorizontalAlign, LinePlacement, Percent, Positioning, VerticalAlign,
};

use super::DamageEvent;

/// What the settings line produced: positioning parts (partial — merged
/// onto the default by the caller) plus zero or more damage notes.
#[derive(Debug, Default)]
pub(super) struct SettingsOutcome {
    /// Horizontal anchor, if any setting set it.
    pub horizontal: Option<HorizontalAlign>,
    /// Vertical anchor, if any setting set it.
    pub vertical: Option<VerticalAlign>,
    /// Vertical placement, if `line` was given.
    pub line: Option<LinePlacement>,
    /// Horizontal position, if `position` was given.
    pub position: Option<Percent>,
}

/// Parse the whitespace-separated settings that follow a cue's second
/// timestamp. `line_no` is the 1-based number of the timing line (for
/// the damage events).
pub(super) fn parse_settings(
    settings: &str,
    line_no: u32,
    out: &mut Vec<DamageEvent>,
) -> SettingsOutcome {
    let mut result = SettingsOutcome::default();
    // First valid occurrence of a setting name wins (spec); later
    // duplicates are ignored outright.
    let mut seen_line = false;
    let mut seen_position = false;
    let mut seen_align = false;
    let mut seen_vertical = false;
    let mut seen_size = false;
    for token in settings.split_ascii_whitespace() {
        let Some((name, value)) = token.split_once(':') else {
            out.push(DamageEvent::bad_setting("", line_no));
            continue;
        };
        match name {
            "line" => {
                if !seen_line && parse_line(value, &mut result, line_no, out) {
                    seen_line = true;
                }
            }
            "position" => {
                if !seen_position && parse_position(value, &mut result, line_no, out) {
                    seen_position = true;
                }
            }
            "align" => {
                if !seen_align && parse_align(value, &mut result, line_no, out) {
                    seen_align = true;
                }
            }
            "vertical" => {
                if !seen_vertical {
                    seen_vertical = true;
                    out.push(DamageEvent::unsupported_setting(name, line_no));
                }
            }
            "size" => {
                if !seen_size {
                    seen_size = true;
                    out.push(DamageEvent::unsupported_setting(name, line_no));
                }
            }
            other => out.push(DamageEvent::unknown_setting(other, line_no)),
        }
    }
    result
}

/// `line:<(-)digits[%][,start|center|end]>`
fn parse_line(
    value: &str,
    result: &mut SettingsOutcome,
    line_no: u32,
    out: &mut Vec<DamageEvent>,
) -> bool {
    let (main, suffix) = match value.split_once(',') {
        Some((m, s)) => (m, Some(s)),
        None => (value, None),
    };
    let placement = if let Some(digits) = main.strip_suffix('%') {
        match parse_i32(digits) {
            Some(v) => match Percent::new(v) {
                Ok(p) => Some(LinePlacement::Percent(p)),
                Err(_) => None,
            },
            None => None,
        }
    } else {
        parse_i64(main).map(LinePlacement::LineNumber)
    };
    let Some(placement) = placement else {
        out.push(DamageEvent::bad_setting("line", line_no));
        return false;
    };
    if let Some(suffix) = suffix {
        match suffix {
            "start" => result.vertical = Some(VerticalAlign::Top),
            "center" => result.vertical = Some(VerticalAlign::Middle),
            "end" => result.vertical = Some(VerticalAlign::Bottom),
            _ => {
                out.push(DamageEvent::bad_setting("line", line_no));
                return false;
            }
        }
    }
    result.line = Some(placement);
    true
}

/// `position:<digits%[,left|center|right|line-left|line-right]>`
fn parse_position(
    value: &str,
    result: &mut SettingsOutcome,
    line_no: u32,
    out: &mut Vec<DamageEvent>,
) -> bool {
    let (main, suffix) = match value.split_once(',') {
        Some((m, s)) => (m, Some(s)),
        None => (value, None),
    };
    let Some(digits) = main.strip_suffix('%') else {
        out.push(DamageEvent::bad_setting("position", line_no));
        return false;
    };
    let Some(number) = parse_i32(digits) else {
        out.push(DamageEvent::bad_setting("position", line_no));
        return false;
    };
    let Ok(percent) = Percent::new(number) else {
        out.push(DamageEvent::bad_setting("position", line_no));
        return false;
    };
    if let Some(suffix) = suffix {
        match suffix {
            "left" | "line-left" => result.horizontal = Some(HorizontalAlign::Left),
            "center" => result.horizontal = Some(HorizontalAlign::Center),
            "right" | "line-right" => result.horizontal = Some(HorizontalAlign::Right),
            _ => {
                out.push(DamageEvent::bad_setting("position", line_no));
                return false;
            }
        }
    }
    result.position = Some(percent);
    true
}

/// `align:start|center|end|left|right` (start/end are writing-direction
/// relative; horizontal scripts map start→left, end→right).
fn parse_align(
    value: &str,
    result: &mut SettingsOutcome,
    line_no: u32,
    out: &mut Vec<DamageEvent>,
) -> bool {
    match value {
        "start" | "left" => result.horizontal = Some(HorizontalAlign::Left),
        "center" | "middle" => result.horizontal = Some(HorizontalAlign::Center),
        "end" | "right" => result.horizontal = Some(HorizontalAlign::Right),
        _ => {
            out.push(DamageEvent::bad_setting("align", line_no));
            return false;
        }
    }
    true
}

fn parse_i32(s: &str) -> Option<i32> {
    if s.is_empty() || s.len() > 11 {
        return None;
    }
    s.parse().ok()
}

fn parse_i64(s: &str) -> Option<i64> {
    if s.is_empty() || s.len() > 20 {
        return None;
    }
    s.parse().ok()
}

/// Merge a [`SettingsOutcome`] onto the classic bottom-center default.
pub(super) fn to_positioning(outcome: &SettingsOutcome) -> Positioning {
    let mut positioning = Positioning::default();
    if let Some(horizontal) = outcome.horizontal {
        positioning.alignment.horizontal = horizontal;
    }
    if let Some(vertical) = outcome.vertical {
        positioning.alignment.vertical = vertical;
    }
    positioning.line = outcome.line;
    positioning.position = outcome.position;
    positioning
}

#[cfg(test)]
mod tests {
    use super::{parse_settings, to_positioning};
    use crate::model::position::{HorizontalAlign, LinePlacement, Percent, VerticalAlign};
    use crate::vtt::damage::DamageKind;

    fn run(settings: &str) -> (super::SettingsOutcome, Vec<crate::vtt::DamageEvent>) {
        let mut damage = Vec::new();
        let outcome = parse_settings(settings, 1, &mut damage);
        (outcome, damage)
    }

    #[test]
    fn subset_maps_to_model() {
        let (o, d) = run("line:0% position:50% align:start");
        assert!(d.is_empty());
        assert_eq!(
            o.line,
            Some(LinePlacement::Percent(Percent::new(0).unwrap()))
        );
        assert_eq!(o.position, Some(Percent::new(50).unwrap()));
        assert_eq!(o.horizontal, Some(HorizontalAlign::Left));

        let (o, d) = run("align:center");
        assert!(d.is_empty());
        assert_eq!(o.horizontal, Some(HorizontalAlign::Center));

        let (o, d) = run("line:-1,end position:12%");
        assert!(d.is_empty());
        assert_eq!(o.line, Some(LinePlacement::LineNumber(-1)));
        assert_eq!(o.vertical, Some(VerticalAlign::Bottom));
        assert_eq!(o.position, Some(Percent::new(12).unwrap()));
        assert_eq!(o.horizontal, None);

        let (o, d) = run("position:75%,line-right");
        assert!(d.is_empty());
        assert_eq!(o.horizontal, Some(HorizontalAlign::Right));
    }

    #[test]
    fn unsupported_and_unknown_are_typed_notes() {
        let (o, d) = run("vertical:rl size:50%");
        assert_eq!(o.line, None);
        assert_eq!(d.len(), 2);
        assert!(matches!(d[0].kind, DamageKind::UnsupportedSetting { .. }));
        assert!(matches!(d[1].kind, DamageKind::UnsupportedSetting { .. }));

        let (_o, d) = run("futurething:yes");
        assert!(matches!(d[0].kind, DamageKind::UnknownSetting { .. }));
    }

    #[test]
    fn bad_values_damage_not_panic() {
        for bad in [
            "line:abc", "line:101%", "line:-101%", "position:150%", "position:50",
            "position:", "align:diagonal", "line:5,sideways", "position:9,up",
        ] {
            let (_o, d) = run(bad);
            assert_eq!(d.len(), 1, "{bad:?} produced {d:?}");
            assert!(matches!(d[0].kind, DamageKind::BadSetting { .. }));
        }
    }

    #[test]
    fn first_valid_occurrence_wins_and_order_overrides() {
        // Duplicate `align`: the first valid one (center) wins.
        let (o, d) = run("align:center align:left");
        assert!(d.is_empty());
        assert_eq!(o.horizontal, Some(HorizontalAlign::Center));
        // A later `align` overrides a `position` suffix (encounter order).
        let (o, _) = run("position:10%,left align:right");
        assert_eq!(o.horizontal, Some(HorizontalAlign::Right));
        assert_eq!(o.position, Some(Percent::new(10).unwrap()));
    }

    #[test]
    fn positioning_merge_default_is_bottom_center() {
        let outcome = run("position:50%").0;
        let p = to_positioning(&outcome);
        assert_eq!(p.alignment.vertical, VerticalAlign::Bottom);
        assert_eq!(p.alignment.horizontal, HorizontalAlign::Center);
        assert_eq!(p.position, Some(Percent::new(50).unwrap()));
        assert_eq!(p.line, None);
    }
}
