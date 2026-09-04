use anstyle::{AnsiColor, Color, Style};

pub const ORIGIN: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::BrightCyan)))
    .bold();
pub const TERMINAL_PATH: Style = Style::new().bold();
pub const SYMLINK: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
pub const WRAPPER: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
pub const TERMINAL: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
pub const MUTED: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)))
    .dimmed();
