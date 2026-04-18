//! Portable-mode auto-spawn of a sibling `ollama` binary.
//!
//! When `lux` is extracted from the portable tarball and a sibling `ollama`
//! executable and `models/` directory exist next to it, start ollama on an
//! ephemeral localhost port so the agent works with no system-wide install.
//! The child is linked to our process via `PR_SET_PDEATHSIG`, so it dies
//! with us even on SIGKILL or panic.

use anyhow::{Context, Result, anyhow};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct PortableOllama {
    child: Child,
    pub url: String,
}

impl Drop for PortableOllama {
    fn drop(&mut self) {
        unsafe {
            libc::kill(self.child.id() as libc::pid_t, libc::SIGTERM);
        }
        let _ = self.child.wait();
    }
}

/// Spawn the sibling ollama if portable mode applies; otherwise return None.
/// `user_specified_url` short-circuits portable mode when the caller passed
/// `--ollama-url` explicitly.
pub fn maybe_spawn(user_specified_url: bool) -> Result<Option<PortableOllama>> {
    if user_specified_url {
        return Ok(None);
    }
    if std::env::var_os("LUX_NO_PORTABLE").is_some() {
        return Ok(None);
    }
    let Some(bin_dir) = sibling_dir() else {
        return Ok(None);
    };
    if !detect(&bin_dir) {
        return Ok(None);
    }
    spawn_at(&bin_dir).map(Some)
}

/// Spawn ollama from `bin_dir` on an ephemeral port and wait for it to accept
/// connections. Caller must hold the returned guard for as long as the server
/// should stay alive — dropping it sends SIGTERM.
fn spawn_at(bin_dir: &Path) -> Result<PortableOllama> {
    let ollama = bin_dir.join("ollama");
    let models = bin_dir.join("models");

    let port = pick_port()?;
    let host = format!("127.0.0.1:{port}");
    tracing::info!("portable mode: spawning sibling ollama at {host}");

    let mut cmd = Command::new(&ollama);
    cmd.arg("serve")
        .env("OLLAMA_HOST", &host)
        .env("OLLAMA_MODELS", &models)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    unsafe {
        cmd.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let child = cmd.spawn().context("spawn sibling ollama")?;
    let guard = PortableOllama {
        child,
        url: format!("http://{host}"),
    };
    wait_ready(&host).context("sibling ollama failed to start")?;
    Ok(guard)
}

/// True when `bin_dir` contains a sibling ollama binary + models directory.
pub fn detect(bin_dir: &Path) -> bool {
    bin_dir.join("ollama").is_file() && bin_dir.join("models").is_dir()
}

fn sibling_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(PathBuf::from)
}

fn pick_port() -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

fn wait_ready(host: &str) -> Result<()> {
    let addr: SocketAddr = host.parse()?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!("timed out waiting for ollama on {host}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn detect_needs_both_ollama_and_models() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        assert!(!detect(base));

        fs::write(base.join("ollama"), b"#!/bin/sh\n").unwrap();
        assert!(!detect(base), "ollama alone is not enough");

        fs::create_dir(base.join("models")).unwrap();
        assert!(detect(base), "ollama + models/ triggers portable");

        fs::remove_file(base.join("ollama")).unwrap();
        assert!(!detect(base), "models/ alone is not enough");
    }

    /// Fake ollama that reads the port from $OLLAMA_HOST and answers any GET
    /// with `{}`. Enough for `wait_ready` to accept the TCP connection.
    const FAKE_OLLAMA: &str = r#"#!/bin/sh
port="${OLLAMA_HOST##*:}"
exec python3 -c "
import http.server
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200); self.end_headers(); self.wfile.write(b'{}')
    def log_message(self, *a): pass
http.server.HTTPServer(('127.0.0.1', $port), H).serve_forever()
"
"#;

    #[test]
    fn spawn_at_starts_server_and_drop_reaps_child() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        let ollama = base.join("ollama");
        fs::write(&ollama, FAKE_OLLAMA).unwrap();
        fs::set_permissions(&ollama, fs::Permissions::from_mode(0o755)).unwrap();
        fs::create_dir(base.join("models")).unwrap();

        let guard = spawn_at(base).expect("spawn_at should succeed");
        let pid = guard.child.id() as libc::pid_t;
        assert!(guard.url.starts_with("http://127.0.0.1:"));

        drop(guard);

        // Drop sends SIGTERM and waits; kill(pid, 0) should now return ESRCH.
        std::thread::sleep(Duration::from_millis(100));
        let alive = unsafe { libc::kill(pid, 0) == 0 };
        assert!(!alive, "child {pid} should be reaped after Drop");
    }
}
