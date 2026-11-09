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
//! Accents (9): yellow (default), orange, red, green, cyan, blue, violet, pink, gray.
//! One ramp per accent (doc 08 §3.3): `ACCENT_{ID}_{ROLE}` with
//! roles PRIMARY, STRONG, TINTED_DARK, TINTED_LIGHT, BRIGHT_LIGHT,
//! ICON_LIGHT, ON_ACCENT.
//!
//! Source shape: 146 primitive / 93 alias / 94 semantic tokens.

use crate::color::{Rgb8, Shadow};

/// Registered modes, in declaration order (doc 08 §3.3).
pub const MODES: &[&str] = &["dark", "light"];
/// The default mode.
pub const DEFAULT_MODE: &str = "dark";
/// Registered accent ids, in `data-accent` order (doc 08 §3.3).
#[rustfmt::skip]
pub const ACCENTS: &[&str] = &[
    "yellow",
    "orange",
    "red",
    "green",
    "cyan",
    "blue",
    "violet",
    "pink",
    "gray",
];
/// The default accent.
pub const DEFAULT_ACCENT: &str = "yellow";

// -- semantic tokens (sorted by constant name) -----------------------------

pub const BLUR_BACKDROP: f32 = 25.0;
pub const BLUR_BACKDROP_SMALL: f32 = 13.75;
pub const COLOR_ACCENT: Rgb8 = Rgb8::new(0xf2, 0xc4, 0x1d);
pub const COLOR_ACCENT_BRIGHT_DARK: Rgb8 = Rgb8::new(0xf2, 0xc4, 0x1d);
pub const COLOR_ACCENT_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xff, 0xd8, 0x4d);
pub const COLOR_ACCENT_ICON_DARK: Rgb8 = Rgb8::new(0xf2, 0xc4, 0x1d);
pub const COLOR_ACCENT_ICON_LIGHT: Rgb8 = Rgb8::new(0xd9, 0xa9, 0x12);
pub const COLOR_ACCENT_STRONG: Rgb8 = Rgb8::new(0xc8, 0x9a, 0x10);
pub const COLOR_ACCENT_TINTED_DARK: Rgb8 = Rgb8::new(0x3b, 0x2f, 0x08);
pub const COLOR_ACCENT_TINTED_LIGHT: Rgb8 = Rgb8::new(0xe8, 0xd3, 0x9a);
pub const COLOR_CONTENT_BRIGHT_DARK: Rgb8 = Rgb8::new(0xf5, 0xee, 0xdc);
pub const COLOR_CONTENT_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0x1f, 0x1b, 0x14);
pub const COLOR_CONTENT_ON_ACCENT: Rgb8 = Rgb8::new(0x1f, 0x1b, 0x14);
pub const COLOR_CONTENT_PRIMARY_DARK: Rgb8 = Rgb8::new(0xcb, 0xbd, 0xa1);
pub const COLOR_CONTENT_PRIMARY_LIGHT: Rgb8 = Rgb8::new(0x4a, 0x3e, 0x2a);
pub const COLOR_CONTENT_SECONDARY_DARK: Rgb8 = Rgb8::new(0xb7, 0xa9, 0x8d);
pub const COLOR_CONTENT_SECONDARY_LIGHT: Rgb8 = Rgb8::new(0x3e, 0x33, 0x20);
pub const COLOR_CONTENT_TERTIARY: Rgb8 = Rgb8::new(0x8c, 0x7c, 0x60);
pub const COLOR_KEYCAP_DARK: Rgb8 = Rgb8::new(0x2a, 0x24, 0x17);
pub const COLOR_KEYCAP_EDGE_DARK: Rgb8 = Rgb8::new(0x4a, 0x3e, 0x28);
pub const COLOR_KEYCAP_EDGE_LIGHT: Rgb8 = Rgb8::new(0xd9, 0xcc, 0xa8);
pub const COLOR_KEYCAP_INK_DARK: Rgb8 = Rgb8::new(0xf2, 0xe9, 0xd5);
pub const COLOR_KEYCAP_INK_LIGHT: Rgb8 = Rgb8::new(0x1f, 0x1b, 0x16);
pub const COLOR_KEYCAP_LIGHT: Rgb8 = Rgb8::new(0xff, 0xfd, 0xf4);
pub const COLOR_KEYCAP_SHADOW_DARK: Rgb8 = Rgb8::new(0x14, 0x10, 0x0a);
pub const COLOR_KEYCAP_SHADOW_LIGHT: Rgb8 = Rgb8::new(0xc9, 0xbb, 0x98);
pub const COLOR_NOTE_DARK: Rgb8 = Rgb8::new(0x26, 0x20, 0x19);
pub const COLOR_NOTE_EDGE_DARK: Rgb8 = Rgb8::new(0x3b, 0x31, 0x22);
pub const COLOR_NOTE_EDGE_LIGHT: Rgb8 = Rgb8::new(0xe0, 0xd2, 0xb0);
pub const COLOR_NOTE_INK_DARK: Rgb8 = Rgb8::new(0xf2, 0xe9, 0xd5);
pub const COLOR_NOTE_INK_LIGHT: Rgb8 = Rgb8::new(0x1f, 0x1b, 0x16);
pub const COLOR_NOTE_INK_MUTED_DARK: Rgb8 = Rgb8::new(0xb9, 0xab, 0x8f);
pub const COLOR_NOTE_INK_MUTED_LIGHT: Rgb8 = Rgb8::new(0x6e, 0x62, 0x50);
pub const COLOR_NOTE_LIGHT: Rgb8 = Rgb8::new(0xef, 0xe6, 0xd2);
pub const COLOR_RECORD: Rgb8 = Rgb8::new(0xff, 0x00, 0x00);
pub const COLOR_STROKE_BORDER_DARK: Rgb8 = Rgb8::new(0x28, 0x20, 0x13);
pub const COLOR_STROKE_BORDER_LIGHT: Rgb8 = Rgb8::new(0xef, 0xe3, 0xc6);
pub const COLOR_STROKE_ICON_DARK: Rgb8 = Rgb8::new(0xcb, 0xbd, 0xa1);
pub const COLOR_STROKE_ICON_LIGHT: Rgb8 = Rgb8::new(0x4a, 0x3e, 0x2a);
pub const COLOR_STROKE_WARM: Rgb8 = Rgb8::new(0x1a, 0x16, 0x11);
pub const COLOR_SUNSET_1_DARK: Rgb8 = Rgb8::new(0x19, 0x16, 0x31);
pub const COLOR_SUNSET_1_LIGHT: Rgb8 = Rgb8::new(0xfb, 0xf3, 0xe2);
pub const COLOR_SUNSET_2_DARK: Rgb8 = Rgb8::new(0x2e, 0x3e, 0x80);
pub const COLOR_SUNSET_2_LIGHT: Rgb8 = Rgb8::new(0xf9, 0xd9, 0xa0);
pub const COLOR_SUNSET_3_DARK: Rgb8 = Rgb8::new(0x6c, 0x69, 0xba);
pub const COLOR_SUNSET_3_LIGHT: Rgb8 = Rgb8::new(0xf4, 0xb1, 0x83);
pub const COLOR_SUNSET_4_DARK: Rgb8 = Rgb8::new(0xa4, 0x79, 0xad);
pub const COLOR_SUNSET_4_LIGHT: Rgb8 = Rgb8::new(0xe5, 0x8f, 0x87);
pub const COLOR_SUNSET_5_DARK: Rgb8 = Rgb8::new(0xe6, 0x83, 0x73);
pub const COLOR_SUNSET_5_LIGHT: Rgb8 = Rgb8::new(0xc8, 0x73, 0x9e);
pub const COLOR_SUNSET_6_DARK: Rgb8 = Rgb8::new(0xef, 0x9b, 0x4c);
pub const COLOR_SUNSET_6_LIGHT: Rgb8 = Rgb8::new(0x8e, 0x6f, 0xb5);
pub const COLOR_SUNSET_7_DARK: Rgb8 = Rgb8::new(0x7a, 0x3b, 0x1e);
pub const COLOR_SUNSET_7_LIGHT: Rgb8 = Rgb8::new(0x5d, 0x5a, 0x9c);
pub const COLOR_SURFACE_CHROME_DARK: Rgb8 = Rgb8::new(0x28, 0x20, 0x13);
pub const COLOR_SURFACE_CHROME_LIGHT: Rgb8 = Rgb8::new(0xef, 0xe3, 0xc6);
pub const COLOR_SURFACE_DARK: Rgb8 = Rgb8::new(0x24, 0x1d, 0x13);
pub const COLOR_SURFACE_DEEP_DARK: Rgb8 = Rgb8::new(0x12, 0x10, 0x0b);
pub const COLOR_SURFACE_DEEP_LIGHT: Rgb8 = Rgb8::new(0xf6, 0xef, 0xdd);
pub const COLOR_SURFACE_HOVER_DARK: Rgb8 = Rgb8::new(0x2f, 0x26, 0x17);
pub const COLOR_SURFACE_HOVER_LIGHT: Rgb8 = Rgb8::new(0xe5, 0xd8, 0xb8);
pub const COLOR_SURFACE_LIGHT: Rgb8 = Rgb8::new(0xef, 0xe3, 0xc6);
pub const COLOR_SURFACE_MUTED_DARK: Rgb8 = Rgb8::new(0x5f, 0x50, 0x36);
pub const COLOR_SURFACE_MUTED_LIGHT: Rgb8 = Rgb8::new(0xb3, 0xa7, 0x8e);
pub const COLOR_SURFACE_PANEL_DARK: Rgb8 = Rgb8::new(0x1a, 0x15, 0x0e);
pub const COLOR_SURFACE_PANEL_LIGHT: Rgb8 = Rgb8::new(0xf8, 0xf1, 0xe0);
pub const COLOR_SURFACE_RAISED_DARK: Rgb8 = Rgb8::new(0x3e, 0x33, 0x20);
pub const COLOR_SURFACE_RAISED_LIGHT: Rgb8 = Rgb8::new(0xc6, 0xb6, 0x94);
pub const COLOR_SURFACE_SELECTED_DARK: Rgb8 = Rgb8::new(0x38, 0x2d, 0x1b);
pub const COLOR_SURFACE_SELECTED_LIGHT: Rgb8 = Rgb8::new(0xcf, 0xc1, 0xa4);
pub const COLOR_TAPE: Rgb8 = Rgb8::new(0xdf, 0xc2, 0x7f);
pub const COLOR_TAPE_EDGE: Rgb8 = Rgb8::new(0xc6, 0xa9, 0x68);
pub const FONT_FAMILY_SANS: &[&str] = &["Nunito", "Inter", "system-ui", "sans-serif"];
pub const FONT_FAMILY_TIME: &[&str] = &[
    "ui-monospace",
    "SFMono-Regular",
    "Cascadia Mono",
    "Menlo",
    "Consolas",
    "monospace",
];
pub const FONT_SIZE_BODY: f32 = 14.0;
pub const FONT_SIZE_CAPTION: f32 = 11.0;
pub const FONT_SIZE_DISPLAY: f32 = 32.0;
pub const FONT_SIZE_LABEL: f32 = 16.0;
pub const FONT_SIZE_MICRO: f32 = 6.0;
pub const FONT_SIZE_TIME: f32 = 15.0;
pub const FONT_SIZE_TITLE: f32 = 18.0;
pub const FONT_WEIGHT_BOLD: f32 = 700.0;
pub const FONT_WEIGHT_EXTRABOLD: f32 = 800.0;
pub const FONT_WEIGHT_MEDIUM: f32 = 500.0;
pub const FONT_WEIGHT_REGULAR: f32 = 400.0;
pub const FONT_WEIGHT_SEMIBOLD: f32 = 600.0;
pub const GRADIENT_VISUALIZER_1: Rgb8 = Rgb8::new(0xff, 0x5e, 0x5b);
pub const GRADIENT_VISUALIZER_2: Rgb8 = Rgb8::new(0xf2, 0xc4, 0x1d);
pub const GRADIENT_VISUALIZER_3: Rgb8 = Rgb8::new(0xff, 0x9e, 0x4a);
pub const GRADIENT_VISUALIZER_4: Rgb8 = Rgb8::new(0x6c, 0x69, 0xba);
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
pub const RADIUS_CARD: f32 = 16.0;
pub const RADIUS_CARD_LG: f32 = 30.0;
pub const RADIUS_CONTROL: f32 = 10.0;
pub const RADIUS_HAIRLINE: f32 = 1.0;
pub const RADIUS_INSET: f32 = 6.0;
pub const RADIUS_MENU: f32 = 12.0;
pub const RADIUS_PANEL: f32 = 12.0;
pub const RADIUS_RING: f32 = 21.0;
pub const RADIUS_THUMBNAIL: f32 = 8.0;
pub const RADIUS_WINDOW: f32 = 16.0;
pub const SHADOW_CARD: Shadow = Shadow {
    offset_x: 0.0,
    offset_y: 5.0,
    blur: 16.0,
    spread: 0.0,
    color: Rgb8::new(0x14, 0x0d, 0x05),
    alpha: 0.38,
};
pub const SHADOW_NOTE: Shadow = Shadow {
    offset_x: 0.0,
    offset_y: 14.0,
    blur: 36.0,
    spread: 0.0,
    color: Rgb8::new(0x0c, 0x08, 0x03),
    alpha: 0.5,
};
pub const SHADOW_OVERLAY: Shadow = Shadow {
    offset_x: 0.0,
    offset_y: 12.0,
    blur: 36.0,
    spread: 0.0,
    color: Rgb8::new(0x0c, 0x08, 0x03),
    alpha: 0.6,
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

pub const ACCENT_YELLOW_PRIMARY: Rgb8 = Rgb8::new(0xf2, 0xc4, 0x1d);
pub const ACCENT_YELLOW_STRONG: Rgb8 = Rgb8::new(0xc8, 0x9a, 0x10);
pub const ACCENT_YELLOW_TINTED_DARK: Rgb8 = Rgb8::new(0x3b, 0x2f, 0x08);
pub const ACCENT_YELLOW_TINTED_LIGHT: Rgb8 = Rgb8::new(0xe8, 0xd3, 0x9a);
pub const ACCENT_YELLOW_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xff, 0xd8, 0x4d);
pub const ACCENT_YELLOW_ICON_LIGHT: Rgb8 = Rgb8::new(0xd9, 0xa9, 0x12);
pub const ACCENT_YELLOW_ON_ACCENT: Rgb8 = Rgb8::new(0x1f, 0x1b, 0x14);
pub const ACCENT_ORANGE_PRIMARY: Rgb8 = Rgb8::new(0xd3, 0x79, 0x18);
pub const ACCENT_ORANGE_STRONG: Rgb8 = Rgb8::new(0xaa, 0x61, 0x13);
pub const ACCENT_ORANGE_TINTED_DARK: Rgb8 = Rgb8::new(0x37, 0x1f, 0x06);
pub const ACCENT_ORANGE_TINTED_LIGHT: Rgb8 = Rgb8::new(0xd8, 0xba, 0x9a);
pub const ACCENT_ORANGE_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xe8, 0x85, 0x1a);
pub const ACCENT_ORANGE_ICON_LIGHT: Rgb8 = Rgb8::new(0xc2, 0x71, 0x1a);
pub const ACCENT_ORANGE_ON_ACCENT: Rgb8 = Rgb8::new(0x1f, 0x1b, 0x14);
pub const ACCENT_RED_PRIMARY: Rgb8 = Rgb8::new(0xe1, 0x50, 0x32);
pub const ACCENT_RED_STRONG: Rgb8 = Rgb8::new(0xc8, 0x3a, 0x1d);
pub const ACCENT_RED_TINTED_DARK: Rgb8 = Rgb8::new(0x35, 0x10, 0x08);
pub const ACCENT_RED_TINTED_LIGHT: Rgb8 = Rgb8::new(0xd6, 0xa6, 0x9c);
pub const ACCENT_RED_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xe6, 0x60, 0x44);
pub const ACCENT_RED_ICON_LIGHT: Rgb8 = Rgb8::new(0xdb, 0x47, 0x28);
pub const ACCENT_RED_ON_ACCENT: Rgb8 = Rgb8::new(0x12, 0x10, 0x0b);
pub const ACCENT_GREEN_PRIMARY: Rgb8 = Rgb8::new(0x19, 0xa2, 0x34);
pub const ACCENT_GREEN_STRONG: Rgb8 = Rgb8::new(0x13, 0x7a, 0x27);
pub const ACCENT_GREEN_TINTED_DARK: Rgb8 = Rgb8::new(0x08, 0x35, 0x11);
pub const ACCENT_GREEN_TINTED_LIGHT: Rgb8 = Rgb8::new(0x9c, 0xd6, 0xa7);
pub const ACCENT_GREEN_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0x1a, 0xb8, 0x39);
pub const ACCENT_GREEN_ICON_LIGHT: Rgb8 = Rgb8::new(0x1a, 0x92, 0x31);
pub const ACCENT_GREEN_ON_ACCENT: Rgb8 = Rgb8::new(0x1f, 0x1b, 0x14);
pub const ACCENT_CYAN_PRIMARY: Rgb8 = Rgb8::new(0x18, 0xaa, 0xd3);
pub const ACCENT_CYAN_STRONG: Rgb8 = Rgb8::new(0x13, 0x89, 0xaa);
pub const ACCENT_CYAN_TINTED_DARK: Rgb8 = Rgb8::new(0x06, 0x2c, 0x37);
pub const ACCENT_CYAN_TINTED_LIGHT: Rgb8 = Rgb8::new(0x9a, 0xca, 0xd8);
pub const ACCENT_CYAN_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0x1a, 0xbb, 0xe8);
pub const ACCENT_CYAN_ICON_LIGHT: Rgb8 = Rgb8::new(0x1a, 0x9d, 0xc2);
pub const ACCENT_CYAN_ON_ACCENT: Rgb8 = Rgb8::new(0x1f, 0x1b, 0x14);
pub const ACCENT_BLUE_PRIMARY: Rgb8 = Rgb8::new(0x18, 0x5d, 0xd3);
pub const ACCENT_BLUE_STRONG: Rgb8 = Rgb8::new(0x13, 0x4b, 0xaa);
pub const ACCENT_BLUE_TINTED_DARK: Rgb8 = Rgb8::new(0x06, 0x18, 0x37);
pub const ACCENT_BLUE_TINTED_LIGHT: Rgb8 = Rgb8::new(0x9a, 0xb1, 0xd8);
pub const ACCENT_BLUE_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0x1a, 0x66, 0xe8);
pub const ACCENT_BLUE_ICON_LIGHT: Rgb8 = Rgb8::new(0x1a, 0x58, 0xc2);
pub const ACCENT_BLUE_ON_ACCENT: Rgb8 = Rgb8::new(0xf5, 0xee, 0xdc);
pub const ACCENT_VIOLET_PRIMARY: Rgb8 = Rgb8::new(0x8e, 0x42, 0xff);
pub const ACCENT_VIOLET_STRONG: Rgb8 = Rgb8::new(0x73, 0x14, 0xff);
pub const ACCENT_VIOLET_TINTED_DARK: Rgb8 = Rgb8::new(0x19, 0x00, 0x3d);
pub const ACCENT_VIOLET_TINTED_LIGHT: Rgb8 = Rgb8::new(0xb1, 0x92, 0xe0);
pub const ACCENT_VIOLET_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0x9c, 0x59, 0xff);
pub const ACCENT_VIOLET_ICON_LIGHT: Rgb8 = Rgb8::new(0x85, 0x36, 0xfc);
pub const ACCENT_VIOLET_ON_ACCENT: Rgb8 = Rgb8::new(0xff, 0xff, 0xff);
pub const ACCENT_PINK_PRIMARY: Rgb8 = Rgb8::new(0xf0, 0x52, 0xb3);
pub const ACCENT_PINK_STRONG: Rgb8 = Rgb8::new(0xec, 0x28, 0xa0);
pub const ACCENT_PINK_TINTED_DARK: Rgb8 = Rgb8::new(0x38, 0x05, 0x24);
pub const ACCENT_PINK_TINTED_LIGHT: Rgb8 = Rgb8::new(0xda, 0x98, 0xc0);
pub const ACCENT_PINK_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xf3, 0x65, 0xbd);
pub const ACCENT_PINK_ICON_LIGHT: Rgb8 = Rgb8::new(0xec, 0x47, 0xac);
pub const ACCENT_PINK_ON_ACCENT: Rgb8 = Rgb8::new(0x1f, 0x1b, 0x14);
pub const ACCENT_GRAY_PRIMARY: Rgb8 = Rgb8::new(0xa9, 0x9c, 0x82);
pub const ACCENT_GRAY_STRONG: Rgb8 = Rgb8::new(0x96, 0x86, 0x67);
pub const ACCENT_GRAY_TINTED_DARK: Rgb8 = Rgb8::new(0x24, 0x20, 0x19);
pub const ACCENT_GRAY_TINTED_LIGHT: Rgb8 = Rgb8::new(0xc0, 0xbb, 0xb2);
pub const ACCENT_GRAY_BRIGHT_LIGHT: Rgb8 = Rgb8::new(0xb4, 0xa7, 0x8e);
pub const ACCENT_GRAY_ICON_LIGHT: Rgb8 = Rgb8::new(0x9f, 0x94, 0x7c);
pub const ACCENT_GRAY_ON_ACCENT: Rgb8 = Rgb8::new(0x1f, 0x1b, 0x14);
