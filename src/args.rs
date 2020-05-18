use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "rhc")]
pub struct Args {
    #[structopt(short, long, parse(from_os_str))]
    pub file: Option<PathBuf>,

    #[structopt(short, long, parse(from_os_str))]
    pub environment: Option<PathBuf>,

    #[structopt(short, long)]
    pub verbose: bool,
}
