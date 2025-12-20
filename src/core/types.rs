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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symlink_chain_new() {
        let path = PathBuf::from("/test/path");
        let chain = SymlinkChain::new(path.clone());

        assert_eq!(chain.origin, path);
        assert!(chain.is_empty());
        assert_eq!(chain.links.len(), 0);
    }

    #[test]
    fn test_add_link() {
        let mut chain = SymlinkChain::new(PathBuf::from("/origin"));

        chain.add_link(PathBuf::from("/link1"), false, LinkType::Symlink);
        assert!(!chain.is_empty());
        assert_eq!(chain.links.len(), 1);
        assert!(!chain.links[0].is_final);

        chain.add_link(
            PathBuf::from("/link2"),
            true,
            LinkType::Terminal(FileKind::Binary),
        );
        assert_eq!(chain.links.len(), 2);
        assert!(chain.links[1].is_final);
    }

    #[test]
    fn test_file_location_current_directory() {
        let loc = FileLocation::CurrentDirectory(PathBuf::from("/cwd/file"));
        match loc {
            FileLocation::CurrentDirectory(path) => assert_eq!(path, PathBuf::from("/cwd/file")),
            FileLocation::PathEnvironment(_) => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_file_location_path_environment() {
        let paths = vec![
            PathBuf::from("/usr/bin/exe"),
            PathBuf::from("/usr/local/bin/exe"),
        ];
        let loc = FileLocation::PathEnvironment(paths.clone());

        match loc {
            FileLocation::PathEnvironment(matched_paths) => {
                assert_eq!(matched_paths.len(), 2);
                assert_eq!(matched_paths, paths);
            }
            FileLocation::CurrentDirectory(_) => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_symlink_node_creation() {
        let node = SymlinkNode {
            target: PathBuf::from("/target"),
            is_final: true,
            link_type: LinkType::Terminal(FileKind::Binary),
            metadata: None,
        };

        assert_eq!(node.target, PathBuf::from("/target"));
        assert!(node.is_final);
    }

    #[test]
    fn test_chain_with_multiple_links() {
        let mut chain = SymlinkChain::new(PathBuf::from("/start"));

        for i in 0..5 {
            let is_final = i == 4;
            chain.add_link(
                PathBuf::from(format!("/link{i}")),
                is_final,
                LinkType::Symlink,
            );
        }

        assert_eq!(chain.links.len(), 5);
        assert!(!chain.is_empty());
        assert!(chain.links[4].is_final);
        assert!(!chain.links[0].is_final);
    }
}
