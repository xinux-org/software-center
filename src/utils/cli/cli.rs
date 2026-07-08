use clap::Parser;

use crate::utils::cli::scheme::{Scheme, SchemeParser};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(value_parser=SchemeParser, name="URI")]
    pub scheme: Option<Scheme>,
}

pub fn parse_cli() -> Cli {
    Cli::parse()
}
