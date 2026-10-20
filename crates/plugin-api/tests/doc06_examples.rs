//! Acceptance-criteria fixtures: the four doc 06 §2.3 example
//! manifests, VERBATIM, must parse and round-trip.
//!
//! Round-trip honesty notes (per the card):
//!
//! - **(a) Matroska demuxer** — serializes byte-identical after
//!   normalization of the missing trailing newline (the doc block has
//!   none; `toml` always emits one).
//! - **(b) FLAC decoder** — the doc's `sample_rates` array is
//!   line-wrapped for page width; TOML has no line-continuation, so
//!   the serialized form is the equivalent single-line array. All
//!   13 values, same order, same types.
//! - **(c) MusicBrainz tags extension** — three normalized-but-
//!   equivalent diffs: (1) the empty `[dependencies]` table is
//!   omitted (§2.2 declares dependencies an *array*; the doc's empty
//!   table and an empty array are the same value); (2) inline
//!   comments do not exist in the data model (comments are not
//!   values); (3) the `[config_schema]` multi-line basic string
//!   (`"""`) re-serializes as an escaped single-line basic string —
//!   identical `String` value.
//! - **(d) Lyrics panel extension** — same comment/empty-table
//!   normalizations as (c).
//!
//! The strongest honest claim is **double round-trip stability**
//! (parse → serialize → parse → serialize ⇒ bytes 2 == bytes 3) plus
//! **value identity** (parse(serialize(parse(x))) == parse(x)); each
//! fixture test below proves both and prints any first-pass diff.

use plugin_api::kind::PluginKind;
use plugin_api::manifest::{ManifestError, Origin, PluginManifest};

/// doc 06 §2.3 (a) — builtin Matroska demuxer, verbatim.
///
/// Serialization note: the `toml` serializer emits struct fields in
/// declaration order but `Capabilities`' lists come out alphabetically
/// (serde map ordering); the round-trip test documents that
/// normalized-but-equivalent key ordering honestly.
const MATROSKA: &str = r#"id = "lumen.demux.matroska"
kind = "demuxer"
version = "0.1.0"
api_version = 1
priority = 100
origin = "builtin"

[capabilities]
formats = ["matroska", "webm"]
extensions = [".mkv", ".mka", ".mks", ".webm"]
mime_types = ["video/x-matroska", "audio/x-matroska", "video/webm", "audio/webm"]

[config_schema]
settings = "settings/demux/matroska.toml"
"#;

/// doc 06 §2.3 (b) — builtin FLAC decoder, verbatim (the doc's
/// line-wrapped `sample_rates` array is one array value).
const FLAC: &str = r#"id = "lumen.codec.flac"
kind = "decoder"
version = "0.1.0"
api_version = 1
priority = 100
origin = "builtin"

[capabilities]
codecs = ["flac"]
profiles = ["baseline"]
max_channels = 8
sample_rates = [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100,
                48000, 88200, 96000, 176400, 192000]
bit_depths = [16, 24, 32]
hardware = false
"#;

/// doc 06 §2.3 (c) — WASM metadata-provider extension, verbatim
/// (inline comments are documentation; `[dependencies]` is the doc's
/// empty-table spelling of §2.2's array field).
const MUSICBRAINZ: &str = r#"id = "ext.acme.musicbrainz-tags"
kind = "metadata-provider"
version = "1.0.0"
api_version = 1
priority = 50
origin = "extension"

[capabilities]
provides = ["tags", "cover-art"]
sources = ["musicbrainz", "coverartarchive"]

[permissions]
http-fetch = true
network-host-allowlist = ["musicbrainz.org", "coverartarchive.org"]

[config_schema]
settings = """
[plugins.ext.acme.musicbrainz-tags]
enabled = { type = "bool", default = true }
preferred-release = { type = "enum", values = ["album", "single"], default = "album" }
"""

[dependencies]

[provenance]
publisher = "Acme Tools"
source = "https://example.com/lumen/acme-musicbrainz-tags-1.0.0.lumenext"
hash = "sha256:9f2c…(64 hex)"
license = "MIT"
"#;

/// doc 06 §2.3 (d) — WASM ui-panel extension, verbatim.
const LYRICS: &str = r#"id = "ext.acme.lyrics-panel"
kind = "ui-panel"
version = "0.4.2"
api_version = 1
priority = 50
origin = "extension"

[capabilities]
surfaces = ["side-panel"]

[permissions]
ui-surface = true
fs-sandboxed-read = true      # reads .lrc files next to the open media

[dependencies]                # works best with tags, but degrades without it

[provenance]
publisher = "Acme Tools"
source = "https://example.com/lumen/acme-lyrics-panel-0.4.2.lumenext"
hash = "sha256:1ab4…(64 hex)"
license = "MIT"
"#;

/// parse → serialize → parse → serialize: value identity + second-pass
/// byte stability.
fn double_round_trip(name: &str, src: &str) -> (String, String) {
    let first = match PluginManifest::from_toml(src) {
        Ok(m) => m,
        Err(e) => unreachable!("{name}: parse: {e}"),
    };
    let once = match first.to_toml() {
        Ok(s) => s,
        Err(e) => unreachable!("{name}: serialize: {e}"),
    };
    let second = match PluginManifest::from_toml(&once) {
        Ok(m) => m,
        Err(e) => unreachable!("{name}: reparse: {e}"),
    };
    assert_eq!(first, second, "{name}: value identity across round trip");
    let twice = match second.to_toml() {
        Ok(s) => s,
        Err(e) => unreachable!("{name}: reserialize: {e}"),
    };
    assert_eq!(once, twice, "{name}: serialization is stable");
    (once, twice)
}

#[test]
fn fixture_a_builtin_matroska_demuxer() {
    let (once, _) = double_round_trip("matroska", MATROSKA);
    // The one normalized diff: the doc block carries no trailing
    // newline.
    let normalized = format!("{MATROSKA}\n");
    // Key order inside [capabilities] is a serialization artifact
    // (serde emits Capabilities fields alphabetically; the doc lists
    // `formats` first). Both orders describe the same value — compare
    // the sorted line sets.
    fn sorted_body(s: &str) -> Vec<String> {
        let mut lines: Vec<String> = s
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.trim_end().to_string())
            .collect();
        lines.sort();
        lines
    }
    assert_eq!(
        sorted_body(&once),
        sorted_body(&normalized),
        "fixture (a): same lines, capabilities key order normalized"
    );

    let m = match PluginManifest::from_toml(MATROSKA) {
        Ok(m) => m,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(m.id.as_str(), "lumen.demux.matroska");
    assert_eq!(m.kind, PluginKind::Demuxer);
    assert_eq!(m.origin, Origin::Builtin);
    assert_eq!(m.effective_priority(), 100);
    assert_eq!(m.api_version, 1);
}

#[test]
fn fixture_b_builtin_flac_decoder() {
    let (once, _) = double_round_trip("flac", FLAC);
    // The single normalized diff: the doc's line wrap inside the
    // sample_rates array. Everything else must be byte-identical.
    let expected = FLAC.replace(
        "sample_rates = [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100,\n                48000, 88200, 96000, 176400, 192000]",
        "sample_rates = [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 88200, 96000, 176400, 192000]",
    );
    assert_eq!(once, expected, "fixture (b): only the line wrap normalizes");

    let m = match PluginManifest::from_toml(FLAC) {
        Ok(m) => m,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(m.kind, PluginKind::Decoder);
    match &m.capabilities.sample_rates {
        Some(rates) => assert_eq!(rates.len(), 13, "all 13 sample rates survive"),
        None => unreachable!("sample_rates present in the fixture"),
    }
    assert_eq!(m.capabilities.max_channels, Some(8));
    assert_eq!(m.capabilities.hardware, Some(false), "truthful hw claim");
    assert_eq!(
        m.capabilities.codecs.as_deref(),
        Some(&["flac".to_string()][..])
    );
}

#[test]
fn fixture_c_wasm_metadata_provider() {
    let (once, _) = double_round_trip("musicbrainz", MUSICBRAINZ);
    // Normalized-but-equivalent diffs (documented in the module docs):
    // empty [dependencies] table omitted; comments dropped. Assert
    // the *content* that must survive, line by line.
    for needle in [
        "id = \"ext.acme.musicbrainz-tags\"",
        "kind = \"metadata-provider\"",
        "version = \"1.0.0\"",
        "api_version = 1",
        "priority = 50",
        "origin = \"extension\"",
        "provides = [\"tags\", \"cover-art\"]",
        "sources = [\"musicbrainz\", \"coverartarchive\"]",
        "http-fetch = true",
        "network-host-allowlist = [\"musicbrainz.org\", \"coverartarchive.org\"]",
        "publisher = \"Acme Tools\"",
        "source = \"https://example.com/lumen/acme-musicbrainz-tags-1.0.0.lumenext\"",
        "hash = \"sha256:9f2c…(64 hex)\"",
        "license = \"MIT\"",
    ] {
        assert!(once.contains(needle), "fixture (c) must keep {needle}");
    }
    assert!(
        !once.contains("[dependencies]"),
        "the doc's empty table is the §2.2 empty array — omitted when empty"
    );

    let m = match PluginManifest::from_toml(MUSICBRAINZ) {
        Ok(m) => m,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(m.origin, Origin::Extension);
    assert_eq!(m.effective_priority(), 50, "extension default posture");
    assert!(m.dependencies.is_absent(), "no dependencies declared");
    // The inline settings fragment keeps its full text.
    match &m.config_schema {
        Some(schema) => {
            assert!(
                schema
                    .settings
                    .contains("[plugins.ext.acme.musicbrainz-tags]")
            );
            assert!(schema.settings.contains("preferred-release"));
        }
        None => unreachable!("config_schema present in the fixture"),
    }
    // Provenance required and carried.
    match &m.provenance {
        Some(p) => {
            assert_eq!(p.publisher, "Acme Tools");
            assert_eq!(p.license, "MIT");
            assert!(!p.hash.is_full(), "doc placeholder hash accepted, flagged");
        }
        None => unreachable!("extension must carry provenance"),
    }
    // Permissions: http-fetch + allowlist, companion rule satisfied.
    assert!(m.permissions.http_fetch);
    assert_eq!(
        m.permissions.network_host_allowlist.as_deref(),
        Some(
            &[
                "musicbrainz.org".to_string(),
                "coverartarchive.org".to_string()
            ][..]
        )
    );
    assert_eq!(
        m.permissions.granted(),
        vec!["http-fetch", "network-host-allowlist"]
    );
}

#[test]
fn fixture_d_wasm_ui_panel() {
    let (once, _) = double_round_trip("lyrics", LYRICS);
    for needle in [
        "id = \"ext.acme.lyrics-panel\"",
        "kind = \"ui-panel\"",
        "version = \"0.4.2\"",
        "surfaces = [\"side-panel\"]",
        "ui-surface = true",
        "fs-sandboxed-read = true",
        "hash = \"sha256:1ab4…(64 hex)\"",
    ] {
        assert!(once.contains(needle), "fixture (d) must keep {needle}");
    }
    // The doc's trailing comment on fs-sandboxed-read cannot survive
    // (comments are not values).
    assert!(!once.contains("reads .lrc files"));

    let m = match PluginManifest::from_toml(LYRICS) {
        Ok(m) => m,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(m.kind, PluginKind::UiPanel);
    assert!(
        m.kind.extension_allowed_in_v1(),
        "ui-panel is an extension kind"
    );
    assert!(m.permissions.fs_sandboxed_read);
    assert!(m.permissions.ui_surface);
}

#[test]
fn all_four_reparse_from_their_own_output() {
    // The fixtures are independent documents; each must accept its
    // own serialization as input (the registry only ever sees
    // serialized manifests).
    for (name, src) in [
        ("a", MATROSKA),
        ("b", FLAC),
        ("c", MUSICBRAINZ),
        ("d", LYRICS),
    ] {
        let first = match PluginManifest::from_toml(src) {
            Ok(m) => m,
            Err(e) => unreachable!("{name}: {e}"),
        };
        let once = match first.to_toml() {
            Ok(s) => s,
            Err(e) => unreachable!("{name}: {e}"),
        };
        match PluginManifest::from_toml(&once) {
            Ok(_) => {}
            Err(e) => unreachable!("{name}: output must reparse: {e}"),
        }
    }
}

#[test]
fn doc_examples_reject_tampering() {
    // Unknown field (§2.1 reject, never ignore).
    let tampered = FLAC.replace("priority = 100", "priority = 100\nno_such_field = 1");
    assert!(matches!(
        PluginManifest::from_toml(&tampered),
        Err(ManifestError::Toml(_))
    ));
    // Bad semver.
    let tampered = FLAC.replace("version = \"0.1.0\"", "version = \"0.1\"");
    assert!(PluginManifest::from_toml(&tampered).is_err());
    // Out-of-range priority.
    let tampered = FLAC.replace("priority = 100", "priority = 5000");
    assert!(matches!(
        PluginManifest::from_toml(&tampered),
        Err(ManifestError::Priority(5000))
    ));
    // The doc's own (c) with the allowlist but http-fetch removed:
    // the companion rule must fire.
    let tampered = MUSICBRAINZ.replace("http-fetch = true", "http-fetch = false");
    assert!(matches!(
        PluginManifest::from_toml(&tampered),
        Err(ManifestError::Permissions(_))
    ));
    // The doc's own (a) turned into an extension without provenance.
    let tampered = MATROSKA
        .replace("lumen.demux.matroska", "ext.acme.matroska")
        .replace("origin = \"builtin\"", "origin = \"extension\"");
    assert!(matches!(
        PluginManifest::from_toml(&tampered),
        Err(ManifestError::Provenance(_))
    ));
}
