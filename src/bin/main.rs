use clap::{App, Arg};
use failure::Error;
use rustrest::http;
use rustrest::request_definition::RequestDefinition;
use std::path::PathBuf;

fn main() -> Result<(), Error> {
    let matches = App::new("rustrest")
        .arg(
            Arg::with_name("FILE")
                .help("The request definition file to use")
                .required(true)
                .index(1),
        )
        .get_matches();

    let path = matches.value_of("FILE").unwrap();
    let path = PathBuf::from(path);

    let request_definition = RequestDefinition::new(&path)?;

    let res = http::send_request(&request_definition)?;

    println!("{}", res);

    Ok(())
}
