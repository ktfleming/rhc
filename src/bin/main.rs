use anyhow::Context;
use clap::{App, Arg};
use rustrest::http;
use rustrest::request_definition::RequestDefinition;
use std::path::PathBuf;

fn main() {
    if let Err(e) = run() {
        eprintln!("{:#}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
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

    let request_definition = RequestDefinition::new(&path).with_context(|| {
        format!(
            "Failed to parse request definition file at {}",
            path.to_string_lossy()
        )
    })?;
    let res = http::send_request(request_definition).context("Failed sending request")?;
    println!("{}", res);
    Ok(())
}
