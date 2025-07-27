use clap::Parser;

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the root of the life directory
    #[clap(long, value_name = "PATH")]
    pub life_path: String,
}

pub fn parse_args() -> Args {
    Args::parse()
}
