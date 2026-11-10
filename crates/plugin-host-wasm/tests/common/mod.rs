#![allow(dead_code)]
//! Shared fixtures for the plugin-host-wasm integration suites
//! (doc 06 §9.4/§9.5; cards 7.F1–7.F3).
//!
//! Three builders, all std-only and dependency-free (the crate's own
//! discipline — no `wat`/`wast` crates):
//!
//! - [`Wasm`] — a minimal **wasm binary emitter** for the guest
//!   fixtures. The WebAssembly binary format is public-spec math
//!   (the same clean-room posture as the ZIP reader in
//!   `src/package.rs`): sections, LEB128, imports, memories, code.
//!   Every fixture module a §9.4/§9.5 case needs is a dozen lines
//!   of builder calls; wasmi's validating `Module::new` (the
//!   `validate` feature) rejects any emitter bug loudly.
//! - [`zip_stored`] — a stored-entry ZIP writer for `.lumenext`
//!   packages (local headers + central directory + EOCD; CRCs via
//!   the crate's own `crc32`).
//! - [`manifest_toml`] — an extension manifest whose
//!   `[provenance].hash` is the real digest of the module it will
//!   ship with, computed with the crate's own `module_digest`.
//!
//! Plus the house `ok`/`some` test helpers (media-probe precedent)
//! and a self-cleaning `TempDir`.

use std::path::{Path, PathBuf};

use plugin_host_wasm::http::{HttpResponse, HttpTransport, TransportError};
use plugin_host_wasm::instance::{ArcLedger, WasmInstance, engine};
use plugin_host_wasm::limits::ResourceLimits;
use plugin_host_wasm::package::module_digest;
use plugin_host_wasm::state::{HostServices, SecurityLedger};
use plugin_host_wasm::{HostError, crc32::crc32};

use plugin_api::PluginManifest;

// ---------------------------------------------------------------------------
// House helpers
// ---------------------------------------------------------------------------

/// House test helper (media-probe precedent).
pub fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => unreachable!("{e}"),
    }
}

/// The Option twin of [`ok`].
pub fn some<T>(o: Option<T>) -> T {
    match o {
        Some(v) => v,
        None => unreachable!("expected Some, got None"),
    }
}

/// Unique self-cleaning temp dir (std-only).
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!(
            "lumen-pjwasm-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        ok(std::fs::create_dir_all(&dir)
            .map_err(|e| format!("test temp dir {}: {e}", dir.display())));
        TempDir(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------------------
// The wasm binary emitter (public-spec format, std-only)
// ---------------------------------------------------------------------------

/// `i32` value type byte.
pub const I32: u8 = 0x7F;
/// `i64` value type byte.
pub const I64: u8 = 0x7E;

/// Unsigned LEB128.
fn uleb(mut v: u64, out: &mut Vec<u8>) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

/// Signed LEB128 (i32.const / i64.const immediates).
fn sleb(mut v: i64, out: &mut Vec<u8>) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        let sign_bit = byte & 0x40 != 0;
        if (v == 0 && !sign_bit) || (v == -1 && sign_bit) {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

fn section(id: u8, body: &[u8], out: &mut Vec<u8>) {
    out.push(id);
    uleb(body.len() as u64, out);
    out.extend_from_slice(body);
}

fn name(s: &str, out: &mut Vec<u8>) {
    uleb(s.len() as u64, out);
    out.extend_from_slice(s.as_bytes());
}

/// One function body: local declarations + instruction bytes. The
/// terminating `end` (0x0B) is appended by [`Wasm::finish`].
struct Body {
    locals: Vec<(u32, u8)>,
    code: Vec<u8>,
}

/// A minimal wasm module builder: typed imports, local functions,
/// one memory, exports, an optional start section and active data
/// segments — everything the doc 06 §9.4/§9.5 fixtures need.
pub struct Wasm {
    types: Vec<(Vec<u8>, Vec<u8>)>,
    imports: Vec<(String, String, usize)>,
    funcs: Vec<usize>,
    bodies: Vec<Body>,
    mem: Option<(u32, u32)>,
    exports: Vec<(String, u8, u32)>,
    start: Option<u32>,
    data: Vec<(u32, Vec<u8>)>,
}

impl Default for Wasm {
    fn default() -> Self {
        Wasm::new()
    }
}

impl Wasm {
    /// A module with one exported memory of 1..16 pages (the fixtures'
    /// working set; 16 pages = 1 MiB, matching the tiny-limit cap).
    pub fn new() -> Wasm {
        Wasm {
            types: Vec::new(),
            imports: Vec::new(),
            funcs: Vec::new(),
            bodies: Vec::new(),
            mem: Some((1, 16)),
            exports: Vec::new(),
            start: None,
            data: Vec::new(),
        }
    }

    /// Override the memory bounds in pages (min, max). Fixtures that
    /// test the **store-limiter** cap set a module max above the host
    /// cap so the denial comes from the limiter, not the module.
    pub fn memory(&mut self, min: u32, max: u32) {
        self.mem = Some((min, max));
    }

    fn ty(&mut self, params: &[u8], results: &[u8]) -> usize {
        if let Some(i) = self
            .types
            .iter()
            .position(|(p, r)| p == params && r == results)
        {
            return i;
        }
        self.types.push((params.to_vec(), results.to_vec()));
        self.types.len() - 1
    }

    /// Declare an imported function; returns its function index.
    pub fn import(&mut self, module: &str, field: &str, params: &[u8], results: &[u8]) -> u32 {
        let ty = self.ty(params, results);
        self.imports
            .push((module.to_string(), field.to_string(), ty));
        (self.imports.len() - 1) as u32
    }

    /// Declare a local function with its body; returns its function
    /// index (imports come first in the index space).
    pub fn func(
        &mut self,
        params: &[u8],
        results: &[u8],
        locals: &[(u32, u8)],
        code: &[u8],
    ) -> u32 {
        let ty = self.ty(params, results);
        self.funcs.push(ty);
        self.bodies.push(Body {
            locals: locals.to_vec(),
            code: code.to_vec(),
        });
        (self.imports.len() + self.funcs.len() - 1) as u32
    }

    /// Export a function under `name`.
    pub fn export_func(&mut self, name: &str, idx: u32) {
        self.exports.push((name.to_string(), 0x00, idx));
    }

    /// Set the start section's function (must be `() -> ()`).
    pub fn start(&mut self, idx: u32) {
        self.start = Some(idx);
    }

    /// An active data segment at a constant offset.
    pub fn data(&mut self, offset: u32, bytes: &[u8]) {
        self.data.push((offset, bytes.to_vec()));
    }

    /// Serialize to wasm bytes (magic + version + sections, in
    /// canonical order; the memory is exported under the conventional
    /// `memory` name when nothing else exports it).
    pub fn finish(mut self) -> Vec<u8> {
        if self.mem.is_some() && !self.exports.iter().any(|(_, kind, _)| kind == &0x02) {
            self.exports.push(("memory".to_string(), 0x02, 0));
        }
        let mut out = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        if !self.types.is_empty() {
            let mut b = Vec::new();
            uleb(self.types.len() as u64, &mut b);
            for (params, results) in &self.types {
                b.push(0x60);
                uleb(params.len() as u64, &mut b);
                b.extend_from_slice(params);
                uleb(results.len() as u64, &mut b);
                b.extend_from_slice(results);
            }
            section(1, &b, &mut out);
        }
        if !self.imports.is_empty() {
            let mut b = Vec::new();
            uleb(self.imports.len() as u64, &mut b);
            for (module, field, ty) in &self.imports {
                name(module, &mut b);
                name(field, &mut b);
                b.push(0x00);
                uleb(*ty as u64, &mut b);
            }
            section(2, &b, &mut out);
        }
        if !self.funcs.is_empty() {
            let mut b = Vec::new();
            uleb(self.funcs.len() as u64, &mut b);
            for ty in &self.funcs {
                uleb(*ty as u64, &mut b);
            }
            section(3, &b, &mut out);
        }
        if let Some((min, max)) = self.mem {
            let mut b = Vec::new();
            uleb(1, &mut b);
            b.push(0x01);
            uleb(min as u64, &mut b);
            uleb(max as u64, &mut b);
            section(5, &b, &mut out);
        }
        if !self.exports.is_empty() {
            let mut b = Vec::new();
            uleb(self.exports.len() as u64, &mut b);
            for (n, kind, idx) in &self.exports {
                name(n, &mut b);
                b.push(*kind);
                uleb(*idx as u64, &mut b);
            }
            section(7, &b, &mut out);
        }
        if let Some(start) = self.start {
            let mut b = Vec::new();
            uleb(start as u64, &mut b);
            section(8, &b, &mut out);
        }
        if !self.bodies.is_empty() {
            let mut b = Vec::new();
            uleb(self.bodies.len() as u64, &mut b);
            for body in &self.bodies {
                let mut entry = Vec::new();
                uleb(body.locals.len() as u64, &mut entry);
                for (count, ty) in &body.locals {
                    uleb(*count as u64, &mut entry);
                    entry.push(*ty);
                }
                entry.extend_from_slice(&body.code);
                entry.push(0x0B);
                uleb(entry.len() as u64, &mut b);
                b.extend_from_slice(&entry);
            }
            section(10, &b, &mut out);
        }
        if !self.data.is_empty() {
            let mut b = Vec::new();
            uleb(self.data.len() as u64, &mut b);
            for (offset, bytes) in &self.data {
                b.push(0x00);
                b.push(0x41);
                sleb(i64::from(*offset), &mut b);
                b.push(0x0B);
                uleb(bytes.len() as u64, &mut b);
                b.extend_from_slice(bytes);
            }
            section(11, &b, &mut out);
        }
        out
    }
}

// -- instruction byte helpers (bodies are short concatenations) --------------

/// Concatenate code parts.
pub fn code(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

/// `i32.const v`.
pub fn i32c(v: i32) -> Vec<u8> {
    let mut out = vec![0x41];
    sleb(i64::from(v), &mut out);
    out
}

/// `i64.const v`.
pub fn i64c(v: i64) -> Vec<u8> {
    let mut out = vec![0x42];
    sleb(v, &mut out);
    out
}

/// `call f`.
pub fn call(f: u32) -> Vec<u8> {
    let mut out = vec![0x10];
    uleb(u64::from(f), &mut out);
    out
}

/// `drop`.
pub fn dropv() -> Vec<u8> {
    vec![0x1A]
}

/// `local.get i`.
pub fn lget(i: u32) -> Vec<u8> {
    let mut out = vec![0x20];
    uleb(u64::from(i), &mut out);
    out
}

/// `local.tee i`.
pub fn ltee(i: u32) -> Vec<u8> {
    let mut out = vec![0x22];
    uleb(u64::from(i), &mut out);
    out
}

/// `memory.grow` (memory 0).
pub fn mem_grow() -> Vec<u8> {
    vec![0x40, 0x00]
}

/// `unreachable`.
pub fn unreachablev() -> Vec<u8> {
    vec![0x00]
}

/// `br_if d`.
pub fn br_if(d: u32) -> Vec<u8> {
    let mut out = vec![0x0D];
    uleb(u64::from(d), &mut out);
    out
}

/// `br d` (unconditional).
pub fn brv(d: u32) -> Vec<u8> {
    let mut out = vec![0x0C];
    uleb(u64::from(d), &mut out);
    out
}

/// `loop` (void block type).
pub fn loopv() -> Vec<u8> {
    vec![0x03, 0x40]
}

/// `end`.
pub fn endv() -> Vec<u8> {
    vec![0x0B]
}

/// `i32.ne`.
pub fn nev() -> Vec<u8> {
    vec![0x47]
}

// ---------------------------------------------------------------------------
// The .lumenext ZIP writer (stored entries only, like the reader accepts)
// ---------------------------------------------------------------------------

/// Build a stored-entry ZIP archive (local headers + central
/// directory + EOCD), CRC-32 checked by the crate's own reader.
pub fn zip_stored(entries: &[(&str, &[u8])]) -> Vec<u8> {
    zip_raw(entries, None)
}

/// A ZIP where one entry (by index) carries a deliberately wrong
/// CRC and/or a non-stored compression method — the malformed-input
/// cases of §9.4 `plugin_malicious_manifest`. `corrupt` selects the
/// entry index; `method` overrides its compression method (0 = the
/// honest stored form).
pub fn zip_raw(entries: &[(&str, &[u8])], corrupt: Option<(usize, u16, bool)>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offsets = Vec::new();
    for (i, (entry, data)) in entries.iter().enumerate() {
        offsets.push(out.len() as u32);
        let (method, bad_crc) = match corrupt {
            Some((idx, method, bad)) if idx == i => (method, bad),
            _ => (0u16, false),
        };
        let crc = crc32(data) ^ if bad_crc { 0xFFFF_FFFF } else { 0 };
        out.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]);
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time
        out.extend_from_slice(&0u16.to_le_bytes()); // mod date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes()); // compressed
        out.extend_from_slice(&(data.len() as u32).to_le_bytes()); // uncompressed
        out.extend_from_slice(&(entry.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(entry.as_bytes());
        out.extend_from_slice(data);
    }
    let cd_offset = out.len() as u32;
    let mut central = Vec::new();
    for (i, ((entry, data), offset)) in entries.iter().zip(&offsets).enumerate() {
        let (method, bad_crc) = match corrupt {
            Some((idx, method, bad)) if idx == i => (method, bad),
            _ => (0u16, false),
        };
        let crc = crc32(data) ^ if bad_crc { 0xFFFF_FFFF } else { 0 };
        central.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]);
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0u16.to_le_bytes()); // flags
        central.extend_from_slice(&method.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // time
        central.extend_from_slice(&0u16.to_le_bytes()); // date
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(entry.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra
        central.extend_from_slice(&0u16.to_le_bytes()); // comment
        central.extend_from_slice(&0u16.to_le_bytes()); // disk start
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(entry.as_bytes());
    }
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06]);
    out.extend_from_slice(&0u16.to_le_bytes()); // this disk
    out.extend_from_slice(&0u16.to_le_bytes()); // cd start disk
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment len
    out
}

// ---------------------------------------------------------------------------
// Manifests and packages
// ---------------------------------------------------------------------------

/// An extension manifest for `wasm` with the REAL provenance digest
/// (what install/spawn consume). `perms` is the `[permissions]`
/// block body (empty string = no permissions section).
pub fn manifest_toml(wasm: &[u8], perms: &str) -> String {
    let digest = module_digest(wasm);
    let perms_block = if perms.is_empty() {
        String::new()
    } else {
        format!("[permissions]\n{perms}\n")
    };
    format!(
        "id = \"ext.acme.hello\"\nkind = \"metadata-provider\"\nversion = \"0.1.0\"\n\
         api_version = 1\norigin = \"extension\"\n\n[capabilities]\nprovides = [\"greeting\"]\n\n\
         {perms_block}[provenance]\npublisher = \"Acme Tools\"\nsource = \
         \"https://example.com/acme-hello-0.1.0.lumenext\"\nhash = \"{digest}\"\nlicense = \"MIT\"\n"
    )
}

/// A *self-reported* manifest payload for a guest `report_manifest`
/// call: same identity, arbitrary-but-valid provenance (the digest
/// of the empty string — the check is parse + identity, never byte
/// equality with the installed text).
pub fn reported_manifest_toml() -> String {
    format!(
        "id = \"ext.acme.hello\"\nkind = \"metadata-provider\"\nversion = \"0.1.0\"\n\
         api_version = 1\norigin = \"extension\"\n\n[capabilities]\nprovides = [\"greeting\"]\n\n\
         [provenance]\npublisher = \"Acme Tools\"\nsource = \
         \"https://example.com/acme-hello-0.1.0.lumenext\"\nhash = \"{}\"\nlicense = \"MIT\"\n",
        module_digest(&[])
    )
}

/// A `.lumenext` package: manifest + module (+ optional detached
/// signature block text, newline-normalized).
pub fn package(manifest: &str, wasm: &[u8], signature: Option<&str>) -> Vec<u8> {
    let normalized = signature.map(|sig| {
        let mut text = sig.to_string();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text
    });
    let mut entries: Vec<(&str, &[u8])> = vec![("manifest.toml", manifest.as_bytes())];
    if let Some(text) = normalized.as_deref() {
        entries.push(("signature", text.as_bytes()));
    }
    entries.push(("plugin.wasm", wasm));
    zip_stored(&entries)
}

/// The detached-signature block text for `suite`/`key`/`value`.
pub fn signature_block(suite: &str, key: &str, value: &str) -> String {
    format!("lumenext-signature-v1\nsuite: {suite}\nkey: {key}\nvalue: {value}\n")
}

// ---------------------------------------------------------------------------
// Spawning
// ---------------------------------------------------------------------------

/// Parse a manifest text (the house `ok` helper; callers that EXPECT
/// failure call `PluginManifest::from_toml` directly).
pub fn manifest(text: &str) -> PluginManifest {
    ok(PluginManifest::from_toml(text))
}

/// Spawn a fixture instance under the tiny test limits.
pub fn spawn(
    wasm: &[u8],
    manifest: PluginManifest,
    services: HostServices,
) -> Result<WasmInstance, HostError> {
    WasmInstance::spawn(
        &engine(),
        wasm,
        manifest,
        services,
        ResourceLimits::test_tiny(),
        SecurityLedger::new(),
    )
}

/// Spawn with an explicit ledger (escalation assertions keep the
/// share after the instance faults).
pub fn spawn_with_ledger(
    wasm: &[u8],
    manifest: PluginManifest,
    services: HostServices,
    ledger: ArcLedger,
) -> Result<WasmInstance, HostError> {
    WasmInstance::spawn(
        &engine(),
        wasm,
        manifest,
        services,
        ResourceLimits::test_tiny(),
        ledger,
    )
}

/// Spawn under explicit limits — for fixtures whose payloads exceed a
/// tiny-budget cap (e.g. the ~330-byte manifest report against the
/// 256-byte `max_descriptor_bytes` of [`ResourceLimits::test_tiny`]).
pub fn spawn_with_limits(
    wasm: &[u8],
    manifest: PluginManifest,
    services: HostServices,
    limits: ResourceLimits,
) -> Result<WasmInstance, HostError> {
    WasmInstance::spawn(
        &engine(),
        wasm,
        manifest,
        services,
        limits,
        SecurityLedger::new(),
    )
}

// ---------------------------------------------------------------------------
// Transport double
// ---------------------------------------------------------------------------

/// A transport that always succeeds with a fixed body (the §9.4
/// allowlist happy path — policy is exercised in front of the seam).
pub struct OkTransport;

impl HttpTransport for OkTransport {
    fn fetch(&mut self, _url: &str) -> Result<HttpResponse, TransportError> {
        Ok(HttpResponse {
            status: 200,
            body: b"payload-from-transport".to_vec(),
        })
    }
}

// ---------------------------------------------------------------------------
// Crash-injection fixtures (card 7.F4 / doc 06 §7.5, §9.5) — the
// trap-happy extension shapes the conformance kit and the crash
// isolation suite drive through the supervisor. All test-only wasm,
// built with the same in-test emitter as the §9.4 fixtures.
// ---------------------------------------------------------------------------

/// Traps on entry: EVERY exported entry point is `unreachable` — the
/// maximally hostile extension (doc 06 §7.5 case 1).
pub fn crash_trap_on_entry_wasm() -> Vec<u8> {
    let mut w = Wasm::new();
    let init = w.func(&[], &[I32], &[], &code(&[&unreachablev()]));
    let work = w.func(&[], &[I32], &[], &code(&[&unreachablev()]));
    w.export_func("init", init);
    w.export_func("work", work);
    w.finish()
}

/// Traps mid-call: one honest import call, then `unreachable` (doc 06
/// §7.5 case 2 — the trap fires with host state touched).
pub fn crash_trap_mid_call_wasm() -> Vec<u8> {
    let mut w = Wasm::new();
    let log_write = w.import("lumen:v1.log", "log_write", &[I32, I32, I32], &[I32]);
    w.data(0, b"about to trap");
    let work = w.func(
        &[],
        &[I32],
        &[],
        &code(&[
            &i32c(2),
            &i32c(0),
            &i32c(13),
            &call(log_write),
            &dropv(),
            &unreachablev(),
        ]),
    );
    w.export_func("work", work);
    w.finish()
}

/// Escalates to the host fault from INSIDE an import callback (doc 06
/// §7.5 case 3): three allowlist-violating fetches — the third trips
/// the session threshold and the host faults the instance mid-import.
/// (The trailing `i32.const 0` only satisfies the validator: the third
/// fault stops execution before it.)
pub fn crash_escalate_wasm() -> Vec<u8> {
    let mut w = Wasm::new();
    let http_get = w.import(
        "lumen:v1.http",
        "http_get",
        &[I32, I32, I32, I32, I32],
        &[I32],
    );
    w.data(0, b"https://evil.test/x");
    let mut body = Vec::new();
    for _ in 0..3 {
        body.extend_from_slice(&i32c(0));
        body.extend_from_slice(&i32c(20));
        body.extend_from_slice(&i32c(1024));
        body.extend_from_slice(&i32c(32));
        body.extend_from_slice(&i32c(960));
        body.extend_from_slice(&call(http_get));
        body.extend_from_slice(&dropv());
    }
    body.extend_from_slice(&i32c(0));
    let work = w.func(&[], &[I32], &[], &code(&[&body]));
    w.export_func("work", work);
    w.finish()
}

/// The fuel burner: `loop { br 0 }` — an infinite loop the per-call
/// fuel budget must kill (doc 06 §7.5 trap loop / §9.4
/// `plugin_trap_loop` shape). The trailing `i32.const 0` satisfies
/// the validator; the fuel kill makes it unreachable.
pub fn crash_fuel_burner_wasm() -> Vec<u8> {
    let mut w = Wasm::new();
    let spin = w.func(
        &[],
        &[I32],
        &[],
        &code(&[&loopv(), &brv(0), &endv(), &i32c(0)]),
    );
    w.export_func("work", spin);
    w.finish()
}

/// The memory hog: grows one page at a time until the store limiter
/// traps (doc 06 §7.5 memory-exhaustion loop). Module max is 100
/// pages — ABOVE the tiny-limit 1 MiB host cap — so the denial
/// provably comes from the host limiter.
pub fn crash_oom_wasm() -> Vec<u8> {
    let mut w = Wasm::new();
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
    w.export_func("work", grow);
    w.finish()
}

/// The healthy survivor: one log record, no fault — proves the host
/// (and any other instance) keeps serving after a contained crash.
pub fn crash_survivor_wasm() -> Vec<u8> {
    let mut w = Wasm::new();
    let log_write = w.import("lumen:v1.log", "log_write", &[I32, I32, I32], &[I32]);
    w.data(0, b"survivor-still-alive");
    let hello = w.func(
        &[],
        &[I32],
        &[],
        &code(&[&i32c(2), &i32c(0), &i32c(20), &call(log_write)]),
    );
    w.export_func("hello", hello);
    w.finish()
}
