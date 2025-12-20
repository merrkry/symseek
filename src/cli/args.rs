use crate::output::OutputFormat;
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

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
}

impl Args {
    /// Parse command-line arguments.
    #[must_use]
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }

    /// Get the output format based on flags.
    #[must_use]
    pub const fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            OutputFormat::Tree
        }
    }
}
