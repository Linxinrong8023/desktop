use ora_process_manager::{ManagedProcess, ProcessSpawner};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn spawns_process_from_program_and_args() {
    let spawner = ProcessSpawner::new();
    let mut process: ManagedProcess = spawner
        .spawn(shell_program(), shell_args("echo process-manager-stdout"))
        .await
        .unwrap_or_else(|error| panic!("expected process spawn to succeed: {error}"));
    let mut stdout = process
        .take_stdout()
        .unwrap_or_else(|| panic!("expected stdout pipe"));

    let mut output = String::new();
    stdout
        .read_to_string(&mut output)
        .await
        .unwrap_or_else(|error| panic!("expected stdout read to succeed: {error}"));
    let exit = process
        .wait()
        .await
        .unwrap_or_else(|error| panic!("expected process wait to succeed: {error}"));

    assert!(exit.success());
    assert!(output.contains("process-manager-stdout"));
}

#[tokio::test]
async fn exposes_stdin_stdout_and_stderr_as_owned_pipes() {
    let spawner = ProcessSpawner::new();
    let mut process = spawner
        .spawn(stdin_echo_program(), stdin_echo_args())
        .await
        .unwrap_or_else(|error| panic!("expected process spawn to succeed: {error}"));
    let mut stdin = process
        .take_stdin()
        .unwrap_or_else(|| panic!("expected stdin pipe"));
    let mut stdout = process
        .take_stdout()
        .unwrap_or_else(|| panic!("expected stdout pipe"));
    let stderr = process
        .take_stderr()
        .unwrap_or_else(|| panic!("expected stderr pipe"));

    assert!(process.take_stdin().is_none());
    assert!(process.take_stdout().is_none());
    assert!(process.take_stderr().is_none());

    stdin
        .write_all(b"process-manager-stdin\n")
        .await
        .unwrap_or_else(|error| panic!("expected stdin write to succeed: {error}"));
    drop(stdin);
    drop(stderr);

    let mut output = String::new();
    stdout
        .read_to_string(&mut output)
        .await
        .unwrap_or_else(|error| panic!("expected stdout read to succeed: {error}"));
    let exit = process
        .wait()
        .await
        .unwrap_or_else(|error| panic!("expected process wait to succeed: {error}"));

    assert!(exit.success());
    assert!(output.contains("process-manager-stdin"));
}

#[tokio::test]
async fn reports_running_state_and_kills_child_process() {
    let spawner = ProcessSpawner::new();
    let mut process = spawner
        .spawn(long_running_program(), long_running_args())
        .await
        .unwrap_or_else(|error| panic!("expected process spawn to succeed: {error}"));

    assert!(
        process
            .try_wait()
            .unwrap_or_else(|error| panic!("expected try_wait to succeed: {error}"))
            .is_none()
    );

    process
        .kill()
        .await
        .unwrap_or_else(|error| panic!("expected process kill to succeed: {error}"));
    let exit = process
        .wait()
        .await
        .unwrap_or_else(|error| panic!("expected wait after kill to succeed: {error}"));

    assert!(!exit.success());
}

#[cfg(windows)]
fn shell_program() -> &'static str {
    "cmd.exe"
}

#[cfg(windows)]
fn shell_args(script: &'static str) -> [&'static str; 2] {
    ["/C", script]
}

#[cfg(not(windows))]
fn shell_program() -> &'static str {
    "sh"
}

#[cfg(not(windows))]
fn shell_args(script: &'static str) -> [&'static str; 2] {
    ["-c", script]
}

#[cfg(windows)]
fn stdin_echo_program() -> &'static str {
    "cmd.exe"
}

#[cfg(windows)]
fn stdin_echo_args() -> [&'static str; 2] {
    ["/C", "more"]
}

#[cfg(not(windows))]
fn stdin_echo_program() -> &'static str {
    "cat"
}

#[cfg(not(windows))]
fn stdin_echo_args() -> [&'static str; 0] {
    []
}

#[cfg(windows)]
fn long_running_program() -> &'static str {
    "cmd.exe"
}

#[cfg(windows)]
fn long_running_args() -> [&'static str; 2] {
    ["/C", "ping -n 6 127.0.0.1 > nul"]
}

#[cfg(not(windows))]
fn long_running_program() -> &'static str {
    "sh"
}

#[cfg(not(windows))]
fn long_running_args() -> [&'static str; 2] {
    ["-c", "sleep 5"]
}
