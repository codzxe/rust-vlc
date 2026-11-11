//! The Linux dialog chain (card 7.A2): xdg-desktop-portal probe +
//! zenity/kdialog command fallback.
//!
//! # What is honest here
//!
//! - **Session-bus probe** (real, std-only): resolve
//!   `$DBUS_SESSION_BUS_ADDRESS` and try connecting the socket. This
//!   answers "could a portal client talk to the bus at all".
//! - **The portal `FileChooser` client itself is not implemented in
//!   v0** — it needs a raw D-Bus wire client (SASL EXTERNAL auth +
//!   message marshalling + signal matching) that this sandbox cannot
//!   exercise (no portal daemon), and shipping unexercised protocol
//!   code would violate the no-fabrication rule. [`PortalStatus`]
//!   says exactly this; `[PROPOSAL]` follow-up: full D-Bus client.
//! - **zenity / kdialog** (real, testable): the classic XDG dialog
//!   commands, spawned with explicit argv (no shell). The seam is
//!   injectable (`with_config`) so tests drive fake binaries.
//!
//! Cancel is a first-class outcome ([`FilePick::Cancelled`]), not an
//! error.

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::dialog::{FileDialogRequest, FileDialogs, FilePick};
use crate::error::PlatformError;

/// The resolved portal situation (honest, queryable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortalStatus {
    /// The session bus socket is reachable — a portal client could
    /// talk to it (the FileChooser client itself is the pending
    /// follow-up, see [`portal_filechooser_implemented`]).
    BusReachable {
        /// The socket path that connected.
        socket: PathBuf,
    },
    /// No session bus (unset variable, absent socket, …).
    BusUnreachable {
        /// Why.
        detail: String,
    },
}

/// Whether the portal `FileChooser` D-Bus client is implemented.
/// v0: **not** — a raw D-Bus wire client (SASL EXTERNAL auth +
/// message marshalling + signal matching) is the recorded
/// `[PROPOSAL]` follow-up; dialogs use the command fallback chain
/// instead (this function is the single truth the capability report
/// and `no_backend_error` quote).
pub const fn portal_filechooser_implemented() -> bool {
    false
}

/// The dialog chain instance.
#[derive(Debug, Clone)]
pub struct XdgDialogs {
    search_dirs: Vec<PathBuf>,
    bus_address: Option<String>,
    zenity: Option<PathBuf>,
    kdialog: Option<PathBuf>,
}

impl XdgDialogs {
    /// Discover from the environment: `$PATH` for the command
    /// fallbacks, `$DBUS_SESSION_BUS_ADDRESS` for the portal probe.
    #[must_use = "constructs the dialog chain"]
    pub fn discover() -> XdgDialogs {
        let path_var = std::env::var_os("PATH").map(|v| {
            std::env::split_paths(&v)
                .filter(|p| !p.as_os_str().is_empty())
                .collect::<Vec<_>>()
        });
        let search_dirs = path_var.unwrap_or_default();
        let bus_address = std::env::var("DBUS_SESSION_BUS_ADDRESS").ok();
        let mut dialogs = XdgDialogs {
            search_dirs,
            bus_address,
            zenity: None,
            kdialog: None,
        };
        dialogs.resolve_commands();
        dialogs
    }

    /// Fully-injected construction (the test seam: fake binaries in
    /// a temp `search_dirs`, a fixture bus address).
    #[must_use = "constructs the injectable dialog chain"]
    pub fn with_config(search_dirs: Vec<PathBuf>, bus_address: Option<String>) -> XdgDialogs {
        let mut dialogs = XdgDialogs {
            search_dirs,
            bus_address,
            zenity: None,
            kdialog: None,
        };
        dialogs.resolve_commands();
        dialogs
    }

    /// Resolve zenity/kdialog once (idempotent).
    fn resolve_commands(&mut self) {
        self.zenity = find_executable(&self.search_dirs, "zenity");
        self.kdialog = find_executable(&self.search_dirs, "kdialog");
    }

    /// The zenity binary, when found.
    #[must_use = "pure accessor"]
    pub fn zenity(&self) -> Option<&Path> {
        self.zenity.as_deref()
    }

    /// The kdialog binary, when found.
    #[must_use = "pure accessor"]
    pub fn kdialog(&self) -> Option<&Path> {
        self.kdialog.as_deref()
    }

    /// Probe the session bus for the portal follow-up (real connect
    /// attempt; returns the honest status either way).
    #[must_use = "live probe, no side effects beyond a connect"]
    pub fn portal_status(&self) -> PortalStatus {
        match self.bus_socket() {
            Ok(socket) => PortalStatus::BusReachable { socket },
            Err(detail) => PortalStatus::BusUnreachable { detail },
        }
    }

    /// Resolve + connect the session-bus socket from the configured
    /// address. Abstract sockets are beyond the std client and say so.
    fn bus_socket(&self) -> Result<PathBuf, String> {
        let Some(address) = self.bus_address.as_ref() else {
            return Err("DBUS_SESSION_BUS_ADDRESS is not set".to_string());
        };
        let Some(body) = address.strip_prefix("unix:") else {
            return Err(format!("unsupported bus address {address:?} (want unix:…)"));
        };
        for field in body.split(',') {
            if let Some(path) = field.strip_prefix("path=") {
                let socket = PathBuf::from(path);
                return match std::os::unix::net::UnixStream::connect(&socket) {
                    Ok(_) => Ok(socket),
                    Err(err) => Err(format!("connect to {path} failed: {err}")),
                };
            }
            if field.starts_with("abstract=") {
                return Err(
                    "abstract-socket bus addresses are not supported by the std client".to_string(),
                );
            }
        }
        Err(format!("no path= field in bus address {address:?}"))
    }

    /// The error returned when no command fallback exists — names
    /// everything that was tried (the honest refusal).
    fn no_backend_error(&self) -> PlatformError {
        let bus_detail = match self.bus_socket() {
            Ok(path) => format!("session bus reachable at {}", path.display()),
            Err(detail) => format!("session bus unreachable ({detail})"),
        };
        PlatformError::ServiceUnavailable {
            service: "file dialogs",
            detail: format!(
                "no dialog backend: portal FileChooser client {}\
                 ({bus_detail}); zenity {} kdialog {}",
                if portal_filechooser_implemented() {
                    "available"
                } else {
                    "not implemented ([PROPOSAL] D-Bus follow-up)"
                },
                if self.zenity.is_some() {
                    "found"
                } else {
                    "not found"
                },
                if self.kdialog.is_some() {
                    "found"
                } else {
                    "not found"
                },
            ),
        }
    }

    /// Run one dialog round through the best available command.
    fn run_command(
        &self,
        save: bool,
        request: &FileDialogRequest,
    ) -> Result<FilePick, PlatformError> {
        if let Some(zenity) = self.zenity.as_ref() {
            return run_zenity(zenity, save, request);
        }
        if let Some(kdialog) = self.kdialog.as_ref() {
            return run_kdialog(kdialog, save, request);
        }
        Err(self.no_backend_error())
    }
}

impl FileDialogs for XdgDialogs {
    fn open_file(&mut self, request: &FileDialogRequest) -> Result<FilePick, PlatformError> {
        self.run_command(false, request)
    }

    fn save_file(&mut self, request: &FileDialogRequest) -> Result<FilePick, PlatformError> {
        self.run_command(true, request)
    }
}

/// Find `name` in `dirs` (executable regular file).
fn find_executable(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    for dir in dirs {
        let candidate = dir.join(name);
        let Ok(meta) = std::fs::metadata(&candidate) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        // The x-bits are group bits OR'd; 0o111 covers u/g/o execute.
        if meta.permissions().mode() & 0o111 != 0 {
            return Some(candidate);
        }
    }
    None
}

/// Shared exit-code interpretation for both commands.
fn interpret(
    program: &'static str,
    output: &std::process::Output,
) -> Result<FilePick, PlatformError> {
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let paths = stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        if paths.is_empty() {
            // Success with no output: treat as cancel (both tools use
            // exit 1 for cancel, but a GUI kill can land here too).
            return Ok(FilePick::Cancelled);
        }
        return Ok(FilePick::Paths(paths));
    }
    match output.status.code() {
        Some(1) if output.stdout.is_empty() => Ok(FilePick::Cancelled),
        code => {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            Err(PlatformError::CallFailed {
                api: program,
                code: code.map(i64::from),
                detail: if stderr.is_empty() {
                    "the dialog tool exited with a failure status".to_string()
                } else {
                    stderr.trim().to_string()
                },
            })
        }
    }
}

/// Build + run the zenity invocation.
fn run_zenity(
    program: &Path,
    save: bool,
    request: &FileDialogRequest,
) -> Result<FilePick, PlatformError> {
    let mut command = Command::new(program);
    command.arg("--file-selection");
    if save {
        command.arg("--save");
    }
    if let Some(title) = request.title.as_ref() {
        command.arg(format!("--title={title}"));
    }
    if request.multiple && !save {
        command.arg("--multiple").arg("--separator=\n");
    }
    if let Some(dir) = request.default_dir.as_ref() {
        // A trailing separator makes zenity treat it as a directory.
        let mut base = dir.display().to_string();
        if !base.ends_with('/') {
            base.push('/');
        }
        command.arg(format!("--filename={base}"));
    }
    for filter in &request.filters {
        command.arg(format!(
            "--file-filter={}|{}",
            filter.label,
            filter.patterns.join(" ")
        ));
    }
    let output = command.output().map_err(|err| PlatformError::SpawnFailed {
        program: program.display().to_string(),
        detail: err.to_string(),
    })?;
    interpret("zenity", &output)
}

/// Build + run the kdialog invocation.
fn run_kdialog(
    program: &Path,
    save: bool,
    request: &FileDialogRequest,
) -> Result<FilePick, PlatformError> {
    let mut command = Command::new(program);
    if save {
        command.arg("--getsavefilename");
    } else {
        command.arg("--getopenfilename");
    }
    let start = request
        .default_dir
        .as_ref()
        .map(|d| d.display().to_string())
        .unwrap_or_default();
    command.arg(if start.is_empty() {
        ".".to_string()
    } else {
        start
    });
    let filters = if request.filters.is_empty() {
        String::from("*")
    } else {
        request
            .filters
            .iter()
            .map(|f| f.patterns.join(" "))
            .collect::<Vec<_>>()
            .join("\n")
    };
    command.arg(filters);
    if let Some(title) = request.title.as_ref() {
        command.arg("--title").arg(title);
    }
    if request.multiple && !save {
        command.arg("--multiple").arg("--separate-output");
    }
    let output = command.output().map_err(|err| PlatformError::SpawnFailed {
        program: program.display().to_string(),
        detail: err.to_string(),
    })?;
    interpret("kdialog", &output)
}
