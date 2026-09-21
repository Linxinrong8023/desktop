use std::process::ExitCode;

/// Runs the shared coordinator runtime with executable-owned signals.
fn main() -> ExitCode {
    #[cfg(target_os = "linux")]
    {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args.len() == 1 {
            return match run(std::path::Path::new(&args[0])) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("ora-controller: {error}");
                    ExitCode::FAILURE
                }
            };
        }
    }
    eprintln!("usage (Linux): ora-controller <absolute-config-file>");
    ExitCode::FAILURE
}

/// Opens injected state and stops only this runtime's connections on process shutdown.
#[cfg(target_os = "linux")]
fn run(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use ora_controller::{ControllerRuntime, RuntimeConfig};
    use tokio::signal::unix::{SignalKind, signal};
    if !path.is_absolute() {
        return Err("configuration path must be absolute".into());
    }
    let config: RuntimeConfig = serde_json::from_slice(&std::fs::read(path)?)?;
    let _logging = ora_logging::init_logging(ora_logging::LoggingConfig::new(
        ora_logging::LogLevel::Info,
        ora_logging::LogOutput::Stdout,
        config.timezone.parse()?,
    ))?;
    let runtime = ControllerRuntime::open(config)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut terminate = signal(SignalKind::terminate())?;
            let mut interrupt = signal(SignalKind::interrupt())?;
            runtime
                .run(async {
                    tokio::select! { _ = terminate.recv() => {}, _ = interrupt.recv() => {} }
                })
                .await
        })?;
    Ok(())
}
