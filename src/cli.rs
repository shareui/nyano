use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "nyano", version, about, long_about = None)]
pub struct Cli {
    pub path: String,
    #[arg(short = 'c', long = "create")]
    pub create: bool,
    #[arg(short = 'p', long = "parent", requires = "create")]
    pub parent: bool,
    #[arg(short = 'r', long = "read")]
    pub read_only: bool,
    #[arg(short = 'b', long = "backup")]
    pub backup: bool,
}
