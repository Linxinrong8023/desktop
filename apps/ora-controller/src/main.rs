use std::process::ExitCode;

#[cfg(target_os = "linux")]
mod cli;

/// Runs the composed Controller executable with executable-owned signals.
fn main() -> ExitCode {
    #[cfg(target_os = "linux")]
    {
        match run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("ora-controller: {error}");
                ExitCode::FAILURE
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("ora-controller currently requires the Linux local Node runtime");
        ExitCode::FAILURE
    }
}

/// Validates flags and configuration before opening state; stops only this composition on shutdown.
#[cfg(target_os = "linux")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use clap::Parser;
    use ora_controller::{DeploymentConfig, Service};
    use tokio::signal::unix::{SignalKind, signal};
    let cli = cli::Cli::parse();
    if !cli.config.is_absolute() {
        return Err("configuration path must be absolute".into());
    }
    let transport = cli.transport()?;
    let hosting = cli.hosting();
    let config: DeploymentConfig = serde_json::from_slice(&std::fs::read(&cli.config)?)?;
    let _logging = ora_logging::init_logging(ora_logging::LoggingConfig::new(
        ora_logging::LogLevel::Info,
        ora_logging::LogOutput::Stdout,
        config.controller.timezone.parse()?,
    ))?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut terminate = signal(SignalKind::terminate())?;
            let mut interrupt = signal(SignalKind::interrupt())?;
            let service = Service::start(config, transport, hosting).await?;
            println!("ora-controller listening on {}", service.endpoint()?);
            service
                .run(async {
                    tokio::select! { _ = terminate.recv() => {}, _ = interrupt.recv() => {} }
                })
                .await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}
