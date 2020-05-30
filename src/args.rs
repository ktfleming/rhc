use crate::keyvalue::KeyValue;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "rhc")]
pub struct Args {
    #[structopt(
        short,
        long,
        parse(from_os_str),
        help = "The request definition file to use"
    )]
    pub file: Option<PathBuf>,

    #[structopt(short, long, parse(from_os_str), help = "The environment file to use")]
    pub environment: Option<PathBuf>,

    #[structopt(short, long, help = "Only print the response body to stdout")]
    pub only_body: bool,

    #[structopt(
        short,
        long,
        help = "Bindings to use when constructing the request. Example: -b key=value"
    )]
    pub binding: Option<Vec<KeyValue>>,

    #[structopt(short, long, help = "The config file to use")]
    pub config: Option<PathBuf>,

    #[structopt(short, long, help = "Print more detailed information")]
    pub verbose: bool,
}
