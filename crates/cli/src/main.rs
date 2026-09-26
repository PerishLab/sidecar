fn main() {
    if let Err(error) = plumb::identity!("SIDECAR") {
        eprintln!("{error}");
        std::process::exit(1);
    }
    if let Err(error) = sidecar_cli::run(std::env::args().collect()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
