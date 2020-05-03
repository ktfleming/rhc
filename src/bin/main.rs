use anyhow::Context;
use clap::{App, Arg};
use rustrest::environment::Environment;
use rustrest::http;
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
            Arg::with_name("FILE")
                .help("The request definition file to use")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("environment")
                .short("e")
                .long("environment")
                .help("The environment file to use")
                .takes_value(true),
        )
        .get_matches();

    let def_path = matches.value_of("FILE").unwrap();
    let def_path = Path::new(def_path);

    let request_definition = RequestDefinition::new(def_path).with_context(|| {
        format!(
            "Failed to parse request definition file at {}",
            def_path.to_string_lossy()
        )
    })?;

    let env: anyhow::Result<Option<Environment>> =
        matches
            .value_of("environment")
            .map_or(Ok(None), |env_path| {
                let env_path = Path::new(env_path);

                Environment::new(env_path)
                    .with_context(|| {
                        format!(
                            "Failed to parse environment file at {}",
                            env_path.to_string_lossy()
                        )
                    })
                    .map(Some)
            });

    let env = env?;

    let res = http::send_request(
        request_definition,
        env.as_ref().map(|e| &e.variables).unwrap_or(&vec![]),
    )
    .context("Failed sending request")?;
    println!("{}", res);
    Ok(())
}
