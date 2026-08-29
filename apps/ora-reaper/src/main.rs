fn main() {
    if let Err(error) = ora_process::run_reaper() {
        eprintln!("ora-reaper failed: {error}");
        std::process::exit(1);
    }
}
