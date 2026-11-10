//! The doc 06 §5.7 install-flow suite (cards 7.F3): the `.lumenext`
//! package format under adversarial inputs (§9.4
//! `plugin_malicious_manifest`), the no-auto-exec proof (§9.4
//! `plugin_no_auto_exec`, MP §6), and the user-initiated
//! install/remove lifecycle with version coexistence and tamper
//! detection (§9.5 `plugin_install_remove`).

mod common;

use common::{
    I32, code, i32c, manifest, manifest_toml, ok, package, signature_block, zip_raw, zip_stored,
};

use plugin_host_wasm::HostError;
use plugin_host_wasm::error::PackageError;
use plugin_host_wasm::install::{ExtensionStore, InstallConsent};
use plugin_host_wasm::package::SignatureStatus;
use plugin_host_wasm::state::HostServices;

/// The minimal well-formed module: one exported function, no imports.
fn tiny_wasm() -> Vec<u8> {
    let mut w = common::Wasm::new();
    let core = w.func(&[], &[I32], &[], &code(&[&i32c(42)]));
    w.export_func("core", core);
    w.finish()
}

/// A module whose START SECTION traps (`unreachable`) — executed the
/// moment anything instantiates it. The §9.4 no-auto-exec probe: if
/// ANY step other than spawn ever ran guest code, that step fails.
fn start_trap_wasm() -> Vec<u8> {
    let mut w = common::Wasm::new();
    let boom = w.func(&[], &[], &[], &code(&[&common::unreachablev()]));
    w.start(boom);
    w.finish()
}

/// The unsigned consent matching the standard fixture manifest.
fn unsigned_consent() -> InstallConsent {
    InstallConsent::confirm("Acme Tools", &[], SignatureStatus::Unsigned)
}

/// Assert a package refusal with the exact `PackageError` variant.
fn refused(pkg: &[u8], store: &ExtensionStore, consent: &InstallConsent) -> HostError {
    store
        .install(pkg, consent)
        .err()
        .unwrap_or_else(|| unreachable!("this package must be refused"))
}

// ---------------------------------------------------------------------------
// §9.4 plugin_malicious_manifest — every forged/corrupt package dies
// at a named validation step, never at a panic.
// ---------------------------------------------------------------------------

#[test]
fn plugin_malicious_manifest() {
    let tmp = common::TempDir::new("malicious");
    let store = ok(ExtensionStore::open(tmp.path()));
    let consent = unsigned_consent();
    let wasm = tiny_wasm();

    // (a) Native payload: an ELF binary where plugin.wasm belongs —
    // the `\0asm` magic is the native-code fence (doc 06 §6).
    let elf = b"\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00";
    let pkg = package(&manifest_toml(elf, ""), elf, None);
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::BadMagic { .. })
        ),
        "ELF payload refused by the wasm-magic fence"
    );

    // (b) Origin forgery: a disk manifest claiming `builtin`
    // (plugin-api-valid on its own — the host's extension store is
    // where the forgery dies, doc 06 §2.2/§9.4).
    let forged = "id = \"lumen.acme.hello\"\nkind = \"metadata-provider\"\nversion = \"0.1.0\"\n\
         api_version = 1\norigin = \"builtin\"\n\n[capabilities]\nprovides = [\"greeting\"]\n"
        .to_string();
    let pkg = package(&forged, &wasm, None);
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::OriginForgery)
        ),
        "origin forgery refused"
    );

    // (c) Provenance hash mismatch: the manifest declares the digest
    // of DIFFERENT bytes than the module it ships.
    let lying = manifest_toml(&[9u8; 32], "");
    let pkg = package(&lying, &wasm, None);
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::HashMismatch { .. })
        ),
        "hash mismatch refused"
    );

    // (c') The doc-placeholder digest form is documentation, not a
    // package: refused at install (is_full check).
    let placeholder =
        manifest_toml(&wasm, "").replace(&digest_of(&wasm), "sha256:9f2c\u{2026}(64 hex)");
    let pkg = package(&placeholder, &wasm, None);
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::HashMismatch { .. })
        ),
        "placeholder digest refused"
    );

    // (d) api_version outside the window: typed mismatch at install.
    let future = manifest_toml(&wasm, "").replace("api_version = 1", "api_version = 2");
    let pkg = package(&future, &wasm, None);
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::ApiVersion { found: 2, .. }
        ),
        "api_version 2 refused with the window error"
    );

    // (d') api_version 0 is a manifest-schema failure.
    let zero = manifest_toml(&wasm, "").replace("api_version = 1", "api_version = 0");
    let pkg = package(&zero, &wasm, None);
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::Manifest(_))
        ),
        "api_version 0 refused by manifest validation"
    );

    // (e) Refused kind: a demuxer EXTENSION (doc 06 §5.9 — byte
    // sources stay builtin in v1).
    let demuxer = format!(
        "id = \"ext.acme.hello\"\nkind = \"demuxer\"\nversion = \"0.1.0\"\napi_version = 1\n\
         origin = \"extension\"\n\n[capabilities]\nformats = [\"ogg\"]\n\n[provenance]\n\
         publisher = \"Acme Tools\"\nsource = \"https://example.com/x.lumenext\"\nhash = \"{}\"\n\
         license = \"MIT\"\n",
        digest_of(&wasm)
    );
    let pkg = package(&demuxer, &wasm, None);
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::KindRefused { ref kind } if kind == "demuxer"
        ),
        "demuxer extension refused by the v1 fence"
    );

    // (f) CRC corruption of plugin.wasm.
    let m = manifest_toml(&wasm, "");
    let pkg = zip_raw(
        &[("manifest.toml", m.as_bytes()), ("plugin.wasm", &wasm)],
        Some((1, 0, true)),
    );
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::CrcMismatch { .. })
        ),
        "CRC mismatch refused"
    );

    // (g) Compressed entry: v1 accepts stored entries only.
    let pkg = zip_raw(
        &[("manifest.toml", m.as_bytes()), ("plugin.wasm", &wasm)],
        Some((1, 8, false)),
    );
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::CompressedEntry { method: 8 })
        ),
        "deflate entry refused"
    );

    // (h) Missing required entries.
    let pkg = zip_stored(&[("manifest.toml", m.as_bytes())]);
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::MissingEntry { ref name } if name == "plugin.wasm")
        ),
        "missing plugin.wasm refused"
    );
    let pkg = zip_stored(&[("plugin.wasm", &wasm)]);
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::MissingEntry { ref name } if name == "manifest.toml")
        ),
        "missing manifest.toml refused"
    );

    // (i) Not a ZIP at all.
    let err = refused(b"this is not a zip", &store, &consent);
    assert!(
        matches!(err, HostError::Package(ref e) if matches!(**e, PackageError::NotZip)),
        "garbage refused"
    );

    // (j) Package over the 64 MiB cap (an assets blob).
    let blob = vec![0u8; 64 * 1024 * 1024 + 1];
    let pkg = zip_stored(&[
        ("manifest.toml", m.as_bytes()),
        ("plugin.wasm", &wasm),
        ("assets/blob", &blob),
    ]);
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::TooLarge { .. })
        ),
        "oversized package refused"
    );
    drop(blob);

    // (k) Signature bookkeeping mismatches. The manifest declaring a
    // signature forces the block to agree (scheme AND key); every
    // disagreement is refused.
    let signed_manifest = format!(
        "{m}[provenance.signature]\nscheme = \"ed25519-detached-v1\"\nkey = \"9f00cafe\"\n"
    );
    // (k1) block absent although declared.
    let pkg = package(&signed_manifest, &wasm, None);
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::SignatureUnparsable { .. })
        ),
        "declared signature without a block refused"
    );
    // (k2) block present although NOT declared.
    let pkg = package(
        &m,
        &wasm,
        Some(&signature_block("ed25519-detached-v1", "9f00cafe", "ff")),
    );
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::SignatureUnparsable { .. })
        ),
        "undeclared signature block refused"
    );
    // (k3) scheme disagreement.
    let pkg = package(
        &signed_manifest,
        &wasm,
        Some(&signature_block("rot13-rsa-v9", "9f00cafe", "ff")),
    );
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::SignatureUnparsable { .. })
        ),
        "scheme disagreement refused"
    );
    // (k4) key disagreement.
    let pkg = package(
        &signed_manifest,
        &wasm,
        Some(&signature_block("ed25519-detached-v1", "deadbeef", "ff")),
    );
    assert!(
        matches!(
            refused(&pkg, &store, &consent),
            HostError::Package(ref e) if matches!(**e, PackageError::SignatureUnparsable { .. })
        ),
        "key disagreement refused"
    );

    // Nothing above may have installed anything.
    assert!(
        ok(store.installed()).is_empty(),
        "every malicious package was refused before any write"
    );
}

/// The `sha256:<hex>` digest string of `wasm` (helper for surgical
/// manifest edits above).
fn digest_of(wasm: &[u8]) -> String {
    plugin_host_wasm::package::module_digest(wasm)
}

// ---------------------------------------------------------------------------
// §9.4 plugin_no_auto_exec — opening/downloading installs nothing;
// install never executes; only an explicit spawn runs guest code.
// ---------------------------------------------------------------------------

#[test]
fn plugin_no_auto_exec() {
    let tmp = common::TempDir::new("noautoexec");
    let store = ok(ExtensionStore::open(tmp.path()));
    let wasm = start_trap_wasm();
    let pkg = package(&manifest_toml(&wasm, ""), &wasm, None);

    // (a) Inspecting (the "double-open the downloaded file" path)
    // validates everything and installs NOTHING: no registry change,
    // no execution, no files under the extensions root.
    let preview = ok(store.inspect(&pkg));
    assert_eq!(preview.manifest.id.as_str(), "ext.acme.hello");
    assert!(ok(store.installed()).is_empty());
    assert!(!tmp.path().join("extensions").exists());

    // (b) Install without a matching consent is refused, still
    // nothing on disk.
    let wrong = InstallConsent::confirm("Evil Corp", &[], SignatureStatus::Unsigned);
    assert!(matches!(
        store.install(&pkg, &wrong),
        Err(HostError::ConsentRequired)
    ));
    assert!(ok(store.installed()).is_empty());

    // (c) A consented install copies files and — the proof — does
    // NOT run the trapping start section (it returns Ok; guest code
    // has no path to run at install time).
    let receipt = ok(store.install(&pkg, &unsigned_consent()));
    assert_eq!(receipt.version, "0.1.0");
    assert!(receipt.path.join("plugin.wasm").exists());

    // (d) Loading also does not execute (digest re-verified, no
    // instantiation).
    let loaded = ok(store.load("ext.acme.hello"));

    // (e) FIRST execution happens only at spawn — and the trap fires
    // exactly there, isolating to a typed instantiation error.
    let err = common::spawn(&loaded.wasm, loaded.manifest.clone(), HostServices::empty())
        .err()
        .unwrap_or_else(|| unreachable!("the trapping start section must fault the spawn"));
    assert!(
        matches!(err, HostError::Instantiation(_)),
        "start-section trap at spawn, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// §9.5 plugin_install_remove — the user-initiated lifecycle.
// ---------------------------------------------------------------------------

#[test]
fn plugin_install_remove() {
    let tmp = common::TempDir::new("lifecycle");
    let store = ok(ExtensionStore::open(tmp.path()));

    // Install v0.1.0.
    let wasm_a = tiny_wasm();
    let manifest_a = manifest_toml(&wasm_a, "");
    let pkg_a = package(&manifest_a, &wasm_a, None);
    let receipt = ok(store.install(&pkg_a, &unsigned_consent()));
    assert_eq!(receipt.id, "ext.acme.hello");
    assert_eq!(receipt.version, "0.1.0");
    assert_eq!(receipt.signature, SignatureStatus::Unsigned);
    assert!(receipt.permissions.is_empty());
    assert!(receipt.path.ends_with("ext.acme.hello/0.1.0"));
    assert!(receipt.path.join("manifest.toml").exists());
    assert!(receipt.path.join("plugin.wasm").exists());

    // Same version, same bytes: idempotent.
    ok(store.install(&pkg_a, &unsigned_consent()));

    // Same version, DIFFERENT bytes: refused (the recorded digest
    // belongs to what the user consented to).
    let wasm_c = start_trap_wasm();
    let manifest_c = manifest_toml(&wasm_c, "");
    let pkg_c = package(&manifest_c, &wasm_c, None);
    assert!(matches!(
        store.install(&pkg_c, &unsigned_consent()),
        Err(HostError::Layout(_))
    ));

    // Version coexistence: install v0.2.0, load picks the highest.
    let wasm_b = {
        let mut w = common::Wasm::new();
        let core = w.func(&[], &[I32], &[], &code(&[&i32c(43)]));
        w.export_func("core", core);
        w.finish()
    };
    let manifest_b =
        manifest_toml(&wasm_b, "").replace("version = \"0.1.0\"", "version = \"0.2.0\"");
    ok(store.install(&package(&manifest_b, &wasm_b, None), &unsigned_consent()));
    let loaded = ok(store.load("ext.acme.hello"));
    assert_eq!(loaded.manifest.version.to_string(), "0.2.0");

    // Remove the newest version: load falls back to 0.1.0.
    ok(store.remove("ext.acme.hello", Some("0.2.0"), &unsigned_consent()));
    let loaded = ok(store.load("ext.acme.hello"));
    assert_eq!(loaded.manifest.version.to_string(), "0.1.0");

    // Tamper detection: rewriting the installed module refuses the
    // next load (digest re-verified on EVERY load).
    let module_path = loaded.path.join("plugin.wasm");
    let mut tampered = loaded.wasm.clone();
    tampered.push(0x00);
    ok(std::fs::write(&module_path, &tampered).map_err(|e| e.to_string()));
    assert!(
        matches!(
            store.load("ext.acme.hello"),
            Err(HostError::Package(ref e))
                if matches!(**e, PackageError::InstalledHashMismatch { .. })
        ),
        "tampered install refused at load"
    );

    // Remove the whole extension: gone, and further removes/loads are
    // typed NotInstalled.
    ok(store.remove("ext.acme.hello", None, &unsigned_consent()));
    assert!(ok(store.installed()).is_empty());
    assert!(matches!(
        store.remove("ext.acme.hello", None, &unsigned_consent()),
        Err(HostError::NotInstalled { .. })
    ));
    assert!(matches!(
        store.load("ext.acme.hello"),
        Err(HostError::NotInstalled { .. })
    ));

    // The signed path: a coherent manifest+block pair installs with
    // the honest Unverified status — and only a consent that SAID
    // "unverified" covers it (the explicit-consent moment for
    // unverified signatures, doc 06 §5.7 step 3 + §10 Q5).
    let wasm = tiny_wasm();
    let m = manifest_toml(&wasm, "");
    let signed_manifest = format!(
        "{m}[provenance.signature]\nscheme = \"ed25519-detached-v1\"\nkey = \"9f00cafe\"\n"
    );
    let pkg = package(
        &signed_manifest,
        &wasm,
        Some(&signature_block(
            "ed25519-detached-v1",
            "9f00cafe",
            "deadbeef01",
        )),
    );
    // A consent claiming Unsigned does not cover an Unverified
    // package — the posture the user saw must match the package.
    assert!(matches!(
        store.install(&pkg, &unsigned_consent()),
        Err(HostError::ConsentRequired)
    ));
    let unverified = SignatureStatus::Unverified {
        suite: "ed25519-detached-v1".to_string(),
    };
    let receipt = ok(store.install(
        &pkg,
        &InstallConsent::confirm("Acme Tools", &[], unverified),
    ));
    assert_eq!(
        receipt.signature,
        SignatureStatus::Unverified {
            suite: "ed25519-detached-v1".to_string()
        },
        "v1 never claims `verified` — the verifier is a Lead-approved crypto ADR away"
    );
    // And the loaded copy carries the same honest status.
    let loaded = ok(store.load("ext.acme.hello"));
    assert_eq!(loaded.signature, receipt.signature);
}

// ---------------------------------------------------------------------------
// Sanity: the tiny module actually runs (the lifecycle fixtures are
// real wasm, not inert bytes).
// ---------------------------------------------------------------------------

#[test]
fn fixture_module_executes() {
    let wasm = tiny_wasm();
    let mut instance = ok(common::spawn(
        &wasm,
        manifest(&manifest_toml(&wasm, "")),
        HostServices::empty(),
    ));
    assert_eq!(ok(instance.invoke("core", &[])), vec![42]);
}
