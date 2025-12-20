use symseek::cli::{Cli, args};

fn main() {
    // Parse args early to check verbose flag before logger init
    let args = args::Args::parse();

    // Initialize logger based on verbose flag and RUST_LOG env var
    init_logger(args.verbose);

    // Now create CLI with the already-parsed args
    let cli = Cli::with_args(args);

    if let Err(e) = cli.run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn init_logger(verbose: bool) {
    let mut builder = env_logger::Builder::new();

    // RUST_LOG env var takes precedence, then --verbose flag, then silent by default
    match std::env::var("RUST_LOG") {
        Ok(rust_log) => {
            // Use RUST_LOG value
            builder.parse_filters(&rust_log);
        }
        Err(_) => {
            // RUST_LOG not set, use --verbose flag or default to silent
            if verbose {
                builder.filter_level(log::LevelFilter::Debug);
            } else {
                builder.filter_level(log::LevelFilter::Off);
            }
        }
    }

    builder.format_timestamp(None).try_init().ok();
}
