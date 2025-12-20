pub mod args;

use crate::core::{resolver, search, types::FileLocation};
use crate::error::Result;
use crate::output::formatter;
use log::debug;

pub struct Cli {
    args: args::Args,
}

impl Cli {
    pub fn new() -> Self {
        Cli {
            args: args::Args::parse(),
        }
    }

    pub fn with_args(args: args::Args) -> Self {
        Cli { args }
    }

    pub fn args(&self) -> &args::Args {
        &self.args
    }

    pub fn run(&self) -> Result<()> {
        debug!("Searching for target: {}", self.args.target);
        let location = search::find_file(&self.args.target)?;

        match location {
            FileLocation::CurrentDirectory(path) => {
                debug!("Found in current directory: {}", path.display());
                let chain = resolver::resolve(&path)?;
                formatter::print_tree(&chain);
            }
            FileLocation::PathEnvironment(paths) => {
                debug!("Found {} matches in PATH", paths.len());
                formatter::print_header(paths.len());
                for (idx, path) in paths.iter().enumerate() {
                    debug!("Resolving PATH match {}/{}: {}", idx + 1, paths.len(), path.display());
                    let chain = resolver::resolve(path)?;
                    formatter::print_tree(&chain);
                    formatter::print_separator();
                }
            }
        }

        Ok(())
    }
}
