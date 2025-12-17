pub mod args;

use crate::core::{resolver, search, types::FileLocation};
use crate::error::Result;
use crate::output::formatter;

pub struct Cli {
    args: args::Args,
}

impl Cli {
    pub fn new() -> Self {
        Cli {
            args: args::Args::parse(),
        }
    }

    pub fn run(&self) -> Result<()> {
        let location = search::find_file(&self.args.target)?;

        match location {
            FileLocation::CurrentDirectory(path) => {
                let chain = resolver::resolve(&path)?;
                formatter::print_tree(&chain);
            }
            FileLocation::PathEnvironment(paths) => {
                formatter::print_header(paths.len());
                for path in paths {
                    let chain = resolver::resolve(&path)?;
                    formatter::print_tree(&chain);
                    formatter::print_separator();
                }
            }
        }

        Ok(())
    }
}
