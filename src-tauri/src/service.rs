//! Process management for the dsh web service.
//!
//! dsh and npm are installed as npm globals, i.e. dsh.cmd / npm.cmd wrappers.
//! std::process::Command uses CreateProcess directly and does not resolve
//! PATHEXT, so all invocations go through cmd /C. The spawned cmd.exe is the
//! root of the process tree; killing it with taskkill /T /F also kills the
//! node/dsh grandchildren (plain child.kill() would orphan them and leave
//! port 3080 held).

use std::io::Read;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub const HOST: &str = "127.0.0.1";
pub const PORT: u16 = 3080;
pub const READY_TIMEOUT_SECS: u64 = 30;
pub const POLL_INTERVAL_MS: u64 = 300;

/// CREATE_NO_WINDOW: keep spawned processes from flashing console windows.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// True if something is already listening on the given port.
pub fn port_busy_on(port: u16) -> bool {
    TcpStream::connect((HOST, port)).is_ok()
}

/// True if something is already listening on the service port.
pub fn port_busy() -> bool {
    port_busy_on(PORT)
}

/// Run a command, wait up to timeout_secs, return trimmed stdout on success.
pub fn run_out(cmd: &str, args: &[&str], timeout_secs: u64) -> Option<String> {
    let mut child = Command::new("cmd")
        .args(["/C", cmd])
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                break;
            }
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return None,
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    Some(out.trim().to_string())
}

/// Run a command discarding output; true if it exited successfully in time.
pub fn run_wait(cmd: &str, args: &[&str], timeout_secs: u64) -> bool {
    let mut child = match Command::new("cmd")
        .args(["/C", cmd])
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(_) => return false,
        }
    }
}

/// Installed dsh version, read from dsh --version (None if dsh is missing).
pub fn dsh_version() -> Option<String> {
    run_out("dsh", &["--version"], 15)
}

/// Spawn dsh web hidden on the given port; returns the cmd.exe wrapper
/// child that roots the whole process tree.
pub fn spawn_dsh_on(port: u16) -> std::io::Result<Child> {
    Command::new("cmd")
        .args(["/C", "dsh", "web", "--host", HOST, "--port"])
        .arg(port.to_string())
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
}

/// Kill a process tree by its root PID (taskkill /T /F), then reap the handle.
pub fn kill_tree_by_pid(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

/// Spawn dsh web on the default service port.
pub fn spawn_dsh() -> std::io::Result<Child> {
    spawn_dsh_on(PORT)
}

/// Kill via the Child handle (used when we still own it).
pub fn kill_child_tree(child: &mut Child) {
    kill_tree_by_pid(child.id());
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Port precheck: a listening socket makes port_busy_on true, and
    /// releasing it makes it false again.
    #[test]
    fn port_precheck_detects_listener() {
        let port: u16 = 3099;
        assert!(!port_busy_on(port), "port should be free before the test");
        let listener = std::net::TcpListener::bind((HOST, port)).expect("bind test port");
        assert!(port_busy_on(port), "port_busy_on must see the listener");
        drop(listener);
        assert!(
            !port_busy_on(port),
            "port should be free after dropping the listener"
        );
    }

    /// Tree kill: cmd /C ping -t builds a cmd.exe -> ping.exe tree that never
    /// exits on its own; kill_tree_by_pid must take down the whole tree.
    #[test]
    fn tree_kill_takes_down_grandchildren() {
        let mut root = Command::new("cmd")
            .args(["/C", "ping", "-t", "127.0.0.1"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("spawn ping tree");
        let pid = root.id();
        std::thread::sleep(Duration::from_millis(500));

        kill_tree_by_pid(pid);

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(Some(_)) = root.try_wait() {
                break;
            }
            assert!(Instant::now() < deadline, "root process was not killed");
            std::thread::sleep(Duration::from_millis(100));
        }
        // No orphaned ping.exe may remain anywhere in the tree we killed.
        let out = Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq PING.EXE", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .expect("run tasklist");
        let listing = String::from_utf8_lossy(&out.stdout);
        assert!(
            !listing.to_ascii_lowercase().contains("ping.exe"),
            "orphaned ping.exe survived the tree kill: {listing}"
        );
    }
}
