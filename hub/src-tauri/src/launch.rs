//! Starting tools, noticing that they started, and stopping them.
//!
//! The hub is a process manager, not a browser (D2). Five of the ten tools are
//! hardened loopback web apps that set `frame-ancestors 'none'` and two more are
//! Tkinter; there is no version of this that embeds them in a tab without
//! dismantling anti-DNS-rebinding defences to get a cosmetic one. So each tool
//! runs as its own process in its own window, and what the hub tracks is a PID.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::catalog::{Health, Tool};

#[derive(Debug)]
pub enum LaunchError {
    Io(String),
    /// The user declined the UAC prompt, or elevation is unavailable.
    Elevation(String),
    Missing(String),
    Blocked(String),
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchError::Io(e) | LaunchError::Missing(e) | LaunchError::Blocked(e) => {
                write!(f, "{e}")
            }
            LaunchError::Elevation(e) => write!(f, "{e}"),
        }
    }
}

impl From<std::io::Error> for LaunchError {
    fn from(value: std::io::Error) -> Self {
        LaunchError::Io(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, LaunchError>;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Started {
    /// A process the hub can show as Running and can Stop.
    Process { pid: u32 },
    /// A document handed to the default browser. There is no process of ours.
    Document { path: String },
}

/// Spawn a tool from an installed directory.
pub fn launch(tool: &Tool, install_dir: &Path) -> Result<Started> {
    let entry = install_dir.join(tool.launch.exe.replace('/', std::path::MAIN_SEPARATOR_STR));
    if !entry.exists() {
        return Err(LaunchError::Missing(format!(
            "{} is not where the catalog says it is ({}). Try Verify, then reinstall.",
            tool.launch.exe,
            entry.display()
        )));
    }

    if tool.is_document() {
        open_document(&entry)?;
        return Ok(Started::Document {
            path: entry.to_string_lossy().to_string(),
        });
    }

    let pid = if tool.launch.elevate {
        spawn_elevated(&entry, &tool.launch.args, install_dir)?
    } else {
        spawn_plain(&entry, &tool.launch.args, install_dir)?
    };
    Ok(Started::Process { pid })
}

/// Run a tool out of its checked-out submodule instead of an installed copy.
///
/// This is what makes the hub exercisable before any release exists, and it is
/// why `source_launch` is in the catalog at all: with the submodules checked
/// out, the whole library works against source on a machine that has never
/// downloaded an artifact.
pub fn launch_from_source(tool: &Tool, submodule_dir: &Path) -> Result<Started> {
    let Some(source) = tool.source_launch.as_ref() else {
        return Err(LaunchError::Missing(format!(
            "{} has no source entry point -- its repository ships a built \
             executable and nothing else.",
            tool.name
        )));
    };
    if !submodule_dir.is_dir() {
        return Err(LaunchError::Missing(format!(
            "{} is not checked out at {}. Run `git submodule update --init`.",
            tool.submodule,
            submodule_dir.display()
        )));
    }
    let pid = spawn_command(&source.cmd, &source.args, submodule_dir)?;
    Ok(Started::Process { pid })
}

fn spawn_plain(entry: &Path, args: &[String], cwd: &Path) -> Result<u32> {
    let child = std::process::Command::new(entry)
        .args(args)
        // Several tools resolve data files relative to the working directory --
        // ForgePact looks for `modfiles/` next to the executable -- so this is
        // load-bearing rather than tidiness.
        .current_dir(cwd)
        .spawn()
        .map_err(|e| LaunchError::Io(format!("could not start {}: {e}", entry.display())))?;
    Ok(child.id())
}

fn spawn_command(program: &str, args: &[String], cwd: &Path) -> Result<u32> {
    let child = std::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .spawn()
        .map_err(|e| LaunchError::Io(format!("could not run `{program}`: {e}")))?;
    Ok(child.id())
}

// ---------------------------------------------------------------------------
// Elevation
// ---------------------------------------------------------------------------

/// Start a process elevated, returning its PID.
///
/// `CreateProcess` -- which is what `Command::spawn` is -- cannot elevate; a
/// non-elevated parent asking for an elevated child gets
/// ERROR_ELEVATION_REQUIRED. `ShellExecuteExW` with the `runas` verb is the way
/// through, and `SEE_MASK_NOCLOSEPROCESS` is what makes it hand back a handle,
/// so the hub can still show the tool as Running and stop it.
///
/// The hub itself stays non-elevated. Only the three memory tools that need
/// `SeDebugPrivilege` get the prompt, one prompt per launch.
#[cfg(windows)]
fn spawn_elevated(entry: &Path, args: &[String], cwd: &Path) -> Result<u32> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::GetProcessId;
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    let verb = wide(std::ffi::OsStr::new("runas"));
    let file = wide(entry.as_os_str());
    let directory = wide(cwd.as_os_str());
    let parameters = if args.is_empty() {
        None
    } else {
        Some(wide(std::ffi::OsStr::new(&quote_args(args))))
    };

    // SAFETY: every pointer below points at a `Vec<u16>` that outlives the call,
    // and the struct is zeroed before its fields are set so no field is read
    // uninitialised.
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpDirectory = directory.as_ptr();
    info.nShow = SW_SHOWNORMAL;
    if let Some(parameters) = parameters.as_ref() {
        info.lpParameters = parameters.as_ptr();
    }

    let ok = unsafe { ShellExecuteExW(&mut info) };
    if ok == 0 || info.hProcess.is_null() {
        let error = std::io::Error::last_os_error();
        // ERROR_CANCELLED is the user saying no at the UAC prompt. That is a
        // decision, not a fault, and it should not read like a crash.
        if error.raw_os_error() == Some(1223) {
            return Err(LaunchError::Elevation(format!(
                "{} needs Administrator and the prompt was declined.",
                entry.file_name().unwrap_or_default().to_string_lossy()
            )));
        }
        return Err(LaunchError::Elevation(format!(
            "could not start {} elevated: {error}",
            entry.display()
        )));
    }

    let pid = unsafe { GetProcessId(info.hProcess) };
    unsafe { CloseHandle(info.hProcess) };
    if pid == 0 {
        return Err(LaunchError::Elevation(
            "the elevated process started but Windows did not report its id".into(),
        ));
    }
    Ok(pid)
}

#[cfg(not(windows))]
fn spawn_elevated(entry: &Path, args: &[String], cwd: &Path) -> Result<u32> {
    // Nothing in the catalog that elevates is anything but Windows-only, so this
    // exists so the crate builds elsewhere rather than to be used.
    spawn_plain(entry, args, cwd)
}

/// Join arguments the way `CommandLineToArgvW` will split them again.
fn quote_args(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg.is_empty() || arg.contains([' ', '\t', '"']) {
                format!("\"{}\"", arg.replace('"', "\\\""))
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(windows)]
fn open_document(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let verb: Vec<u16> = std::ffi::OsStr::new("open")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let file: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: both buffers are NUL-terminated and outlive the call.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW returns a value greater than 32 on success. The API is old
    // enough that this is genuinely how it reports one.
    if (result as isize) <= 32 {
        return Err(LaunchError::Io(format!(
            "Windows would not open {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(not(windows))]
fn open_document(path: &Path) -> Result<()> {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    std::process::Command::new(opener)
        .arg(path)
        .spawn()
        .map_err(|e| LaunchError::Io(format!("could not open {}: {e}", path.display())))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Liveness
// ---------------------------------------------------------------------------

pub fn is_alive(pid: u32) -> bool {
    let mut system = sysinfo::System::new();
    let pid = sysinfo::Pid::from_u32(pid);
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    system.process(pid).is_some()
}

pub fn stop(pid: u32) -> Result<()> {
    let mut system = sysinfo::System::new();
    let pid = sysinfo::Pid::from_u32(pid);
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    let Some(process) = system.process(pid) else {
        // Already gone. The caller wanted it stopped; it is stopped.
        return Ok(());
    };
    if process.kill() {
        Ok(())
    } else {
        Err(LaunchError::Io(format!(
            "process {pid} would not stop. If it is running elevated, close it \
             from its own window."
        )))
    }
}

/// Ask a tool's own health endpoint whether it is up.
///
/// These endpoints already exist -- the item editor's `/api/instance` returns
/// `{version, pid, port}` and HSCraftSim's `/_health` returns
/// `{application, version}` -- so the hub reuses them instead of inventing a
/// protocol that would need a commit to all ten repositories.
pub fn probe(health: &Health) -> bool {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(700))
        .timeout(Duration::from_secs(3))
        .build();
    match agent.get(&health.url).call() {
        Ok(_) => true,
        // A 401/403 from HS Offline Launcher's HMAC-guarded server still proves
        // something is listening, which is the only thing being asked.
        Err(ureq::Error::Status(_, _)) => health.any_status,
        Err(_) => false,
    }
}

/// Poll a health endpoint until it answers or the tool's declared timeout runs
/// out. Returns false for a tool that has no endpoint -- the caller falls back
/// to "the process is alive", which is all a Tkinter app can offer.
pub fn wait_until_healthy(tool: &Tool) -> bool {
    let Some(health) = tool.launch.health.as_ref() else {
        return false;
    };
    let deadline = Instant::now() + Duration::from_secs(health.timeout_s.clamp(1, 120));
    while Instant::now() < deadline {
        if probe(health) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    false
}

/// Is one of a tool's declared ports already taken?
///
/// Used to notice a copy started outside the hub, so the card does not offer
/// Launch for something that is already up and then fail on the tool's own
/// single-instance lock.
pub fn any_port_in_use(ports: &[u16]) -> bool {
    ports.iter().any(|port| {
        std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], *port)),
            Duration::from_millis(120),
        )
        .is_ok()
    })
}

pub fn submodule_path(repo_root: &Path, submodule: &str) -> PathBuf {
    repo_root.join(crate::paths::safe_component(submodule))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_quoted_the_way_windows_will_split_them() {
        assert_eq!(quote_args(&[]), "");
        assert_eq!(quote_args(&["-3".into()]), "-3");
        assert_eq!(
            quote_args(&["src/forgepact.py".into()]),
            "src/forgepact.py"
        );
        assert_eq!(
            quote_args(&["C:/Program Files/x.py".into()]),
            "\"C:/Program Files/x.py\""
        );
        assert_eq!(quote_args(&["".into()]), "\"\"");
        assert_eq!(quote_args(&["say \"hi\"".into()]), "\"say \\\"hi\\\"\"");
    }

    #[test]
    fn this_process_is_alive_and_a_silly_pid_is_not() {
        assert!(is_alive(std::process::id()));
        assert!(!is_alive(0xFFFF_FFF0));
    }

    #[test]
    fn stopping_something_that_is_already_gone_is_not_an_error() {
        assert!(stop(0xFFFF_FFF0).is_ok());
    }

    #[test]
    fn a_port_nobody_is_listening_on_reads_as_free() {
        // 47 is reserved and nothing binds it; the point is that the check
        // returns rather than hanging on a closed port.
        assert!(!any_port_in_use(&[47]));
        assert!(!any_port_in_use(&[]));
    }

    #[test]
    fn a_port_that_is_bound_reads_as_in_use() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(any_port_in_use(&[port]));
        drop(listener);
    }

    #[test]
    fn launching_a_tool_whose_entry_point_is_missing_says_so_before_spawning() {
        let catalog = crate::catalog::embedded().unwrap().catalog;
        let tool = catalog.tool("forgepact").unwrap();
        let error = launch(tool, Path::new("C:/nowhere/at/all")).unwrap_err();
        assert!(matches!(error, LaunchError::Missing(_)), "{error}");
        assert!(error.to_string().contains("ForgePact.exe"), "{error}");
    }

    #[test]
    fn a_binary_only_tool_cannot_be_run_from_source() {
        let catalog = crate::catalog::embedded().unwrap().catalog;
        // HS-ValueEditor's repository contains a built .exe and nothing else.
        let tool = catalog.tool("hs-value-editor").unwrap();
        assert!(tool.source_launch.is_none());
        let error = launch_from_source(tool, Path::new(".")).unwrap_err();
        assert!(error.to_string().contains("built executable"), "{error}");
    }

    #[test]
    fn source_launch_reports_a_submodule_that_is_not_checked_out() {
        let catalog = crate::catalog::embedded().unwrap().catalog;
        let tool = catalog.tool("forgepact").unwrap();
        let error = launch_from_source(tool, Path::new("C:/not/checked/out")).unwrap_err();
        assert!(error.to_string().contains("submodule update"), "{error}");
    }

    #[test]
    fn a_health_probe_against_nothing_fails_rather_than_hanging() {
        let health = Health {
            // Port 1 on loopback: refused immediately on every platform.
            url: "http://127.0.0.1:1/".into(),
            timeout_s: 1,
            any_status: false,
        };
        let started = Instant::now();
        assert!(!probe(&health));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_tool_without_a_health_endpoint_falls_back_rather_than_waiting() {
        let catalog = crate::catalog::embedded().unwrap().catalog;
        // Tkinter: no HTTP surface at all.
        let tool = catalog.tool("hssaveeditor").unwrap();
        assert!(tool.launch.health.is_none());
        let started = Instant::now();
        assert!(!wait_until_healthy(tool));
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
