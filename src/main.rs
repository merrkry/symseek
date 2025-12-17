use symseek::cli::Cli;

fn main() {
    let cli = Cli::new();

    if let Err(e) = cli.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
