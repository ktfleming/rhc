use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "rhc")]
pub struct Args {
    #[structopt(short, long, parse(from_os_str))]
    pub file: Option<PathBuf>,

    #[structopt(short, long, parse(from_os_str))]
    pub environment: Option<PathBuf>,

    // If this flag is set, the termion Terminal will never be allocated and interactive mode is
    // not possible. Mostly to work around problems doing integration tests, where it would crash
    // with "Inappropriate ioctl for device" as soon as the Terminal was allocated.
    #[structopt(long)]
    pub no_interactive: bool,
}
