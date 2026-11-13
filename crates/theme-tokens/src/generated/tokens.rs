//! GENERATED FILE — DO NOT EDIT.
//!
//! Produced by `cargo xtask tokens` (crates/theme-tokens `tokens-gen`,
//! card 1.B4) from `themes/tokens.json` — doc 08 §3. Regeneration is
//! idempotent; the committed copy is diff-tested by
//! `crates/theme-tokens/tests/diff.rs`.
//!
//! Semantic-layer constants for engine-side consumption (doc 01 §2.3,
//! doc 08 §3.1). Components and engine code consume SEMANTIC tokens
//! only (doc 08 §3.2) — the primitive and alias layers are never
//! referenced directly (`theme_tokens::lint` checks this).
//!
//! Modes (2): dark (default), light. Per-mode tokens carry a `_{MODE}` suffix for
//! every registered mode.
//! Accents (9): orange (default), red, yellow, green, cyan, blue, violet, pink, gray.
//! One ramp per accent (doc 08 §3.3): `ACCENT_{ID}_{ROLE}` with
//! roles PRIMARY, STRONG, TINTED_DARK, TINTED_LIGHT, BRIGHT_LIGHT,
//! BRIGHT_DARK, ICON_LIGHT, ON_ACCENT (j18 F2: the list now names
//! all eight emitted roles).
//!
//! Source shape: 123 primitive / 86 alias / 86 semantic tokens.

use crate::color::{Rgb8, Shadow};

/// Registered modes, in declaration order (doc 08 §3.3).
pub const MODES: &[&str] = &["dark", "light"];
/// The default mode.
pub const DEFAULT_MODE: &str = "dark";
/// Registered accent ids, in `data-accent` order (doc 08 §3.3).
#[rustfmt::skip]
pub const ACCENTS: &[&str] = &[
    "orange",
    "red",
    "yellow",
    "green",
    "cyan",
    "blue",
    "violet",
    "pink",
    "gray",
];
/// The default accent.
pub const DEFAULT_ACCENT: &str = "orange";

// -- semantic tokens (sorted by constant name) -----------------------------

pub const BLUR_BACKDROP: f32 = 25.0;
pub const BLUR_BACKDROP_SMALL: f32 = 13.75;
pub const COLOR_ACCENT: Rgb8 = Rgb8::new(0xd3, 0x79, 0x18);
pub const COLOR_ACCENT_BRIGHT_DARK: Rgb8 = Rgb8::new(0xe8, 0x85, 0x1a);
pub const COLOR_ACCENT_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xb3, 0x5f, 0x0d);
pub const COLOR_ACCENT_ICON_DARK: Rgb8 = Rgb8::new(0xd3, 0x79, 0x18);
pub const COLOR_ACCENT_ICON_LIGHT: Rgb8 = Rgb8::new(0xc2, 0x71, 0x1a);
pub const COLOR_ACCENT_STRONG: Rgb8 = Rgb8::new(0xab, 0x63, 0x14);
pub const COLOR_ACCENT_TINTED_DARK: Rgb8 = Rgb8::new(0x37, 0x1f, 0x06);
pub const COLOR_ACCENT_TINTED_LIGHT: Rgb8 = Rgb8::new(0xd8, 0xba, 0x9a);
pub const COLOR_CONTENT_ACCELERATOR_DARK: Rgb8 = Rgb8::new(0x6b, 0x6b, 0x6b);
pub const COLOR_CONTENT_ACCELERATOR_LIGHT: Rgb8 = Rgb8::new(0x5f, 0x5f, 0x5f);
pub const COLOR_CONTENT_BRIGHT_DARK: Rgb8 = Rgb8::new(0xea, 0xea, 0xea);
pub const COLOR_CONTENT_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0x16, 0x16, 0x16);
pub const COLOR_CONTENT_ON_ACCENT: Rgb8 = Rgb8::new(0x09, 0x09, 0x09);
pub const COLOR_CONTENT_PRIMARY_DARK: Rgb8 = Rgb8::new(0xc6, 0xc6, 0xc6);
pub const COLOR_CONTENT_PRIMARY_LIGHT: Rgb8 = Rgb8::new(0x16, 0x16, 0x16);
pub const COLOR_CONTENT_QUIET_DARK: Rgb8 = Rgb8::new(0x6b, 0x6b, 0x6b);
pub const COLOR_CONTENT_QUIET_LIGHT: Rgb8 = Rgb8::new(0x5f, 0x5f, 0x5f);
pub const COLOR_CONTENT_SECONDARY_DARK: Rgb8 = Rgb8::new(0x8f, 0x8f, 0x8f);
pub const COLOR_CONTENT_SECONDARY_LIGHT: Rgb8 = Rgb8::new(0x3a, 0x3a, 0x3a);
pub const COLOR_CONTENT_TERTIARY_DARK: Rgb8 = Rgb8::new(0x82, 0x82, 0x82);
pub const COLOR_CONTENT_TERTIARY_LIGHT: Rgb8 = Rgb8::new(0x47, 0x47, 0x47);
pub const COLOR_EQ_1: Rgb8 = Rgb8::new(0xe1, 0x50, 0x32);
pub const COLOR_EQ_2: Rgb8 = Rgb8::new(0xc4, 0x9e, 0x23);
pub const COLOR_EQ_3: Rgb8 = Rgb8::new(0x19, 0xa2, 0x34);
pub const COLOR_EQ_4: Rgb8 = Rgb8::new(0x18, 0xaa, 0xd3);
pub const COLOR_EQ_5: Rgb8 = Rgb8::new(0x18, 0x5d, 0xd3);
pub const COLOR_EQ_6: Rgb8 = Rgb8::new(0x8e, 0x42, 0xff);
pub const COLOR_EQ_7: Rgb8 = Rgb8::new(0xf0, 0x52, 0xb3);
pub const COLOR_FOCUS_RING_DARK: Rgb8 = Rgb8::new(0xd3, 0x79, 0x18);
pub const COLOR_FOCUS_RING_LIGHT: Rgb8 = Rgb8::new(0x82, 0x45, 0x05);
pub const COLOR_RECORD: Rgb8 = Rgb8::new(0xe1, 0x50, 0x32);
pub const COLOR_STROKE_BORDER_DARK: Rgb8 = Rgb8::new(0x40, 0x40, 0x40);
pub const COLOR_STROKE_BORDER_LIGHT: Rgb8 = Rgb8::new(0x9a, 0x9a, 0x9a);
pub const COLOR_STROKE_ICON_DARK: Rgb8 = Rgb8::new(0x8f, 0x8f, 0x8f);
pub const COLOR_STROKE_ICON_LIGHT: Rgb8 = Rgb8::new(0x6b, 0x6b, 0x6b);
pub const COLOR_SURFACE_CHROME_DARK: Rgb8 = Rgb8::new(0x1b, 0x1b, 0x1b);
pub const COLOR_SURFACE_CHROME_LIGHT: Rgb8 = Rgb8::new(0xab, 0xab, 0xab);
pub const COLOR_SURFACE_DARK: Rgb8 = Rgb8::new(0x09, 0x09, 0x09);
pub const COLOR_SURFACE_DEEP_DARK: Rgb8 = Rgb8::new(0x0e, 0x0e, 0x0e);
pub const COLOR_SURFACE_DEEP_LIGHT: Rgb8 = Rgb8::new(0xba, 0xba, 0xba);
pub const COLOR_SURFACE_ELEVATED_DARK: Rgb8 = Rgb8::new(0x1b, 0x1b, 0x1b);
pub const COLOR_SURFACE_ELEVATED_LIGHT: Rgb8 = Rgb8::new(0xc9, 0xc9, 0xc9);
pub const COLOR_SURFACE_HOVER_DARK: Rgb8 = Rgb8::new(0x26, 0x26, 0x26);
pub const COLOR_SURFACE_HOVER_LIGHT: Rgb8 = Rgb8::new(0xb4, 0xb4, 0xb4);
pub const COLOR_SURFACE_LIGHT: Rgb8 = Rgb8::new(0xc2, 0xc2, 0xc2);
pub const COLOR_SURFACE_MUTED_DARK: Rgb8 = Rgb8::new(0x15, 0x15, 0x15);
pub const COLOR_SURFACE_MUTED_LIGHT: Rgb8 = Rgb8::new(0xc9, 0xc9, 0xc9);
pub const COLOR_SURFACE_PANEL_DARK: Rgb8 = Rgb8::new(0x19, 0x19, 0x19);
pub const COLOR_SURFACE_PANEL_LIGHT: Rgb8 = Rgb8::new(0xc6, 0xc6, 0xc6);
pub const COLOR_SURFACE_PANEL_RAISED_DARK: Rgb8 = Rgb8::new(0x19, 0x19, 0x19);
pub const COLOR_SURFACE_PANEL_RAISED_LIGHT: Rgb8 = Rgb8::new(0xde, 0xde, 0xde);
pub const COLOR_SURFACE_RAISED_DARK: Rgb8 = Rgb8::new(0x22, 0x22, 0x22);
pub const COLOR_SURFACE_RAISED_LIGHT: Rgb8 = Rgb8::new(0xde, 0xde, 0xde);
pub const COLOR_SURFACE_SELECTED_DARK: Rgb8 = Rgb8::new(0x1f, 0x1f, 0x1f);
pub const COLOR_SURFACE_SELECTED_LIGHT: Rgb8 = Rgb8::new(0xae, 0xae, 0xae);
pub const FONT_FAMILY_SANS: &[&str] =
    &["Inter", "-apple-system", "Segoe UI", "Roboto", "sans-serif"];
pub const FONT_FAMILY_TIME: &[&str] =
    &["Inter", "-apple-system", "Segoe UI", "Roboto", "sans-serif"];
pub const FONT_SIZE_BODY: f32 = 14.0;
pub const FONT_SIZE_CAPTION: f32 = 11.0;
pub const FONT_SIZE_DISPLAY: f32 = 32.0;
pub const FONT_SIZE_LABEL: f32 = 15.0;
pub const FONT_SIZE_MICRO: f32 = 6.0;
pub const FONT_SIZE_SCALE_LABEL: f32 = 12.0;
pub const FONT_SIZE_SECTION: f32 = 15.0;
pub const FONT_SIZE_SIDEBAR: f32 = 13.0;
pub const FONT_SIZE_TIME: f32 = 15.0;
pub const FONT_SIZE_TITLE: f32 = 18.0;
pub const FONT_WEIGHT_BOLD: f32 = 700.0;
pub const FONT_WEIGHT_EXTRABOLD: f32 = 800.0;
pub const FONT_WEIGHT_MEDIUM: f32 = 500.0;
pub const FONT_WEIGHT_REGULAR: f32 = 400.0;
pub const FONT_WEIGHT_SEMIBOLD: f32 = 600.0;
pub const MOTION_DURATION_FAST: f32 = 120.0;
pub const MOTION_DURATION_SLOW: f32 = 480.0;
pub const MOTION_DURATION_STEP: f32 = 280.0;
pub const OPACITY_CHROME_DARK: f32 = 1.0;
pub const OPACITY_CHROME_LIGHT: f32 = 0.75;
pub const OPACITY_DISABLED: f32 = 0.5;
pub const OPACITY_EMPHASIS: f32 = 0.7;
pub const OPACITY_MUTED: f32 = 0.3;
pub const OPACITY_PANEL_DARK: f32 = 0.95;
pub const OPACITY_PANEL_LIGHT: f32 = 0.8;
pub const OPACITY_SCRIM: f32 = 0.2;
pub const OPACITY_SCRIM_STRONG: f32 = 0.24;
pub const OPACITY_SCRIM_SUBTLE: f32 = 0.14;
pub const OPACITY_SWATCH_REST: f32 = 0.7;
pub const RADIUS_APP: f32 = 19.0;
pub const RADIUS_CARD: f32 = 14.0;
pub const RADIUS_CARD_LG: f32 = 30.0;
pub const RADIUS_CONTROL: f32 = 6.0;
pub const RADIUS_HAIRLINE: f32 = 1.0;
pub const RADIUS_INSET: f32 = 4.0;
pub const RADIUS_MENU: f32 = 12.0;
pub const RADIUS_PANEL: f32 = 12.0;
pub const RADIUS_RING: f32 = 21.0;
pub const RADIUS_THUMBNAIL: f32 = 8.0;
pub const RADIUS_WINDOW: f32 = 14.0;
pub const SHADOW_CARD: Shadow = Shadow {
    offset_x: 0.0,
    offset_y: 4.76,
    blur: 4.76,
    spread: 0.0,
    color: Rgb8::new(0x00, 0x00, 0x00),
    alpha: 0.38,
};
pub const SHADOW_OVERLAY: Shadow = Shadow {
    offset_x: 0.0,
    offset_y: 4.76,
    blur: 5.9,
    spread: 0.0,
    color: Rgb8::new(0x00, 0x00, 0x00),
    alpha: 0.7,
};
pub const SPACE_GRID: f32 = 4.0;
pub const SPACE_INSET: f32 = 8.0;
pub const SPACE_ITEM_PITCH: f32 = 48.0;
pub const SPACE_TRACK_INSET: f32 = 2.0;
pub const STROKE_ICON: f32 = 1.5;
pub const STROKE_ICON_MAX: f32 = 3.0;
pub const STROKE_ICON_STRONG: f32 = 2.0;
pub const STROKE_RING: f32 = 4.0;
pub const STROKE_WINDOW: f32 = 20.0;

// -- accent ramps (declared accent order; doc 08 §3.3) ----------------------

pub const ACCENT_ORANGE_PRIMARY: Rgb8 = Rgb8::new(0xd3, 0x79, 0x18);
pub const ACCENT_ORANGE_STRONG: Rgb8 = Rgb8::new(0xab, 0x63, 0x14);
pub const ACCENT_ORANGE_TINTED_DARK: Rgb8 = Rgb8::new(0x37, 0x1f, 0x06);
pub const ACCENT_ORANGE_TINTED_LIGHT: Rgb8 = Rgb8::new(0xd8, 0xba, 0x9a);
pub const ACCENT_ORANGE_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xb3, 0x5f, 0x0d);
pub const ACCENT_ORANGE_BRIGHT_DARK: Rgb8 = Rgb8::new(0xe8, 0x85, 0x1a);
pub const ACCENT_ORANGE_ICON_LIGHT: Rgb8 = Rgb8::new(0xc2, 0x71, 0x1a);
pub const ACCENT_ORANGE_ON_ACCENT: Rgb8 = Rgb8::new(0x09, 0x09, 0x09);
pub const ACCENT_RED_PRIMARY: Rgb8 = Rgb8::new(0xe1, 0x50, 0x32);
pub const ACCENT_RED_STRONG: Rgb8 = Rgb8::new(0xc8, 0x3a, 0x1d);
pub const ACCENT_RED_TINTED_DARK: Rgb8 = Rgb8::new(0x35, 0x10, 0x08);
pub const ACCENT_RED_TINTED_LIGHT: Rgb8 = Rgb8::new(0xd6, 0xa6, 0x9c);
pub const ACCENT_RED_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xe6, 0x60, 0x44);
pub const ACCENT_RED_BRIGHT_DARK: Rgb8 = Rgb8::new(0xe6, 0x60, 0x44);
pub const ACCENT_RED_ICON_LIGHT: Rgb8 = Rgb8::new(0xdb, 0x47, 0x28);
pub const ACCENT_RED_ON_ACCENT: Rgb8 = Rgb8::new(0x16, 0x16, 0x16);
pub const ACCENT_YELLOW_PRIMARY: Rgb8 = Rgb8::new(0xc4, 0x9e, 0x23);
pub const ACCENT_YELLOW_STRONG: Rgb8 = Rgb8::new(0x9d, 0x7f, 0x1c);
pub const ACCENT_YELLOW_TINTED_DARK: Rgb8 = Rgb8::new(0x34, 0x2a, 0x09);
pub const ACCENT_YELLOW_TINTED_LIGHT: Rgb8 = Rgb8::new(0xd4, 0xc7, 0x9e);
pub const ACCENT_YELLOW_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xda, 0xaf, 0x24);
pub const ACCENT_YELLOW_BRIGHT_DARK: Rgb8 = Rgb8::new(0xda, 0xaf, 0x24);
pub const ACCENT_YELLOW_ICON_LIGHT: Rgb8 = Rgb8::new(0xb4, 0x92, 0x24);
pub const ACCENT_YELLOW_ON_ACCENT: Rgb8 = Rgb8::new(0x16, 0x16, 0x16);
pub const ACCENT_GREEN_PRIMARY: Rgb8 = Rgb8::new(0x19, 0xa2, 0x34);
pub const ACCENT_GREEN_STRONG: Rgb8 = Rgb8::new(0x13, 0x7a, 0x27);
pub const ACCENT_GREEN_TINTED_DARK: Rgb8 = Rgb8::new(0x08, 0x35, 0x11);
pub const ACCENT_GREEN_TINTED_LIGHT: Rgb8 = Rgb8::new(0x9c, 0xd6, 0xa7);
pub const ACCENT_GREEN_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0x1a, 0xb8, 0x39);
pub const ACCENT_GREEN_BRIGHT_DARK: Rgb8 = Rgb8::new(0x1a, 0xb8, 0x39);
pub const ACCENT_GREEN_ICON_LIGHT: Rgb8 = Rgb8::new(0x1a, 0x92, 0x31);
pub const ACCENT_GREEN_ON_ACCENT: Rgb8 = Rgb8::new(0x16, 0x16, 0x16);
pub const ACCENT_CYAN_PRIMARY: Rgb8 = Rgb8::new(0x18, 0xaa, 0xd3);
pub const ACCENT_CYAN_STRONG: Rgb8 = Rgb8::new(0x13, 0x89, 0xaa);
pub const ACCENT_CYAN_TINTED_DARK: Rgb8 = Rgb8::new(0x06, 0x2c, 0x37);
pub const ACCENT_CYAN_TINTED_LIGHT: Rgb8 = Rgb8::new(0x9a, 0xca, 0xd8);
pub const ACCENT_CYAN_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0x1a, 0xbb, 0xe8);
pub const ACCENT_CYAN_BRIGHT_DARK: Rgb8 = Rgb8::new(0x1a, 0xbb, 0xe8);
pub const ACCENT_CYAN_ICON_LIGHT: Rgb8 = Rgb8::new(0x1a, 0x9d, 0xc2);
pub const ACCENT_CYAN_ON_ACCENT: Rgb8 = Rgb8::new(0x16, 0x16, 0x16);
pub const ACCENT_BLUE_PRIMARY: Rgb8 = Rgb8::new(0x18, 0x5d, 0xd3);
pub const ACCENT_BLUE_STRONG: Rgb8 = Rgb8::new(0x13, 0x4b, 0xaa);
pub const ACCENT_BLUE_TINTED_DARK: Rgb8 = Rgb8::new(0x06, 0x18, 0x37);
pub const ACCENT_BLUE_TINTED_LIGHT: Rgb8 = Rgb8::new(0x9a, 0xb1, 0xd8);
pub const ACCENT_BLUE_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0x1a, 0x66, 0xe8);
pub const ACCENT_BLUE_BRIGHT_DARK: Rgb8 = Rgb8::new(0x1a, 0x66, 0xe8);
pub const ACCENT_BLUE_ICON_LIGHT: Rgb8 = Rgb8::new(0x1a, 0x58, 0xc2);
pub const ACCENT_BLUE_ON_ACCENT: Rgb8 = Rgb8::new(0xea, 0xea, 0xea);
pub const ACCENT_VIOLET_PRIMARY: Rgb8 = Rgb8::new(0x8e, 0x42, 0xff);
pub const ACCENT_VIOLET_STRONG: Rgb8 = Rgb8::new(0x73, 0x14, 0xff);
pub const ACCENT_VIOLET_TINTED_DARK: Rgb8 = Rgb8::new(0x19, 0x00, 0x3d);
pub const ACCENT_VIOLET_TINTED_LIGHT: Rgb8 = Rgb8::new(0xb1, 0x92, 0xe0);
pub const ACCENT_VIOLET_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0x9c, 0x59, 0xff);
pub const ACCENT_VIOLET_BRIGHT_DARK: Rgb8 = Rgb8::new(0x9c, 0x59, 0xff);
pub const ACCENT_VIOLET_ICON_LIGHT: Rgb8 = Rgb8::new(0x85, 0x36, 0xfc);
pub const ACCENT_VIOLET_ON_ACCENT: Rgb8 = Rgb8::new(0xff, 0xff, 0xff);
pub const ACCENT_PINK_PRIMARY: Rgb8 = Rgb8::new(0xf0, 0x52, 0xb3);
pub const ACCENT_PINK_STRONG: Rgb8 = Rgb8::new(0xec, 0x28, 0xa0);
pub const ACCENT_PINK_TINTED_DARK: Rgb8 = Rgb8::new(0x38, 0x05, 0x24);
pub const ACCENT_PINK_TINTED_LIGHT: Rgb8 = Rgb8::new(0xda, 0x98, 0xc0);
pub const ACCENT_PINK_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xf3, 0x65, 0xbd);
pub const ACCENT_PINK_BRIGHT_DARK: Rgb8 = Rgb8::new(0xf3, 0x65, 0xbd);
pub const ACCENT_PINK_ICON_LIGHT: Rgb8 = Rgb8::new(0xec, 0x47, 0xac);
pub const ACCENT_PINK_ON_ACCENT: Rgb8 = Rgb8::new(0x16, 0x16, 0x16);
pub const ACCENT_GRAY_PRIMARY: Rgb8 = Rgb8::new(0xaf, 0xaf, 0xaf);
pub const ACCENT_GRAY_STRONG: Rgb8 = Rgb8::new(0x98, 0x98, 0x98);
pub const ACCENT_GRAY_TINTED_DARK: Rgb8 = Rgb8::new(0x1e, 0x1e, 0x1e);
pub const ACCENT_GRAY_TINTED_LIGHT: Rgb8 = Rgb8::new(0xb9, 0xb9, 0xb9);
pub const ACCENT_GRAY_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xba, 0xba, 0xba);
pub const ACCENT_GRAY_BRIGHT_DARK: Rgb8 = Rgb8::new(0xba, 0xba, 0xba);
pub const ACCENT_GRAY_ICON_LIGHT: Rgb8 = Rgb8::new(0xa7, 0xa7, 0xa7);
pub const ACCENT_GRAY_ON_ACCENT: Rgb8 = Rgb8::new(0x16, 0x16, 0x16);
