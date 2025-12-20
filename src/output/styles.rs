pub struct TreeChars {
    pub branch: &'static str,
    pub last: &'static str,
    pub connector: &'static str,
}

impl Default for TreeChars {
    fn default() -> Self {
        Self {
            branch: "├",
            last: "└",
            connector: "─",
        }
    }
}
