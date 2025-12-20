//! Output formatting utilities for symlink chains.

pub mod formatter;
pub mod json;
pub mod styles;

/// Output format for symlink chain display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Human-readable tree format (default)
    #[default]
    Tree,
    /// Machine-readable JSON format
    Json,
}
