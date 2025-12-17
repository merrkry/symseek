use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "symseek")]
#[command(version, about, long_about = None)]
pub struct Args {
    pub target: String,
}

impl Args {
    pub fn parse() -> Self {
        <Args as Parser>::parse()
    }
}
