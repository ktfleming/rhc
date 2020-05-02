use clap::{App, Arg};
use rustrest::http;
use rustrest::request_definition::RequestDefinition;
use std::path::PathBuf;
use std::process;

fn main() {
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

    let request_definition = RequestDefinition::new(&path);

    match request_definition {
        Ok(request_definition) => {
            // TODO: handle this error
            let res = http::send_request(request_definition).unwrap();
            println!("{}", res);
            process::exit(0);
        }
        Err(e) => {
            eprintln!(
                "Error parsing request definition at {}\n{}",
                path.to_string_lossy(),
                e
            );
            process::exit(1);
        }
    }
}
