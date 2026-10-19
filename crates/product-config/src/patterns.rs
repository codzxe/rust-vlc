//! Hand-written pattern matchers for every `pattern` constraint in
//! `product.schema.json`.
//!
//! JSON Schema regex is not available without a regex engine, and none is
//! pre-audited (docs/dependencies/preaudit-foundation.md), so each pattern
//! string that appears in the schema has a dedicated hand-written matcher
//! here. The registry below maps the *exact schema pattern string* to its
//! matcher; `Schema::load` walks the schema and fails loudly if any pattern
//! lacks a matcher (schema drift becomes a typed error, not silence).

/// A compiled-in matcher for one schema pattern string.
pub type Matcher = fn(&str) -> bool;

/// Every pattern string used by `product.schema.json`, with its matcher.
/// Keep in sync with the schema; the `schema_coverage` test plus the
/// `Schema::load` check enforce this mechanically.
pub fn matcher_for(pattern: &str) -> Option<Matcher> {
    Some(match pattern {
        // identity.short_name: 1-12 chars of ASCII letters/digits/hyphen.
        "^[A-Za-z0-9-]{1,12}$" => is_short_name,
        // identity.display_version: semver-shaped.
        "^[0-9]+\\.[0-9]+\\.[0-9]+(-[A-Za-z0-9.]+)?$" => is_display_version,
        // identity.ids.reverse_dns: lowercase reverse-DNS root.
        "^[a-z0-9]+(\\.[a-z0-9-]+)+$" => is_reverse_dns,
        // identity.ids.windows.msi_upgrade_code: stable GUID.
        "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$" => is_guid,
        // window.title_template: only {product_name}/{media_title} variables.
        "^(\\{product_name\\}|\\{media_title\\}|[^{}]|\\{\\{)*$" => is_title_template,
        // brand.accent_color: lowercase hex RGB with leading '#'.
        "^#[0-9a-f]{6}$" => is_hex_color,
        // $defs.msgid: dot-separated lowercase message key, >= 2 segments.
        "^[a-z0-9-]+(\\.[a-z0-9-]+)+$" => is_msgid,
        // $defs.bcp47: BCP-47 language tag.
        "^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$" => is_bcp47,
        // $defs.https_url: https:// prefix.
        "^https://.+$" => is_https_url,
        // $defs.repo_path: repo-relative, not rooted at '/'.
        "^[^/].*$" => is_repo_relative_path,
        // Simple lowercase-ASCII token (used by schema test fixtures).
        "^[a-z]+$" => is_lowercase_ascii_token,
        _ => return None,
    })
}

fn is_short_name(s: &str) -> bool {
    let len = s.chars().count();
    (1..=12).contains(&len) && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn is_display_version(s: &str) -> bool {
    // [0-9]+ "." [0-9]+ "." [0-9]+ ( "-" [A-Za-z0-9.]+ )?
    let (core, pre) = match s.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (s, None),
    };
    if let Some(pre) = pre
        && (pre.is_empty() || !pre.chars().all(|c| c.is_ascii_alphanumeric() || c == '.'))
    {
        return false;
    }
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn is_reverse_dns(s: &str) -> bool {
    // First segment: [a-z0-9]+ (no hyphen); later segments: [a-z0-9-]+ (non-empty).
    let mut segments = s.split('.');
    match segments.next() {
        Some(first) => {
            if first.is_empty()
                || !first
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            {
                return false;
            }
        }
        None => return false,
    }
    let mut later = 0;
    for seg in segments {
        later += 1;
        if seg.is_empty()
            || !seg
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return false;
        }
    }
    later >= 1
}

fn is_guid(s: &str) -> bool {
    let groups = [8usize, 4, 4, 4, 12];
    let mut rest = s;
    for (i, want) in groups.iter().enumerate() {
        let (group, tail) = match rest.split_once('-') {
            Some((g, t)) => (g, t),
            None => (rest, ""),
        };
        if i == groups.len() - 1 && !tail.is_empty() {
            return false;
        }
        if group.chars().count() != *want || !group.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
        rest = tail;
    }
    rest.is_empty()
}

fn is_title_template(s: &str) -> bool {
    // Zero or more of: "{product_name}", "{media_title}", a non-brace
    // char, or "{{". Unknown {variables} match no alternative. Advancement is
    // always on UTF-8 char boundaries (never slices mid-char).
    let mut rest = s;
    while !rest.is_empty() {
        if rest.starts_with("{product_name}") {
            rest = &rest["{product_name}".len()..];
        } else if rest.starts_with("{media_title}") {
            rest = &rest["{media_title}".len()..];
        } else if rest.starts_with("{{") {
            rest = &rest[2..];
        } else {
            match rest.chars().next() {
                Some(c) if c != '{' && c != '}' => rest = &rest[c.len_utf8()..],
                _ => return false,
            }
        }
    }
    true
}

fn is_hex_color(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 7
        && bytes.first() == Some(&b'#')
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
}

fn is_msgid(s: &str) -> bool {
    // Dot-separated segments of [a-z0-9-], at least two segments.
    let mut count = 0usize;
    for seg in s.split('.') {
        count += 1;
        if seg.is_empty()
            || !seg
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return false;
        }
    }
    count >= 2
}

fn is_bcp47(s: &str) -> bool {
    // [a-z]{2,3} ( "-" [A-Za-z0-9]{2,8} )*
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut lead = 0usize;
    while let Some(b) = bytes.get(i) {
        if b.is_ascii_lowercase() {
            lead += 1;
            i += 1;
        } else {
            break;
        }
    }
    if !(2..=3).contains(&lead) {
        return false;
    }
    while i < bytes.len() {
        if bytes.get(i) != Some(&b'-') {
            return false;
        }
        i += 1;
        let sub_len = {
            let mut n = 0usize;
            while let Some(b) = bytes.get(i) {
                if b.is_ascii_alphanumeric() {
                    n += 1;
                    i += 1;
                } else {
                    break;
                }
            }
            n
        };
        if !(2..=8).contains(&sub_len) {
            return false;
        }
    }
    true
}

fn is_https_url(s: &str) -> bool {
    s.starts_with("https://") && s.len() > "https://".len()
}

fn is_repo_relative_path(s: &str) -> bool {
    !s.is_empty() && !s.starts_with('/')
}

fn is_lowercase_ascii_token(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_name_rules() {
        assert!(is_short_name("Lumen"));
        assert!(is_short_name("lumen-player"));
        assert!(!is_short_name("Lumen Player"));
        assert!(!is_short_name(""));
        assert!(!is_short_name("twelve-characters"));
    }

    #[test]
    fn display_version_rules() {
        assert!(is_display_version("0.1.0-dev"));
        assert!(is_display_version("1.2.3"));
        assert!(!is_display_version("version one"));
        assert!(!is_display_version("1.2"));
        assert!(!is_display_version("1.2.3-"));
    }

    #[test]
    fn reverse_dns_rules() {
        assert!(is_reverse_dns("dev.lumen"));
        assert!(!is_reverse_dns("Dev.Lumen"));
        assert!(!is_reverse_dns("dev"));
        assert!(!is_reverse_dns("dev."));
    }

    #[test]
    fn guid_rules() {
        assert!(is_guid("7c1f6a52-9d3e-4b8a-a001-3f9c2d64e701"));
        assert!(!is_guid("not-a-guid"));
        assert!(!is_guid("7c1f6a52-9d3e-4b8a-a001-3f9c2d64e70"));
    }

    #[test]
    fn title_template_rules() {
        assert!(is_title_template("{media_title} — {product_name}"));
        assert!(is_title_template("plain title"));
        assert!(!is_title_template("{unknown_var} — {product_name}"));
        assert!(!is_title_template("trailing } brace"));
    }

    #[test]
    fn hex_color_rules() {
        assert!(is_hex_color("#4f8cff"));
        assert!(!is_hex_color("4f8cff"));
        assert!(!is_hex_color("#4F8CFF"));
        assert!(!is_hex_color("#4f8cf"));
    }

    #[test]
    fn msgid_rules() {
        assert!(is_msgid("product.description"));
        assert!(is_msgid("window.title.idle"));
        assert!(!is_msgid("Product Description"));
        assert!(!is_msgid("product"));
    }

    #[test]
    fn bcp47_rules() {
        assert!(is_bcp47("en"));
        assert!(is_bcp47("en-GB"));
        assert!(!is_bcp47("english"));
        assert!(!is_bcp47("e"));
    }

    #[test]
    fn https_url_rules() {
        assert!(is_https_url("https://lumen-player.example.org/docs"));
        assert!(!is_https_url("http://lumen-player.example.org/docs"));
        assert!(!is_https_url("https://"));
    }

    #[test]
    fn repo_path_rules() {
        assert!(is_repo_relative_path("assets/brand/icon.ico"));
        assert!(!is_repo_relative_path("/etc/passwd"));
    }

    #[test]
    fn every_matcher_is_registered_under_its_schema_string() {
        // The exact schema pattern strings (as they appear in the JSON file,
        // i.e. with JSON escaping applied where the schema uses it).
        let schema_patterns = [
            "^[A-Za-z0-9-]{1,12}$",
            "^[0-9]+\\.[0-9]+\\.[0-9]+(-[A-Za-z0-9.]+)?$",
            "^[a-z0-9]+(\\.[a-z0-9-]+)+$",
            "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
            "^(\\{product_name\\}|\\{media_title\\}|[^{}]|\\{\\{)*$",
            "^#[0-9a-f]{6}$",
            "^[a-z0-9-]+(\\.[a-z0-9-]+)+$",
            "^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$",
            "^https://.+$",
            "^[^/].*$",
        ];
        for p in schema_patterns {
            match matcher_for(p) {
                Some(_) => {}
                None => unreachable!("no matcher registered for schema pattern {p}"),
            }
        }
        // An unknown pattern string must not silently validate.
        assert!(matcher_for("^unknown$").is_none());
    }
}
