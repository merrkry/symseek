use crate::core::types::SymlinkChain;
use crate::error::Result;
use crate::output::styles::TreeChars;
use std::path::Path;

pub fn print_tree(chain: &SymlinkChain) {
    println!("{}", format_path(&chain.origin));

    if chain.is_empty() {
        return;
    }

    let chars = TreeChars::default();
    for (idx, node) in chain.links.iter().enumerate() {
        let is_last = idx == chain.links.len() - 1;
        let prefix = if is_last { chars.last } else { chars.branch };
        println!("{}{}{}", prefix, chars.connector, format_path(&node.target));
    }
}

pub fn print_json(_chain: &SymlinkChain) -> Result<()> {
    todo!("JSON output not implemented yet")
}

pub fn print_header(count: usize) {
    println!("Found {} matches in PATH\n", count);
}

pub fn print_separator() {
    println!();
}

fn format_path(path: &Path) -> String {
    path_clean::clean(path)
        .to_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "<invalid UTF-8>".to_string())
}
