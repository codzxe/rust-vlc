//! Deterministic emitters for the resolved token set (card 1.B4; doc 01 §2.3
//! interface sketch, doc 08 §3.1 generation targets).
//!
//! * [`emit_css`] renders `themes/generated/tokens.css` — semantic-layer CSS
//!   custom properties. Mode-independent values, the default mode and the
//!   default accent live in `:root`; other modes in `:root[data-theme]`
//!   blocks; the eight alternate accents in `:root[data-accent]` blocks
//!   (plus their per-mode variants). See doc 08 §3.3.
//! * [`emit_rust`] renders `crates/theme-tokens/src/generated/tokens.rs` —
//!   semantic-layer Rust constants for engine-side needs (doc 01 §2.3),
//!   plus mode/accent metadata and the nine accent ramps.
//!
//! Determinism contract (doc 08 §3.4 byte-stability): declarations are
//! sorted by name within every block, numbers use a fixed shortest form,
//! and nothing depends on map iteration order — the same [`TokenSet`]
//! always renders byte-identical output.

use std::collections::HashSet;
use std::io::Write;

use crate::TokenError;
use crate::color::AccentRamp;
use crate::model::{ModeValues, ResolvedValue, TokenSet};

/// Role order of [`TokenSet::accent_family_css`], matching the
/// `accent_alias_roles` table in `parse.rs`:
/// `color.accent`, `color.accent.bright`, `color.accent.icon`,
/// `color.accent.strong`, `color.accent.tinted`, `color.content.on-accent`.
const ROLE_ACCENT: usize = 0;
const ROLE_BRIGHT: usize = 1;
const ROLE_ICON: usize = 2;
const ROLE_STRONG: usize = 3;
const ROLE_TINTED: usize = 4;
const ROLE_ON_ACCENT: usize = 5;

/// CSS generic family keywords — the only family names emitted unquoted
/// (CSS Namespaces: a quoted `"sans-serif"` would be a literal family
/// name, not the generic).
const CSS_GENERIC_FAMILIES: &[&str] = &[
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-serif",
    "ui-sans-serif",
    "ui-monospace",
    "ui-rounded",
];

/// rustfmt's default line width — single-line literals are only used when
/// the whole line fits, mirroring what `cargo fmt` keeps stable.
const LINE_WIDTH: usize = 100;

// ---------------------------------------------------------------------------
// Public API (doc 01 §2.3)
// ---------------------------------------------------------------------------

/// Emit the CSS custom-property sheet for `tokens` (semantic layer only,
/// doc 08 §3.2).
pub fn emit_css(tokens: &TokenSet, out: &mut impl Write) -> Result<(), TokenError> {
    let document = css_document(tokens)?;
    out.write_all(document.as_bytes())
        .map_err(|e| TokenError::io("writing CSS output", e))
}

/// Emit the Rust constant module for `tokens` (semantic layer + accent
/// ramps + mode/accent metadata).
pub fn emit_rust(tokens: &TokenSet, out: &mut impl Write) -> Result<(), TokenError> {
    let document = rust_document(tokens)?;
    out.write_all(document.as_bytes())
        .map_err(|e| TokenError::io("writing Rust output", e))
}

// ---------------------------------------------------------------------------
// CSS emission
// ---------------------------------------------------------------------------

/// One CSS declaration (`--name: value;`).
type Decl = (String, String);

fn css_document(tokens: &TokenSet) -> Result<String, TokenError> {
    validate_mode_pair(tokens)?;
    let default_mode = tokens.default_mode().to_owned();
    let family = tokens.accent_family_css();
    let family_set: HashSet<&str> = family.iter().map(String::as_str).collect();

    let mut root: Vec<Decl> = Vec::new();
    let mut mode_blocks: Vec<(String, Vec<Decl>)> = tokens
        .modes()
        .iter()
        .filter(|m| m.as_str() != default_mode)
        .map(|m| (m.clone(), Vec::new()))
        .collect();

    // Non-accent semantic tokens: modeless + default-mode values into :root,
    // every other mode into its [data-theme] block.
    for token in tokens.semantic() {
        if family_set.contains(token.css_name.as_str()) {
            continue; // the accent family is emitted from the ramps below
        }
        match &token.values {
            ModeValues::Single(value) => {
                root.push((token.css_name.clone(), css_value(value)?));
            }
            ModeValues::PerMode(pairs) => {
                for (mode, value) in pairs {
                    let decl = (token.css_name.clone(), css_value(value)?);
                    if mode == &default_mode {
                        root.push(decl);
                    } else if let Some(block) = mode_blocks.iter_mut().find(|(m, _)| m == mode) {
                        block.1.push(decl);
                    } else {
                        return Err(TokenError::internal(format!(
                            "per-mode value for unregistered mode `{mode}` on `{}`",
                            token.css_name
                        )));
                    }
                }
            }
        }
    }

    // Accent family: the default accent is inlined into :root and the mode
    // blocks; the alternates get their own [data-accent] blocks below.
    let default_ramp = ramp_for(tokens, tokens.default_accent())?;
    for (role, css_name) in family.iter().enumerate() {
        root.push((
            css_name.clone(),
            ramp_color(default_ramp, role, &default_mode)?.to_hex(),
        ));
    }
    for (mode, block) in &mut mode_blocks {
        for (role, css_name) in family.iter().enumerate() {
            if role_is_per_mode(tokens, css_name) {
                block.push((
                    css_name.clone(),
                    ramp_color(default_ramp, role, mode)?.to_hex(),
                ));
            }
        }
    }

    let mut document = css_header(tokens);
    document.push_str(&render_block(":root", &mut root)?);
    for (mode, block) in &mut mode_blocks {
        document.push_str(&render_block(
            &format!(":root[data-theme=\"{mode}\"]"),
            block,
        )?);
    }
    for accent in tokens.accents() {
        if accent == tokens.default_accent() {
            continue; // default accent values already live in :root/mode blocks
        }
        let ramp = ramp_for(tokens, accent)?;
        let mut dark: Vec<Decl> = family
            .iter()
            .enumerate()
            .map(|(role, css_name)| {
                Ok((
                    css_name.clone(),
                    ramp_color(ramp, role, &default_mode)?.to_hex(),
                ))
            })
            .collect::<Result<_, TokenError>>()?;
        document.push_str(&render_block(
            &format!(":root[data-accent=\"{accent}\"]"),
            &mut dark,
        )?);
        for (mode, _) in &mode_blocks {
            let mut values: Vec<Decl> = family
                .iter()
                .enumerate()
                .filter(|(_, css_name)| role_is_per_mode(tokens, css_name))
                .map(|(role, css_name)| {
                    Ok((css_name.clone(), ramp_color(ramp, role, mode)?.to_hex()))
                })
                .collect::<Result<_, TokenError>>()?;
            if values.is_empty() {
                continue;
            }
            document.push_str(&render_block(
                &format!(":root[data-theme=\"{mode}\"][data-accent=\"{accent}\"]"),
                &mut values,
            )?);
        }
    }
    // Every block ends "}\n\n"; keep exactly one final newline.
    document.pop();
    Ok(document)
}

/// Render one selector block, sorting and duplicate-checking `decls`.
fn render_block(selector: &str, decls: &mut [Decl]) -> Result<String, TokenError> {
    decls.sort_by(|a, b| a.0.cmp(&b.0));
    for pair in decls.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(TokenError::emit(format!(
                "duplicate CSS custom property `{}` in block `{selector}`",
                pair[0].0
            )));
        }
    }
    let mut out = format!("{selector} {{\n");
    for (name, value) in &*decls {
        out.push_str(&format!("  {name}: {value};\n"));
    }
    out.push_str("}\n\n");
    Ok(out)
}

/// The accent-family color for `role` under `mode` (doc 08 §3.3): dark-mode
/// bright/icon equal the primary; light-mode values come from the ramp.
fn ramp_color(
    ramp: &AccentRamp,
    role: usize,
    mode: &str,
) -> Result<crate::color::Rgb8, TokenError> {
    let color = match (role, mode) {
        (ROLE_ACCENT, _) => ramp.primary,
        (ROLE_STRONG, _) => ramp.strong,
        (ROLE_ON_ACCENT, _) => ramp.on_accent,
        (ROLE_ICON, "dark") => ramp.primary,
        (ROLE_BRIGHT, "dark") => ramp.bright_dark,
        (ROLE_BRIGHT, "light") => ramp.bright_light,
        (ROLE_ICON, "light") => ramp.icon_light,
        (ROLE_TINTED, "dark") => ramp.tinted_dark,
        (ROLE_TINTED, "light") => ramp.tinted_light,
        (role, mode) => {
            return Err(TokenError::internal(format!(
                "no ramp value for accent role {role} in mode `{mode}`"
            )));
        }
    };
    Ok(color)
}

fn ramp_for<'a>(tokens: &'a TokenSet, accent: &str) -> Result<&'a AccentRamp, TokenError> {
    tokens
        .ramp_for(accent)
        .ok_or_else(|| TokenError::internal(format!("accent `{accent}` has no ramp (doc 08 §3.3)")))
}

/// Whether the semantic token behind `css_name` carries per-mode values
/// (drives which accent-family names appear in `[data-theme]` blocks).
fn role_is_per_mode(tokens: &TokenSet, css_name: &str) -> bool {
    tokens
        .semantic_by_css_name(css_name)
        .map(|t| t.values.is_per_mode())
        .unwrap_or(false)
}

/// The accent-ramp model (doc 08 §3.3) is defined for exactly the
/// `dark`/`light` mode pair.
fn validate_mode_pair(tokens: &TokenSet) -> Result<(), TokenError> {
    let modes = tokens.modes();
    let has = |name: &str| modes.iter().any(|m| m == name);
    if modes.len() == 2 && has("dark") && has("light") {
        Ok(())
    } else {
        Err(TokenError::emit(format!(
            "the accent-ramp model assumes exactly the `dark`/`light` mode pair \
             (doc 08 §3.3); registered modes: [{}]",
            modes.join(", ")
        )))
    }
}

/// Format one value as a CSS token value.
fn css_value(value: &ResolvedValue) -> Result<String, TokenError> {
    match value {
        ResolvedValue::Color(c) => Ok(c.to_hex()),
        ResolvedValue::Dimension { value, unit } => {
            Ok(format!("{}{}", css_num(*value), unit.as_css()))
        }
        ResolvedValue::Number(n) => Ok(css_num(*n)),
        ResolvedValue::Shadow(s) => Ok(format!(
            "{}px {}px {}px {}px {}",
            css_num32(s.offset_x),
            css_num32(s.offset_y),
            css_num32(s.blur),
            css_num32(s.spread),
            css_shadow_color(s.color, s.alpha),
        )),
        ResolvedValue::Family(names) => {
            if names.is_empty() {
                return Err(TokenError::emit("fontFamily stack must not be empty"));
            }
            Ok(names
                .iter()
                .map(|n| css_family(n))
                .collect::<Vec<_>>()
                .join(", "))
        }
    }
}

/// CSS number form: integral values without a fraction (`25`), everything
/// else in shortest round-trip form (`4.76`).
fn css_num(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1.0e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// [`css_num`] for `f32` fields (shadows carry `f32` — formatting the `f32`
/// directly keeps `0.38` short instead of expanding its `f64` image).
fn css_num32(v: f32) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1.0e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn css_shadow_color(color: crate::color::Rgb8, alpha: f32) -> String {
    if alpha >= 1.0 {
        color.to_hex()
    } else {
        format!(
            "rgba({}, {}, {}, {})",
            color.r,
            color.g,
            color.b,
            css_num32(alpha)
        )
    }
}

fn css_family(name: &str) -> String {
    if CSS_GENERIC_FAMILIES.contains(&name) {
        name.to_owned()
    } else {
        format!("\"{name}\"")
    }
}

/// `name (default)` for the default entry, plain `name` otherwise.
fn list_with_default<'a>(entries: &'a [String], default: &str) -> Vec<std::borrow::Cow<'a, str>> {
    entries
        .iter()
        .map(|e| {
            if e == default {
                std::borrow::Cow::Owned(format!("{e} (default)"))
            } else {
                std::borrow::Cow::Borrowed(e.as_str())
            }
        })
        .collect()
}

fn css_header(tokens: &TokenSet) -> String {
    let counts = tokens.counts();
    let modes = list_with_default(tokens.modes(), tokens.default_mode());
    let accents = list_with_default(tokens.accents(), tokens.default_accent());
    let mut lines: Vec<String> = vec![
        "/* GENERATED FILE — DO NOT EDIT.".to_owned(),
        " *".to_owned(),
        " * Produced by `cargo xtask tokens` (crates/theme-tokens `tokens-gen`,".to_owned(),
        " * card 1.B4) from themes/tokens.json — doc 08 §3. Regeneration is".to_owned(),
        " * idempotent; the committed copy is diff-tested by".to_owned(),
        " * crates/theme-tokens/tests/diff.rs.".to_owned(),
        " *".to_owned(),
        " * Components consume SEMANTIC tokens only, via `var(--<name>)`".to_owned(),
        " * (doc 08 §3.2). The primitive and alias layers are never emitted".to_owned(),
        " * here; they exist for re-derivation from the Figma variables.".to_owned(),
        " *".to_owned(),
        " * Block layout (doc 08 §3.3 — attributes on the root element):".to_owned(),
        " *   :root                                 mode-independent values,".to_owned(),
        " *                                        the default mode ({dm}) and".to_owned(),
        " *                                        the default accent ({da})".to_owned(),
        " *   :root[data-theme=\"<mode>\"]            per-mode values".to_owned(),
        " *   :root[data-accent=\"<id>\"]             alternate-accent values,".to_owned(),
        " *                                        default mode".to_owned(),
        " *   :root[data-theme=\"<mode>\"][data-accent=\"<id>\"]".to_owned(),
        " *                                        ...and their per-mode values".to_owned(),
        " *".to_owned(),
        format!(" * Modes ({}): {}.", tokens.modes().len(), modes.join(", ")),
        format!(
            " * Accents ({}): {}.",
            tokens.accents().len(),
            accents.join(", ")
        ),
        format!(
            " * Tokens: {} primitive / {} alias / {} semantic (source:",
            counts.primitives, counts.aliases, counts.semantic
        ),
        " * themes/tokens.json).".to_owned(),
        " */".to_owned(),
    ];
    let default_mode = tokens.default_mode().to_owned();
    let default_accent = tokens.default_accent().to_owned();
    for line in &mut lines {
        *line = line
            .replacen("{dm}", &default_mode, 1)
            .replacen("{da}", &default_accent, 1);
    }
    lines.push(String::new());
    lines.push(String::new());
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Rust emission
// ---------------------------------------------------------------------------

/// One generated constant: name, type, expression.
struct Const {
    name: String,
    ty: &'static str,
    expr: String,
}

fn rust_document(tokens: &TokenSet) -> Result<String, TokenError> {
    validate_mode_pair(tokens)?;
    // Semantic and ramp constants are collected in SEPARATE vectors (j18 F2):
    // the previous single vector split them by `len() - ramps * 7`, but each
    // ramp emits EIGHT constants, so nine ramp constants leaked into the
    // "sorted by constant name" semantic section. Splitting at the source
    // removes the magic count entirely — the section boundary is structural,
    // not arithmetic.
    let mut semantic_consts: Vec<Const> = Vec::new();
    let mut ramp_consts: Vec<Const> = Vec::new();

    for token in tokens.semantic() {
        match &token.values {
            ModeValues::Single(value) => {
                let (ty, expr) = rust_const(&token.rust_name, value)?;
                semantic_consts.push(Const {
                    name: token.rust_name.clone(),
                    ty,
                    expr,
                });
            }
            ModeValues::PerMode(pairs) => {
                for (mode, value) in pairs {
                    let name = format!("{}_{}", token.rust_name, screaming(mode));
                    let (ty, expr) = rust_const(&name, value)?;
                    semantic_consts.push(Const { name, ty, expr });
                }
            }
        }
    }
    for ramp in tokens.accent_ramps() {
        let id = screaming(&ramp.id);
        // The eight ramp roles, in emission order. Keep this table in sync
        // with the role list in `rust_header` (both list the same eight).
        for (suffix, color) in [
            ("PRIMARY", ramp.primary),
            ("STRONG", ramp.strong),
            ("TINTED_DARK", ramp.tinted_dark),
            ("TINTED_LIGHT", ramp.tinted_light),
            ("BRIGHT_LIGHT", ramp.bright_light),
            ("BRIGHT_DARK", ramp.bright_dark),
            ("ICON_LIGHT", ramp.icon_light),
            ("ON_ACCENT", ramp.on_accent),
        ] {
            ramp_consts.push(Const {
                name: format!("ACCENT_{id}_{suffix}"),
                ty: "Rgb8",
                expr: rgb_expr(color),
            });
        }
    }

    // Duplicate constant names would silently shadow in the generated
    // module — reject them explicitly.
    let mut names: Vec<&str> = semantic_consts
        .iter()
        .chain(ramp_consts.iter())
        .map(|c| c.name.as_str())
        .collect();
    names.sort_unstable();
    for pair in names.windows(2) {
        if pair[0] == pair[1] {
            return Err(TokenError::emit(format!(
                "duplicate Rust constant name `{}` in the generated module",
                pair[0]
            )));
        }
    }
    // Semantic constants sorted by name; ramp constants stay in declared
    // accent order (the `data-accent` order matters to consumers).
    semantic_consts.sort_by(|a, b| a.name.cmp(&b.name));

    let mut document = rust_header(tokens);
    document.push_str("use crate::color::{Rgb8, Shadow};\n\n");
    document.push_str(&format!(
        "/// Registered modes, in declaration order (doc 08 §3.3).\npub const MODES: &[&str] = &{};\n",
        rust_array(&tokens.modes().iter().map(|m| rust_str_lit(m)).collect::<Vec<_>>(), "pub const MODES: &[&str] = &".len())
    ));
    document.push_str(&format!(
        "/// The default mode.\npub const DEFAULT_MODE: &str = \"{}\";\n",
        tokens.default_mode()
    ));
    document.push_str(&format!(
        "/// Registered accent ids, in `data-accent` order (doc 08 §3.3).\n#[rustfmt::skip]\npub const ACCENTS: &[&str] = &{};\n",
        rust_array(&tokens.accents().iter().map(|a| rust_str_lit(a)).collect::<Vec<_>>(), "pub const ACCENTS: &[&str] = &".len())
    ));
    document.push_str(&format!(
        "/// The default accent.\npub const DEFAULT_ACCENT: &str = \"{}\";\n",
        tokens.default_accent()
    ));
    document.push_str(
        "\n// -- semantic tokens (sorted by constant name) -----------------------------\n\n",
    );
    for c in &semantic_consts {
        document.push_str(&const_declaration(c));
    }
    document.push_str(
        "\n// -- accent ramps (declared accent order; doc 08 §3.3) ----------------------\n\n",
    );
    for c in &ramp_consts {
        document.push_str(&const_declaration(c));
    }
    Ok(document)
}

/// One `pub const` declaration, emitted in exactly the form rustfmt
/// keeps stable (the 2026-11-13 fmt drift, CI run 43373340074):
///   1. the whole line fits → single line;
///   2. it doesn't, but `    <expr>;` fits on a continuation line →
///      break after `=` (rustfmt's choice for FONT_FAMILY_*: the array
///      stays inline on the indented rhs);
///   3. neither → the expression itself must already carry its own
///      multi-line layout (Family/Shadow arms in `rust_const`), so the
///      break-after-`=` form is still the right shape.
fn const_declaration(c: &Const) -> String {
    let single = format!("pub const {}: {} = {};", c.name, c.ty, c.expr);
    if single.len() <= LINE_WIDTH {
        return format!("{single}\n");
    }
    // Overflow: rustfmt keeps the expression's FIRST line on the
    // declaration line (`pub const X: Shadow = Shadow {`, `… = &[`) and
    // leaves the remaining lines at their own indentation; only a
    // short single-line expression takes the bare break-after-`=` form.
    match c.expr.find('\n') {
        Some(i) => {
            let head = &c.expr[..i];
            let rest = &c.expr[i + 1..];
            format!("pub const {}: {} = {}\n{};\n", c.name, c.ty, head, rest)
        }
        None => format!("pub const {}: {} =\n    {};\n", c.name, c.ty, c.expr),
    }
}

/// The `f32`/literal pair for one value. The historical fmt drift
/// (2026-11-13, CI run 43373340074): the wrap decisions used the
/// placeholder prefix `pub const X: &[&str] = &` (23 chars), which
/// under-estimated the real `FONT_FAMILY_SANS` declaration by 15 —
/// the generator emitted a 104-column line and `cargo fmt --check`
/// rejected the committed output. The decisions now live where they
/// belong: the FAMILY/SHADOW arms judge only the INDENTED continuation
/// line (`    <expr>;`, 4-space indent), and the outer `const_declaration`
/// breaks after `=` when the single-line declaration overflows — the
/// exact shapes rustfmt keeps stable.
fn rust_const(_name: &str, value: &ResolvedValue) -> Result<(&'static str, String), TokenError> {
    match value {
        ResolvedValue::Color(c) => Ok(("Rgb8", rgb_expr(*c))),
        ResolvedValue::Dimension { value, .. } => Ok(("f32", rust_f32(*value))),
        ResolvedValue::Number(n) => Ok(("f32", rust_f32(*n))),
        ResolvedValue::Family(names) => {
            if names.is_empty() {
                return Err(TokenError::emit("fontFamily stack must not be empty"));
            }
            let items: Vec<String> = names.iter().map(|n| rust_str_lit(n)).collect();
            // Continuation-line budget: the outer `const_declaration` breaks
            // after `=` when the single-line declaration overflows, so the
            // ARRAY's own wrap decision must be made against the INDENTED
            // rhs line (`    &[…];`), not the single-line declaration —
            // 5 = 4-space indent + the `&` that precedes the array.
            let array = rust_array(&items, 5);
            Ok(("&[&str]", format!("&{array}")))
        }
        ResolvedValue::Shadow(s) => {
            let single = format!(
                "Shadow {{ offset_x: {}, offset_y: {}, blur: {}, spread: {}, color: {}, alpha: {} }}",
                rust_f32_lit(s.offset_x),
                rust_f32_lit(s.offset_y),
                rust_f32_lit(s.blur),
                rust_f32_lit(s.spread),
                rgb_expr(s.color),
                rust_f32_lit(s.alpha),
            );
            // Same continuation rule as the Family arm: keep the single-line
            // struct literal when it fits on the INDENTED rhs line
            // (`    <literal>;` = 4 + literal + 1), not the single-line
            // declaration — the outer `const_declaration` breaks after `=`.
            if single.len() + 5 <= LINE_WIDTH {
                return Ok(("Shadow", single));
            }
            Ok((
                "Shadow",
                format!(
                    "Shadow {{\n    offset_x: {},\n    offset_y: {},\n    blur: {},\n    spread: {},\n    color: {},\n    alpha: {},\n}}",
                    rust_f32_lit(s.offset_x),
                    rust_f32_lit(s.offset_y),
                    rust_f32_lit(s.blur),
                    rust_f32_lit(s.spread),
                    rgb_expr(s.color),
                    rust_f32_lit(s.alpha),
                ),
            ))
        }
    }
}

fn rgb_expr(c: crate::color::Rgb8) -> String {
    format!("Rgb8::new(0x{:02x}, 0x{:02x}, 0x{:02x})", c.r, c.g, c.b)
}

/// `f32` literal: integral values keep a `.0` suffix (`25.0`), everything
/// else uses the shortest round-trip form (`4.76`).
fn rust_f32(v: f64) -> String {
    rust_f32_lit(v as f32)
}

fn rust_f32_lit(v: f32) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1.0e15 {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}

/// Uppercase identifier form of a mode/accent id.
fn screaming(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Rust string literal with `"`/`\` escaped.
fn rust_str_lit(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Array literal over pre-quoted item literals, single-line when the whole
/// `pub const` line fits, one item per line otherwise (mirrors what
/// rustfmt keeps stable).
fn rust_array(items: &[String], prefix_len: usize) -> String {
    let single = format!("[{}]", items.join(", "));
    if prefix_len + single.len() + ";".len() <= LINE_WIDTH {
        single
    } else {
        let mut out = String::from("[\n");
        for item in items {
            out.push_str(&format!("    {item},\n"));
        }
        out.push(']');
        out
    }
}

fn rust_header(tokens: &TokenSet) -> String {
    let counts = tokens.counts();
    let modes = list_with_default(tokens.modes(), tokens.default_mode());
    let accents = list_with_default(tokens.accents(), tokens.default_accent());
    let mut lines: Vec<String> = vec![
        "//! GENERATED FILE — DO NOT EDIT.".to_owned(),
        "//!".to_owned(),
        "//! Produced by `cargo xtask tokens` (crates/theme-tokens `tokens-gen`,".to_owned(),
        "//! card 1.B4) from `themes/tokens.json` — doc 08 §3. Regeneration is".to_owned(),
        "//! idempotent; the committed copy is diff-tested by".to_owned(),
        "//! `crates/theme-tokens/tests/diff.rs`.".to_owned(),
        "//!".to_owned(),
        "//! Semantic-layer constants for engine-side consumption (doc 01 §2.3,".to_owned(),
        "//! doc 08 §3.1). Components and engine code consume SEMANTIC tokens".to_owned(),
        "//! only (doc 08 §3.2) — the primitive and alias layers are never".to_owned(),
        "//! referenced directly (`theme_tokens::lint` checks this).".to_owned(),
        "//!".to_owned(),
        format!(
            "//! Modes ({}): {}. Per-mode tokens carry a `_{{MODE}}` suffix for",
            tokens.modes().len(),
            modes.join(", ")
        ),
        "//! every registered mode.".to_owned(),
        format!(
            "//! Accents ({}): {}.",
            tokens.accents().len(),
            accents.join(", ")
        ),
        "//! One ramp per accent (doc 08 §3.3): `ACCENT_{ID}_{ROLE}` with".to_owned(),
        "//! roles PRIMARY, STRONG, TINTED_DARK, TINTED_LIGHT, BRIGHT_LIGHT,".to_owned(),
        "//! BRIGHT_DARK, ICON_LIGHT, ON_ACCENT (j18 F2: the list now names".to_owned(),
        "//! all eight emitted roles).".to_owned(),
        "//!".to_owned(),
        format!(
            "//! Source shape: {} primitive / {} alias / {} semantic tokens.",
            counts.primitives, counts.aliases, counts.semantic
        ),
    ];
    lines.push(String::new());
    lines.push(String::new());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Rgb8, Shadow};

    #[test]
    fn css_numbers_use_shortest_stable_form() {
        assert_eq!(css_num(25.0), "25");
        assert_eq!(css_num(4.76), "4.76");
        assert_eq!(css_num(13.75), "13.75");
        assert_eq!(css_num(0.95), "0.95");
        assert_eq!(css_num(400.0), "400");
        assert_eq!(css_num32(0.38), "0.38");
        assert_eq!(css_num32(4.76), "4.76");
        assert_eq!(css_num32(0.0), "0");
    }

    #[test]
    fn rust_floats_keep_the_point_suffix_for_integers() {
        assert_eq!(rust_f32(25.0), "25.0");
        assert_eq!(rust_f32(4.76), "4.76");
        assert_eq!(rust_f32(400.0), "400.0");
        assert_eq!(rust_f32(0.95), "0.95");
        assert_eq!(rust_f32_lit(0.38), "0.38");
    }

    #[test]
    fn css_families_quote_non_generic_names() -> Result<(), TokenError> {
        assert_eq!(css_family("sans-serif"), "sans-serif");
        assert_eq!(css_family("system-ui"), "system-ui");
        assert_eq!(css_family("Inter"), "\"Inter\"");
        let value = css_value(&ResolvedValue::Family(vec![
            "Inter".to_owned(),
            "system-ui".to_owned(),
            "sans-serif".to_owned(),
        ]))?;
        assert_eq!(value, "\"Inter\", system-ui, sans-serif");
        Ok(())
    }

    #[test]
    fn css_shadow_renders_box_shadow_form() -> Result<(), TokenError> {
        let value = css_value(&ResolvedValue::Shadow(Shadow {
            offset_x: 0.0,
            offset_y: 4.76,
            blur: 4.76,
            spread: 0.0,
            color: Rgb8::new(0, 0, 0),
            alpha: 0.38,
        }))?;
        assert_eq!(value, "0px 4.76px 4.76px 0px rgba(0, 0, 0, 0.38)");
        Ok(())
    }

    #[test]
    fn rust_shadow_literal_is_multiline_when_long() -> Result<(), TokenError> {
        let shadow = Shadow {
            offset_x: 0.0,
            offset_y: 4.76,
            blur: 4.76,
            spread: 0.0,
            color: Rgb8::new(0, 0, 0),
            alpha: 0.38,
        };
        let (_, expr) = rust_const("SHADOW_TEST", &ResolvedValue::Shadow(shadow))?;
        assert!(expr.starts_with("Shadow {\n    offset_x: 0.0,\n"));
        assert!(expr.ends_with("alpha: 0.38,\n}"));
        Ok(())
    }

    #[test]
    fn rust_array_wraps_only_past_the_line_width() {
        let two: Vec<String> = ["dark", "light"].iter().map(|s| rust_str_lit(s)).collect();
        assert_eq!(rust_array(&two, 23), "[\"dark\", \"light\"]");
        let nine: Vec<String> = [
            "orange", "red", "yellow", "green", "cyan", "blue", "violet", "pink", "gray",
        ]
        .iter()
        .map(|s| rust_str_lit(s))
        .collect();
        let wrapped = rust_array(&nine, 23);
        assert!(wrapped.starts_with("[\n    \"orange\",\n"));
        assert!(wrapped.ends_with("\"gray\",\n]"));
    }

    #[test]
    fn rust_string_literals_escape_quotes_and_backslashes() {
        assert_eq!(rust_str_lit("Inter"), "\"Inter\"");
        assert_eq!(rust_str_lit("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }

    #[test]
    fn screaming_sanitizes_ids() {
        assert_eq!(screaming("dark"), "DARK");
        assert_eq!(screaming("card-lg"), "CARD_LG");
    }
}
