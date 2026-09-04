use symseek::cli::{Cli, args};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

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
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_writer(std::io::stderr);

    if std::env::var_os(EnvFilter::DEFAULT_ENV).is_some() {
        subscriber
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    } else {
        let level = if verbose {
            LevelFilter::DEBUG
        } else {
            LevelFilter::OFF
        };
        subscriber.with_max_level(level).init();
    }
}
