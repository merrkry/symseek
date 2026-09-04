use crate::core::types::{FileKind, LinkType, ScriptType, SymlinkChain, SymlinkNode, WrapperKind};
use crate::output::styles::{MUTED, ORIGIN, SYMLINK, TERMINAL, TERMINAL_PATH, WRAPPER};
use anstyle::Style;
use std::fmt;
use std::path::Path;
use termtree::Tree;

pub fn print_tree(chain: &SymlinkChain) {
    anstream::print!("{}", styled_tree(chain));
}

pub fn print_header(count: usize) {
    anstream::println!("{ORIGIN}{count}{ORIGIN:#} {MUTED}matches in PATH{MUTED:#}\n");
}

pub fn print_separator() {
    anstream::println!();
}

fn styled_tree(chain: &SymlinkChain) -> Tree<StyledNode> {
    let root = Tree::new(StyledNode::origin(&chain.origin));

    chain
        .links
        .iter()
        .rev()
        .map(StyledNode::link)
        .fold(None, |child, node| {
            Some(match child {
                Some(child) => Tree::new(node).with_leaves([child]),
                None => Tree::new(node),
            })
        })
        .map_or(root.clone(), |child| root.with_leaves([child]))
}

#[derive(Clone, Debug)]
struct StyledNode {
    path: String,
    path_style: Style,
    label: Option<(&'static str, Style)>,
}

impl StyledNode {
    fn origin(path: &Path) -> Self {
        Self {
            path: format_path(path),
            path_style: ORIGIN,
            label: None,
        }
    }

    fn link(node: &SymlinkNode) -> Self {
        let (label, style) = match &node.link_type {
            LinkType::Symlink => ("symlink", SYMLINK),
            LinkType::Wrapper(WrapperKind::Binary) => ("binary wrapper", WRAPPER),
            LinkType::Wrapper(WrapperKind::Text(script)) => match script {
                ScriptType::Shell => ("shell wrapper", WRAPPER),
                ScriptType::Python => ("python wrapper", WRAPPER),
                ScriptType::Perl => ("perl wrapper", WRAPPER),
                ScriptType::Unknown => ("script wrapper", WRAPPER),
            },
            LinkType::Terminal(FileKind::Binary) => ("binary", TERMINAL),
            LinkType::Terminal(FileKind::Text) => ("text", TERMINAL),
        };

        Self {
            path: format_path(&node.target),
            path_style: if node.is_final {
                TERMINAL_PATH
            } else {
                Style::new()
            },
            label: Some((label, style)),
        }
    }
}

impl fmt::Display for StyledNode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}{}{}",
            self.path_style,
            self.path,
            self.path_style.render_reset()
        )?;

        if let Some((label, style)) = self.label {
            write!(formatter, "  {style}{label}{style:#}")?;
        }

        Ok(())
    }
}

fn format_path(path: &Path) -> String {
    path_clean::clean(path).to_str().map_or_else(
        || "<invalid UTF-8>".to_string(),
        std::string::ToString::to_string,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_chain_as_nested_nodes() {
        let mut chain = SymlinkChain::new("/usr/bin/tool".into());
        chain.add_link("/opt/tool".into(), false, LinkType::Symlink);
        chain.add_link(
            "/nix/store/tool/bin/tool".into(),
            true,
            LinkType::Terminal(FileKind::Binary),
        );

        let styled = styled_tree(&chain).to_string();
        let plain = anstream::adapter::strip_str(&styled).to_string();

        assert_eq!(
            plain,
            concat!(
                "/usr/bin/tool\n",
                "└── /opt/tool  symlink\n",
                "    └── /nix/store/tool/bin/tool  binary\n",
            )
        );
    }

    #[test]
    fn includes_ansi_styles_before_stream_adaptation() {
        let chain = SymlinkChain::new("/usr/bin/tool".into());

        assert!(styled_tree(&chain).to_string().contains("\u{1b}["));
    }
}
