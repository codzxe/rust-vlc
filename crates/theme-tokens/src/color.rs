//! Color primitives and arithmetic for the theme-token pipeline (card 1.B4).
//!
//! Pure `f64` math (add/mul/div only — bit-deterministic on every platform we
//! target). All derivation rules are calibrated on the mined orange accent
//! family (doc 08 §1.2) and are `[PROPOSAL]` until the Figma variables export
//! arrives: re-pin per doc 08 §2/§3.3.

use std::fmt;

/// An 8-bit RGB color. The only color type that crosses the crate boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb8 {
    /// Red component, 0–255.
    pub r: u8,
    /// Green component, 0–255.
    pub g: u8,
    /// Blue component, 0–255.
    pub b: u8,
}

impl Rgb8 {
    /// Build a color from components (a `const fn` so generated constants
    /// can use it).
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parse a `#rrggbb` hex string (case-insensitive, exactly 7 chars).
    pub fn parse_hex(s: &str) -> Result<Self, String> {
        let bytes = s.as_bytes();
        if bytes.len() != 7 || bytes[0] != b'#' {
            return Err(format!("expected `#rrggbb`, got `{s}`"));
        }
        let mut comps = [0u8; 3];
        // `bytes` is exactly 7 bytes here, so `bytes[1..]` is exactly three
        // 2-byte chunks with no remainder.
        for (i, pair) in bytes[1..].as_chunks::<2>().0.iter().enumerate() {
            let hi = hex_digit(pair[0])?;
            let lo = hex_digit(pair[1])?;
            comps[i] = hi * 16 + lo;
        }
        Ok(Self::new(comps[0], comps[1], comps[2]))
    }

    /// Lowercase `#rrggbb` form (the canonical emission form).
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl fmt::Display for Rgb8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

fn hex_digit(c: u8) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(format!("invalid hex digit `{}`", c as char)),
    }
}

/// A drop shadow (doc 08 §1.1 `feOffset`/`feGaussianBlur` chains).
/// Plain data so generated constants can construct it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    /// Horizontal offset.
    pub offset_x: f32,
    /// Vertical offset (positive = down).
    pub offset_y: f32,
    /// Gaussian blur radius.
    pub blur: f32,
    /// Spread radius (0 in the mined corpus).
    pub spread: f32,
    /// Shadow color (always black in the mined corpus).
    pub color: Rgb8,
    /// Shadow alpha, carried separately from the color (doc 08 §1.2).
    pub alpha: f32,
}

/// HSL triple (h in degrees 0–360, s and l in 0–1).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hsl {
    h: f64,
    s: f64,
    l: f64,
}

fn rgb_to_hsl(c: Rgb8) -> Hsl {
    let r = f64::from(c.r) / 255.0;
    let g = f64::from(c.g) / 255.0;
    let b = f64::from(c.b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let (h, s) = if (max - min).abs() < f64::EPSILON {
        (0.0, 0.0)
    } else {
        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        let h = match (max == r, max == g, max == b) {
            (true, _, _) => (g - b) / d + if g < b { 6.0 } else { 0.0 },
            (_, true, _) => (b - r) / d + 2.0,
            _ => (r - g) / d + 4.0,
        };
        (h * 60.0, s)
    };
    Hsl { h, s, l }
}

fn hue_to_rgb(p: f64, q: f64, t: f64) -> f64 {
    let mut t = t;
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

fn hsl_to_rgb(c: Hsl) -> Rgb8 {
    if c.s == 0.0 {
        let v = (c.l * 255.0).round().clamp(0.0, 255.0) as u8;
        return Rgb8::new(v, v, v);
    }
    let q = if c.l < 0.5 {
        c.l * (1.0 + c.s)
    } else {
        c.l + c.s - c.l * c.s
    };
    let p = 2.0 * c.l - q;
    let h = c.h / 360.0;
    let chan = |t: f64| (hue_to_rgb(p, q, t) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb8::new(chan(h + 1.0 / 3.0), chan(h), chan(h - 1.0 / 3.0))
}

/// WCAG 2.x relative-luminance contrast ratio between two colors.
pub fn contrast_ratio(a: Rgb8, b: Rgb8) -> f64 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

fn relative_luminance(c: Rgb8) -> f64 {
    let chan = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * chan(c.r) + 0.7152 * chan(c.g) + 0.0722 * chan(c.b)
}

/// Minimum text-on-accent contrast required by doc 08 §3.3.
pub const MIN_ON_ACCENT_CONTRAST: f64 = 4.5;

/// Pick the on-accent glyph color: the first candidate with contrast ≥ 4.5:1
/// against `primary` (doc 08 §3.3) `[PROPOSAL]`. The candidate list is
/// declared in `themes/tokens.json` (`on-accent-candidates`) and follows the
/// design's dark-glyph-first language: play-button glyph `#161616`, deepest
/// well `#090909`, brightest text `#EAEAEA` — the first three are mined
/// values (doc 08 §1.2); the final pure-white fallback `#FFFFFF`
/// (`primitive.color.gray.0`, `[PROPOSAL]` added by card 1.B4-R) exists
/// because the violet accent reaches 4.5:1 against none of the mined values.
pub fn select_on_accent(primary: Rgb8, candidates: &[Rgb8]) -> Result<Rgb8, String> {
    if candidates.is_empty() {
        return Err("no on-accent candidates declared".to_owned());
    }
    let mut best = 0.0_f64;
    for cand in candidates {
        let ratio = contrast_ratio(*cand, primary);
        if ratio >= MIN_ON_ACCENT_CONTRAST {
            return Ok(*cand);
        }
        best = best.max(ratio);
    }
    Err(format!(
        "no on-accent candidate reaches {MIN_ON_ACCENT_CONTRAST}:1 against {} \
         (best {best:.2}:1)",
        primary.to_hex()
    ))
}

/// One accent ramp. For the default (orange) accent these values come from
/// the token file (mined); for the 8 alternates they are computed by
/// [`derive_ramp`] per doc 08 §3.3 `[PROPOSAL]`.
#[derive(Clone, Debug, PartialEq)]
pub struct AccentRamp {
    /// Accent id (`data-accent` value).
    pub id: String,
    /// Primary swatch (mined for all 9).
    pub primary: Rgb8,
    /// Darker variant (ON-knob, EQ thumbs, pressed fills). Dark+light.
    pub strong: Rgb8,
    /// Tinted fill on dark (seek progress, segmented selected, ON track).
    pub tinted_dark: Rgb8,
    /// Tinted fill on light (progress tint).
    pub tinted_light: Rgb8,
    /// Brighter accent used for knobs on light.
    pub bright_light: Rgb8,
    /// Hover-bright accent in DARK mode (w26 Lead: the contract §2 hover —
    /// dark hover previously equaled the primary, an invisible hover state).
    /// Same formula as bright_light: `hsl(h, s + 0.02, l + 0.045)`.
    pub bright_dark: Rgb8,
    /// Icon-tint variant on light.
    pub icon_light: Rgb8,
    /// Glyph color on accent fills (contrast-selected).
    pub on_accent: Rgb8,
}

/// Derive the 6 non-primary ramp entries for an accent hue, calibrated on the
/// mined orange family (doc 08 §1.2 → §3.3) `[PROPOSAL]`:
///
/// * `strong`       = `hsl(h, s, l − 0.09)`        (orange: 46% → 37% ≈ #AB6314)
/// * `tinted_dark`  = `hsl(h, s, 0.1196)`           (reproduces #371F06
///   exactly — 0.1196 is the mined color's own HSL lightness; the naive
///   0.12 lands one green step high at #372006)
/// * `tinted_light` = `hsl(h, s × 0.56, 0.725)`    (≈ #D8BA9A)
/// * `bright_light` = `hsl(h, s + 0.02, l + 0.045)` (≈ #E8851A; the saturation
///   boost is skipped for near-achromatic accents so gray stays gray)
/// * `icon_light`   = `hsl(h, s − 0.03, l − 0.03)`  (≈ #C2711A)
/// * `on_accent`    = first contrast-passing candidate from `candidates`
///   (see [`select_on_accent`])
pub fn derive_ramp(id: &str, primary: Rgb8, candidates: &[Rgb8]) -> Result<AccentRamp, String> {
    let hsl = rgb_to_hsl(primary);
    let strong = hsl_to_rgb(Hsl {
        l: (hsl.l - 0.09).max(0.0),
        ..hsl
    });
    let tinted_dark = hsl_to_rgb(Hsl { l: 0.1196, ..hsl });
    let tinted_light = hsl_to_rgb(Hsl {
        s: hsl.s * 0.56,
        l: 0.725,
        ..hsl
    });
    let sat_boost = if hsl.s < 0.05 {
        hsl.s
    } else {
        (hsl.s + 0.02).min(1.0)
    };
    let bright_light = hsl_to_rgb(Hsl {
        s: sat_boost,
        l: (hsl.l + 0.045).min(1.0),
        ..hsl
    });
    let icon_light = hsl_to_rgb(Hsl {
        s: (hsl.s - 0.03).max(0.0),
        l: (hsl.l - 0.03).max(0.0),
        ..hsl
    });
    let on_accent = select_on_accent(primary, candidates)?;
    Ok(AccentRamp {
        id: id.to_owned(),
        primary,
        strong,
        tinted_dark,
        tinted_light,
        bright_light,
        bright_dark: bright_light,
        icon_light,
        on_accent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tinted-dark rule must reproduce the mined #371F06 exactly, and the
    /// on-accent pick must be the mined play-glyph #161616.
    #[test]
    fn tinted_dark_rule_reproduces_mined_value() -> Result<(), Box<dyn std::error::Error>> {
        let candidates = [
            Rgb8::parse_hex("#161616")?,
            Rgb8::parse_hex("#090909")?,
            Rgb8::parse_hex("#EAEAEA")?,
            Rgb8::parse_hex("#FFFFFF")?,
        ];
        let orange = Rgb8::parse_hex("#D37918")?;
        let ramp = derive_ramp("orange", orange, &candidates)?;
        assert_eq!(ramp.tinted_dark, Rgb8::parse_hex("#371F06")?);
        assert_eq!(ramp.on_accent, Rgb8::parse_hex("#161616")?);
        Ok(())
    }

    #[test]
    fn contrast_of_mined_play_glyph_passes() {
        let orange = Rgb8::new(0xD3, 0x79, 0x18);
        let glyph = Rgb8::new(0x16, 0x16, 0x16);
        assert!(contrast_ratio(glyph, orange) >= MIN_ON_ACCENT_CONTRAST);
    }

    /// Blue is too dark for any dark glyph and falls through to the light
    /// candidates; violet reaches 4.5:1 against none of the mined values
    /// and falls through to the pure-white fallback (doc 08 §3.3 contrast
    /// rule; white fallback [PROPOSAL] card 1.B4-R).
    #[test]
    fn dark_accents_fall_through_to_light_glyphs() -> Result<(), Box<dyn std::error::Error>> {
        let candidates = [
            Rgb8::parse_hex("#161616")?,
            Rgb8::parse_hex("#090909")?,
            Rgb8::parse_hex("#EAEAEA")?,
            Rgb8::parse_hex("#FFFFFF")?,
        ];
        let blue = derive_ramp("blue", Rgb8::parse_hex("#185DD3")?, &candidates)?;
        let violet = derive_ramp("violet", Rgb8::parse_hex("#8E42FF")?, &candidates)?;
        assert_eq!(blue.on_accent, Rgb8::parse_hex("#EAEAEA")?);
        assert_eq!(violet.on_accent, Rgb8::parse_hex("#FFFFFF")?);
        Ok(())
    }

    #[test]
    fn hex_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        // `to_hex` is the lowercase canonical emission form (doc 08 §3.1).
        for hex in ["#090909", "#d37918", "#eaeaea", "#371f06", "#ffffff"] {
            assert_eq!(Rgb8::parse_hex(hex)?.to_hex(), hex);
        }
        assert_eq!(
            Rgb8::parse_hex("#D37918")?.to_hex(),
            "#d37918",
            "uppercase input must normalize to the lowercase emission form"
        );
        assert!(Rgb8::parse_hex("#0909").is_err());
        assert!(Rgb8::parse_hex("090909").is_err());
        assert!(Rgb8::parse_hex("#09090g").is_err());
        Ok(())
    }

    #[test]
    fn gray_accent_stays_achromatic() -> Result<(), Box<dyn std::error::Error>> {
        let candidates = [Rgb8::parse_hex("#161616")?];
        let gray = Rgb8::new(0xAF, 0xAF, 0xAF);
        let ramp = derive_ramp("gray", gray, &candidates)?;
        assert_eq!(ramp.strong.r, ramp.strong.b);
        assert_eq!(ramp.strong.g, ramp.strong.b);
        assert_eq!(ramp.tinted_light.r, ramp.tinted_light.b);
        Ok(())
    }
}
