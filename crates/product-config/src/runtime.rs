//! Runtime identity resolution — the rebrand runtime plane (card 4.H2).
//!
//! Doc 07 §8.1 splits rebrand into a **build-time** plane (`cargo xtask
//! rebrand` rewrites packaging) and a **runtime** plane: "every surface
//! reads `product-config` at run time". Phase 1 shipped the model + the
//! build-time tool; the functions here are the *resolution* half of the
//! runtime plane — the single seam every identity-bearing runtime surface
//! (window title, About facts, CLI banner, log prefix, installer ids)
//! composes from a loaded [`ProductConfig`]. The rebrand acceptance test
//! (`tests/rebrand/rebrand_acceptance.rs`, doc 07 §8.3 step 4) drives
//! them with the Nova fixture.
//!
//! # Honest scope (card 4.H2)
//!
//! These are pure functions over the validated model — no I/O, no clock,
//! no localization rendering (wording joins via `identity.description_key`
//! / `window.title_idle_key` with the localization layer; technical
//! separators like the em-dash template glue are not translatable prose,
//! product.toml §2.2). Hosts that do not exist yet (the CLI banner
//! printer, the runtime logger) are recorded as honest gaps in the
//! acceptance report, not faked here.

use crate::model::{Brand, Identity, Window};

/// Installer/packaging identity derived from the config (doc 07 §8.2 row
/// `identity.name` / `identity.ids.*` → installer columns).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerIdentity {
    /// Human product name on installers and desktop entries.
    pub product_name: String,
    /// Desktop bundle identifier (Tauri `identifier` shape:
    /// `{reverse_dns}.desktop`, matching the committed tauri.conf.json).
    pub desktop_bundle_identifier: String,
    /// macOS bundle id.
    pub macos_bundle_id: String,
    /// Windows AppUserModelId.
    pub windows_app_user_model_id: String,
    /// Linux desktop-entry / AppStream id.
    pub linux_appstream_id: String,
}

/// About-panel facts (doc 07 §8.2 rows `identity.*` + `brand.logo_svg` →
/// About columns). The UI's build-info seam (card 4.E4) injects the same
/// fields at build time; this is the config-driven source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutFacts {
    /// Heading: `identity.name`.
    pub product_name: String,
    /// Version line: `identity.display_version`.
    pub display_version: String,
    /// Copyright line: `identity.publisher`.
    pub publisher: String,
    /// Logo path: `brand.logo_svg`.
    pub logo_svg: String,
}

/// Resolve the window title from the `[window]` template (doc 07 §2.2,
/// §8.3 step 4). With media open: `{product_name}` → [`Identity::name`],
/// `{media_title}` → the playing title. With **no** media open the whole
/// title is the idle wording (product.toml §2.2: "`{media_title}` … else
/// the idle title"): localized via `window.title_idle_key` rendered with
/// `{product_name}` — whose pre-localization rendering is the product
/// name itself (doc 07 §8.3 step 4 asserts the idle title "Nova Player").
#[must_use = "pure reporter"]
pub fn resolve_window_title(
    window: &Window,
    identity: &Identity,
    media_title: Option<&str>,
) -> String {
    match media_title {
        Some(media) => window
            .title_template
            .replace("{product_name}", &identity.name)
            .replace("{media_title}", media),
        None => identity.name.clone(),
    }
}

/// The log-line identity prefix (doc 07 §8.2 `identity.name`/`short_name`
/// → logs): lowercase `short_name` (e.g. `nova` for the Nova fixture).
#[must_use = "pure reporter"]
pub fn log_line_prefix(identity: &Identity) -> String {
    identity.short_name.to_lowercase()
}

/// The startup log identity line (doc 07 §8.2 `identity.ids.*` →
/// "startup log (ids)"): name, display version and the reverse-DNS root.
#[must_use = "pure reporter"]
pub fn log_startup_line(identity: &Identity) -> String {
    format!(
        "{} {} ({})",
        identity.name, identity.display_version, identity.ids.reverse_dns
    )
}

/// The CLI `--version` banner (doc 07 §8.2 `identity.name` +
/// `display_version` → "`--version` banner"): `{name} {display_version}`.
#[must_use = "pure reporter"]
pub fn cli_version_line(identity: &Identity) -> String {
    format!("{} {}", identity.name, identity.display_version)
}

/// The CLI usage header (doc 07 §8.2 `identity.name` → "usage header").
#[must_use = "pure reporter"]
pub fn cli_usage_header(identity: &Identity) -> String {
    format!("The {} command-line player", identity.name)
}

/// Derive the installer/packaging identity (doc 07 §8.2 installer
/// columns). The desktop bundle identifier keeps the committed
/// tauri.conf.json shape: `{reverse_dns}.desktop`.
#[must_use = "pure reporter"]
pub fn installer_identity(identity: &Identity) -> InstallerIdentity {
    InstallerIdentity {
        product_name: identity.name.clone(),
        desktop_bundle_identifier: format!("{}.desktop", identity.ids.reverse_dns),
        macos_bundle_id: identity.ids.macos.bundle_id.clone(),
        windows_app_user_model_id: identity.ids.windows.app_user_model_id.clone(),
        linux_appstream_id: identity.ids.linux.appstream_id.clone(),
    }
}

/// Collect the About-panel facts from identity + brand (doc 07 §8.2 About
/// columns).
#[must_use = "pure reporter"]
pub fn about_facts(identity: &Identity, brand: &Brand) -> AboutFacts {
    AboutFacts {
        product_name: identity.name.clone(),
        display_version: identity.display_version.clone(),
        publisher: identity.publisher.clone(),
        logo_svg: brand.logo_svg.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped product.toml resolves every runtime surface to the
    /// Lumen identity — the real file, not a copy (drift is loud here).
    /// The Nova-fixture proof lives in tests/rebrand/rebrand_acceptance.rs.
    #[test]
    fn the_shipped_product_toml_resolves_lumen_identity() {
        let config: crate::ProductConfig = crate::load_str(
            "product.toml",
            include_str!("../../../product.toml"),
            &crate::schema::Schema::empty_for_tests(),
        )
        .unwrap_or_else(|diags| unreachable!("{}", crate::render_all(&diags)));
        let (identity, window, brand) = (&config.identity, &config.window, &config.brand);

        assert_eq!(
            resolve_window_title(window, identity, Some("Big Buck Bunny")),
            "Big Buck Bunny — Lumen"
        );
        assert_eq!(resolve_window_title(window, identity, None), "Lumen");
        assert_eq!(log_line_prefix(identity), "lumen");
        assert_eq!(cli_version_line(identity), "Lumen 0.1.0-dev");
        assert_eq!(cli_usage_header(identity), "The Lumen command-line player");
        let installer = installer_identity(identity);
        assert_eq!(installer.product_name, "Lumen");
        assert_eq!(installer.desktop_bundle_identifier, "dev.lumen.desktop");
        assert_eq!(installer.macos_bundle_id, "dev.lumen.Lumen");
        let about = about_facts(identity, brand);
        assert_eq!(about.product_name, "Lumen");
        assert_eq!(about.logo_svg, "assets/brand/logo.svg");
    }

    /// Template variables not present in the template are inert; the two
    /// legal variables are the only substitution points (schema-validated
    /// upstream by the desktop reader, card 4.H1).
    #[test]
    fn title_resolution_substitutes_only_the_legal_variables() {
        let identity = Identity {
            name: "Nova Player".to_string(),
            short_name: "Nova".to_string(),
            publisher: "Nova Player Contributors".to_string(),
            display_version: "0.2.0-nova".to_string(),
            version_code: 1,
            description_key: "product.description".to_string(),
            ids: crate::model::IdentityIds {
                reverse_dns: "org.novaplayer".to_string(),
                windows: crate::model::WindowsIds {
                    app_user_model_id: "org.novaplayer.NovaPlayer".to_string(),
                    msi_upgrade_code: "9a6f2c48-1e5b-4c3a-8d7f-2b4e6a8c0d1f".to_string(),
                },
                macos: crate::model::MacosIds {
                    bundle_id: "org.novaplayer.NovaPlayer".to_string(),
                },
                linux: crate::model::LinuxIds {
                    appstream_id: "org.novaplayer.NovaPlayer".to_string(),
                },
                web: crate::model::WebIds {
                    app_id: "org.novaplayer.NovaPlayer".to_string(),
                },
            },
        };
        let window = Window {
            width: 1280,
            height: 720,
            min_width: 480,
            min_height: 320,
            start_maximized: false,
            remember_geometry: true,
            title_template: "{product_name}".to_string(),
            title_idle_key: "window.title.idle".to_string(),
        };
        assert_eq!(
            resolve_window_title(&window, &identity, Some("ignored — template has no slot")),
            "Nova Player"
        );
    }
}
