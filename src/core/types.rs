use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum LinkType {
    Symlink,
    Wrapper(WrapperKind),
    Terminal(FileKind),
}

#[derive(Debug, Clone)]
pub enum WrapperKind {
    Binary,
    Text(ScriptType),
}

#[derive(Debug, Clone)]
pub enum ScriptType {
    Shell,
    Python,
    Perl,
    Unknown,
}

#[derive(Debug, Clone)]
pub enum FileKind {
    Binary,
    Text,
}

#[derive(Debug, Clone)]
pub enum FileLocation {
    CurrentDirectory(PathBuf),
    PathEnvironment(Vec<PathBuf>),
}

#[derive(Debug, Clone)]
pub struct SymlinkChain {
    pub origin: PathBuf,
    pub links: Vec<SymlinkNode>,
}

#[derive(Debug, Clone)]
pub struct SymlinkNode {
    pub target: PathBuf,
    pub is_final: bool,
    pub link_type: LinkType,
    pub metadata: Option<NodeMetadata>,
}

#[derive(Debug, Clone)]
pub struct NodeMetadata {
    pub is_broken: bool,
    pub file_type: Option<String>,
}

impl SymlinkChain {
    /// Create a new symlink chain starting from the given origin path.
    #[must_use]
    pub const fn new(origin: PathBuf) -> Self {
        Self {
            origin,
            links: Vec::new(),
        }
    }

    /// Add a link to the chain.
    pub fn add_link(&mut self, target: PathBuf, is_final: bool, link_type: LinkType) {
        self.links.push(SymlinkNode {
            target,
            is_final,
            link_type,
            metadata: None,
        });
    }

    /// Check if the chain is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}
