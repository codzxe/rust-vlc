//! §7 conformance kit over the two example extensions (card 7.G1,
//! issue #46; doc 06 §7, §2.3 (c)/(d)): the committed
//! `plugins/extensions/{metadata-provider,ui-panel}` artifacts are
//! driven through the REAL developer flow — regenerate-check, the
//! `lumenext package`/`inspect` CLI, install with consent, load,
//! spawn under production limits, and dispatch — and every manifest
//! claim is demonstrated (§7.2 capability honesty: a claim the
//! example cannot show is a manifest lie that fails this suite).
//!
//! What each kit section proves, per doc 06 §7:
//!
//! - §7.0 drift: the committed `plugin.wasm` blobs are exactly what
//!   `lumenext-examples` regenerates (no silent hand-edits — the
//!   provenance digest in every packaged manifest pins them).
//! - §7.1 functional: the metadata example answers the command-layer
//!   `extract(path)` request with the sidecar's tag text (corpus: one
//!   real generated sidecar file, expectation: its exact bytes).
//! - §7.2 honesty: `provides = ["tags"]` and `surfaces =
//!   ["side-panel"]` are exercised, not just declared.
//! - §7.3 error paths: missing/traversing paths produce typed guest
//!   statuses (`E_PERM`), never panics; the instance stays `Ready`.
//! - §7.4 limits: the whole run is under the production
//!   `ResourceLimits::for_kind` ceilings (memory class by kind, fuel,
//!   I/O transfer caps) — a breach would be a typed fault, not a
//!   crash (the dedicated breach fixtures live in the §9.4/§9.5
//!   suites; here the examples simply run inside the ceilings).
//! - §7.6 api matrix (v1 slice): the examples load at api_version 1
//!   and the same module refuses with the typed window error when
//!   the manifest lies about a future major.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ok, package, some};
use plugin_api::PluginManifest;
use plugin_host_wasm::HostError;
use plugin_host_wasm::fsjail::PathJail;
use plugin_host_wasm::install::{ExtensionStore, InstallConsent};
use plugin_host_wasm::instance::WasmInstance;
use plugin_host_wasm::limits::ResourceLimits;
use plugin_host_wasm::package::{SignatureStatus, module_digest, stamp_manifest_hash};
use plugin_host_wasm::state::HostServices;

// ---------------------------------------------------------------------------
// The committed artifacts under test
// ---------------------------------------------------------------------------

/// The repo root (crates/plugin-host-wasm/../..), canonicalized —
/// the jail and the generator both need real paths.
fn repo_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    ok(std::fs::canonicalize(&root)
        .map_err(|e| format!("cannot canonicalize {}: {e}", root.display())))
}

/// One example extension's committed source pair.
struct Example {
    /// Directory name under plugins/extensions/.
    name: &'static str,
}

impl Example {
    fn dir(&self) -> PathBuf {
        repo_root()
            .join("plugins")
            .join("extensions")
            .join(self.name)
    }

    /// The committed module bytes.
    fn module(&self) -> Vec<u8> {
        ok(std::fs::read(self.dir().join("plugin.wasm"))
            .map_err(|e| format!("reading {}/plugin.wasm: {e}", self.dir().display())))
    }

    /// The committed manifest text (placeholder provenance hash).
    fn manifest_text(&self) -> String {
        ok(std::fs::read_to_string(self.dir().join("manifest.toml"))
            .map_err(|e| format!("reading {}/manifest.toml: {e}", self.dir().display())))
    }

    /// The installable package: the manifest with its provenance hash
    /// stamped to the module's real digest (the same
    /// `stamp_manifest_hash` library call `lumenext package` runs),
    /// stored-ZIPed by the test-side writer.
    fn lumenext(&self) -> Vec<u8> {
        let module = self.module();
        let digest = module_digest(&module);
        let stamped =
            ok(stamp_manifest_hash(&self.manifest_text(), &digest).map_err(|e| e.to_string()));
        package(&stamped, &module, None)
    }

    /// The stamped manifest, parsed (the identity install will accept).
    fn manifest(&self) -> PluginManifest {
        let module = self.module();
        let digest = module_digest(&module);
        let stamped =
            ok(stamp_manifest_hash(&self.manifest_text(), &digest).map_err(|e| e.to_string()));
        ok(PluginManifest::from_toml(&stamped).map_err(|e| e.to_string()))
    }
}

const METADATA: Example = Example {
    name: "metadata-provider",
};
const PANEL: Example = Example { name: "ui-panel" };

/// Install + load an example into a fresh store under `root`, and
/// spawn it under the production limits for its kind with the given
/// host services — the kit's load path (doc 06 §5.7 steps 2–5).
fn spawn_installed(
    example: &Example,
    root: &Path,
    services: HostServices,
) -> Result<WasmInstance, HostError> {
    let store = ok(ExtensionStore::open(root));
    let manifest = example.manifest();
    let consent = InstallConsent::confirm(
        "Example Extensions",
        &manifest.permissions.granted(),
        SignatureStatus::Unsigned,
    );
    let receipt = ok(store.install(&example.lumenext(), &consent));
    assert_eq!(receipt.id, manifest.id.as_str());
    let loaded = ok(store.load(manifest.id.as_str()));
    WasmInstance::spawn(
        &plugin_host_wasm::instance::engine(),
        &loaded.wasm,
        loaded.manifest,
        services,
        ResourceLimits::for_kind(manifest.kind),
        plugin_host_wasm::state::SecurityLedger::new(),
    )
}

// ---------------------------------------------------------------------------
// The guest memory layout — mirrors src/bin/lumenext-examples.rs
// (the generator is the single source of these addresses; a change
// there without here fails this suite loudly)
// ---------------------------------------------------------------------------

/// `fs_read` lands the sidecar bytes here.
const RESULT_AT: u64 = 4096;
/// `fs_read`'s length out-parameter (u32, little-endian).
const OUT_LEN_AT: u64 = 2052;
/// Where the host writes command-layer request payloads (well past
/// the guest's data segments, inside its first page).
const REQUEST_AT: u64 = 8192;

/// Read the little-endian u32 the host/guest left at `at`.
fn guest_u32(instance: &WasmInstance, at: u64) -> u32 {
    let bytes = ok(instance.read_guest_memory(at, 4));
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

// ---------------------------------------------------------------------------
// §7.0 — committed-blob drift
// ---------------------------------------------------------------------------

#[test]
fn committed_example_modules_match_regeneration() {
    // The generator binary is the same emitter the fixtures use; the
    // committed blobs must be byte-identical to what it regenerates.
    let out = ok(Command::new(env!("CARGO_BIN_EXE_lumenext-examples"))
        .arg("--check")
        .arg(repo_root())
        .output()
        .map_err(|e| e.to_string()));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "lumenext-examples --check failed:\n{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("all committed modules match regeneration"),
        "unexpected --check output: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// §7.1/§7.2/§7.3 — the metadata-provider example
// ---------------------------------------------------------------------------

#[test]
fn metadata_provider_example_passes_conformance() {
    let tmp = common::TempDir::new("conformance-metadata");
    // The user-granted media directory: the fs-sandboxed-read jail
    // root, holding one real generated sidecar (the kit's corpus).
    let media = tmp.path().join("media");
    ok(std::fs::create_dir_all(&media).map_err(|e| format!("mkdir {}: {e}", media.display())));
    let sidecar = "title = \"Wave Field\"\nartist = \"Example\"\nalbum = \"Signal Paths\"\n";
    ok(std::fs::write(media.join("sample-1.tags"), sidecar)
        .map_err(|e| format!("write sidecar: {e}")));

    let jail = ok(PathJail::new([&media]));
    let mut instance = ok(spawn_installed(
        &METADATA,
        tmp.path(),
        HostServices {
            jail: Some(jail),
            ..HostServices::empty()
        },
    ));
    let manifest = instance.manifest().clone();

    // init: api handshake (returns the negotiated major 1), honest
    // manifest report, ready log line.
    assert_eq!(
        ok(instance.invoke("init", &[])),
        vec![i64::from(manifest.api_version)],
        "init returns the negotiated api major"
    );
    let reported = some(instance.manifest_report());
    let parsed = ok(PluginManifest::from_toml(reported).map_err(|e| e.to_string()));
    assert_eq!(parsed.id.as_str(), "ext.example.metadata");
    assert_eq!(parsed.kind, manifest.kind);
    assert!(
        instance
            .log_records()
            .iter()
            .any(|(_, line)| line == "metadata-provider example ready")
    );

    // The command-layer request/response (doc 06 §5.2): the host
    // writes the media path into the GUEST's memory, dispatches
    // `extract(ptr, len)`, and reads the tags out of the guest's
    // result buffer — the no-host-pointers model both directions.
    let path = b"sample-1.tags";
    ok(instance.write_guest_memory(REQUEST_AT, path));
    assert_eq!(
        ok(instance.invoke(
            "extract",
            &[i64::from(REQUEST_AT as i32), path.len() as i64]
        )),
        vec![0],
        "extract returns the Ok import status"
    );
    let len = guest_u32(&instance, OUT_LEN_AT) as usize;
    assert_eq!(len, sidecar.len(), "the sidecar's full length crossed back");
    let tags = ok(instance.read_guest_memory(RESULT_AT, len as u64));
    assert_eq!(
        String::from_utf8_lossy(&tags),
        sidecar,
        "capability honesty: provides = [\"tags\"] — the tag text, verbatim"
    );

    // The import trace shows the jailed fs round trip, all Ok.
    let trace = instance.import_trace();
    assert!(
        trace
            .iter()
            .any(|(import, status)| import.ends_with("fs::fs_open") && *status == 0)
    );
    assert!(
        trace
            .iter()
            .any(|(import, status)| import.ends_with("fs::fs_read") && *status == 0)
    );
    assert!(
        trace
            .iter()
            .any(|(import, status)| import.ends_with("fs::fs_close") && *status == 0)
    );

    // §7.3 error paths — typed statuses, no panic, instance Ready.
    // A path that does not exist inside the jail is the jail's
    // denial (E_PERM: the jail refuses to confirm what exists
    // outside the granted roots — no information leak).
    let missing = b"no-such-file.tags";
    ok(instance.write_guest_memory(REQUEST_AT, missing));
    assert_eq!(
        ok(instance.invoke(
            "extract",
            &[i64::from(REQUEST_AT as i32), missing.len() as i64]
        )),
        vec![1],
        "missing sidecar: the typed E_PERM guest status"
    );
    // Traversal: a path that escapes the jail root is denied the
    // same typed way (both land on the session ledger).
    let escape = b"../../../etc/passwd";
    ok(instance.write_guest_memory(REQUEST_AT, escape));
    assert_eq!(
        ok(instance.invoke(
            "extract",
            &[i64::from(REQUEST_AT as i32), escape.len() as i64]
        )),
        vec![1],
        "jail escape attempt: the typed E_PERM guest status"
    );
    // Still dispatchable after the error paths (bounded recovery —
    // the extension is not faulted by honest denials).
    ok(instance.write_guest_memory(REQUEST_AT, path));
    assert_eq!(
        ok(instance.invoke(
            "extract",
            &[i64::from(REQUEST_AT as i32), path.len() as i64]
        )),
        vec![0],
        "a later extract still succeeds — nothing faulted"
    );
    assert_eq!(
        format!("{:?}", instance.status()),
        "Ready",
        "the instance survived every error path"
    );
    // The two denials were recorded as security events (shapes, not
    // contents) — under the session escalation threshold.
    assert_eq!(instance.ledger().escalation_count(), 2);
}

// ---------------------------------------------------------------------------
// §7.1/§7.2 — the ui-panel example
// ---------------------------------------------------------------------------

#[test]
fn ui_panel_example_passes_conformance() {
    let tmp = common::TempDir::new("conformance-panel");
    let mut instance = ok(spawn_installed(&PANEL, tmp.path(), HostServices::empty()));
    let manifest = instance.manifest().clone();

    // init: handshake, honest manifest, ONE registered panel.
    assert_eq!(
        ok(instance.invoke("init", &[])),
        vec![i64::from(manifest.api_version)],
        "init returns the negotiated api major"
    );
    let reported = some(instance.manifest_report());
    let parsed = ok(PluginManifest::from_toml(reported).map_err(|e| e.to_string()));
    assert_eq!(parsed.id.as_str(), "ext.example.panel");
    assert_eq!(parsed.kind, manifest.kind);

    let panels = instance.panels();
    assert_eq!(panels.len(), 1, "exactly one panel registered");
    // The descriptor is a declarative JSON object (doc 06 §5.8:
    // panels exchange data through the command layer only). v1 has
    // no JSON parser in this crate's dependency closure, so the kit
    // asserts the semantic fields by their serialized forms —
    // honest containment, not a fabricated parse.
    let descriptor = &panels[0].descriptor;
    assert!(
        descriptor.contains("\"panel_id\":\"main\""),
        "panel id in the descriptor: {descriptor}"
    );
    assert!(
        descriptor.contains("\"surface\":\"side-panel\""),
        "capability honesty: surfaces = [\"side-panel\"] — the registered surface, \
         verbatim: {descriptor}"
    );

    // The command-layer event path: the host writes one event payload
    // into the guest's memory and dispatches `on_command(ptr, len)`;
    // the observable acknowledgment is the bounded log record. (The
    // em-dash is escaped \xE2\x80\x94 — the guest payload is raw
    // bytes, and byte-string literals must stay ASCII.)
    let event = b"now-playing-changed: Wave Field \xE2\x80\x94 Example";
    ok(instance.write_guest_memory(REQUEST_AT, event));
    assert_eq!(
        ok(instance.invoke(
            "on_command",
            &[i64::from(REQUEST_AT as i32), event.len() as i64]
        )),
        vec![0],
        "on_command returns the Ok import status"
    );
    assert!(
        instance
            .log_records()
            .iter()
            .any(|(_, line)| line.as_str() == "now-playing-changed: Wave Field \u{2014} Example")
    );

    // Re-dispatchable; nothing faulted.
    ok(instance.write_guest_memory(REQUEST_AT, event));
    assert_eq!(
        ok(instance.invoke(
            "on_command",
            &[i64::from(REQUEST_AT as i32), event.len() as i64]
        )),
        vec![0]
    );
    assert_eq!(format!("{:?}", instance.status()), "Ready");
}

// ---------------------------------------------------------------------------
// §7.6 — api-version compatibility (v1 slice of the matrix)
// ---------------------------------------------------------------------------

#[test]
fn examples_refuse_future_api_major_with_the_typed_error() {
    // The same committed modules, a manifest lying about api_version
    // 2: the load must refuse with the typed window error (never a
    // silent best-effort load) — for both examples.
    for example in [&METADATA, &PANEL] {
        let module = example.module();
        let digest = module_digest(&module);
        let stamped =
            ok(stamp_manifest_hash(&example.manifest_text(), &digest).map_err(|e| e.to_string()));
        let future = stamped.replace("api_version = 1", "api_version = 2");
        let manifest = ok(PluginManifest::from_toml(&future).map_err(|e| e.to_string()));
        let err = WasmInstance::spawn(
            &plugin_host_wasm::instance::engine(),
            &module,
            manifest,
            HostServices::empty(),
            ResourceLimits::for_kind(
                PluginManifest::from_toml(&stamped)
                    .unwrap_or_else(|e| unreachable!("{e}"))
                    .kind,
            ),
            plugin_host_wasm::state::SecurityLedger::new(),
        )
        .err()
        .unwrap_or_else(|| unreachable!("api_version 2 must refuse to load"));
        assert!(
            matches!(
                err,
                HostError::ApiVersion {
                    found: 2,
                    supported_min: 1,
                    supported_max: 1,
                }
            ),
            "{}: typed window error, got: {err}",
            example.name
        );
    }
}

// ---------------------------------------------------------------------------
// The developer CLI round trip — the exact commands the walkthrough
// documents (docs/plugins-examples.md), executed for real
// ---------------------------------------------------------------------------

#[test]
fn lumenext_cli_round_trip_packages_and_inspects_both_examples() {
    let tmp = common::TempDir::new("conformance-cli");
    for example in [&METADATA, &PANEL] {
        let dir = example.dir();
        let out_file = tmp.path().join(format!("{}.lumenext", example.name));
        // package (into the temp dir — the source tree stays clean)
        let package = ok(Command::new(env!("CARGO_BIN_EXE_lumenext"))
            .arg("package")
            .arg(&dir)
            .arg("--out")
            .arg(&out_file)
            .output()
            .map_err(|e| e.to_string()));
        assert!(
            package.status.success(),
            "lumenext package {example_name} failed:\n{out}{err}",
            example_name = example.name,
            out = String::from_utf8_lossy(&package.stdout),
            err = String::from_utf8_lossy(&package.stderr)
        );
        let package_text = String::from_utf8_lossy(&package.stdout).to_string();
        assert!(package_text.contains("packaged"), "{package_text}");
        // inspect: the same structural chain the installer runs
        let inspect = ok(Command::new(env!("CARGO_BIN_EXE_lumenext"))
            .arg("inspect")
            .arg(&out_file)
            .output()
            .map_err(|e| e.to_string()));
        assert!(
            inspect.status.success(),
            "lumenext inspect {example_name} failed:\n{out}{err}",
            example_name = example.name,
            out = String::from_utf8_lossy(&inspect.stdout),
            err = String::from_utf8_lossy(&inspect.stderr)
        );
        let inspect_text = String::from_utf8_lossy(&inspect.stdout).to_string();
        assert!(inspect_text.contains("VALID"), "{inspect_text}");
        assert!(inspect_text.contains("sha256 matches"), "{inspect_text}");
        // And the CLI-packaged artifact is installable by the real
        // store (same bytes the kit installed above, through the
        // developer tool's own writer).
        let bytes =
            ok(std::fs::read(&out_file).map_err(|e| format!("read {}: {e}", out_file.display())));
        let store = ok(ExtensionStore::open(tmp.path().join("store")));
        let manifest = example.manifest();
        let consent = InstallConsent::confirm(
            "Example Extensions",
            &manifest.permissions.granted(),
            SignatureStatus::Unsigned,
        );
        let receipt = ok(store.install(&bytes, &consent));
        assert_eq!(receipt.id, manifest.id.as_str());
        assert_eq!(receipt.version, manifest.version.to_string());
    }
}
