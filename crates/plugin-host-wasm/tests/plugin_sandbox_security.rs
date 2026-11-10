//! The doc 06 §9.4 sandbox & security suite — every escalation test
//! DENIED, at the real wasm boundary (cards 7.F1–7.F2):
//!
//! - `plugin_sandbox_escape`      — out-of-bounds guest buffers and
//!   symlink/traversal escapes under `fs-sandboxed-read`: typed
//!   denials the guest observes, security events recorded, no UB.
//! - `plugin_permission_escalation` — undeclared imports never
//!   instantiate; allowlist-violating fetches are `E_PERM` the guest
//!   sees; the escalation counter faults + disables at the threshold.
//! - `plugin_trap_loop`           — an infinite loop is killed by the
//!   fuel budget; only that instance dies; the host keeps serving.
//! - `plugin_memory_exhaustion`    — a grow-loop traps at the
//!   store-limiter cap (`MemoryOom`); the host heap is unaffected.
//! - `plugin_io_abuse`             — burst/rate quotas and the
//!   single-transfer cap produce typed `E_QUOTA`/`E_TOO_LARGE`.
//! - `plugin_host_pointer_smuggling` — fabricated handles are
//!   refused by the ownership check on every use.

mod common;

use common::{
    br_if, brv, call, code, dropv, endv, i32c, lget, loopv, ltee, manifest, manifest_toml, mem_grow,
    nev, ok, spawn, spawn_with_ledger, I32,
};

use plugin_host_wasm::fsjail::PathJail;
use plugin_host_wasm::instance::InstanceStatus;
use plugin_host_wasm::state::{HostServices, SecurityLedger};
use plugin_host_wasm::HostError;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The out-of-bounds poker: `log_write` with guest pointers that do
/// not address guest memory.
fn oob_wasm() -> Vec<u8> {
    let mut w = common::Wasm::new();
    let log_write = w.import("lumen:v1.log", "log_write", &[I32, I32, I32], &[I32]);
    let poke_oob = w.func(
        &[],
        &[I32],
        &[],
        &code(&[&i32c(2), &i32c(0x4000_0000), &i32c(16), &call(log_write)]),
    );
    let poke_neg = w.func(
        &[],
        &[I32],
        &[],
        &code(&[&i32c(2), &i32c(-1), &i32c(16), &call(log_write)]),
    );
    let poke_ok = w.func(
        &[],
        &[I32],
        &[],
        &code(&[&i32c(1), &i32c(0), &i32c(0), &call(log_write)]),
    );
    w.export_func("poke_oob", poke_oob);
    w.export_func("poke_neg", poke_neg);
    w.export_func("poke_ok", poke_ok);
    w.finish()
}

/// The fs probe: opens real files through the jail, smuggles
/// fabricated handles, and escapes via symlink/traversal paths.
fn fs_wasm() -> Vec<u8> {
    let mut w = common::Wasm::new();
    let fs_open = w.import("lumen:v1.fs", "fs_open", &[I32, I32, I32], &[I32]);
    let fs_read = w.import("lumen:v1.fs", "fs_read", &[I32, I32, I32, I32], &[I32]);
    let fs_size = w.import("lumen:v1.fs", "fs_size", &[I32, I32], &[I32]);
    let fs_close = w.import("lumen:v1.fs", "fs_close", &[I32], &[I32]);
    w.data(0, b"a.txt");
    w.data(64, b"escape/secret.txt");
    w.data(128, b"../outside.txt");
    w.data(192, b"nope.txt");
    let open = |w: &mut common::Wasm, name: &str, ptr: i32, len: i32| {
        let f = w.func(
            &[],
            &[I32],
            &[],
            &code(&[&i32c(ptr), &i32c(len), &i32c(4096), &call(fs_open)]),
        );
        w.export_func(name, f);
    };
    open(&mut w, "open_ok", 0, 5);
    open(&mut w, "open_symlink", 64, 17);
    open(&mut w, "open_traversal", 128, 14);
    open(&mut w, "open_missing", 192, 8);
    let read_bogus = w.func(
        &[],
        &[I32],
        &[],
        &code(&[&i32c(999), &i32c(8192), &i32c(16), &i32c(8256), &call(fs_read)]),
    );
    w.export_func("read_bogus", read_bogus);
    let read_first = w.func(
        &[],
        &[I32],
        &[],
        &code(&[&i32c(0), &i32c(8192), &i32c(16), &i32c(8256), &call(fs_read)]),
    );
    w.export_func("read_first", read_first);
    let size_first = w.func(
        &[],
        &[I32],
        &[],
        &code(&[&i32c(0), &i32c(9000), &call(fs_size)]),
    );
    w.export_func("size_first", size_first);
    let close_first = w.func(&[], &[I32], &[], &code(&[&i32c(0), &call(fs_close)]));
    w.export_func("close_first", close_first);
    let close_unopened = w.func(&[], &[I32], &[], &code(&[&i32c(7), &call(fs_close)]));
    w.export_func("close_unopened", close_unopened);
    let open_three = w.func(
        &[],
        &[I32],
        &[],
        &code(&[
            &i32c(0),
            &i32c(5),
            &i32c(4096),
            &call(fs_open),
            &dropv(),
            &i32c(0),
            &i32c(5),
            &i32c(4100),
            &call(fs_open),
            &dropv(),
            &i32c(0),
            &i32c(5),
            &i32c(4104),
            &call(fs_open),
        ]),
    );
    w.export_func("open_three", open_three);
    w.finish()
}

/// The http probe: allowlisted and non-allowlisted URLs.
fn http_wasm() -> Vec<u8> {
    let mut w = common::Wasm::new();
    let http_get = w.import(
        "lumen:v1.http",
        "http_get",
        &[I32, I32, I32, I32, I32],
        &[I32],
    );
    w.data(0, b"https://evil.test/x");
    w.data(64, b"https://good.test/x");
    let fetch = |w: &mut common::Wasm, name: &str, ptr: i32| {
        let f = w.func(
            &[],
            &[I32],
            &[],
            &code(&[
                &i32c(ptr),
                &i32c(20),
                &i32c(1024),
                &i32c(32),
                &i32c(960),
                &call(http_get),
            ]),
        );
        w.export_func(name, f);
    };
    fetch(&mut w, "fetch_evil", 0);
    fetch(&mut w, "fetch_ok", 64);
    w.finish()
}

/// The fuel-eater: `loop { br 0 }`.
fn spin_wasm() -> Vec<u8> {
    let mut w = common::Wasm::new();
    let spin = w.func(&[], &[], &[], &code(&[&loopv(), &brv(0), &endv()]));
    w.export_func("spin", spin);
    w.finish()
}

/// The memory hog: grow one page at a time until the limiter traps.
/// Module max is 100 pages — ABOVE the host's 1 MiB store cap — so
/// the denial provably comes from the host limiter, not the module.
fn memhog_wasm() -> Vec<u8> {
    let mut w = common::Wasm::new();
    w.memory(1, 100);
    let grow = w.func(
        &[],
        &[I32],
        &[(1, I32)],
        &code(&[
            &loopv(),
            &i32c(1),
            &mem_grow(),
            &ltee(0),
            &i32c(-1),
            &nev(),
            &br_if(0),
            &endv(),
            &lget(0),
        ]),
    );
    w.export_func("grow", grow);
    w.finish()
}

/// The io spammer: three 600-byte log records (burst 1 KiB, refill
/// 1 B/s under tiny limits) and one over-cap transfer.
fn spam_wasm() -> Vec<u8> {
    let mut w = common::Wasm::new();
    let log_write = w.import("lumen:v1.log", "log_write", &[I32, I32, I32], &[I32]);
    let line: Vec<u8> = vec![b'x'; 600];
    w.data(0, &line);
    let spam3 = w.func(
        &[],
        &[I32],
        &[],
        &code(&[
            &i32c(1),
            &i32c(0),
            &i32c(600),
            &call(log_write),
            &dropv(),
            &i32c(1),
            &i32c(0),
            &i32c(600),
            &call(log_write),
            &dropv(),
            &i32c(1),
            &i32c(0),
            &i32c(600),
            &call(log_write),
        ]),
    );
    w.export_func("spam3", spam3);
    let oversize = w.func(
        &[],
        &[I32],
        &[],
        &code(&[&i32c(1), &i32c(0), &i32c(2000), &call(log_write)]),
    );
    w.export_func("oversize", oversize);
    w.finish()
}

/// A jail rooted at `media`, with `a.txt` inside and (unix) a
/// symlink `escape` pointing at a sibling secret directory.
fn media_jail(tmp: &common::TempDir) -> PathJail {
    let media = tmp.path().join("media");
    ok(std::fs::create_dir_all(&media).map_err(|e| e.to_string()));
    ok(std::fs::write(media.join("a.txt"), b"media-file-bytes").map_err(|e| e.to_string()));
    #[cfg(unix)]
    {
        let secret = tmp.path().join("secret");
        ok(std::fs::create_dir_all(&secret).map_err(|e| e.to_string()));
        ok(std::fs::write(secret.join("secret.txt"), b"no").map_err(|e| e.to_string()));
        ok(std::os::unix::fs::symlink(&secret, media.join("escape")).map_err(|e| e.to_string()));
    }
    #[cfg(windows)]
    {
        // Junction-free environments: the canonicalization logic is
        // exercised by the traversal case below (fsjail unit tests
        // cover the component-aware prefix rules cross-platform).
    }
    ok(PathJail::new([&media]))
}

// ---------------------------------------------------------------------------
// §9.4 sandbox escape
// ---------------------------------------------------------------------------

#[test]
fn plugin_sandbox_escape() {
    // (a) Out-of-bounds guest pointers: the host's bounds-checked
    // read yields the typed E_OOB the guest observes, a security
    // event is recorded with shapes only, and the instance — and the
    // test process — survive (no UB).
    let wasm = oob_wasm();
    let mut poke = ok(spawn(
        &wasm,
        manifest(&manifest_toml(&wasm, "")),
        HostServices::empty(),
    ));
    assert_eq!(
        ok(poke.invoke("poke_oob", &[])),
        vec![4],
        "E_OOB is the guest-observable denial"
    );
    let events = poke.ledger().events();
    assert_eq!(events.len(), 1, "one security event");
    assert!(events[0].reason.contains("out-of-bounds"));
    assert!(events[0].arg_shape.contains("ptr="));
    // Negative pointers are argument errors, not memory accesses.
    assert_eq!(ok(poke.invoke("poke_neg", &[])), vec![2], "E_ARG");
    // The instance still dispatches: a typed denial is not a fault.
    assert_eq!(ok(poke.invoke("poke_ok", &[])), vec![0]);

    // (b) The fs jail: relative symlink escape and parent traversal
    // both resolve outside the roots and are denied.
    let tmp = common::TempDir::new("escape");
    let jail = media_jail(&tmp);
    let wasm = fs_wasm();
    let mut fs = ok(spawn(
        &wasm,
        manifest(&manifest_toml(&wasm, "fs-sandboxed-read = true")),
        HostServices {
            jail: Some(jail),
            ..HostServices::empty()
        },
    ));
    #[cfg(unix)]
    {
        assert_eq!(
            ok(fs.invoke("open_symlink", &[])),
            vec![1],
            "symlink escape is E_PERM"
        );
    }
    assert_eq!(
        ok(fs.invoke("open_traversal", &[])),
        vec![1],
        "parent traversal is E_PERM"
    );
    let events = fs.ledger().events();
    #[cfg(unix)]
    assert_eq!(events.len(), 2, "symlink + traversal both logged");
    #[cfg(windows)]
    assert_eq!(events.len(), 1, "traversal logged");
    assert!(
        events.iter().all(|e| e.reason.contains("sandbox roots")),
        "denial reasons name the jail rule: {:?}",
        events
    );
    // The happy path still works in the same session.
    assert_eq!(ok(fs.invoke("open_ok", &[])), vec![0]);
}

// ---------------------------------------------------------------------------
// §9.4 permission escalation
// ---------------------------------------------------------------------------

#[test]
fn plugin_permission_escalation() {
    // (a) An undeclared gated import never instantiates: the module
    // imports lumen:v1.fs but the manifest declares no
    // fs-sandboxed-read (doc 06 §5.3 — import absent from the table).
    let wasm = fs_wasm();
    let err = spawn(
        &wasm,
        manifest(&manifest_toml(&wasm, "")),
        HostServices::empty(),
    )
    .err()
    .unwrap_or_else(|| unreachable!("undeclared import must refuse instantiation"));
    assert!(
        matches!(err, HostError::UndeclaredImport { ref module, ref name, .. }
            if module == plugin_host_wasm::api::MOD_FS && name == "fs_open"),
        "typed undeclared-import error, got: {err}"
    );

    // (b) A WASI import is an unknown module: no ambient authority
    // exists (ADR-002 no-WASI posture).
    let mut wasi = common::Wasm::new();
    wasi.import(
        "wasi_snapshot_preview1",
        "fd_read",
        &[I32, I32, I32, I32],
        &[I32],
    );
    let wasi_bytes = wasi.finish();
    let err = spawn(
        &wasi_bytes,
        manifest(&manifest_toml(&wasi_bytes, "")),
        HostServices::empty(),
    )
    .err()
    .unwrap_or_else(|| unreachable!("WASI import must refuse instantiation"));
    assert!(
        matches!(err, HostError::UnknownImportModule { ref module } if module == "wasi_snapshot_preview1"),
        "typed unknown-module error, got: {err}"
    );

    // (c) Allowlist violations are E_PERM the guest OBSERVES, are
    // counted, and the third escalation faults + disables (§5.3
    // threshold 3 per session).
    let wasm = http_wasm();
    let ledger = SecurityLedger::new();
    let manifest_text = manifest_toml(
        &wasm,
        "http-fetch = true\nnetwork-host-allowlist = [\"good.test\"]",
    );
    let mut http = ok(spawn_with_ledger(
        &wasm,
        manifest(&manifest_text),
        HostServices::empty(),
        ledger.clone(),
    ));
    assert_eq!(
        ok(http.invoke("fetch_evil", &[])),
        vec![1],
        "allowlist violation is E_PERM, guest-observable"
    );
    assert_eq!(ok(http.invoke("fetch_evil", &[])), vec![1]);
    assert_eq!(http.ledger().escalation_count(), 2);
    // Third attempt: threshold reached — the instance faults with the
    // disable reason; further dispatch is refused.
    let err = http
        .invoke("fetch_evil", &[])
        .err()
        .unwrap_or_else(|| unreachable!("third escalation must fault"));
    assert!(
        matches!(err, HostError::ExtensionDisabled { .. }),
        "escalation threshold disables, got: {err}"
    );
    assert_eq!(http.ledger().escalation_count(), 3);
    assert!(matches!(http.status(), InstanceStatus::Faulted { .. }));
    assert!(matches!(
        http.invoke("fetch_evil", &[]),
        Err(HostError::InstanceFaulted { .. })
    ));
}

// ---------------------------------------------------------------------------
// §9.4 trap loop / memory exhaustion (host survival in both)
// ---------------------------------------------------------------------------

#[test]
fn plugin_trap_loop() {
    let wasm = spin_wasm();
    let mut spin = ok(spawn(
        &wasm,
        manifest(&manifest_toml(&wasm, "")),
        HostServices::empty(),
    ));
    // The infinite loop is killed by the fuel budget (deterministic:
    // exactly call_fuel instructions) — CpuLimit, instance faulted.
    let start = std::time::Instant::now();
    let err = spin
        .invoke("spin", &[])
        .err()
        .unwrap_or_else(|| unreachable!("the tight loop must be killed by fuel"));
    assert!(matches!(err, HostError::CpuLimit), "got: {err}");
    // Well within 2× the intended budget semantics (§7.4): the tiny
    // test budget is 10k fuel — the kill is immediate, not a hang.
    assert!(
        start.elapsed() < std::time::Duration::from_secs(5),
        "no hang: killed in {:?}",
        start.elapsed()
    );
    assert!(matches!(spin.status(), InstanceStatus::Faulted { .. }));
    // Dispatch after the fault is refused.
    assert!(matches!(
        spin.invoke("spin", &[]),
        Err(HostError::InstanceFaulted { .. })
    ));
    // THE HOST SURVIVES: a fresh instance spawns and completes work.
    let hello_wasm = hello_for_survivor();
    let mut survivor = ok(spawn(
        &hello_wasm,
        manifest(&manifest_toml(&hello_wasm, "")),
        HostServices::empty(),
    ));
    assert_eq!(ok(survivor.invoke("hello", &[])), vec![0]);
}

#[test]
fn plugin_memory_exhaustion() {
    let wasm = memhog_wasm();
    let mut hog = ok(spawn(
        &wasm,
        manifest(&manifest_toml(&wasm, "")),
        HostServices::empty(),
    ));
    // The grow-loop traps at the store-limiter cap (1 MiB under tiny
    // limits): MemoryOom, instance faulted, host heap untouched.
    let err = hog
        .invoke("grow", &[])
        .err()
        .unwrap_or_else(|| unreachable!("the grow loop must trap at the cap"));
    assert!(
        matches!(err, HostError::MemoryLimit { cap_bytes } if cap_bytes == 1024 * 1024),
        "MemoryOom at the store cap, got: {err}"
    );
    assert!(matches!(hog.status(), InstanceStatus::Faulted { .. }));
    // THE HOST SURVIVES (bounded memory everywhere, D-2).
    let hello_wasm = hello_for_survivor();
    let survivor = spawn(
        &hello_wasm,
        manifest(&manifest_toml(&hello_wasm, "")),
        HostServices::empty(),
    );
    assert!(survivor.is_ok(), "a fresh instance spawns");
}

/// The minimal survivor module used by the containment tests.
fn hello_for_survivor() -> Vec<u8> {
    let mut w = common::Wasm::new();
    let log_write = w.import("lumen:v1.log", "log_write", &[I32, I32, I32], &[I32]);
    w.data(0, b"survivor");
    let hello = w.func(
        &[],
        &[I32],
        &[],
        &code(&[&i32c(2), &i32c(0), &i32c(8), &call(log_write)]),
    );
    w.export_func("hello", hello);
    w.finish()
}

// ---------------------------------------------------------------------------
// §9.4 io abuse
// ---------------------------------------------------------------------------

#[test]
fn plugin_io_abuse() {
    // Burst quota: 3×600-byte log records against a 1024-byte burst
    // bucket — the second and third are E_QUOTA, all guest-observable
    // and recorded in the import trace.
    let wasm = spam_wasm();
    let mut spam = ok(spawn(
        &wasm,
        manifest(&manifest_toml(&wasm, "")),
        HostServices::empty(),
    ));
    assert_eq!(ok(spam.invoke("spam3", &[])), vec![7], "E_QUOTA on burst exhaustion");
    let trace = spam.import_trace();
    assert_eq!(
        trace
            .iter()
            .map(|(import, status)| (import.as_str(), *status))
            .collect::<Vec<_>>(),
        vec![
            ("lumen:v1.log::log_write", 0),
            ("lumen:v1.log::log_write", 7),
            ("lumen:v1.log::log_write", 7),
        ],
        "the trace shows throttle-then-refuse"
    );

    // Single-transfer cap: a 2000-byte transfer against the 1024-byte
    // cap is E_TOO_LARGE before any bucket is charged.
    assert_eq!(ok(spam.invoke("oversize", &[])), vec![6], "E_TOO_LARGE");

    // HTTP rate window: 3 fetches against the 2/min tiny cap — the
    // first two reach the (absent) transport as E_TRANSPORT, the
    // third is E_QUOTA. The policy chain order is observable.
    let http_bytes = http_wasm();
    let mut http = ok(spawn(
        &http_bytes,
        manifest(&manifest_toml(
            &http_bytes,
            "http-fetch = true\nnetwork-host-allowlist = [\"good.test\"]",
        )),
        HostServices::empty(),
    ));
    assert_eq!(ok(http.invoke("fetch_ok", &[])), vec![8], "E_TRANSPORT: the seam is honestly unwired");
    assert_eq!(ok(http.invoke("fetch_ok", &[])), vec![8]);
    assert_eq!(ok(http.invoke("fetch_ok", &[])), vec![7], "E_QUOTA: per-minute cap");

    // With a wired transport, the ALLOWED host round-trips real bytes
    // through the seam (policy passes, payload crosses, caps hold).
    let mut wired = ok(spawn(
        &http_bytes,
        manifest(&manifest_toml(
            &http_bytes,
            "http-fetch = true\nnetwork-host-allowlist = [\"good.test\"]",
        )),
        HostServices {
            transport: Some(Box::new(common::OkTransport)),
            ..HostServices::empty()
        },
    ));
    assert_eq!(ok(wired.invoke("fetch_ok", &[])), vec![0]);
    let body = ok(wired.read_guest_memory(1024, 22));
    assert_eq!(body, b"payload-from-transport");
}

// ---------------------------------------------------------------------------
// §9.4 host pointer smuggling
// ---------------------------------------------------------------------------

#[test]
fn plugin_host_pointer_smuggling() {
    let tmp = common::TempDir::new("smuggle");
    let jail = media_jail(&tmp);
    let wasm = fs_wasm();
    let mut fs = ok(spawn(
        &wasm,
        manifest(&manifest_toml(&wasm, "fs-sandboxed-read = true")),
        HostServices {
            jail: Some(jail),
            ..HostServices::empty()
        },
    ));

    // A fabricated handle value is refused by the ownership check —
    // handles are per-instance table indices, never host pointers.
    assert_eq!(
        ok(fs.invoke("read_bogus", &[])),
        vec![3],
        "E_HANDLE: handle 999 was never opened"
    );
    assert_eq!(
        ok(fs.invoke("close_unopened", &[])),
        vec![3],
        "E_HANDLE: handle 7 was never opened"
    );

    // The real handle lifecycle works end to end: open → read → size
    // → close → double-close refused (ownership on every use).
    assert_eq!(ok(fs.invoke("open_ok", &[])), vec![0]);
    let handle_bytes = ok(fs.read_guest_memory(4096, 4));
    assert_eq!(
        u32::from_le_bytes([handle_bytes[0], handle_bytes[1], handle_bytes[2], handle_bytes[3]]),
        0,
        "the first opened handle is index 0"
    );
    assert_eq!(ok(fs.invoke("read_first", &[])), vec![0]);
    let content = ok(fs.read_guest_memory(8192, 16));
    assert_eq!(content, b"media-file-bytes");
    let len_bytes = ok(fs.read_guest_memory(8256, 4));
    assert_eq!(
        u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]),
        16
    );
    assert_eq!(ok(fs.invoke("size_first", &[])), vec![0]);
    let size_bytes = ok(fs.read_guest_memory(9000, 8));
    assert_eq!(
        u64::from_le_bytes([
            size_bytes[0],
            size_bytes[1],
            size_bytes[2],
            size_bytes[3],
            size_bytes[4],
            size_bytes[5],
            size_bytes[6],
            size_bytes[7]
        ]),
        16
    );
    assert_eq!(ok(fs.invoke("close_first", &[])), vec![0]);
    assert_eq!(
        ok(fs.invoke("close_first", &[])),
        vec![3],
        "E_HANDLE: double close is an ownership violation"
    );

    // Handle-table cap (tiny limits: 2), on a FRESH instance so the
    // slot count starts at zero: opens 0 and 1 succeed, the third is
    // E_LIMIT — a refusal, not an accommodation (doc 06 §5.4 row 7).
    // A fresh temp root too: `media_jail` is not idempotent (its
    // unix symlink fixture is create-once), and the point here is the
    // fresh HANDLE TABLE, not the jail.
    let tmp2 = common::TempDir::new("smuggle-cap");
    let mut fresh = ok(spawn(
        &wasm,
        manifest(&manifest_toml(&wasm, "fs-sandboxed-read = true")),
        HostServices {
            jail: Some(media_jail(&tmp2)),
            ..HostServices::empty()
        },
    ));
    assert_eq!(
        ok(fresh.invoke("open_three", &[])),
        vec![9],
        "E_LIMIT: the handle table is capped"
    );
    let open_statuses: Vec<i32> = fresh
        .import_trace()
        .iter()
        .filter(|(import, _)| import == "lumen:v1.fs::fs_open")
        .map(|(_, status)| *status)
        .collect();
    assert!(
        open_statuses.ends_with(&[0, 0, 9]),
        "open_three's trace is [Ok, Ok, E_LIMIT], got {open_statuses:?}"
    );
}
