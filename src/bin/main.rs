use anyhow::Context;
use clap::{App, Arg};
use rustrest::config::Config;
use rustrest::environment::Environment;
use rustrest::files::load_file;
use rustrest::http;
use rustrest::interactive;
use rustrest::keyvalue::KeyValue;
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

    let request_definition_arg = matches.value_of("file");
    let env_arg = matches.value_of("environment");

    // The environment arg can be applied to both interactive and non-interactive modes, so we
    // might as well load the specified environment now.
    let environment: Option<Environment> = env_arg
        .map(|path| load_file(Path::new(path), Environment::new, "environment"))
        .transpose()?;

    // If the user specified a request definition file, just use that; otherwise, enter interactive
    // mode to allow them to choose a request definition.
    let result: Option<(RequestDefinition, Vec<KeyValue>)> = {
        match request_definition_arg {
            Some(path) => {
                let def: RequestDefinition = load_file(
                    Path::new(path),
                    RequestDefinition::new,
                    "request definition",
                )?;
                let vars: Vec<KeyValue> = environment.map_or(vec![], |e| e.variables);

                Some((def, vars))
            }
            None => {
                let interactive_result = interactive::interactive_mode(&config, env_arg)?;
                interactive_result.map(|(def, env)| (def, env.map_or(vec![], |e| e.variables)))
            }
        }
    };

    // TODO: add in variables specified via args

    // I think normally interactive_mode should always return Some, but just in case, don't do
    // anything if it somehow returns None. It returns an Option because the `primed` path is
    // initialized as None, to account for the case where there are no request definition files to
    // load. However, in the interactive_mode logic, it doesn't let them break from the UI loop
    // without selecting an entry.
    if let Some((request_definition, vars)) = result {
        let res =
            http::send_request(request_definition, &vars[..]).context("Failed sending request")?;
        println!("{}", res);
    }
    Ok(())
}
