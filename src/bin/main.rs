use anyhow::Context;
use clap::{App, Arg};
use rustrest::config::Config;
use rustrest::environment::Environment;
use rustrest::files::load_file;
use rustrest::http;
use rustrest::interactive;
use rustrest::keyvalue::KeyValue;
use rustrest::request_definition::RequestDefinition;
use rustrest::templating;
use std::borrow::Cow;
use std::io::Write;
use std::path::Path;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::Terminal;

fn main() {
    if let Err(e) = run() {
        // If an error was raised during an interactive mode call while the alternate screen is in
        // use, we have to flush stdout here or the user will not see the error message.
        std::io::stdout().flush().unwrap();
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
        .arg(Arg::with_name("no_interactive").long("no-interactive"))
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

    // If this flag is set, the termion Terminal will never be allocated and interactive mode is
    // not possible. Mostly to work around problems doing integration tests, where it would crash
    // with "Inappropriate ioctl for device" as soon as the Terminal was allocated.
    let no_interactive = matches.is_present("no_interactive");

    // If term_tools is None (due to the --no-interactive flag), the interactive functions will be
    // skipped and we'll act like they just returned None.
    let mut term_tools = match no_interactive {
        false => {
            // Use the same async_stdin iterator and terminal for all interactive prompts to make everything smooth.
            let stdout = std::io::stdout().into_raw_mode()?;
            let stdout = AlternateScreen::from(stdout);
            let backend = TermionBackend::new(stdout);
            let terminal = Terminal::new(backend)?;
            let stdin = termion::async_stdin().keys();

            Some((stdin, terminal))
        }
        true => None,
    };

    // If the user specified a request definition file, just use that; otherwise, enter interactive
    // mode to allow them to choose a request definition.
    let result: Option<(RequestDefinition, Vec<KeyValue>)> = {
        match (request_definition_arg, &mut term_tools) {
            (Some(path), _) => {
                let def: RequestDefinition = load_file(
                    Path::new(path),
                    RequestDefinition::new,
                    "request definition",
                )?;
                let vars: Vec<KeyValue> = environment.map_or(vec![], |e| e.variables);

                Some((def, vars))
            }
            (None, Some((ref mut stdin, ref mut terminal))) => {
                let interactive_result =
                    interactive::interactive_mode(&config, env_arg, stdin, terminal)?;
                interactive_result.map(|(def, env)| (def, env.map_or(vec![], |e| e.variables)))
            }
            (None, None) => None,
        }
    };

    // TODO: add in variables specified via args

    // `interactive_mode` will return None if they Ctrl-C out without selecting anything.
    if let Some((mut request_definition, vars)) = result {
        // Substitute the variables that we have at this point into all the places of the
        // RequestDefinitions that they can be used (URL, headers, body, query string)
        templating::substitute_all(&mut request_definition, &vars);

        // // If any unbound variables remain, prompt the user to enter them interactively
        let unbound_variables = templating::list_unbound_variables(&request_definition);

        let additional_vars: Option<Vec<KeyValue>> = {
            if unbound_variables.len() > 0 {
                match &mut term_tools {
                    Some((ref mut stdin, ref mut terminal)) => interactive::prompt_for_variables(
                        &config,
                        unbound_variables,
                        stdin,
                        terminal,
                    )?,
                    None => Some(vec![]),
                }
            } else {
                Some(vec![])
            }
        };

        // Switch back to the original screen
        drop(term_tools);

        // Flush stdout so the interactive terminal screen is cleared immediately
        std::io::stdout().flush().ok();

        // `prompt_for_variables` returning None means the user aborted with Ctrl-C and we
        // should not send the request
        if let Some(additional_vars) = additional_vars {
            // Do the final substition with the user-provided variables
            templating::substitute_all(&mut request_definition, &additional_vars);

            let res = http::send_request(request_definition).context("Failed sending request")?;
            println!("{}", res);
        }
    }
    Ok(())
}
