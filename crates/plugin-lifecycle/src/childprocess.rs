//! Serves `ora/childprocess/*`: lets a plugin ask the host to spawn, write to, and kill a
//! subprocess, with its stdout, stderr, and exit pushed back as host-originated notifications.
//!
//! The host owns the OS process instead of the plugin's own sandboxed runtime spawning it
//! directly. It is created and torn down through `ora-process`'s tree-wide termination (a Windows
//! Job Object or a Unix process group), which is the same guarantee every other Ora-managed child
//! process already relies on. Every process tracked for one plugin generation is killed, best
//! effort, the moment that generation's [`PluginRuntime`](ora_plugin_runtime::PluginRuntime) stops
//! for any reason; see [`PluginProcessHost::kill_all`] and its wiring in
//! `runtime::DenoPluginRuntimeLauncher::launch`.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex, PoisonError};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use ora_logging::ora_warn;
use ora_plugin_runtime::{
    HostRequestError, HostRequestHandler, PluginRuntime as ProcessPluginRuntime,
};
use ora_process::{ManagedProcess, ProcessSpawner, ProcessSpec, ProcessStdio};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

/// Spawns one child process; returns `{ processId, pid }`.
pub const CHILDPROCESS_SPAWN_METHOD: &str = "ora/childprocess/spawn";
/// Writes bytes to one spawned process's stdin.
pub const CHILDPROCESS_WRITE_METHOD: &str = "ora/childprocess/write";
/// Signals EOF on one spawned process's stdin without killing it.
pub const CHILDPROCESS_CLOSE_STDIN_METHOD: &str = "ora/childprocess/closeStdin";
/// Requests best-effort tree-wide termination of one spawned process.
pub const CHILDPROCESS_KILL_METHOD: &str = "ora/childprocess/kill";

/// Host-pushed notification carrying one chunk of a spawned process's stdout.
const CHILDPROCESS_STDOUT_METHOD: &str = "ora/childprocess/stdout";
/// Host-pushed notification carrying one chunk of a spawned process's stderr.
const CHILDPROCESS_STDERR_METHOD: &str = "ora/childprocess/stderr";
/// Host-pushed notification announcing that a spawned process has exited.
const CHILDPROCESS_EXIT_METHOD: &str = "ora/childprocess/exit";

const INVALID_PARAMS_CODE: i64 = -32602;
const NOT_FOUND_CODE: i64 = -32004;
const IO_CODE: i64 = -32000;

/// Chunk size used when pumping a spawned process's stdout or stderr into notifications.
const READ_CHUNK_BYTES: usize = 32 * 1024;

/// Upper bound on one `write` request's decoded payload, mirroring
/// [`crate::storage::MAX_STORAGE_FILE_BYTES`] so a plugin cannot force unbounded host memory
/// growth by streaming an oversized chunk to a spawned process's stdin.
pub(crate) const MAX_WRITE_BYTES: usize = 8 * 1024 * 1024;

/// Longest base64 string that can decode to `MAX_WRITE_BYTES`, checked before `BASE64.decode`
/// allocates so an oversized payload is rejected without ever being decoded.
const MAX_WRITE_BASE64_LEN: usize = MAX_WRITE_BYTES.div_ceil(3) * 4;

/// Stable classification of a child-process failure, serialized as `data.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildProcessErrorKind {
    /// The request params are malformed.
    InvalidParams,
    /// `command` parsed but was empty.
    InvalidCommand,
    /// `processId` does not name a process this handler is tracking.
    NotFound,
    /// Spawn failed because the OS could not resolve the executable.
    ProgramNotFound,
    /// Any other spawn, write, or kill failure.
    Io,
}

impl ChildProcessErrorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::InvalidParams => "invalid_params",
            Self::InvalidCommand => "invalid_command",
            Self::NotFound => "not_found",
            Self::ProgramNotFound => "program_not_found",
            Self::Io => "io",
        }
    }

    fn code(self) -> i64 {
        match self {
            Self::InvalidParams | Self::InvalidCommand => INVALID_PARAMS_CODE,
            Self::NotFound => NOT_FOUND_CODE,
            Self::ProgramNotFound | Self::Io => IO_CODE,
        }
    }
}

/// One failed child-process call before it is rendered as a JSON-RPC error.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ChildProcessError {
    kind: ChildProcessErrorKind,
    message: String,
}

impl ChildProcessError {
    fn new(kind: ChildProcessErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Classifies a spawn failure, distinguishing a missing executable from any other I/O fault.
    fn from_spawn_io(error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::NotFound {
            Self::new(ChildProcessErrorKind::ProgramNotFound, error.to_string())
        } else {
            Self::new(ChildProcessErrorKind::Io, error.to_string())
        }
    }
}

impl From<ChildProcessError> for HostRequestError {
    fn from(error: ChildProcessError) -> Self {
        HostRequestError::new(error.kind.code(), error.message)
            .with_data(json!({ "kind": error.kind.as_str() }))
    }
}

/// One command the plugin asked the host to send to a spawned process's stdin.
enum StdinCommand {
    Write(Vec<u8>),
    Close,
}

/// One process this handler is tracking, keyed by its plugin-local `processId`.
struct Tracked<P> {
    process: Arc<P>,
    stdin_tx: mpsc::Sender<StdinCommand>,
    /// Joined by `watch_exit` before it pushes the exit notification: `wait()` and these pump
    /// tasks race independently against the same pipes, so without joining them first the exit
    /// notification could reach the plugin before the last stdout/stderr chunk does.
    stdout_done: JoinHandle<()>,
    stderr_done: JoinHandle<()>,
}

struct Inner<S: ProcessSpawner> {
    plugin_id: String,
    spawner: S,
    next_id: AtomicU64,
    tracked: StdMutex<HashMap<String, Tracked<S::Process>>>,
    /// Filled in once by [`PluginProcessHost::attach_runtime`] after the plugin connection this
    /// handler serves becomes ready; see the module docs for why this cannot be known upfront.
    runtime: watch::Sender<Option<ProcessPluginRuntime>>,
    /// Kept alive only so `runtime.send` above never observes zero receivers: `watch::Sender::send`
    /// fails (and drops its value) once every receiver is gone, and every other receiver used here
    /// is a short-lived `subscribe()` inside `push`. Never read directly.
    _runtime_rx: watch::Receiver<Option<ProcessPluginRuntime>>,
}

/// Serves `ora/childprocess/*` for one plugin process, spawning through `S`.
///
/// Generic over [`ProcessSpawner`] for the same reason `DenoPluginRuntimeLauncher` is: production
/// always uses [`ora_process::TokioProcessSpawner`], while tests inject a fake that never starts a
/// real OS process.
pub struct PluginProcessHost<S: ProcessSpawner>(Arc<Inner<S>>);

impl<S: ProcessSpawner> Clone for PluginProcessHost<S> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<S> PluginProcessHost<S>
where
    S: ProcessSpawner + Send + Sync + 'static,
    S::Process: Send + Sync + 'static,
{
    /// Binds the handler to the plugin it serves and the spawner it spawns through.
    pub fn new(plugin_id: impl Into<String>, spawner: S) -> Self {
        let (runtime, runtime_rx) = watch::channel(None);
        Self(Arc::new(Inner {
            plugin_id: plugin_id.into(),
            spawner,
            next_id: AtomicU64::new(1),
            tracked: StdMutex::new(HashMap::new()),
            runtime,
            _runtime_rx: runtime_rx,
        }))
    }

    /// Supplies the plugin runtime handle used to push `stdout`/`stderr`/`exit` notifications.
    ///
    /// Called once, right after the launch that used this handler as its `host_requests`
    /// returns: the handler must exist before that launch (it is passed into it), so the runtime
    /// handle it needs in order to talk back to the plugin can only arrive after the fact.
    pub fn attach_runtime(&self, runtime: ProcessPluginRuntime) {
        let _ = self.0.runtime.send(Some(runtime));
    }

    /// Kills every process this handler is still tracking, best effort.
    ///
    /// Called once the plugin generation this handler serves has stopped for any reason —
    /// intentional stop, uninstall, restart, or failure — so a host-spawned process never outlives
    /// the plugin that asked for it.
    pub async fn kill_all(&self) {
        let processes: Vec<Arc<S::Process>> = self
            .0
            .tracked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .map(|tracked| Arc::clone(&tracked.process))
            .collect();
        for process in processes {
            if let Err(error) = process.kill().await {
                ora_warn!(
                    plugin_id = %self.0.plugin_id,
                    error = %error,
                    "failed to kill a host-managed child process during plugin teardown"
                );
            }
        }
    }

    /// Pushes one notification to the plugin, waiting for `attach_runtime` if it has not run yet.
    ///
    /// The wait only ever blocks for the brief window between a launch returning and its caller
    /// calling `attach_runtime`; once that call lands every later push observes it immediately.
    async fn push(&self, method: &str, params: Value) {
        let mut receiver = self.0.runtime.subscribe();
        if receiver.borrow().is_none() && receiver.changed().await.is_err() {
            return;
        }
        let runtime = receiver.borrow().clone();
        if let Some(runtime) = runtime {
            let _ = runtime.notify(method, params).await;
        }
    }

    async fn handle_spawn(&self, params: Value) -> Result<Value, HostRequestError> {
        let request = parse_spawn_params(&params)?;
        let mut spec = ProcessSpec::new(request.command)
            .args(request.args)
            .stdin(ProcessStdio::Piped)
            .stdout(ProcessStdio::Piped)
            .stderr(ProcessStdio::Piped);
        if let Some(cwd) = request.cwd {
            spec = spec.cwd(cwd);
        }
        for (key, value) in request.env {
            spec = spec.env(key, value);
        }

        let mut process = self
            .0
            .spawner
            .spawn(spec)
            .map_err(ChildProcessError::from_spawn_io)?;
        let pid = process.id();
        let stdio = (
            process.take_stdin(),
            process.take_stdout(),
            process.take_stderr(),
        );
        let (Some(stdin), Some(stdout), Some(stderr)) = stdio else {
            let _ = process.kill().await;
            let _ = process.wait().await;
            return Err(ChildProcessError::new(
                ChildProcessErrorKind::Io,
                "spawned process stdio is unavailable",
            )
            .into());
        };

        let process_id = self.0.next_id.fetch_add(1, Ordering::Relaxed).to_string();
        let (stdin_tx, stdin_rx) = mpsc::channel(32);
        tokio::spawn(run_stdin_writer(stdin, stdin_rx));

        let process = Arc::new(process);
        let stdout_done = tokio::spawn(pump_output(
            self.clone(),
            process_id.clone(),
            stdout,
            CHILDPROCESS_STDOUT_METHOD,
        ));
        let stderr_done = tokio::spawn(pump_output(
            self.clone(),
            process_id.clone(),
            stderr,
            CHILDPROCESS_STDERR_METHOD,
        ));
        self.0
            .tracked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                process_id.clone(),
                Tracked {
                    process: Arc::clone(&process),
                    stdin_tx,
                    stdout_done,
                    stderr_done,
                },
            );

        tokio::spawn(watch_exit(self.clone(), process_id.clone(), process));

        Ok(json!({ "processId": process_id, "pid": pid }))
    }

    async fn handle_write(&self, params: Value) -> Result<Value, HostRequestError> {
        let process_id = required_process_id(&params)?;
        let bytes_base64 = params
            .get("bytesBase64")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ChildProcessError::new(
                    ChildProcessErrorKind::InvalidParams,
                    "missing string bytesBase64",
                )
            })?;
        if bytes_base64.len() > MAX_WRITE_BASE64_LEN {
            return Err(ChildProcessError::new(
                ChildProcessErrorKind::InvalidParams,
                format!("bytesBase64 decodes to more than {MAX_WRITE_BYTES} bytes"),
            )
            .into());
        }
        let bytes = BASE64.decode(bytes_base64).map_err(|error| {
            ChildProcessError::new(
                ChildProcessErrorKind::InvalidParams,
                format!("bytesBase64 is not valid base64: {error}"),
            )
        })?;
        self.send_stdin(&process_id, StdinCommand::Write(bytes))
            .await?;
        Ok(json!({}))
    }

    async fn handle_close_stdin(&self, params: Value) -> Result<Value, HostRequestError> {
        let process_id = required_process_id(&params)?;
        self.send_stdin(&process_id, StdinCommand::Close).await?;
        Ok(json!({}))
    }

    async fn send_stdin(
        &self,
        process_id: &str,
        command: StdinCommand,
    ) -> Result<(), ChildProcessError> {
        let stdin_tx = self
            .0
            .tracked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(process_id)
            .map(|tracked| tracked.stdin_tx.clone())
            .ok_or_else(|| {
                ChildProcessError::new(ChildProcessErrorKind::NotFound, "unknown processId")
            })?;
        stdin_tx.send(command).await.map_err(|_| {
            ChildProcessError::new(ChildProcessErrorKind::Io, "process stdin is closed")
        })
    }

    async fn handle_kill(&self, params: Value) -> Result<Value, HostRequestError> {
        let process_id = required_process_id(&params)?;
        let process = self
            .0
            .tracked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&process_id)
            .map(|tracked| Arc::clone(&tracked.process))
            .ok_or_else(|| {
                ChildProcessError::new(ChildProcessErrorKind::NotFound, "unknown processId")
            })?;
        process.kill().await.map_err(|error| {
            ChildProcessError::new(ChildProcessErrorKind::Io, error.to_string())
        })?;
        Ok(json!({}))
    }
}

impl<S> HostRequestHandler for PluginProcessHost<S>
where
    S: ProcessSpawner + Send + Sync + 'static,
    S::Process: Send + Sync + 'static,
{
    async fn handle(&self, method: &str, params: Value) -> Result<Value, HostRequestError> {
        match method {
            CHILDPROCESS_SPAWN_METHOD => self.handle_spawn(params).await,
            CHILDPROCESS_WRITE_METHOD => self.handle_write(params).await,
            CHILDPROCESS_CLOSE_STDIN_METHOD => self.handle_close_stdin(params).await,
            CHILDPROCESS_KILL_METHOD => self.handle_kill(params).await,
            other => Err(HostRequestError::method_not_found(other)),
        }
    }
}

/// Feeds one spawned process's stdin from the channel `write`/`closeStdin` requests publish to.
async fn run_stdin_writer<W: AsyncWrite + Unpin>(
    mut stdin: W,
    mut commands: mpsc::Receiver<StdinCommand>,
) {
    while let Some(command) = commands.recv().await {
        match command {
            StdinCommand::Write(bytes) => {
                if stdin.write_all(&bytes).await.is_err() {
                    return;
                }
            }
            StdinCommand::Close => {
                let _ = stdin.shutdown().await;
                return;
            }
        }
    }
}

/// Forwards every chunk read from one spawned process's stdout or stderr as a notification.
async fn pump_output<S, R>(
    host: PluginProcessHost<S>,
    process_id: String,
    mut reader: R,
    method: &'static str,
) where
    S: ProcessSpawner + Send + Sync + 'static,
    S::Process: Send + Sync + 'static,
    R: AsyncRead + Unpin,
{
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => return,
            Ok(length) => {
                host.push(
                    method,
                    json!({
                        "processId": process_id,
                        "bytesBase64": BASE64.encode(&buffer[..length]),
                    }),
                )
                .await;
            }
            Err(_) => return,
        }
    }
}

/// Waits for one spawned process to exit, then reports it and stops tracking it.
async fn watch_exit<S>(host: PluginProcessHost<S>, process_id: String, process: Arc<S::Process>)
where
    S: ProcessSpawner + Send + Sync + 'static,
    S::Process: Send + Sync + 'static,
{
    let status = process.wait().await;
    let tracked = host
        .0
        .tracked
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&process_id);
    // Drain whatever stdout/stderr the pumps already read before announcing exit, so the plugin
    // never sees the exit notification arrive ahead of the process's last output.
    if let Some(tracked) = tracked {
        let _ = tokio::join!(tracked.stdout_done, tracked.stderr_done);
    }
    if let Err(error) = &status {
        ora_warn!(
            plugin_id = %host.0.plugin_id,
            error = %error,
            "failed to wait for a host-managed child process"
        );
    }
    let (code, signal) = exit_fields(status.as_ref());
    host.push(
        CHILDPROCESS_EXIT_METHOD,
        json!({ "processId": process_id, "code": code, "signal": signal }),
    )
    .await;
}

/// Splits a process's final status into a wire-friendly exit code and, on Unix, a signal number.
fn exit_fields(status: Result<&ExitStatus, &io::Error>) -> (Option<i32>, Option<i32>) {
    let Ok(status) = status else {
        return (None, None);
    };
    let code = status.code();
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;
    (code, signal)
}

/// One validated `spawn` request.
struct SpawnParams {
    command: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    env: Vec<(String, String)>,
}

/// Parses and validates the `spawn` params, rejecting anything not shaped like the documented
/// `{ command, args?, cwd?, env? }`.
fn parse_spawn_params(params: &Value) -> Result<SpawnParams, ChildProcessError> {
    let object = params.as_object().ok_or_else(|| {
        ChildProcessError::new(
            ChildProcessErrorKind::InvalidParams,
            "spawn params must be an object",
        )
    })?;
    let command = object
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ChildProcessError::new(
                ChildProcessErrorKind::InvalidParams,
                "missing string command",
            )
        })?;
    if command.trim().is_empty() {
        return Err(ChildProcessError::new(
            ChildProcessErrorKind::InvalidCommand,
            "command must not be empty",
        ));
    }
    let args = match object.get("args") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str().map(str::to_owned).ok_or_else(|| {
                    ChildProcessError::new(
                        ChildProcessErrorKind::InvalidParams,
                        "args must be strings",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => {
            return Err(ChildProcessError::new(
                ChildProcessErrorKind::InvalidParams,
                "args must be an array of strings",
            ));
        }
    };
    let cwd = match object.get("cwd") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(PathBuf::from(value)),
        Some(_) => {
            return Err(ChildProcessError::new(
                ChildProcessErrorKind::InvalidParams,
                "cwd must be a string",
            ));
        }
    };
    let env = match object.get("env") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Object(entries)) => entries
            .iter()
            .map(|(key, value)| {
                value
                    .as_str()
                    .map(|value| (key.clone(), value.to_owned()))
                    .ok_or_else(|| {
                        ChildProcessError::new(
                            ChildProcessErrorKind::InvalidParams,
                            "env values must be strings",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => {
            return Err(ChildProcessError::new(
                ChildProcessErrorKind::InvalidParams,
                "env must be an object of strings",
            ));
        }
    };
    Ok(SpawnParams {
        command: command.to_owned(),
        args,
        cwd,
        env,
    })
}

/// Extracts and validates the `processId` param shared by `write`, `closeStdin`, and `kill`.
fn required_process_id(params: &Value) -> Result<String, ChildProcessError> {
    params
        .get("processId")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            ChildProcessError::new(
                ChildProcessErrorKind::InvalidParams,
                "missing string processId",
            )
        })
}
