use std::ffi::OsStr;
use std::io;
use std::process::{ExitStatus, Stdio};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

/// Spawns OS child processes and returns a handle that owns their lifecycle and stdio pipes.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessSpawner;

impl ProcessSpawner {
    pub fn new() -> Self {
        Self
    }

    /// Spawns one child process with stdin, stdout, and stderr configured as piped streams.
    pub async fn spawn<Program, Args, Arg>(
        &self,
        program: Program,
        args: Args,
    ) -> io::Result<ManagedProcess>
    where
        Program: AsRef<OsStr>,
        Args: IntoIterator<Item = Arg>,
        Arg: AsRef<OsStr>,
    {
        let mut command = Command::new(program);
        command.args(args);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.kill_on_drop(true);

        command.spawn().map(|child| ManagedProcess { child })
    }
}

/// Owns one child process and exposes its raw async stdio pipes.
#[derive(Debug)]
pub struct ManagedProcess {
    child: Child,
}

impl ManagedProcess {
    /// Returns the platform process identifier when Tokio exposes one.
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    /// Moves the stdin pipe out of the process handle, returning `None` after it has been taken.
    pub fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.child.stdin.take()
    }

    /// Moves the stdout pipe out of the process handle, returning `None` after it has been taken.
    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }

    /// Moves the stderr pipe out of the process handle, returning `None` after it has been taken.
    pub fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.child.stderr.take()
    }

    /// Checks whether the child has exited without waiting for it to finish.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    /// Waits until the child exits and returns its platform exit status.
    pub async fn wait(&mut self) -> io::Result<ExitStatus> {
        self.child.wait().await
    }

    /// Forcefully terminates the child process.
    pub async fn kill(&mut self) -> io::Result<()> {
        self.child.kill().await
    }
}
