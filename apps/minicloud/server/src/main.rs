/// Starts the development HTTP composition; unsupported platforms do not silently weaken Node requirements.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        use ora_minicloud_server::{Server, ServerConfig};
        use tokio::signal::unix::{SignalKind, signal};
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args.len() != 1 || !std::path::Path::new(&args[0]).is_absolute() {
            return Err("usage: ora-minicloud-server <absolute-config-file>".into());
        }
        let config: ServerConfig = serde_json::from_slice(&std::fs::read(&args[0])?)?;
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
                let server = Server::bind(config).await?;
                println!("minicloud listening on {}", server.local_addr()?);
                server
                    .run(async {
                        tokio::select! { _ = terminate.recv() => {}, _ = interrupt.recv() => {} }
                    })
                    .await
            })?;
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    Err("minicloud currently requires the Linux local Node runtime".into())
}
