use std::process::ExitCode;

/// Runs a standalone Node with optional explicitly configured local Controller IPC.
fn main() -> ExitCode {
    #[cfg(target_os = "linux")]
    {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args.len() == 1 {
            return match run(std::path::Path::new(&args[0])) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("ora-node: startup or recovery failed: {error}");
                    ExitCode::FAILURE
                }
            };
        }
    }
    eprintln!("usage (Linux): ora-node <absolute-config-file>");
    ExitCode::FAILURE
}

/// Installs shutdown handling before recovery or IPC starts; no Backend entry point is installed.
#[cfg(target_os = "linux")]
fn run(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::signal::unix::{SignalKind, signal};
    if !path.is_absolute() {
        return Err("configuration path must be absolute".into());
    }
    let config: ora_node::ServiceConfig = serde_json::from_slice(&std::fs::read(path)?)?;
    let _logging = ora_logging::init_logging(ora_logging::LoggingConfig::new(
        ora_logging::LogLevel::Info,
        ora_logging::LogOutput::Stdout,
        config.timezone.parse()?,
    ))?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let mut terminate = signal(SignalKind::terminate())?;
            let mut interrupt = signal(SignalKind::interrupt())?;
            let shutdown = ora_node::Shutdown::default();
            let service = ora_node::serve(config, shutdown.clone());
            tokio::pin!(service);
            tokio::select! {
                result = &mut service => result,
                _ = terminate.recv() => { shutdown.request(); service.await },
                _ = interrupt.recv() => { shutdown.request(); service.await },
            }
        })?;
    Ok(())
}
