//! F1 acceptance — the hello-world extension (doc 06 §5.2, card
//! 7.F1): a real wasm module spawns under wasmi, negotiates the API
//! version through `lumen:v1.host`, reports its manifest through the
//! no-host-pointers buffer ABI, and logs through `lumen:v1.log` —
//! with the host verifying every step against the installed identity.

mod common;

use common::{
    I32, call, code, dropv, i32c, manifest, manifest_toml, ok, reported_manifest_toml, some,
    spawn_with_limits,
};

use plugin_host_wasm::HostError;
use plugin_host_wasm::limits::ResourceLimits;
use plugin_host_wasm::state::HostServices;

/// The hello handshake's budget: the tiny fixture limits, with the
/// descriptor cap raised to the v1 ABI default (64 KiB) so the
/// ~330-byte manifest report fits (the tiny cap is a limit-test
/// budget, not the ABI).
fn hello_limits() -> ResourceLimits {
    ResourceLimits {
        max_descriptor_bytes: 64 * 1024,
        ..ResourceLimits::test_tiny()
    }
}

/// The hello-world fixture: base imports only (host + log — always
/// granted), a greeting string and its self-reported manifest in
/// data memory.
fn hello_wasm() -> Vec<u8> {
    let report = reported_manifest_toml();
    let mut w = common::Wasm::new();
    let api_version = w.import("lumen:v1.host", "api_version", &[], &[I32]);
    let report_manifest = w.import("lumen:v1.host", "report_manifest", &[I32, I32], &[I32]);
    let log_write = w.import("lumen:v1.log", "log_write", &[I32, I32, I32], &[I32]);
    w.data(0, b"hello from the hello-world extension");
    w.data(64, report.as_bytes());
    let check_api = w.func(&[], &[I32], &[], &code(&[&call(api_version)]));
    let hello = w.func(
        &[],
        &[I32],
        &[],
        &code(&[
            &i32c(2),
            &i32c(0),
            &i32c(36),
            &call(log_write),
            &dropv(),
            &i32c(64),
            &i32c(report.len() as i32),
            &call(report_manifest),
        ]),
    );
    w.export_func("check_api", check_api);
    w.export_func("hello", hello);
    w.finish()
}

#[test]
fn hello_world_extension_runs_and_reports_manifest() {
    let wasm = hello_wasm();
    let manifest_text = manifest_toml(&wasm, "");
    let mut instance = ok(spawn_with_limits(
        &wasm,
        manifest(&manifest_text),
        HostServices::empty(),
        hello_limits(),
    ));

    // The version handshake: the guest asks, the engine answers its
    // plugin-API major (doc 06 §5.5; window [1, 1]).
    let api = ok(instance.invoke("check_api", &[]));
    assert_eq!(api, vec![i64::from(plugin_api::ENGINE_PLUGIN_API_MAJOR)]);

    // The hello call: one log record, one manifest report, both Ok.
    let status = ok(instance.invoke("hello", &[]));
    assert_eq!(status, vec![0], "hello returns the import status (Ok)");

    // The manifest report was accepted and recorded verbatim.
    let reported = some(instance.manifest_report());
    assert_eq!(reported, reported_manifest_toml());
    // ...and it is semantically the installed identity (parse +
    // compare, never byte equality with the install text).
    let parsed = plugin_api::PluginManifest::from_toml(reported)
        .unwrap_or_else(|e| unreachable!("reported manifest must parse: {e}"));
    assert_eq!(parsed.id.as_str(), "ext.acme.hello");
    assert_eq!(parsed.api_version, 1);

    // The log line crossed the sandbox through the bounded record
    // ring, level and text intact.
    let logs = instance.log_records();
    assert_eq!(logs.len(), 1);
    assert_eq!(
        logs[0],
        (2u8, "hello from the hello-world extension".to_string())
    );

    // The instance is still Ready and re-dispatchable.
    assert_eq!(
        ok(instance.invoke("hello", &[])),
        vec![0],
        "second dispatch works — nothing faulted"
    );

    // The host→guest buffer direction (doc 06 §5.2 no-host-pointers
    // model): a request payload written into the guest's own memory
    // through the bounds-checked path, read back verbatim; an
    // out-of-bounds write is the typed §9.4 denial, never UB.
    ok(instance.write_guest_memory(128, b"request-payload"));
    assert_eq!(
        ok(instance.read_guest_memory(128, 15)),
        b"request-payload".to_vec()
    );
    // A guest-visible round trip: the payload the host wrote is
    // inside the sandbox — the guest logs it through its own import.
    // (128 is beyond the fixture's data segments, so only the host
    // write put those bytes there.)
    let echo = {
        let mut w = common::Wasm::new();
        let log_write = w.import("lumen:v1.log", "log_write", &[I32, I32, I32], &[I32]);
        let f = w.func(
            &[],
            &[I32],
            &[],
            &code(&[&i32c(2), &i32c(128), &i32c(15), &call(log_write)]),
        );
        w.export_func("echo", f);
        w.finish()
    };
    let mut echoer = ok(spawn_with_limits(
        &echo,
        manifest(&manifest_toml(&echo, "")),
        HostServices::empty(),
        hello_limits(),
    ));
    ok(echoer.write_guest_memory(128, b"request-payload"));
    assert_eq!(ok(echoer.invoke("echo", &[])), vec![0]);
    assert_eq!(
        echoer.log_records(),
        vec![(2u8, "request-payload".to_string())]
    );

    // Out-of-bounds host writes are typed denials (the 1 MiB tiny
    // cap; ptr+len past it), never silent corruption.
    let err = instance
        .write_guest_memory(1024 * 1024 - 4, b"12345678")
        .err()
        .unwrap_or_else(|| unreachable!("a write past the memory cap must be refused"));
    assert!(
        matches!(err, HostError::GuestMemoryOob { ptr, len, .. } if ptr == 1024 * 1024 - 4 && len == 8),
        "typed OOB denial, got: {err}"
    );
}

#[test]
fn api_version_mismatch_is_the_typed_error() {
    let wasm = hello_wasm();
    // A manifest built for api_version 2: parses fine (plugin-api
    // only requires a positive integer) — the WINDOW check is the
    // host's, and it must be the typed mismatch, never a silent
    // load (doc 06 §5.5).
    let future = manifest_toml(&wasm, "").replace("api_version = 1", "api_version = 2");
    let err = spawn_with_limits(
        &wasm,
        manifest(&future),
        HostServices::empty(),
        hello_limits(),
    )
    .err()
    .unwrap_or_else(|| unreachable!("api_version 2 must refuse to load"));
    assert!(
        matches!(
            err,
            HostError::ApiVersion {
                found: 2,
                supported_min: 1,
                supported_max: 1
            }
        ),
        "typed window error, got: {err}"
    );
}
