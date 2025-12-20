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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_default() {
        let args = Args {
            target: "test".to_string(),
            verbose: false,
            json: false,
        };
        assert_eq!(args.output_format(), OutputFormat::Tree);
    }

    #[test]
    fn test_output_format_json() {
        let args = Args {
            target: "test".to_string(),
            verbose: false,
            json: true,
        };
        assert_eq!(args.output_format(), OutputFormat::Json);
    }

    #[test]
    fn test_output_format_with_verbose() {
        let args = Args {
            target: "test".to_string(),
            verbose: true,
            json: false,
        };
        assert_eq!(args.output_format(), OutputFormat::Tree);

        let args_json = Args {
            target: "test".to_string(),
            verbose: true,
            json: true,
        };
        assert_eq!(args_json.output_format(), OutputFormat::Json);
    }

    #[test]
    fn test_output_format_both_flags() {
        // Test combinations of verbose and json flags
        let args_tree_quiet = Args {
            target: "test".to_string(),
            verbose: false,
            json: false,
        };
        assert_eq!(args_tree_quiet.output_format(), OutputFormat::Tree);

        let args_tree_verbose = Args {
            target: "test".to_string(),
            verbose: true,
            json: false,
        };
        assert_eq!(args_tree_verbose.output_format(), OutputFormat::Tree);

        let args_json_quiet = Args {
            target: "test".to_string(),
            verbose: false,
            json: true,
        };
        assert_eq!(args_json_quiet.output_format(), OutputFormat::Json);

        let args_json_verbose = Args {
            target: "test".to_string(),
            verbose: true,
            json: true,
        };
        assert_eq!(args_json_verbose.output_format(), OutputFormat::Json);
    }
}
