//! Parser + validator for `themes/tokens.json` (card 1.B4, doc 08 §3.1).
//!
//! Accepted format: W3C Design Tokens draft subset — nested groups, `$value`,
//! `$type` (inheritable via groups), `$description`, `{dotted.path}` alias
//! references — plus one lumen extension `[PROPOSAL]`: a `$value` object
//! whose keys are all registered mode names is a per-mode value. Any other
//! object `$value` is a composite (e.g. a shadow).
//!
//! Enforced rules (doc 08 §3.2/§3.3/§3.4):
//! * the three layer groups `primitive`/`alias`/`semantic` must exist;
//! * primitives are raw values only (no references);
//! * aliases reference primitives or aliases only (never semantic);
//! * semantic tokens reference the alias layer only — never primitives;
//! * every per-mode token covers every registered mode;
//! * references resolve and are acyclic;
//! * every accent has a primary color and an on-accent glyph with WCAG
//!   contrast ≥ 4.5:1 (doc 08 §3.3).

use std::collections::HashMap;

use serde_json::Value;

use crate::TokenError;
use crate::color::{self, AccentRamp, Rgb8, Shadow};
use crate::model::{DimensionUnit, LayerToken, ModeValues, ResolvedValue, SemanticToken, TokenSet};

/// A token as found in the file, before reference resolution.
struct RawToken {
    path: String,
    token_type: String,
    raw: RawValue,
}

enum RawValue {
    Scalar(Value),
    PerMode(Vec<(String, Value)>),
}

/// Metadata read from `$extensions."lumen.tokens"`.
struct Meta {
    modes: Vec<String>,
    default_mode: String,
    accents: Vec<String>,
    default_accent: String,
    accent_primaries: Vec<(String, String)>,
    on_accent_candidates: Vec<String>,
}

/// Parse and validate the token source (doc 01 §2.3 `parse`).
pub fn parse(source: &str) -> Result<TokenSet, TokenError> {
    let root: Value = serde_json::from_str(source)
        .map_err(|e| TokenError::parse("", format!("invalid JSON: {e}")))?;
    let root_obj = root
        .as_object()
        .ok_or_else(|| TokenError::parse("", "top level must be a JSON object"))?;

    let meta = read_meta(root_obj)?;
    let mut raw: Vec<RawToken> = Vec::new();
    for layer in ["primitive", "alias", "semantic"] {
        let node = root_obj
            .get(layer)
            .ok_or_else(|| TokenError::parse(layer, "missing required layer group"))?;
        walk_group(node, layer, None, &meta, &mut raw)?;
    }

    let index: HashMap<&str, usize> = raw
        .iter()
        .enumerate()
        .map(|(i, t)| (t.path.as_str(), i))
        .collect();

    let mut resolved: Vec<Option<ModeValues>> = vec![None; raw.len()];
    for i in 0..raw.len() {
        resolve_token(i, &raw, &index, &meta, &mut Vec::new(), &mut resolved)?;
    }

    build_set(&raw, &resolved, &meta)
}

/// Recursively collect tokens from a group node, inheriting `$type`.
fn walk_group(
    node: &Value,
    path: &str,
    inherited_type: Option<&str>,
    meta: &Meta,
    out: &mut Vec<RawToken>,
) -> Result<(), TokenError> {
    let obj = node
        .as_object()
        .ok_or_else(|| TokenError::parse(path, "group must be a JSON object"))?;
    let own_type = obj
        .get("$type")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| inherited_type.map(str::to_owned));
    for (key, child) in obj {
        if key.starts_with('$') {
            continue;
        }
        let child_path = format!("{path}.{key}");
        let value = child.get("$value");
        if let Some(value) = value {
            let token_type = own_type
                .clone()
                .or_else(|| {
                    child
                        .get("$type")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .ok_or_else(|| {
                    TokenError::parse(&child_path, "token has no $type (own or inherited)")
                })?;
            let raw = classify_value(value, &child_path, &meta.modes)?;
            out.push(RawToken {
                path: child_path.clone(),
                token_type,
                raw,
            });
            // Lumen extension: a node may carry BOTH a `$value` and nested
            // token children (e.g. `primitive.shadow.card` keeps the legacy
            // value while `…card.solstice` refines it) — the node is a token
            // AND a group, so descend as well.
            if child
                .as_object()
                .is_some_and(|o| o.keys().any(|k| !k.starts_with('$')))
            {
                walk_group(child, &child_path, own_type.as_deref(), meta, out)?;
            }
        } else {
            walk_group(child, &child_path, own_type.as_deref(), meta, out)?;
        }
    }
    Ok(())
}

/// Split a `$value` into scalar vs per-mode (lumen extension): an object
/// whose keys are ALL registered modes is per-mode; anything else is a
/// composite scalar for the type parser.
fn classify_value(value: &Value, path: &str, modes: &[String]) -> Result<RawValue, TokenError> {
    let Value::Object(map) = value else {
        return Ok(RawValue::Scalar(value.clone()));
    };
    if map.is_empty() {
        return Err(TokenError::parse(path, "$value object must not be empty"));
    }
    if map.keys().all(|k| modes.iter().any(|m| m == k)) {
        Ok(RawValue::PerMode(
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        ))
    } else {
        Ok(RawValue::Scalar(value.clone()))
    }
}

fn read_meta(root: &serde_json::Map<String, Value>) -> Result<Meta, TokenError> {
    let ext = root
        .get("$extensions")
        .and_then(|v| v.get("lumen.tokens"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            TokenError::parse(
                "$extensions",
                "missing `$extensions.\"lumen.tokens\"` block",
            )
        })?;

    let strings = |key: &str| -> Result<Vec<String>, TokenError> {
        ext.get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| TokenError::parse("$extensions", format!("missing `{key}` array")))?
            .iter()
            .map(|v| {
                v.as_str().map(str::to_owned).ok_or_else(|| {
                    TokenError::parse("$extensions", format!("`{key}` entries must be strings"))
                })
            })
            .collect()
    };
    let modes = strings("modes")?;
    if modes.is_empty() {
        return Err(TokenError::parse(
            "$extensions",
            "`modes` must not be empty",
        ));
    }
    let default_mode = ext
        .get("default-mode")
        .and_then(Value::as_str)
        .ok_or_else(|| TokenError::parse("$extensions", "missing `default-mode`"))?
        .to_owned();
    if !modes.contains(&default_mode) {
        return Err(TokenError::parse(
            "$extensions",
            format!("`default-mode` `{default_mode}` is not a registered mode"),
        ));
    }
    let accents = strings("accents")?;
    if accents.is_empty() {
        return Err(TokenError::parse(
            "$extensions",
            "`accents` must not be empty",
        ));
    }
    let default_accent = ext
        .get("default-accent")
        .and_then(Value::as_str)
        .ok_or_else(|| TokenError::parse("$extensions", "missing `default-accent`"))?
        .to_owned();
    if !accents.contains(&default_accent) {
        return Err(TokenError::parse(
            "$extensions",
            format!("`default-accent` `{default_accent}` is not a registered accent"),
        ));
    }

    let primary_map = ext
        .get("accent-primaries")
        .and_then(Value::as_object)
        .ok_or_else(|| TokenError::parse("$extensions", "missing `accent-primaries` object"))?;
    let mut accent_primaries = Vec::new();
    for accent in &accents {
        let reference = primary_map
            .get(accent)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                TokenError::parse(
                    "$extensions",
                    format!("`accent-primaries` has no entry for accent `{accent}`"),
                )
            })?;
        accent_primaries.push((accent.clone(), reference.to_owned()));
    }
    let on_accent_candidates = strings("on-accent-candidates")?;
    if on_accent_candidates.is_empty() {
        return Err(TokenError::parse(
            "$extensions",
            "`on-accent-candidates` must not be empty",
        ));
    }
    Ok(Meta {
        modes,
        default_mode,
        accents,
        default_accent,
        accent_primaries,
        on_accent_candidates,
    })
}

/// Resolve token `i` fully (memoized). `visiting` is the cycle stack.
fn resolve_token(
    i: usize,
    raw: &[RawToken],
    index: &HashMap<&str, usize>,
    meta: &Meta,
    visiting: &mut Vec<String>,
    resolved: &mut Vec<Option<ModeValues>>,
) -> Result<ModeValues, TokenError> {
    if let Some(done) = &resolved[i] {
        return Ok(done.clone());
    }
    let path = raw[i].path.clone();
    if visiting.iter().any(|p| p == &path) {
        let chain = visiting.join(" -> ");
        return Err(TokenError::cycle(format!("{chain} -> {path}")));
    }
    visiting.push(path.clone());
    let token_type = raw[i].token_type.clone();
    let result = match &raw[i].raw {
        RawValue::Scalar(v) => {
            if let Some(target) = reference_target(v) {
                check_layer_ref(&path, &target)?;
                let j = lookup(&target, index, &path)?;
                resolve_token(j, raw, index, meta, visiting, resolved)?
            } else {
                ModeValues::Single(parse_typed_value(v, &token_type, &path)?)
            }
        }
        RawValue::PerMode(pairs) => {
            let mut out = Vec::new();
            for (mode, v) in pairs {
                if !meta.modes.contains(mode) {
                    return Err(TokenError::parse(
                        &path,
                        format!("per-mode key `{mode}` is not a registered mode"),
                    ));
                }
                let value = if let Some(target) = reference_target(v) {
                    check_layer_ref(&path, &target)?;
                    let j = lookup(&target, index, &path)?;
                    match resolve_token(j, raw, index, meta, visiting, resolved)? {
                        ModeValues::Single(value) => value,
                        ModeValues::PerMode(pairs) => pairs
                            .iter()
                            .find(|(m, _)| m == mode)
                            .map(|(_, v)| v.clone())
                            .ok_or_else(|| {
                                TokenError::parse(
                                    &path,
                                    format!("reference `{target}` has no value for mode `{mode}`"),
                                )
                            })?,
                    }
                } else {
                    parse_typed_value(v, &token_type, &path)?
                };
                out.push((mode.clone(), value));
            }
            for mode in &meta.modes {
                if !out.iter().any(|(m, _)| m == mode) {
                    return Err(TokenError::parse(
                        &path,
                        format!("per-mode token is missing mode `{mode}` (doc 08 §3.4 coverage)"),
                    ));
                }
            }
            ModeValues::PerMode(out)
        }
    };
    visiting.pop();
    resolved[i] = Some(result.clone());
    Ok(result)
}

/// The `{dotted.path}` reference target, if `v` is a reference.
fn reference_target(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    if s.len() > 2 && s.starts_with('{') && s.ends_with('}') {
        Some(s[1..s.len() - 1].to_owned())
    } else {
        None
    }
}

/// Layer rules (doc 08 §3.2): primitives are raw; aliases reference the
/// primitive/alias layers only; semantic references the alias layer only.
fn check_layer_ref(from: &str, target: &str) -> Result<(), TokenError> {
    let from_layer = from.split('.').next().unwrap_or_default();
    let target_layer = target.split('.').next().unwrap_or_default();
    match (from_layer, target_layer) {
        ("primitive", _) => Err(TokenError::parse(
            from,
            "primitives must be raw values — references are not allowed in the primitive layer",
        )),
        ("alias", "primitive") | ("alias", "alias") => Ok(()),
        ("alias", _) => Err(TokenError::parse(
            from,
            format!("aliases may reference primitives/aliases only, not `{target}`"),
        )),
        ("semantic", "alias") => Ok(()),
        ("semantic", _) => Err(TokenError::parse(
            from,
            format!(
                "semantic tokens must resolve through the alias layer, not `{target}` \
                 (doc 08 §3.2)"
            ),
        )),
        _ => Ok(()),
    }
}

fn lookup(target: &str, index: &HashMap<&str, usize>, from: &str) -> Result<usize, TokenError> {
    index
        .get(target)
        .copied()
        .ok_or_else(|| TokenError::unknown_reference(from, target))
}

/// Parse a raw scalar into a typed value according to `$type`.
fn parse_typed_value(v: &Value, token_type: &str, path: &str) -> Result<ResolvedValue, TokenError> {
    let err = |msg: String| TokenError::parse(path, msg);
    match token_type {
        "color" => {
            let s = v
                .as_str()
                .ok_or_else(|| err("color value must be a string".into()))?;
            Ok(ResolvedValue::Color(
                Rgb8::parse_hex(s).map_err(|e| err(format!("bad color `{s}`: {e}")))?,
            ))
        }
        "dimension" => {
            let s = v
                .as_str()
                .ok_or_else(|| err("dimension value must be a string".into()))?;
            let (value, unit) = parse_dimension(s).map_err(err)?;
            Ok(ResolvedValue::Dimension { value, unit })
        }
        "number" | "fontWeight" => {
            let n = v
                .as_f64()
                .ok_or_else(|| err("value must be a JSON number".into()))?;
            Ok(ResolvedValue::Number(n))
        }
        "fontFamily" => match v {
            Value::Array(items) => {
                let mut names = Vec::new();
                for item in items {
                    names.push(
                        item.as_str()
                            .ok_or_else(|| err("fontFamily entries must be strings".into()))?
                            .to_owned(),
                    );
                }
                Ok(ResolvedValue::Family(names))
            }
            Value::String(s) => Ok(ResolvedValue::Family(vec![s.clone()])),
            _ => Err(err("fontFamily value must be an array of strings".into())),
        },
        "shadow" => parse_shadow(v).map_err(err),
        other => Err(err(format!("unsupported $type `{other}`"))),
    }
}

/// A dimension value: px lengths or ms motion durations (doc 08 §3.1).
fn parse_dimension(s: &str) -> Result<(f64, DimensionUnit), String> {
    let (num, unit) = if let Some(num) = s.strip_suffix("px") {
        (num, DimensionUnit::Px)
    } else if let Some(num) = s.strip_suffix("ms") {
        (num, DimensionUnit::Ms)
    } else {
        return Err(format!("dimension must end in `px` or `ms`, got `{s}`"));
    };
    num.parse::<f64>()
        .map_err(|e| format!("bad dimension `{s}`: {e}"))
        .and_then(|v| {
            if v.is_finite() && v >= 0.0 {
                Ok((v, unit))
            } else {
                Err(format!(
                    "dimension must be finite and non-negative, got `{s}`"
                ))
            }
        })
}

/// A px length (shadow geometry fields).
fn parse_px(s: &str) -> Result<f64, String> {
    let Some(num) = s.strip_suffix("px") else {
        return Err(format!("dimension must end in `px`, got `{s}`"));
    };
    num.parse::<f64>()
        .map_err(|e| format!("bad dimension `{s}`: {e}"))
        .and_then(|v| {
            if v.is_finite() && v >= 0.0 {
                Ok(v)
            } else {
                Err(format!(
                    "dimension must be finite and non-negative, got `{s}`"
                ))
            }
        })
}

/// Shadow values: the structured object form (`offsetX`/`offsetY`/`blur`/
/// `spread`/`color`) or the CSS shorthand string
/// `Xpx Ypx Bpx Spx <color>` (the Solstice source uses the shorthand for
/// its refined shadows).
fn parse_shadow(v: &Value) -> Result<ResolvedValue, String> {
    if let Value::String(s) = v {
        return parse_shadow_shorthand(s);
    }
    let obj = v.as_object().ok_or("shadow value must be an object")?;
    let field = |name: &str| -> Result<f64, String> {
        let raw = obj
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("shadow is missing `{name}`"))?;
        parse_px(raw)
    };
    let color_raw = obj
        .get("color")
        .and_then(Value::as_str)
        .ok_or("shadow is missing `color`")?;
    let (color, alpha) = parse_shadow_color(color_raw)?;
    Ok(ResolvedValue::Shadow(Shadow {
        offset_x: field("offsetX")? as f32,
        offset_y: field("offsetY")? as f32,
        blur: field("blur")? as f32,
        spread: field("spread")? as f32,
        color,
        alpha,
    }))
}

/// The shorthand string form: `Xpx Ypx Bpx Spx #rrggbb|rgba(…)` — the
/// color component may itself contain spaces, so it is the remainder of
/// the string after the four geometry fields.
fn parse_shadow_shorthand(s: &str) -> Result<ResolvedValue, String> {
    let mut rest = s.trim();
    let mut field = |name: &str| -> Result<f64, String> {
        let Some(end) = rest.find(char::is_whitespace) else {
            return Err(format!("shadow shorthand `{s}` is missing `{name}`"));
        };
        let (raw, tail) = rest.split_at(end);
        rest = tail.trim_start();
        parse_px(raw).map_err(|e| format!("shadow shorthand `{s}`: {e}"))
    };
    let offset_x = field("offsetX")?;
    let offset_y = field("offsetY")?;
    let blur = field("blur")?;
    let spread = field("spread")?;
    let color_raw = rest;
    if color_raw.is_empty() {
        return Err(format!("shadow shorthand `{s}` is missing `color`"));
    }
    let (color, alpha) =
        parse_shadow_color(color_raw).map_err(|e| format!("shadow shorthand `{s}`: {e}"))?;
    Ok(ResolvedValue::Shadow(Shadow {
        offset_x: offset_x as f32,
        offset_y: offset_y as f32,
        blur: blur as f32,
        spread: spread as f32,
        color,
        alpha,
    }))
}

/// Shadow colors: `#rrggbb` (opaque) or `rgba(r, g, b, a)`.
fn parse_shadow_color(s: &str) -> Result<(Rgb8, f32), String> {
    if let Some(rest) = s.strip_prefix('#') {
        if rest.len() == 6 {
            return Ok((Rgb8::parse_hex(s)?, 1.0));
        }
        return Err(format!("shadow color must be #rrggbb or rgba(), got `{s}`"));
    }
    let Some(rest) = s.strip_prefix("rgba(") else {
        return Err(format!("shadow color must be #rrggbb or rgba(), got `{s}`"));
    };
    let body = rest.strip_suffix(')').ok_or("unterminated rgba()")?;
    let parts: Vec<&str> = body.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return Err(format!("rgba() needs 4 components, got `{s}`"));
    }
    let chan = |p: &str| -> Result<u8, String> {
        p.parse::<u8>()
            .map_err(|e| format!("bad rgba channel `{p}`: {e}"))
    };
    let alpha: f64 = parts[3]
        .parse::<f64>()
        .map_err(|e| format!("bad rgba alpha `{}`: {e}", parts[3]))?;
    if !(0.0..=1.0).contains(&alpha) {
        return Err(format!("rgba alpha out of range: `{s}`"));
    }
    Ok((
        Rgb8::new(chan(parts[0])?, chan(parts[1])?, chan(parts[2])?),
        alpha as f32,
    ))
}

/// Assemble the public [`TokenSet`] from resolved tokens.
fn build_set(
    raw: &[RawToken],
    resolved: &[Option<ModeValues>],
    meta: &Meta,
) -> Result<TokenSet, TokenError> {
    let mut non_semantic = Vec::new();
    let mut semantic = Vec::new();
    for (i, token) in raw.iter().enumerate() {
        let values = resolved[i].clone().ok_or_else(|| {
            TokenError::internal(format!("token `{}` was not resolved", token.path))
        })?;
        let layer = token.path.split('.').next().unwrap_or_default();
        if layer == "semantic" {
            let tail = token.path.strip_prefix("semantic.").unwrap_or(&token.path);
            let (category, name) = tail.split_once('.').ok_or_else(|| {
                TokenError::parse(&token.path, "semantic token needs a category segment")
            })?;
            let prefix = rust_category_prefix(category, &token.path)?;
            semantic.push(SemanticToken {
                path: tail.to_owned(),
                css_name: format!("--{category}-{}", name.replace('.', "-")),
                rust_name: format!(
                    "{}_{}",
                    prefix,
                    name.replace(['.', '-'], "_").to_uppercase()
                ),
                category: category.to_owned(),
                values,
            });
        } else {
            non_semantic.push(LayerToken {
                name: token.path.strip_prefix("primitive.").map_or_else(
                    || {
                        token
                            .path
                            .strip_prefix("alias.")
                            .unwrap_or(&token.path)
                            .to_owned()
                    },
                    str::to_owned,
                ),
                layer: match layer {
                    "primitive" => "primitive",
                    _ => "alias",
                },
                values,
            });
        }
    }

    // Accent family: convention per doc 08 §3.3 — the semantic tokens that
    // flip per accent are exactly those resolving through the alias roles
    // `accent`, `accent.bright`, `accent.icon`, `accent.strong`,
    // `accent.tinted` and `content.on-accent`.
    let accent_alias_roles = [
        "color.accent",
        "color.accent.bright",
        "color.accent.icon",
        "color.accent.strong",
        "color.accent.tinted",
        "color.content.on-accent",
    ];
    let mut accent_family_css = Vec::new();
    for role in accent_alias_roles {
        let Some(token) = semantic.iter().find(|t| t.path == role) else {
            return Err(TokenError::internal(format!(
                "accent-family semantic token `{role}` is missing (doc 08 §3.3)"
            )));
        };
        accent_family_css.push(token.css_name.clone());
    }

    // Resolve accent primaries + on-accent candidates, then build ramps: the
    // default accent takes its ramp from the file (mined values); the other
    // eight are derived (doc 08 §3.3) and contrast-validated.
    let find = |reference: &str| -> Result<Rgb8, TokenError> {
        match resolve_reference(reference, raw, resolved)? {
            ResolvedValue::Color(c) => Ok(c),
            other => Err(TokenError::internal(format!(
                "accent reference `{reference}` must resolve to a color, got {other:?}"
            ))),
        }
    };
    let mut candidates = Vec::new();
    for reference in &meta.on_accent_candidates {
        candidates.push(find(reference)?);
    }
    let mut ramps = Vec::new();
    for (accent, reference) in &meta.accent_primaries {
        let primary = find(reference)?;
        let ramp = if *accent == meta.default_accent {
            ramp_from_file(&semantic, accent)?
        } else {
            color::derive_ramp(accent, primary, &candidates)
                .map_err(|e| TokenError::contrast(format!("accent `{accent}`: {e}")))?
        };
        ramps.push(ramp);
    }

    Ok(TokenSet {
        modes: meta.modes.clone(),
        default_mode: meta.default_mode.clone(),
        accents: meta.accents.clone(),
        default_accent: meta.default_accent.clone(),
        non_semantic,
        semantic,
        ramps,
        accent_family_css,
    })
}

/// Resolve an extension reference like `{primitive.color.orange.500}` to its
/// single value.
fn resolve_reference(
    reference: &str,
    raw: &[RawToken],
    resolved: &[Option<ModeValues>],
) -> Result<ResolvedValue, TokenError> {
    let target = reference
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .ok_or_else(|| TokenError::internal(format!("bad reference `{reference}`")))?;
    let idx = raw
        .iter()
        .position(|t| t.path == target)
        .ok_or_else(|| TokenError::unknown_reference(reference, target))?;
    match &resolved[idx] {
        Some(ModeValues::Single(v)) => Ok(v.clone()),
        Some(ModeValues::PerMode(_)) => Err(TokenError::internal(format!(
            "extension reference `{reference}` must point at a modeless token"
        ))),
        None => Err(TokenError::internal(format!(
            "extension reference `{reference}` was not resolved"
        ))),
    }
}

/// The default accent's ramp, taken from the (mined) alias values.
fn ramp_from_file(semantic: &[SemanticToken], accent: &str) -> Result<AccentRamp, TokenError> {
    let color = |path: &str, mode: Option<&str>| -> Result<Rgb8, TokenError> {
        let token = semantic.iter().find(|t| t.path == path).ok_or_else(|| {
            TokenError::internal(format!("ramp source token `{path}` is missing"))
        })?;
        match token.values.value_for(mode) {
            Some(ResolvedValue::Color(c)) => Ok(*c),
            _ => Err(TokenError::internal(format!(
                "ramp source token `{path}` is not a color"
            ))),
        }
    };
    Ok(AccentRamp {
        id: accent.to_owned(),
        primary: color("color.accent", None)?,
        strong: color("color.accent.strong", None)?,
        tinted_dark: color("color.accent.tinted", Some("dark"))?,
        tinted_light: color("color.accent.tinted", Some("light"))?,
        bright_light: color("color.accent.bright", Some("light"))?,
        icon_light: color("color.accent.icon", Some("light"))?,
        on_accent: color("color.content.on-accent", None)?,
    })
}

/// Rust constant prefix per semantic category.
fn rust_category_prefix(category: &str, path: &str) -> Result<&'static str, TokenError> {
    let prefix = match category {
        "color" => "COLOR",
        "gradient" => "GRADIENT",
        "font" => "FONT",
        "space" => "SPACE",
        "radius" => "RADIUS",
        "stroke" => "STROKE",
        "shadow" => "SHADOW",
        "blur" => "BLUR",
        "opacity" => "OPACITY",
        "motion" => "MOTION",
        _ => {
            return Err(TokenError::parse(
                path,
                format!("unsupported semantic category `{category}`"),
            ));
        }
    };
    Ok(prefix)
}
