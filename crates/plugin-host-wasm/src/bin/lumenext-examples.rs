//! `lumenext-examples` — the example-extension module generator (card
//! 7.G1, issue #46): emits the committed `plugin.wasm` binaries under
//! `plugins/extensions/<name>/` deterministically, from the same
//! public-spec wasm-binary emitter pattern the integration suites use
//! (the WebAssembly binary format is public-spec math — clean-room;
//! no `wat`/`wasm-tools` dependency, AGENTS §3.2).
//!
//! Why a generator and not hand-checked-in blobs: the examples are
//! real loadable modules whose bytes are pinned by the provenance
//! digest in every packaged `.lumenext`; `--check` regenerates and
//! diffs them, so a stray edit to a committed blob is a loud CI
//! failure, never a silent drift.
//!
//! The two examples (doc 06 §2.3 (c)/(d) shapes):
//!
//! - `ext.example.metadata` (`metadata-provider`) — the command-layer
//!   request/response protocol on the v1 integer ABI: the engine
//!   issues `extract(path_ptr, path_len)`, the guest opens the media
//!   sidecar inside its `fs-sandboxed-read` jail, and the host reads
//!   the tag text out of the guest's own linear memory (the
//!   no-host-pointers model, doc 06 §5.2).
//! - `ext.example.panel` (`ui-panel`) — registers a declarative panel
//!   descriptor through `lumen:v1.ui::ui_register_panel` (doc 06
//!   §5.8: panels exchange data through the command layer only) and
//!   answers `on_command` events.
//!
//! Usage:
//!
//! ```text
//! lumenext-examples [--check] [<repo-root>]
//!   (default root: the current directory)
//!   --check: verify the committed modules match regeneration;
//!            exit 1 with a report when they do not.
//! ```

use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

// ---------------------------------------------------------------------------
// The wasm binary emitter (public-spec format, std-only — the trimmed
// twin of the fixture emitter in tests/common/mod.rs)
// ---------------------------------------------------------------------------

/// `i32` value type byte.
const I32: u8 = 0x7F;

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

/// Signed LEB128 (i32.const immediates).
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

/// One local function: locals + raw code (the terminating `end` is
/// appended by `finish`).
struct Body {
    locals: Vec<(u32, u8)>,
    code: Vec<u8>,
}

/// A minimal module builder for the example guests.
struct Wasm {
    types: Vec<(Vec<u8>, Vec<u8>)>,
    imports: Vec<(String, String, usize)>,
    funcs: Vec<usize>,
    bodies: Vec<Body>,
    mem: Option<(u32, u32)>,
    exports: Vec<(String, u8, u32)>,
    data: Vec<(u32, Vec<u8>)>,
}

impl Wasm {
    fn new() -> Wasm {
        Wasm {
            types: Vec::new(),
            imports: Vec::new(),
            funcs: Vec::new(),
            bodies: Vec::new(),
            mem: Some((1, 16)),
            exports: Vec::new(),
            data: Vec::new(),
        }
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
    fn import(&mut self, module: &str, field: &str, params: &[u8], results: &[u8]) -> u32 {
        let ty = self.ty(params, results);
        self.imports
            .push((module.to_string(), field.to_string(), ty));
        (self.imports.len() - 1) as u32
    }

    /// Declare a local function; returns its function index.
    fn func(&mut self, params: &[u8], results: &[u8], locals: &[(u32, u8)], code: Vec<u8>) -> u32 {
        let ty = self.ty(params, results);
        self.funcs.push(ty);
        self.bodies.push(Body {
            locals: locals.to_vec(),
            code,
        });
        (self.imports.len() + self.funcs.len() - 1) as u32
    }

    fn export_func(&mut self, name: &str, idx: u32) {
        self.exports.push((name.to_string(), 0x00, idx));
    }

    fn data(&mut self, offset: u32, bytes: &[u8]) {
        self.data.push((offset, bytes.to_vec()));
    }

    /// Serialize (magic + version + canonical section order; the
    /// memory is exported under the conventional `memory` name).
    fn finish(mut self) -> Vec<u8> {
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

// -- instruction helpers (bodies are short concatenations) -------------------

/// Concatenate code parts.
fn code(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

/// `i32.const v`.
fn i32c(v: i32) -> Vec<u8> {
    let mut out = vec![0x41];
    sleb(i64::from(v), &mut out);
    out
}

/// `call f`.
fn call(f: u32) -> Vec<u8> {
    let mut out = vec![0x10];
    uleb(u64::from(f), &mut out);
    out
}

/// `drop`.
fn dropv() -> Vec<u8> {
    vec![0x1A]
}

/// `local.get i`.
fn lget(i: u32) -> Vec<u8> {
    let mut out = vec![0x20];
    uleb(u64::from(i), &mut out);
    out
}

/// `local.set i`.
fn lset(i: u32) -> Vec<u8> {
    let mut out = vec![0x21];
    uleb(u64::from(i), &mut out);
    out
}

/// `i32.load` (align 2, offset 0) — reads the little-endian u32 the
/// host wrote into a fixed layout slot (`ptr; i32.load`).
fn i32load() -> Vec<u8> {
    vec![0x28, 0x02, 0x00]
}

/// `i32.eqz` — invert a status for the `if` guards.
fn eqz() -> Vec<u8> {
    vec![0x45]
}

/// `if` with void block type.
fn ifv() -> Vec<u8> {
    vec![0x04, 0x40]
}

/// `end`.
fn endv() -> Vec<u8> {
    vec![0x0B]
}

// ---------------------------------------------------------------------------
// The shared guest memory layout (documented in plugins/extensions/README.md)
// ---------------------------------------------------------------------------

/// Data slot: the self-reported manifest (the F1 honesty handshake).
const REPORT_AT: i32 = 0;
/// Data slot: the ready log line.
const MSG_AT: i32 = 1024;
/// Data slot (metadata): the `fs_open` handle out-parameter.
const HANDLE_AT: i32 = 2048;
/// Data slot (metadata): the `fs_read` length out-parameter.
const OUT_LEN_AT: i32 = 2052;
/// The guest's result buffer: `fs_read` lands the sidecar bytes here.
const RESULT_AT: i32 = 4096;
/// The extract read cap (one bounded transfer; the host-side ceiling
/// is doc 06 §5.4 row 4 — this is the example's own working set).
const READ_CAP: i32 = 4096;
/// Data slot (ui-panel): the panel descriptor.
const DESC_AT: i32 = 2048;

/// The self-reported manifest text both examples carry in their data
/// segment (the handshake checks parse + identity; the provenance
/// digest here is the digest of the empty string, the established
/// fixture convention — the INSTALLED manifest's digest is the real
/// one, enforced at install and every load).
fn report_manifest(id: &str, kind: &str, capabilities: &str, perms: &str) -> String {
    format!(
        "id = \"{id}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\napi_version = 1\n\
         origin = \"extension\"\n\n[capabilities]\n{capabilities}\n\n{perms}\n[provenance]\n\
         publisher = \"Example Extensions\"\nsource = \
         \"https://example.com/lumen/{id}-0.1.0.lumenext\"\nhash = \
         \"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\"\n\
         license = \"MIT\"\n"
    )
}

// ---------------------------------------------------------------------------
// ext.example.metadata — the metadata-provider example
// ---------------------------------------------------------------------------

/// Build `plugins/extensions/metadata-provider/plugin.wasm`.
///
/// Protocol (the command-layer request/response the conformance kit
/// drives, doc 06 §5.2):
///
/// | Export | Shape | Behavior |
/// |---|---|---|
/// | `init` | `() -> i32` | api handshake, `report_manifest`, ready log; returns the negotiated api major |
/// | `extract` | `(path_ptr, path_len) -> i32` | `fs_open` the sidecar inside the jail, `fs_read` up to 4 KiB into the result buffer, `fs_close`; returns the last import status (`0` = ok — the host then reads the length at `OUT_LEN_AT` and the tags at `RESULT_AT`) |
fn metadata_module() -> Vec<u8> {
    let report = report_manifest(
        "ext.example.metadata",
        "metadata-provider",
        "provides = [\"tags\"]",
        "[permissions]\nfs-sandboxed-read = true",
    );
    let msg: &[u8] = b"metadata-provider example ready";
    let mut w = Wasm::new();
    let api_version = w.import("lumen:v1.host", "api_version", &[], &[I32]);
    let report_manifest = w.import("lumen:v1.host", "report_manifest", &[I32, I32], &[I32]);
    let log_write = w.import("lumen:v1.log", "log_write", &[I32, I32, I32], &[I32]);
    let fs_open = w.import("lumen:v1.fs", "fs_open", &[I32, I32, I32], &[I32]);
    let fs_read = w.import("lumen:v1.fs", "fs_read", &[I32, I32, I32, I32], &[I32]);
    let fs_close = w.import("lumen:v1.fs", "fs_close", &[I32], &[I32]);
    w.data(REPORT_AT as u32, report.as_bytes());
    w.data(MSG_AT as u32, msg);

    // init: negotiate, report the manifest, log readiness, return the
    // negotiated major (no params: the one declared i32 local is index 0).
    let init = w.func(
        &[],
        &[I32],
        &[(1, I32)],
        code(&[
            &call(api_version),
            &lset(0),
            &i32c(REPORT_AT),
            &i32c(report.len() as i32),
            &call(report_manifest),
            &dropv(),
            &i32c(2),
            &i32c(MSG_AT),
            &i32c(msg.len() as i32),
            &call(log_write),
            &dropv(),
            &lget(0),
        ]),
    );
    // extract: open the sidecar (jailed), read it into the result
    // buffer, close. The status of the failing import (if any) is the
    // return value. locals: 0 = path_ptr, 1 = path_len, 2 = status.
    let extract = w.func(
        &[I32, I32],
        &[I32],
        &[(1, I32)],
        code(&[
            // status = fs_open(path_ptr, path_len, &handle)
            &lget(0),
            &lget(1),
            &i32c(HANDLE_AT),
            &call(fs_open),
            &lset(2),
            // if status == 0: read the sidecar, close the handle
            &lget(2),
            &eqz(),
            &ifv(),
            &i32c(HANDLE_AT),
            &i32load(),
            &i32c(RESULT_AT),
            &i32c(READ_CAP),
            &i32c(OUT_LEN_AT),
            &call(fs_read),
            &lset(2),
            &i32c(HANDLE_AT),
            &i32load(),
            &call(fs_close),
            &dropv(),
            &endv(),
            &lget(2),
        ]),
    );
    w.export_func("init", init);
    w.export_func("extract", extract);
    w.finish()
}

// ---------------------------------------------------------------------------
// ext.example.panel — the ui-panel example
// ---------------------------------------------------------------------------

/// Build `plugins/extensions/ui-panel/plugin.wasm`.
///
/// Protocol (doc 06 §5.8 — the command layer only):
///
/// | Export | Shape | Behavior |
/// |---|---|---|
/// | `init` | `() -> i32` | api handshake, `report_manifest`, `ui_register_panel` with the declarative descriptor, ready log; returns the negotiated api major |
/// | `on_command` | `(cmd_ptr, cmd_len) -> i32` | observes one command-layer event: logs the payload through `lumen:v1.log` (the observable acknowledgment) and returns the import status |
fn panel_module() -> Vec<u8> {
    let report = report_manifest(
        "ext.example.panel",
        "ui-panel",
        "surfaces = [\"side-panel\"]",
        "[permissions]\nui-surface = true",
    );
    let msg: &[u8] = b"now-playing panel example ready";
    let descriptor: &[u8] = b"{\"panel_id\":\"main\",\"title_key\":\"panel.example.now-playing.title\",\"surface\":\"side-panel\"}";
    let mut w = Wasm::new();
    let api_version = w.import("lumen:v1.host", "api_version", &[], &[I32]);
    let report_manifest = w.import("lumen:v1.host", "report_manifest", &[I32, I32], &[I32]);
    let log_write = w.import("lumen:v1.log", "log_write", &[I32, I32, I32], &[I32]);
    let register = w.import("lumen:v1.ui", "ui_register_panel", &[I32, I32], &[I32]);
    w.data(REPORT_AT as u32, report.as_bytes());
    w.data(MSG_AT as u32, msg);
    w.data(DESC_AT as u32, descriptor);

    let init = w.func(
        &[],
        &[I32],
        &[(1, I32)],
        code(&[
            &call(api_version),
            &lset(0),
            &i32c(REPORT_AT),
            &i32c(report.len() as i32),
            &call(report_manifest),
            &dropv(),
            &i32c(DESC_AT),
            &i32c(descriptor.len() as i32),
            &call(register),
            &dropv(),
            &i32c(2),
            &i32c(MSG_AT),
            &i32c(msg.len() as i32),
            &call(log_write),
            &dropv(),
            &lget(0),
        ]),
    );
    // on_command(cmd_ptr, cmd_len): log the event text, return the
    // import status (0 = observed).
    let on_command = w.func(
        &[I32, I32],
        &[I32],
        &[],
        code(&[&i32c(2), &lget(0), &lget(1), &call(log_write)]),
    );
    w.export_func("init", init);
    w.export_func("on_command", on_command);
    w.finish()
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

/// A failure with its exit class.
enum Failure {
    /// Bad command line.
    Usage(String),
    /// Filesystem failure.
    Io(String),
    /// A committed module does not match regeneration (`--check`).
    Drift(String),
}

impl Failure {
    fn code(&self) -> u8 {
        match self {
            Failure::Usage(_) | Failure::Drift(_) => 1,
            Failure::Io(_) => 2,
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Usage(msg) => write!(f, "usage: {msg}"),
            Failure::Io(msg) => write!(f, "io: {msg}"),
            Failure::Drift(msg) => write!(f, "drift: {msg}"),
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("lumenext-examples: {err}");
            ExitCode::from(err.code())
        }
    }
}

fn run(args: &[String]) -> Result<(), Failure> {
    let mut check = false;
    let mut root: Option<PathBuf> = None;
    for arg in args {
        match arg.as_str() {
            "--check" => check = true,
            other if !other.starts_with("--") && root.is_none() => root = Some(other.into()),
            other => {
                return Err(Failure::Usage(format!(
                    "unexpected argument `{other}` \
                     (usage: lumenext-examples [--check] [<repo-root>])"
                )));
            }
        }
    }
    let root = root.unwrap_or_else(|| PathBuf::from("."));
    let examples = root.join("plugins").join("extensions");
    let modules: [(&str, fn() -> Vec<u8>); 2] = [
        ("metadata-provider", metadata_module),
        ("ui-panel", panel_module),
    ];
    let mut written = 0usize;
    let mut drifted = 0usize;
    for (name, build) in modules {
        let bytes = build();
        let path = examples.join(name).join("plugin.wasm");
        let shown = path.display().to_string();
        let current = match fs::read(&path) {
            Ok(current) => Some(current),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(Failure::Io(format!("{shown}: {e}"))),
        };
        if current.as_deref() == Some(&bytes[..]) {
            continue;
        }
        if check {
            eprintln!(
                "drift: {shown} ({} bytes committed, {} regenerated)",
                current.map_or(0, |c| c.len()),
                bytes.len()
            );
            drifted += 1;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| Failure::Io(format!("{}: {e}", parent.display())))?;
            }
            fs::write(&path, &bytes).map_err(|e| Failure::Io(format!("{shown}: {e}")))?;
            println!("lumenext-examples: wrote {shown} ({} bytes)", bytes.len());
            written += 1;
        }
    }
    if check {
        if drifted > 0 {
            return Err(Failure::Drift(format!(
                "{drifted} committed module(s) do not match regeneration — \
                 rerun without --check to rewrite them"
            )));
        }
        println!("lumenext-examples: all committed modules match regeneration");
    } else if written == 0 {
        println!("lumenext-examples: all modules already up to date");
    } else {
        println!("lumenext-examples: {written} module(s) written, deterministic");
    }
    Ok(())
}
