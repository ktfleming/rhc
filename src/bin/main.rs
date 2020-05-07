use anyhow::Context;
use clap::{App, Arg};
use rustrest::config::Config;
use rustrest::environment::Environment;
use rustrest::files::load_file;
use rustrest::http;
use rustrest::interactive;
use rustrest::request_definition::RequestDefinition;
use std::borrow::Cow;
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

    let config_location: Cow<str> = shellexpand::tilde("~/.config/rustrest/config.toml");
    let config =
        Config::new(Path::new(config_location.as_ref())).context("Could not load config file")?;

    let request_definition = matches.value_of("file").map(|path| {
        load_file(
            Path::new(path),
            RequestDefinition::new,
            "request definition",
        )
        .unwrap()
    });

    let env_arg = matches.value_of("environment");

    // If a request definition file was provided, just send that one request.
    // Otherwise, enter interactive mode.
    match request_definition {
        Some(request_definition) => {
            let env = env_arg
                .map(|path| load_file(Path::new(path), Environment::new, "environment").unwrap());

            let res = http::send_request(
                request_definition,
                env.as_ref().map(|e| &e.variables).unwrap_or(&vec![]),
            )
            .context("Failed sending request")?;
            println!("{}", res);
            Ok(())
        }
        None => {
            interactive::interactive_mode(&config, env_arg)?;
            Ok(())
        }
    }
}
