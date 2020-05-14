use anyhow::Context;
use rhc::args::Args;
use rhc::config::Config;
use rhc::environment::Environment;
use rhc::files::load_file;
use rhc::http;
use rhc::interactive;
use rhc::keyvalue::KeyValue;
use rhc::request_definition::RequestDefinition;
use rhc::templating;
use spinners::{Spinner, Spinners};
use std::borrow::Cow;
use std::io::Write;
use std::path::Path;
use structopt::StructOpt;
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
    let args = Args::from_args();
    let config_location: Cow<str> = shellexpand::tilde("~/.config/rhc/config.toml");
    let config =
        Config::new(Path::new(config_location.as_ref())).context("Could not load config file")?;

    // If term_tools is None (due to the --no-interactive flag), the interactive functions will be
    // skipped and we'll act like they just returned None.
    let mut term_tools = if args.no_interactive {
        None
    } else {
        // Use the same async_stdin iterator and terminal for all interactive prompts to make everything smooth.
        let stdout = std::io::stdout().into_raw_mode()?;
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let stdin = termion::async_stdin().keys();

        Some((stdin, terminal))
    };

    // If the user specified a request definition file, just use that; otherwise, enter interactive
    // mode to allow them to choose a request definition.
    let result: Option<(RequestDefinition, Vec<KeyValue>)> = {
        match (args.file, &mut term_tools) {
            (Some(path), _) => {
                let def: RequestDefinition =
                    load_file(&path, RequestDefinition::new, "request definition")?;
                let environment: Option<Environment> = args
                    .environment
                    .map(|path| load_file(&path, Environment::new, "environment"))
                    .transpose()?;

                let vars: Vec<KeyValue> = environment.map_or(vec![], |e| e.variables);

                Some((def, vars))
            }
            (None, Some((ref mut stdin, ref mut terminal))) => {
                let interactive_result = interactive::interactive_mode(
                    &config,
                    args.environment.as_deref(),
                    stdin,
                    terminal,
                )?;
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
            if !unbound_variables.is_empty() {
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

            let sp = Spinner::new(Spinners::Dots, "Sending request...".into());

            let res = http::send_request(request_definition).context("Failed sending request")?;
            sp.stop();
            println!("\n");
            println!("{}\n", res.status());
            let headers = res.headers();
            for (name, value) in headers {
                let value = value.to_str()?;
                println!("{}: {}", name.as_str(), value);
            }

            println!("\n");

            let is_json = headers
                .get("content-type")
                .map(|h| {
                    let value = h.to_str().unwrap_or("");

                    value == "application/json"
                        || value == "text/json"
                        || value == "application/javascript"
                })
                .unwrap_or(false);

            let body = res.text()?;

            if is_json {
                // TODO: color the JSON
            }
            println!("{}", body);
        }
    }
    Ok(())
}
