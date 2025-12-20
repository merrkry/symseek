//! JSON output formatting for symlink chains.

use crate::core::types::{FileKind, LinkType, ScriptType, SymlinkChain, WrapperKind};
use crate::error::Result;
use serde::Serialize;
use std::path::Path;

/// JSON representation of a symlink chain
#[derive(Debug, Serialize)]
pub struct JsonChain {
    pub origin: String,
    pub links: Vec<JsonLink>,
}

/// JSON representation of a link in the chain
#[derive(Debug, Serialize)]
pub struct JsonLink {
    pub path: String,
    #[serde(rename = "type")]
    pub link_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrapper_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_kind: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_final: bool,
}

impl JsonChain {
    /// Convert a `SymlinkChain` to JSON-serializable format
    pub fn from_chain(chain: &SymlinkChain) -> Self {
        Self {
            origin: format_path(&chain.origin),
            links: chain.links.iter().map(JsonLink::from_node).collect(),
        }
    }
}

impl JsonLink {
    /// Convert a `SymlinkNode` to JSON-serializable format
    fn from_node(node: &crate::core::types::SymlinkNode) -> Self {
        let (link_type, wrapper_kind, file_kind) = match &node.link_type {
            LinkType::Symlink => ("symlink".to_string(), None, None),
            LinkType::Wrapper(kind) => {
                let wrapper_str = match kind {
                    WrapperKind::Binary => "binary",
                    WrapperKind::Text(ScriptType::Shell) => "shell_script",
                    WrapperKind::Text(ScriptType::Python) => "python_script",
                    WrapperKind::Text(ScriptType::Perl) => "perl_script",
                    WrapperKind::Text(ScriptType::Unknown) => "unknown_script",
                };
                ("wrapper".to_string(), Some(wrapper_str.to_string()), None)
            }
            LinkType::Terminal(kind) => {
                let file_str = match kind {
                    FileKind::Binary => "binary",
                    FileKind::Text => "text",
                };
                ("terminal".to_string(), None, Some(file_str.to_string()))
            }
        };

        Self {
            path: format_path(&node.target),
            link_type,
            wrapper_kind,
            file_kind,
            is_final: node.is_final,
        }
    }
}

/// Format a path consistently with the tree formatter
fn format_path(path: &Path) -> String {
    path_clean::clean(path).to_str().map_or_else(
        || "<invalid UTF-8>".to_string(),
        std::string::ToString::to_string,
    )
}

/// Print a single chain as JSON
///
/// # Errors
///
/// Returns an error if JSON serialization fails.
pub fn print_json_single(chain: &SymlinkChain) -> Result<()> {
    let json_chain = JsonChain::from_chain(chain);
    let json = serde_json::to_string_pretty(&json_chain)?;
    println!("{json}");
    Ok(())
}

/// Print multiple chains as a JSON array
///
/// # Errors
///
/// Returns an error if JSON serialization fails.
pub fn print_json_multiple(chains: &[SymlinkChain]) -> Result<()> {
    let json_chains: Vec<JsonChain> = chains.iter().map(JsonChain::from_chain).collect();
    let json = serde_json::to_string_pretty(&json_chains)?;
    println!("{json}");
    Ok(())
}
