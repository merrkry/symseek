use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkType {
    Symlink,
    Wrapper(WrapperKind),
    Terminal(FileKind),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WrapperKind {
    Binary,
    Text(ScriptType),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptType {
    Shell,
    Python,
    Perl,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileKind {
    Binary,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileLocation {
    CurrentDirectory(PathBuf),
    PathEnvironment(Vec<PathBuf>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymlinkChain {
    pub origin: PathBuf,
    pub links: Vec<SymlinkNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymlinkNode {
    pub target: PathBuf,
    pub is_final: bool,
    pub link_type: LinkType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<NodeMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub is_broken: bool,
    pub file_type: Option<String>,
}

impl SymlinkChain {
    pub fn new(origin: PathBuf) -> Self {
        Self {
            origin,
            links: Vec::new(),
        }
    }

    pub fn add_link(&mut self, target: PathBuf, is_final: bool, link_type: LinkType) {
        self.links.push(SymlinkNode {
            target,
            is_final,
            link_type,
            metadata: None,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}
