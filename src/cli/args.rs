use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "symseek")]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Target file or binary name to trace
    pub target: String,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

impl Args {
    pub fn parse() -> Self {
        <Args as Parser>::parse()
    }
}
