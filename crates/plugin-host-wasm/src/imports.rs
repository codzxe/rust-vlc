//! Host-side implementations of every `lumen:v1.*` import (doc 06
//! §5.2–§5.3) and the per-instance import-table builder.
//!
//! # Dispatch model (doc 06 §5.3)
//!
//! The table is built per instance from the manifest's **declared**
//! permissions: a module whose permission was not declared is simply
//! absent, so a guest importing it fails instantiation with
//! [`HostError::UndeclaredImport`] before any guest code runs — that
//! is §2.2's "escalation attempt" case (logged as a security event;
//! "trap the instance" is satisfied by the instance never coming
//! alive). Within a *granted* module, a call that exceeds a
//! sub-policy (a host outside `network-host-allowlist`, a path
//! outside the jail) returns the **typed `E_PERM` status the guest
//! observes** (§5.3), is logged, and is counted; reaching the session
//! threshold faults the instance and disables the extension
//! (`[PROPOSAL]` reconciliation of §2.2 vs §5.3 recorded for the
//! Lead).
//!
//! # No host pointers (doc 06 §5.2)
//!
//! Every buffer argument is `(ptr, len)` into the guest's own linear
//! memory, read/written through wasmi's bounds-checked `Memory`
//! accessors; handles are indices into the per-instance host table,
//! validated on every use.

use wasmi::{Caller, Engine, Extern, Linker, Memory};

use plugin_api::ENGINE_PLUGIN_API_MAJOR;

use crate::api::{
    GuestStatus, Grants, MOD_DEVICES, MOD_FS, MOD_HOST, MOD_HTTP, MOD_LOG, MOD_UI,
    IMP_DEVICES_LIST, IMP_FS_CLOSE, IMP_FS_OPEN, IMP_FS_READ, IMP_FS_SIZE,
    IMP_HOST_REPORT_MANIFEST, IMP_HTTP_GET, IMP_LOG_WRITE, IMP_UI_REGISTER_PANEL,
};
use crate::error::HostError;
use crate::limits::ESCALATION_DISABLE_THRESHOLD;
use crate::state::{InstanceState, SecurityEvent};

/// The store data type all imports close over.
pub type HostState = InstanceState;

/// A host-injected fault: used only to stop a guest mid-call when
/// the escalation threshold trips (the disable path, doc 06 §5.3) —
/// the guest observes a trap; the host observes [`FaultKind`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultKind {
    /// Escalation threshold reached: extension disabled, instance
    /// faulted.
    EscalationDisabled { count: u32 },
}

#[derive(Debug)]
pub struct HostFault(FaultKind);

impl std::fmt::Display for HostFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            FaultKind::EscalationDisabled { count } => write!(
                f,
                "escalation threshold reached ({count} permission \
                 denials this session) — extension disabled, instance \
                 faulted (doc 06 §5.3)"
            ),
        }
    }
}

impl std::error::Error for HostFault {}

impl wasmi::errors::HostError for HostFault {}

impl HostFault {
    /// The fault kind.
    pub fn kind(&self) -> &FaultKind {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Guest-memory helpers (the no-host-pointers model, doc 06 §5.2)
// ---------------------------------------------------------------------------

/// The guest's exported linear memory (`export "memory"`), if any.
fn guest_memory(caller: &Caller<HostState>) -> Option<Memory> {
    caller.get_export("memory").and_then(Extern::into_memory)
}

/// Read `len` bytes at `ptr` from guest memory with cap checks.
///
/// OOB offsets (wasmi bounds check) and a missing memory export both
/// yield `E_OOB` — the typed §9.4 denial — and are recorded as
/// security events (shape only).
fn read_guest(
    caller: &mut Caller<HostState>,
    import: &str,
    ptr: i32,
    len: i32,
    caps: (u64, u64),
) -> Result<Vec<u8>, i32> {
    if ptr < 0 || len < 0 {
        return Err(GuestStatus::EArg.code());
    }
    let cap = caps.0.min(caps.1);
    if len as u64 > cap {
        return Err(GuestStatus::ETooLarge.code());
    }
    if len == 0 {
        return Ok(Vec::new());
    }
    let Some(memory) = guest_memory(caller) else {
        caller.data_mut().ledger.record_escalation(SecurityEvent::new(
            import,
            "buffer passed but the module exports no `memory`",
            format!("ptr={ptr}, len={len}"),
        ));
        return Err(GuestStatus::EOob.code());
    };
    let mut buf = vec![0u8; len as usize];
    match memory.read(&*caller, ptr as usize, &mut buf) {
        Ok(()) => Ok(buf),
        Err(_) => {
            let size = memory.size(&*caller) * 64 * 1024;
            caller.data_mut().ledger.record_escalation(SecurityEvent::new(
                import,
                "out-of-bounds guest buffer (sandbox escape attempt)",
                format!("ptr={ptr}, len={len}, memory_bytes={size}"),
            ));
            Err(GuestStatus::EOob.code())
        }
    }
}

/// Write `data` at `ptr` in guest memory (out-parameters).
fn write_guest(
    caller: &mut Caller<HostState>,
    ptr: i32,
    data: &[u8],
) -> Result<(), i32> {
    if ptr < 0 {
        return Err(GuestStatus::EArg.code());
    }
    if data.is_empty() {
        return Ok(());
    }
    let Some(memory) = guest_memory(caller) else {
        return Err(GuestStatus::EOob.code());
    };
    memory
        .write(&mut *caller, ptr as usize, data)
        .map_err(|_| GuestStatus::EOob.code())
}

/// Write a little-endian u32 out-parameter.
fn write_guest_u32(
    caller: &mut Caller<HostState>,
    ptr: i32,
    value: u32,
) -> Result<(), i32> {
    write_guest(caller, ptr, &value.to_le_bytes())
}

/// Write a little-endian u64 out-parameter.
fn write_guest_u64(
    caller: &mut Caller<HostState>,
    ptr: i32,
    value: u64,
) -> Result<(), i32> {
    write_guest(caller, ptr, &value.to_le_bytes())
}

/// Record a permission denial (the §5.3 security event + counter) and
/// produce the `E_PERM` status — or, when the threshold trips, the
/// host fault that stops the guest.
fn deny(
    caller: &mut Caller<HostState>,
    import: &str,
    reason: String,
    arg_shape: String,
) -> Result<i32, wasmi::Error> {
    let count = caller
        .data_mut()
        .ledger
        .record_escalation(SecurityEvent::new(import, reason, arg_shape));
    if count >= ESCALATION_DISABLE_THRESHOLD {
        return Err(wasmi::Error::host(HostFault(FaultKind::EscalationDisabled {
            count,
        })));
    }
    Ok(GuestStatus::EPerm.code())
}

// ---------------------------------------------------------------------------
// The import table builder (doc 06 §5.3: only declared permissions)
// ---------------------------------------------------------------------------

/// Build the per-instance `Linker` containing exactly the modules the
/// manifest's permissions grant (doc 06 §5.3).
///
/// # Errors
/// [`HostError::Instantiation`] only if wasmi rejects a definition
/// (a duplicate name — impossible by construction, but handled
/// honestly rather than unwrapped).
pub fn build_linker(engine: &Engine, grants: &Grants) -> Result<Linker<HostState>, HostError> {
    let mut linker = <Linker<HostState>>::new(engine);
    define_host(&mut linker)?;
    define_log(&mut linker)?;
    if grants.fs {
        define_fs(&mut linker)?;
    }
    if grants.http {
        define_http(&mut linker)?;
    }
    if grants.ui {
        define_ui(&mut linker)?;
    }
    if grants.devices {
        define_devices(&mut linker)?;
    }
    Ok(linker)
}

/// Import-policy pre-check (doc 06 §5.3): every module the guest
/// imports must be a known `lumen:v1.*` module AND granted by the
/// manifest. Called before instantiation so violations fail before
/// any guest code runs.
pub fn check_import_policy(
    module: &wasmi::Module,
    grants: &Grants,
) -> Result<(), HostError> {
    for import in module.imports() {
        let module_name = import.module().to_string();
        let Some(spec) = crate::api::lookup_module(&module_name) else {
            return Err(HostError::UnknownImportModule {
                module: module_name,
            });
        };
        if !grants.has(spec.permission) {
            return Err(HostError::UndeclaredImport {
                module: module_name,
                name: import.name().to_string(),
                permission: spec.permission.as_str(),
            });
        }
    }
    Ok(())
}

// -- lumen:v1.host ----------------------------------------------------------

fn define_host(linker: &mut Linker<HostState>) -> Result<(), HostError> {
    linker
        .func_wrap(MOD_HOST, "api_version", |_caller: Caller<HostState>| -> i32 {
            ENGINE_PLUGIN_API_MAJOR as i32
        })
        .and_then(|l| {
            l.func_wrap(
                MOD_HOST,
                "report_manifest",
                |mut caller: Caller<HostState>, ptr: i32, len: i32| -> Result<i32, wasmi::Error> {
                    let import = IMP_HOST_REPORT_MANIFEST;
                    let caps = (
                        caller.data().limits.max_descriptor_bytes,
                        caller.data().limits.io_transfer_bytes,
                    );
                    let bytes = match read_guest(&mut caller, import, ptr, len, caps) {
                        Ok(bytes) => bytes,
                        Err(status) => {
                            caller.data_mut().trace_import(import, status);
                            return Ok(status);
                        }
                    };
                    let text = match String::from_utf8(bytes) {
                        Ok(text) => text,
                        Err(_) => {
                            caller.data_mut().trace_import(import, GuestStatus::EArg.code());
                            return Ok(GuestStatus::EArg.code());
                        }
                    };
                    // Parse as a real manifest (the plugin-api
                    // validator — same rules as install time) and
                    // compare against the installed identity, inside
                    // one immutable borrow of the store state.
                    let accepted = {
                        let expected = &caller.data().expectation;
                        match plugin_api::PluginManifest::from_toml(&text) {
                            Ok(reported) => {
                                reported.id.as_str() == expected.id
                                    && reported.kind == expected.kind
                                    && reported.version.to_string() == expected.version
                                    && reported.api_version == expected.api_version
                            }
                            Err(_) => false,
                        }
                    };
                    if accepted {
                        caller.data_mut().manifest_report = Some(text);
                        caller.data_mut().trace_import(import, GuestStatus::Ok.code());
                        return Ok(GuestStatus::Ok.code());
                    }
                    // A guest reporting a manifest that does not
                    // parse, or does not match the installed
                    // identity, is a manifest lie (doc 06 §2.2):
                    // counted as a security event; the threshold
                    // fault applies.
                    let count = caller.data_mut().ledger.record_escalation(SecurityEvent::new(
                        import,
                        "reported manifest does not parse or does not match \
                         the installed identity (manifest lie)"
                            .to_string(),
                        format!("manifest_bytes={len}"),
                    ));
                    caller.data_mut().trace_import(import, GuestStatus::EArg.code());
                    if count >= ESCALATION_DISABLE_THRESHOLD {
                        return Err(wasmi::Error::host(HostFault(
                            FaultKind::EscalationDisabled { count },
                        )));
                    }
                    Ok(GuestStatus::EArg.code())
                },
            )
        })
        .map(|_| ())
        .map_err(|e| HostError::Instantiation(e.to_string()))
}

// -- lumen:v1.log -----------------------------------------------------------

fn define_log(linker: &mut Linker<HostState>) -> Result<(), HostError> {
    linker
        .func_wrap(
            MOD_LOG,
            "log_write",
            |mut caller: Caller<HostState>, level: i32, ptr: i32, len: i32| -> i32 {
                let import = IMP_LOG_WRITE;
                if !(0..=4).contains(&level) {
                    caller.data_mut().trace_import(import, GuestStatus::EArg.code());
                    return GuestStatus::EArg.code();
                }
                let caps = (4096, caller.data().limits.io_transfer_bytes);
                let bytes = match read_guest(&mut caller, import, ptr, len, caps) {
                    Ok(bytes) => bytes,
                    Err(status) => {
                        caller.data_mut().trace_import(import, status);
                        return status;
                    }
                };
                // Log records are I/O too (they leave the sandbox):
                // charge the token bucket.
                if !caller
                    .data_mut()
                    .io_bucket
                    .take(bytes.len() as u64)
                {
                    caller.data_mut().trace_import(import, GuestStatus::EQuota.code());
                    return GuestStatus::EQuota.code();
                }
                let line = String::from_utf8_lossy(&bytes).to_string();
                caller.data_mut().log_record(level as u8, line);
                caller.data_mut().trace_import(import, GuestStatus::Ok.code());
                GuestStatus::Ok.code()
            },
        )
        .map(|_| ())
        .map_err(|e| HostError::Instantiation(e.to_string()))
}

// -- lumen:v1.fs ------------------------------------------------------------

fn define_fs(linker: &mut Linker<HostState>) -> Result<(), HostError> {
    linker
        .func_wrap(
            MOD_FS,
            "fs_open",
            |mut caller: Caller<HostState>, path_ptr: i32, path_len: i32, out_handle: i32| -> Result<i32, wasmi::Error> {
                let import = IMP_FS_OPEN;
                let caps = (4096, caller.data().limits.io_transfer_bytes);
                let bytes = match read_guest(&mut caller, import, path_ptr, path_len, caps) {
                    Ok(bytes) => bytes,
                    Err(status) => {
                        caller.data_mut().trace_import(import, status);
                        return Ok(status);
                    }
                };
                let path = String::from_utf8_lossy(&bytes).to_string();
                let jail = match caller.data().services.jail.clone() {
                    Some(jail) => jail,
                    None => {
                        let shape = format!("path_len={}", path.len());
                        caller.data_mut().trace_import(import, GuestStatus::EPerm.code());
                        return deny(
                            &mut caller,
                            import,
                            "fs-sandboxed-read granted but no sandbox roots \
                             are configured"
                                .to_string(),
                            shape,
                        );
                    }
                };
                let resolved = match jail.resolve(&path) {
                    Ok(resolved) => resolved,
                    Err(HostError::PermissionDenied { detail }) => {
                        let shape = format!("path_len={}", path.len());
                        caller.data_mut().trace_import(import, GuestStatus::EPerm.code());
                        return deny(&mut caller, import, detail, shape);
                    }
                    Err(other) => {
                        caller.data_mut().trace_import(import, GuestStatus::EIo.code());
                        let _ = other;
                        return Ok(GuestStatus::EIo.code());
                    }
                };
                // Read-only, files only (directories refuse with E_ARG).
                let meta = match std::fs::metadata(&resolved) {
                    Ok(meta) => meta,
                    Err(_) => {
                        caller.data_mut().trace_import(import, GuestStatus::EIo.code());
                        return Ok(GuestStatus::EIo.code());
                    }
                };
                if !meta.is_file() {
                    caller.data_mut().trace_import(import, GuestStatus::EArg.code());
                    return Ok(GuestStatus::EArg.code());
                }
                // Size is bounded by the single-transfer cap (a
                // 100 GiB "media file" cannot be slurped in one go).
                let cap = caller.data().limits.io_transfer_bytes;
                if meta.len() > cap {
                    caller.data_mut().trace_import(import, GuestStatus::ETooLarge.code());
                    return Ok(GuestStatus::ETooLarge.code());
                }
                let content = match std::fs::read(&resolved) {
                    Ok(content) => content,
                    Err(_) => {
                        caller.data_mut().trace_import(import, GuestStatus::EIo.code());
                        return Ok(GuestStatus::EIo.code());
                    }
                };
                let handle_cap = caller.data().limits.max_open_handles;
                let display = resolved.to_string_lossy().to_string();
                let handle = match caller
                    .data_mut()
                    .handles
                    .insert(display, content, handle_cap)
                {
                    Ok(handle) => handle,
                    Err(detail) => {
                        caller.data_mut().trace_import(import, GuestStatus::ELimit.code());
                        let _ = detail;
                        return Ok(GuestStatus::ELimit.code());
                    }
                };
                match write_guest_u32(&mut caller, out_handle, handle) {
                    Ok(()) => {
                        caller.data_mut().trace_import(import, GuestStatus::Ok.code());
                        Ok(GuestStatus::Ok.code())
                    }
                    Err(status) => {
                        caller.data_mut().trace_import(import, status);
                        Ok(status)
                    }
                }
            },
        )
        .and_then(|l| {
            l.func_wrap(
                MOD_FS,
                "fs_read",
                |mut caller: Caller<HostState>,
                 handle: i32,
                 buf_ptr: i32,
                 buf_cap: i32,
                 out_len: i32|
                 -> Result<i32, wasmi::Error> {
                    let import = IMP_FS_READ;
                    if handle < 0 || buf_ptr < 0 || buf_cap < 0 {
                        caller.data_mut().trace_import(import, GuestStatus::EArg.code());
                        return Ok(GuestStatus::EArg.code());
                    }
                    let transfer = caller.data().limits.io_transfer_bytes;
                    if buf_cap as u64 > transfer {
                        caller.data_mut().trace_import(import, GuestStatus::ETooLarge.code());
                        return Ok(GuestStatus::ETooLarge.code());
                    }
                    // Charge the rate bucket before reading.
                    if !caller
                        .data_mut()
                        .io_bucket
                        .take(buf_cap as u64)
                    {
                        caller.data_mut().trace_import(import, GuestStatus::EQuota.code());
                        return Ok(GuestStatus::EQuota.code());
                    }
                    let chunk = match caller
                        .data_mut()
                        .handles
                        .read(handle as u32, buf_cap as usize)
                    {
                        Some(chunk) => chunk,
                        None => {
                            caller.data_mut().trace_import(import, GuestStatus::EHandle.code());
                            return Ok(GuestStatus::EHandle.code());
                        }
                    };
                    let n = chunk.len();
                    match write_guest(&mut caller, buf_ptr, &chunk).and_then(|()| {
                        write_guest_u32(&mut caller, out_len, n as u32)
                    }) {
                        Ok(()) => {
                            caller.data_mut().trace_import(import, GuestStatus::Ok.code());
                            Ok(GuestStatus::Ok.code())
                        }
                        Err(status) => {
                            caller.data_mut().trace_import(import, status);
                            Ok(status)
                        }
                    }
                },
            )
        })
        .and_then(|l| {
            l.func_wrap(
                MOD_FS,
                "fs_size",
                |mut caller: Caller<HostState>, handle: i32, out_size: i32| -> i32 {
                    let import = IMP_FS_SIZE;
                    if handle < 0 {
                        caller.data_mut().trace_import(import, GuestStatus::EArg.code());
                        return GuestStatus::EArg.code();
                    }
                    match caller.data_mut().handles.size(handle as u32) {
                        Some(size) => match write_guest_u64(&mut caller, out_size, size) {
                            Ok(()) => {
                                caller.data_mut().trace_import(import, GuestStatus::Ok.code());
                                GuestStatus::Ok.code()
                            }
                            Err(status) => {
                                caller.data_mut().trace_import(import, status);
                                status
                            }
                        },
                        None => {
                            caller.data_mut().trace_import(import, GuestStatus::EHandle.code());
                            GuestStatus::EHandle.code()
                        }
                    }
                },
            )
        })
        .and_then(|l| {
            l.func_wrap(
                MOD_FS,
                "fs_close",
                |mut caller: Caller<HostState>, handle: i32| -> i32 {
                    let import = IMP_FS_CLOSE;
                    if handle < 0 {
                        caller.data_mut().trace_import(import, GuestStatus::EArg.code());
                        return GuestStatus::EArg.code();
                    }
                    match caller.data_mut().handles.close(handle as u32) {
                        Ok(()) => {
                            caller.data_mut().trace_import(import, GuestStatus::Ok.code());
                            GuestStatus::Ok.code()
                        }
                        Err(_) => {
                            caller.data_mut().trace_import(import, GuestStatus::EHandle.code());
                            GuestStatus::EHandle.code()
                        }
                    }
                },
            )
        })
        .map(|_| ())
        .map_err(|e| HostError::Instantiation(e.to_string()))
}

// -- lumen:v1.http ----------------------------------------------------------

fn define_http(linker: &mut Linker<HostState>) -> Result<(), HostError> {
    linker
        .func_wrap(
            MOD_HTTP,
            "http_get",
            |mut caller: Caller<HostState>,
             url_ptr: i32,
             url_len: i32,
             out_ptr: i32,
             out_cap: i32,
             out_len: i32|
             -> Result<i32, wasmi::Error> {
                let import = IMP_HTTP_GET;
                if out_ptr < 0 || out_cap < 0 {
                    caller.data_mut().trace_import(import, GuestStatus::EArg.code());
                    return Ok(GuestStatus::EArg.code());
                }
                let caps = (2048, caller.data().limits.io_transfer_bytes);
                let url_bytes = match read_guest(&mut caller, import, url_ptr, url_len, caps) {
                    Ok(bytes) => bytes,
                    Err(status) => {
                        caller.data_mut().trace_import(import, status);
                        return Ok(status);
                    }
                };
                let url = String::from_utf8_lossy(&url_bytes).to_string();
                // Policy chain, all in front of the transport seam:
                // parse → allowlist → rate → transfer caps.
                let parsed = match crate::http::ParsedUrl::parse(&url) {
                    Ok(parsed) => parsed,
                    Err(HostError::PermissionDenied { detail }) => {
                        let shape = format!("url_len={}", url.len());
                        caller.data_mut().trace_import(import, GuestStatus::EArg.code());
                        return deny(&mut caller, import, detail, shape);
                    }
                    Err(_) => {
                        caller.data_mut().trace_import(import, GuestStatus::EArg.code());
                        return Ok(GuestStatus::EArg.code());
                    }
                };
                let allowed = parsed.allowed_by(&caller.data().http_allowlist);
                if !allowed {
                    let shape = format!("url_len={}, host_len={}", url.len(), parsed.host.len());
                    caller.data_mut().trace_import(import, GuestStatus::EPerm.code());
                    return deny(
                        &mut caller,
                        import,
                        format!("host `{}` is not in network-host-allowlist", parsed.host),
                        shape,
                    );
                }
                if !caller.data_mut().http_admit() {
                    caller.data_mut().trace_import(import, GuestStatus::EQuota.code());
                    return Ok(GuestStatus::EQuota.code());
                }
                let transfer = caller.data().limits.io_transfer_bytes;
                if out_cap as u64 > transfer {
                    caller.data_mut().trace_import(import, GuestStatus::ETooLarge.code());
                    return Ok(GuestStatus::ETooLarge.code());
                }
                if !caller
                    .data_mut()
                    .io_bucket
                    .take(out_cap as u64)
                {
                    caller.data_mut().trace_import(import, GuestStatus::EQuota.code());
                    return Ok(GuestStatus::EQuota.code());
                }
                // The seam: dispatch to the (possibly absent)
                // transport. Policy has already passed; everything
                // below is honest capability reporting.
                let transport = caller.data_mut().services.transport.as_mut();
                let response = match transport {
                    None => {
                        caller.data_mut().trace_import(import, GuestStatus::ETransport.code());
                        return Ok(GuestStatus::ETransport.code());
                    }
                    Some(transport) => transport.fetch(&parsed.url()),
                };
                match response {
                    Ok(resp) => {
                        let n = (resp.body.len() as u64).min(out_cap as u64) as usize;
                        match write_guest(&mut caller, out_ptr, &resp.body[..n]).and_then(|()| {
                            write_guest_u32(&mut caller, out_len, n as u32)
                        }) {
                            Ok(()) => {
                                caller.data_mut().trace_import(import, GuestStatus::Ok.code());
                                Ok(GuestStatus::Ok.code())
                            }
                            Err(status) => {
                                caller.data_mut().trace_import(import, status);
                                Ok(status)
                            }
                        }
                    }
                    Err(crate::http::TransportError::Unavailable) => {
                        caller.data_mut().trace_import(import, GuestStatus::ETransport.code());
                        Ok(GuestStatus::ETransport.code())
                    }
                    Err(crate::http::TransportError::Failed(_)) => {
                        caller.data_mut().trace_import(import, GuestStatus::EIo.code());
                        Ok(GuestStatus::EIo.code())
                    }
                }
            },
        )
        .map(|_| ())
        .map_err(|e| HostError::Instantiation(e.to_string()))
}

// -- lumen:v1.ui ------------------------------------------------------------

fn define_ui(linker: &mut Linker<HostState>) -> Result<(), HostError> {
    linker
        .func_wrap(
            MOD_UI,
            "ui_register_panel",
            |mut caller: Caller<HostState>, desc_ptr: i32, desc_len: i32| -> i32 {
                let import = IMP_UI_REGISTER_PANEL;
                let caps = (
                    caller.data().limits.max_descriptor_bytes,
                    caller.data().limits.io_transfer_bytes,
                );
                let bytes = match read_guest(&mut caller, import, desc_ptr, desc_len, caps) {
                    Ok(bytes) => bytes,
                    Err(status) => {
                        caller.data_mut().trace_import(import, status);
                        return status;
                    }
                };
                // v1 panel cap: 8 panels per instance (doc 06 §5.4
                // host-accounting posture; a UI spammer is refused,
                // not accommodated).
                if caller.data().panels.len() >= 8 {
                    caller.data_mut().trace_import(import, GuestStatus::ELimit.code());
                    return GuestStatus::ELimit.code();
                }
                let descriptor = String::from_utf8_lossy(&bytes).to_string();
                caller.data_mut().panels.push(crate::state::PanelRecord { descriptor });
                caller.data_mut().trace_import(import, GuestStatus::Ok.code());
                GuestStatus::Ok.code()
            },
        )
        .map(|_| ())
        .map_err(|e| HostError::Instantiation(e.to_string()))
}

// -- lumen:v1.devices -------------------------------------------------------

fn define_devices(linker: &mut Linker<HostState>) -> Result<(), HostError> {
    linker
        .func_wrap(
            MOD_DEVICES,
            "devices_list",
            |mut caller: Caller<HostState>, out_ptr: i32, out_cap: i32, out_len: i32| -> i32 {
                let import = IMP_DEVICES_LIST;
                if out_ptr < 0 || out_cap < 0 {
                    caller.data_mut().trace_import(import, GuestStatus::EArg.code());
                    return GuestStatus::EArg.code();
                }
                let transfer = caller.data().limits.io_transfer_bytes;
                if out_cap as u64 > transfer {
                    caller.data_mut().trace_import(import, GuestStatus::ETooLarge.code());
                    return GuestStatus::ETooLarge.code();
                }
                let inventory = caller.data().services.device_inventory.clone();
                let n = (inventory.len() as u64).min(out_cap as u64) as usize;
                match write_guest(&mut caller, out_ptr, &inventory.as_bytes()[..n]).and_then(|()| {
                    write_guest_u32(&mut caller, out_len, n as u32)
                }) {
                    Ok(()) => {
                        caller.data_mut().trace_import(import, GuestStatus::Ok.code());
                        GuestStatus::Ok.code()
                    }
                    Err(status) => {
                        caller.data_mut().trace_import(import, status);
                        status
                    }
                }
            },
        )
        .map(|_| ())
        .map_err(|e| HostError::Instantiation(e.to_string()))
}
