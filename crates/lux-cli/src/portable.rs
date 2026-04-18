//! Portable-mode auto-spawn of a sibling `llama-server` binary.
//!
//! When `lux` is extracted from the portable tarball and a sibling
//! `llama-server` executable and a `.gguf` model file exist next to it,
//! start llama-server on an ephemeral localhost port so the agent works
//! with no system-wide install. The child is linked to our process via
//! `PR_SET_PDEATHSIG`, so it dies with us even on SIGKILL or panic.
//!
//! The model file is searched in `bin_dir/models/*.gguf` first, then
//! `bin_dir/*.gguf`. The `--jinja` flag enables the model's chat template
//! and structured tool-call output on `/v1/chat/completions`.

use anyhow::{Context, Result, anyhow};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct PortableServer {
    child: Child,
    pub url: String,
}

impl Drop for PortableServer {
    fn drop(&mut self) {
        unsafe {
            libc::kill(self.child.id() as libc::pid_t, libc::SIGTERM);
        }
        let _ = self.child.wait();
    }
}

/// Spawn the sibling llama-server if portable mode applies; otherwise return
/// None. `user_specified_url` short-circuits portable mode when the caller
/// passed `--ollama-url` explicitly.
pub fn maybe_spawn(user_specified_url: bool) -> Result<Option<PortableServer>> {
    if user_specified_url {
        return Ok(None);
    }
    if std::env::var_os("LUX_NO_PORTABLE").is_some() {
        return Ok(None);
    }
    let Some(bin_dir) = sibling_dir() else {
        return Ok(None);
    };
    let Some(gguf) = detect(&bin_dir) else {
        return Ok(None);
    };
    spawn_at(&bin_dir, &gguf).map(Some)
}

/// Spawn llama-server from `bin_dir` on an ephemeral port and wait for
/// `/health` to go green. Caller must hold the returned guard for as long
/// as the server should stay alive — dropping it sends SIGTERM.
fn spawn_at(bin_dir: &Path, gguf: &Path) -> Result<PortableServer> {
    let llama_server = bin_dir.join("llama-server");

    let port = pick_port()?;
    let host = format!("127.0.0.1:{port}");
    tracing::info!(
        "portable mode: spawning llama-server at {host} with model {}",
        gguf.display()
    );

    let mut cmd = Command::new(&llama_server);
    cmd.arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--model")
        .arg(gguf)
        .arg("--jinja")
        .arg("--ctx-size")
        .arg("4096")
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
    let child = cmd.spawn().context("spawn sibling llama-server")?;
    let guard = PortableServer {
        child,
        url: format!("http://{host}"),
    };
    wait_ready(&host).context("sibling llama-server failed to start")?;
    Ok(guard)
}

/// Return the .gguf path if `bin_dir` contains a llama-server binary and
/// at least one .gguf file (in `models/` or the dir itself).
pub fn detect(bin_dir: &Path) -> Option<PathBuf> {
    if !bin_dir.join("llama-server").is_file() {
        return None;
    }
    find_gguf(&bin_dir.join("models")).or_else(|| find_gguf(bin_dir))
}

fn find_gguf(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "gguf") && path.is_file() {
            return Some(path);
        }
    }
    None
}

fn sibling_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(PathBuf::from)
}

fn pick_port() -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

/// Poll `/health` until it returns HTTP 200 (model loaded and ready) or we
/// time out. llama-server binds the port before loading the model, so a bare
/// TCP connect is insufficient — the first /v1 call would race the load.
fn wait_ready(host: &str) -> Result<()> {
    let addr: SocketAddr = host.parse()?;
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        if http_health_ok(&addr) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(anyhow!("timed out waiting for llama-server on {host}"))
}

fn http_health_ok(addr: &SocketAddr) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(addr, Duration::from_millis(500)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let req = format!(
        "GET /health HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n",
        addr = addr
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 64];
    let Ok(n) = stream.read(&mut buf) else {
        return false;
    };
    // Status line starts "HTTP/1.1 200". Any other status (503 while loading,
    // 404 on an older server build) means keep waiting.
    buf[..n].starts_with(b"HTTP/1.1 200") || buf[..n].starts_with(b"HTTP/1.0 200")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn detect_needs_llama_server_and_gguf() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        assert!(detect(base).is_none());

        fs::write(base.join("llama-server"), b"#!/bin/sh\n").unwrap();
        assert!(detect(base).is_none(), "llama-server alone is not enough");

        // gguf in a sibling models/ dir should be picked up.
        fs::create_dir(base.join("models")).unwrap();
        let gguf = base.join("models").join("lux.gguf");
        fs::write(&gguf, b"fake weights").unwrap();
        assert_eq!(detect(base), Some(gguf.clone()));

        // gguf directly beside the binary also works when no models/ hit.
        fs::remove_file(&gguf).unwrap();
        let gguf2 = base.join("lux.gguf");
        fs::write(&gguf2, b"fake weights").unwrap();
        assert_eq!(detect(base), Some(gguf2));

        fs::remove_file(base.join("llama-server")).unwrap();
        assert!(detect(base).is_none(), "gguf alone is not enough");
    }

    /// Fake llama-server: reads `--port` from argv, serves `GET /health` with
    /// HTTP 200. Enough to satisfy `wait_ready`.
    const FAKE_LLAMA_SERVER: &str = r#"#!/bin/sh
port=""
while [ $# -gt 0 ]; do
    case "$1" in
        --port) port="$2"; shift 2 ;;
        *) shift ;;
    esac
done
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

        let llama = base.join("llama-server");
        fs::write(&llama, FAKE_LLAMA_SERVER).unwrap();
        fs::set_permissions(&llama, fs::Permissions::from_mode(0o755)).unwrap();
        let gguf = base.join("lux.gguf");
        fs::write(&gguf, b"fake weights").unwrap();

        let guard = spawn_at(base, &gguf).expect("spawn_at should succeed");
        let pid = guard.child.id() as libc::pid_t;
        assert!(guard.url.starts_with("http://127.0.0.1:"));

        drop(guard);

        // Drop sends SIGTERM and waits; kill(pid, 0) should now return ESRCH.
        std::thread::sleep(Duration::from_millis(100));
        let alive = unsafe { libc::kill(pid, 0) == 0 };
        assert!(!alive, "child {pid} should be reaped after Drop");
    }
}
