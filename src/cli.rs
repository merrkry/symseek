pub mod args;

use crate::core::{resolver, search, types::FileLocation};
use crate::error::Result;
use crate::output::{OutputFormat, formatter, json};
use log::debug;

/// Main CLI application struct.
///
/// Handles parsing command-line arguments and running the symlink resolution logic.
pub struct Cli {
    args: args::Args,
}

impl Default for Cli {
    fn default() -> Self {
        Self::new()
    }
}

impl Cli {
    /// Create a new CLI instance with parsed command-line arguments.
    #[must_use]
    pub fn new() -> Self {
        Self {
            args: args::Args::parse(),
        }
    }

    /// Create a new CLI instance with provided arguments.
    #[must_use]
    pub const fn with_args(args: args::Args) -> Self {
        Self { args }
    }

    /// Run the CLI application.
    ///
    /// Searches for the target file/binary and resolves its symlink chain,
    /// printing the results to stdout.
    ///
    /// # Errors
    ///
    /// Returns an error if file lookup or symlink resolution fails.
    pub fn run(&self) -> Result<()> {
        debug!("Searching for target: {}", &self.args.target);
        let location = search::find_file(&self.args.target)?;
        let format = self.args.output_format();

        match location {
            FileLocation::CurrentDirectory(path) => {
                debug!("Found in current directory: {}", path.display());
                let chain = resolver::resolve(&path)?;

                match format {
                    OutputFormat::Json => json::print_json_single(&chain)?,
                    OutputFormat::Tree => formatter::print_tree(&chain),
                }
            }
            FileLocation::PathEnvironment(paths) => {
                debug!("Found {} matches in PATH", paths.len());

                match format {
                    OutputFormat::Json => {
                        let chains: Result<Vec<_>> =
                            paths.iter().map(|p| resolver::resolve(p)).collect();
                        json::print_json_multiple(&chains?)?;
                    }
                    OutputFormat::Tree => {
                        formatter::print_header(paths.len());
                        for (idx, path) in paths.iter().enumerate() {
                            debug!(
                                "Resolving PATH match {}/{}: {}",
                                idx + 1,
                                paths.len(),
                                path.display()
                            );
                            let chain = resolver::resolve(path)?;
                            formatter::print_tree(&chain);
                            formatter::print_separator();
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
