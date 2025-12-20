use crate::core::types::{FileKind, LinkType, ScriptType, SymlinkChain, WrapperKind};
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

        let (indicator, label) = link_type_info(&node.link_type);

        println!(
            "{}{}{} {}{}",
            prefix,
            chars.connector,
            indicator,
            format_path(&node.target),
            label
        );
    }
}

fn link_type_info(link_type: &LinkType) -> (&'static str, String) {
    match link_type {
        LinkType::Symlink => ("", String::new()),
        LinkType::Wrapper(wrapper_kind) => match wrapper_kind {
            WrapperKind::Binary => ("", " [binary wrapper]".to_string()),
            WrapperKind::Text(script_type) => {
                let label = match script_type {
                    ScriptType::Shell => " [sh wrapper]",
                    ScriptType::Python => " [py wrapper]",
                    ScriptType::Perl => " [pl wrapper]",
                    ScriptType::Unknown => " [script wrapper]",
                };
                ("", label.to_string())
            }
        },
        LinkType::Terminal(file_kind) => match file_kind {
            FileKind::Binary => ("", " [binary]".to_string()),
            FileKind::Text => ("", " [text]".to_string()),
        },
    }
}

pub fn print_header(count: usize) {
    println!("Found {count} matches in PATH\n");
}

pub fn print_separator() {
    println!();
}

fn format_path(path: &Path) -> String {
    path_clean::clean(path)
        .to_str()
        .map_or_else(|| "<invalid UTF-8>".to_string(), std::string::ToString::to_string)
}
