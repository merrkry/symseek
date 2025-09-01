use std::env;

use symseek::utils::{self, FileLocation};

fn main() {
    let args: Vec<String> = env::args().collect();

    assert!(args.len() == 2, "Usage: symseek <name>");

    let target = &args[1];

    match utils::search_file(target) {
        Ok(FileLocation::Cwd(path)) => {
            utils::print_trace(&path);
        }
        Ok(FileLocation::Path(paths)) => {
            println!("Found {} matches in PATH\n", paths.len());

            for path in paths {
                utils::print_trace(&path);
                println!();
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
