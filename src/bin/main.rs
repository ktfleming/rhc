use anyhow::Context;
use clap::{App, Arg};
use rustrest::environment::Environment;
use rustrest::files::load_file;
use rustrest::http;
use rustrest::interactive;
use rustrest::request_definition::RequestDefinition;
use std::path::Path;

fn main() {
    if let Err(e) = run() {
        eprintln!("{:#}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let matches = App::new("rustrest")
        .arg(
            Arg::with_name("file")
                .short("f")
                .long("file")
                .help("The request definition file to use")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("environment")
                .short("e")
                .long("environment")
                .help("The environment file to use")
                .takes_value(true),
        )
        .get_matches();

    let request_definition = matches.value_of("file").map(|path| {
        load_file(
            Path::new(path),
            RequestDefinition::new,
            "request definition",
        )
        .unwrap()
    });

    let env = matches
        .value_of("environment")
        .map(|path| load_file(Path::new(path), Environment::new, "environment").unwrap());

    // If a request definition file was provided, just send that one request.
    // Otherwise, enter interactive mode.
    match request_definition {
        Some(request_definition) => {
            let res = http::send_request(
                request_definition,
                env.as_ref().map(|e| &e.variables).unwrap_or(&vec![]),
            )
            .context("Failed sending request")?;
            println!("{}", res);
            Ok(())
        }
        None => {
            interactive::interactive_mode()?;
            Ok(())
        }
    }
}
