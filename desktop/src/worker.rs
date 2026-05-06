/// Manages the lifecycle of the Python FastAPI worker subprocess.
///
/// On startup we open a pipe, pass the write-end fd to the child process via the
/// env var `MONOLITH_READY_FD`, and read `port=<N>\n` from the read end once the
/// worker has bound its socket.
use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub enum WorkerState {
    Starting,
    Ready { port: u16 },
    Failed(String),
    Stopped,
}

pub struct WorkerHandle {
    pub state: Arc<Mutex<WorkerState>>,
    child: Arc<Mutex<Option<Child>>>,
}

impl WorkerHandle {
    pub fn port(&self) -> Option<u16> {
        match &*self.state.lock().unwrap() {
            WorkerState::Ready { port } => Some(*port),
            _ => None,
        }
    }

    pub fn base_url(&self) -> Option<String> {
        self.port().map(|p| format!("http://127.0.0.1:{}", p))
    }

    pub fn stop(&self) {
        let mut guard = self.child.lock().unwrap();
        if let Some(ref mut child) = *guard {
            info!("Sending SIGTERM to Python worker (pid {})", child.id());
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                let _ = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) };
                // Give it up to 3 seconds to flush
                for _ in 0..30 {
                    match child.try_wait() {
                        Ok(Some(_)) => break,
                        _ => std::thread::sleep(Duration::from_millis(100)),
                    }
                }
            }
            #[cfg(not(unix))]
            {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        *guard = None;
        *self.state.lock().unwrap() = WorkerState::Stopped;
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Spawn the Python worker in a background thread.
/// Returns a `WorkerHandle` immediately; the state transitions to `Ready` (or
/// `Failed`) once the worker reports its port.
pub fn spawn_worker() -> Arc<WorkerHandle> {
    let state = Arc::new(Mutex::new(WorkerState::Starting));
    let child_arc: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));

    let handle = Arc::new(WorkerHandle {
        state: state.clone(),
        child: child_arc.clone(),
    });

    let state_clone = state.clone();

    std::thread::spawn(move || {
        // Create a pipe: (read_fd, write_fd)
        #[cfg(unix)]
        let (read_fd, write_fd) = {
            let mut fds = [0i32; 2];
            unsafe {
                if libc::pipe(fds.as_mut_ptr()) != 0 {
                    *state_clone.lock().unwrap() =
                        WorkerState::Failed("Failed to create pipe".into());
                    return;
                }
            }
            (fds[0], fds[1])
        };

        // On non-unix fall back to a TCP loopback approach (Windows):
        // For now just attempt to find a free port and use a temp file signal.
        #[cfg(not(unix))]
        let (read_fd, write_fd): (i32, i32) = (-1, -1);

        let python = find_python();

        // Prefer the `monolith-server` script if it's on PATH; fall back to
        // running the module directly via the same Python interpreter.
        let server_cmd = which_monolith_server(&python);
        let mut cmd = if server_cmd.ends_with("monolith-server") || server_cmd.ends_with("monolith-server.exe") {
            let mut c = Command::new(&server_cmd);
            c.args(["--port", "0", "--host", "127.0.0.1"]);
            c
        } else {
            let mut c = Command::new(&python);
            c.args(["-m", "monolith.server.run", "--port", "0", "--host", "127.0.0.1"]);
            c
        };
        cmd.env("MONOLITH_READY_FD", write_fd.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        info!("Spawning Python worker with cmd: {server_cmd:?}");

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                *state_clone.lock().unwrap() =
                    WorkerState::Failed(format!("Failed to spawn Python: {e}"));
                #[cfg(unix)]
                unsafe {
                    libc::close(read_fd);
                    libc::close(write_fd);
                }
                return;
            }
        };

        info!("Python worker spawned (pid {})", child.id());
        *child_arc.lock().unwrap() = Some(child);

        // Close write end in this process so our read will see EOF if the child exits
        #[cfg(unix)]
        unsafe {
            libc::close(write_fd);
        }

        // Read port from pipe with 30-second timeout
        let port = read_port_from_fd(read_fd);

        #[cfg(unix)]
        unsafe {
            libc::close(read_fd);
        }

        match port {
            Ok(p) => {
                info!("Python worker ready on port {p}");
                *state_clone.lock().unwrap() = WorkerState::Ready { port: p };
            }
            Err(e) => {
                error!("Failed to read port from Python worker: {e}");
                *state_clone.lock().unwrap() = WorkerState::Failed(e);
            }
        }
    });

    handle
}

fn find_python() -> String {
    // Prefer a venv next to the binary, then fall back to system python3/python
    let exe = std::env::current_exe().ok();
    if let Some(ref bin) = exe {
        let venv = bin.parent().unwrap_or(bin).join("venv").join("bin").join("python");
        if venv.exists() {
            return venv.to_string_lossy().into_owned();
        }
    }
    "python3".to_string()
}

fn which_monolith_server(python: &str) -> String {
    // Look for `monolith-server` script next to the Python interpreter first.
    let path = std::path::Path::new(python);
    if let Some(dir) = path.parent() {
        let candidate = dir.join("monolith-server");
        if candidate.exists() {
            return candidate.to_string_lossy().into_owned();
        }
        #[cfg(windows)]
        {
            let candidate = dir.join("monolith-server.exe");
            if candidate.exists() {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }
    // Fall back — caller will use `-m monolith.server.run` instead.
    "monolith-server".to_string()
}

#[cfg(unix)]
fn read_port_from_fd(fd: i32) -> Result<u16, String> {
    use std::os::unix::io::FromRawFd;
    let mut f = unsafe { std::fs::File::from_raw_fd(fd) };
    let mut buf = String::new();
    // Poll with timeout
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        match f.read_to_string(&mut buf) {
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() > deadline {
                    return Err("Timed out waiting for worker to report port".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Pipe read error: {e}")),
        }
    }
    // Prevent the fd from being double-closed (File::drop would close it, but
    // we also call close() in the spawner).  Leak it here.
    std::mem::forget(f);

    parse_port(&buf)
}

#[cfg(not(unix))]
fn read_port_from_fd(_fd: i32) -> Result<u16, String> {
    Err("Worker port negotiation not implemented on this platform".into())
}

fn parse_port(s: &str) -> Result<u16, String> {
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("port=") {
            return rest
                .trim()
                .parse::<u16>()
                .map_err(|e| format!("Invalid port '{rest}': {e}"));
        }
    }
    Err(format!("No 'port=...' line in worker output: {s:?}"))
}

// Pull in libc for Unix signal/fd operations
#[cfg(unix)]
use libc;
