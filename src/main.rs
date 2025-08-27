use std::env;

use symseek::utils::{self, FileLocation};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Usage: symseek <name>");
    }

    let target = &args[1];

    match utils::search_file(target) {
        Ok(FileLocation::Cwd(path)) => {
            let resolved = utils::resolve_symlink(&path).expect("Failed to resolve symlink");

            println!("{}", path.to_str().expect("Invalid path"));
            for (idx, path) in resolved.iter().enumerate() {
                let leading_char = match idx {
                    _ if idx == resolved.len() - 1 => '└',
                    _ => '├',
                };

                println!("{}─{}", leading_char, path.to_str().unwrap_or("0"));
            }
        }
        Ok(FileLocation::Path(paths)) => {
            println!("Found {} matches in PATH", paths.len());

            for path in paths {
                println!("\n{}", path.to_str().expect("Invalid path"));
                let resolved = utils::resolve_symlink(&path).expect("Failed to resolve symlink");
                for (idx, path) in resolved.iter().enumerate() {
                    let leading_char = match idx {
                        _ if idx == resolved.len() - 1 => '└',
                        _ => '├',
                    };

                    println!("{}─{}", leading_char, path.to_str().unwrap_or("0"));
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
