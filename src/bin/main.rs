use anyhow::{anyhow, Context};
use atty::Stream;
use rhc::args::Args;
use rhc::config::Config;
use rhc::environment::Environment;
use rhc::files::load_file;
use rhc::http;
use rhc::interactive;
use rhc::keyvalue::KeyValue;
use rhc::request_definition::RequestDefinition;
use rhc::templating;
use serde_json::{to_string_pretty, Value};
use spinners::{Spinner, Spinners};
use std::borrow::Cow;
use std::io::{Stdout, Write};
use std::path::Path;
use structopt::StructOpt;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use syntect::LoadingError;
use termion::input::{Keys, TermRead};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use termion::AsyncReader;
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

type OurTerminal = Terminal<TermionBackend<AlternateScreen<RawTerminal<Stdout>>>>;

/// Set up/create the terminal for use in interactive mode.
fn get_terminal() -> anyhow::Result<OurTerminal> {
    let stdout = std::io::stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let t = Terminal::new(backend)?;

    Ok(t)
}

fn run() -> anyhow::Result<()> {
    let args = Args::from_args();
    let config_location: Cow<str> = shellexpand::tilde("~/.config/rhc/config.toml");
    let config =
        Config::new(Path::new(config_location.as_ref())).context("Could not load config file")?;

    let is_tty = atty::is(Stream::Stdout);

    // These two are necessary for use in interactive mode; but conversely, when not at an
    // interactive shell, trying to create this `Terminal` will cause an error. So they start as
    // None, and will be created on-demand if necessary (no request definition file provided, or
    // unbound variables exist).
    let mut keys: Option<Keys<AsyncReader>> = None;
    let mut terminal: Option<Terminal<TermionBackend<AlternateScreen<RawTerminal<Stdout>>>>> = None;

    // If the user specified a request definition file, just use that; otherwise, enter interactive
    // mode to allow them to choose a request definition.
    let result: anyhow::Result<Option<(RequestDefinition, Vec<KeyValue>)>> = {
        match args.file {
            Some(path) => {
                let def: RequestDefinition =
                    load_file(&path, RequestDefinition::new, "request definition")?;
                let environment: Option<Environment> = args
                    .environment
                    .map(|path| load_file(&path, Environment::new, "environment"))
                    .transpose()?;

                let vars: Vec<KeyValue> = environment.map_or(vec![], |e| e.variables);

                Ok(Some((def, vars)))
            }
            None => {
                if is_tty {
                    // `terminal` and `keys` must be None at this point, so just create them
                    terminal = Some(get_terminal()?);
                    keys = Some(termion::async_stdin().keys());
                    let interactive_result = interactive::interactive_mode(
                        &config,
                        args.environment.as_deref(),
                        &mut keys.as_mut().unwrap(),
                        &mut terminal.as_mut().unwrap(),
                    )?;
                    Ok(interactive_result
                        .map(|(def, env)| (def, env.map_or(vec![], |e| e.variables))))
                } else {
                    Err(anyhow!("Running in interactive mode requires a TTY"))
                }
            }
        }
    };

    let result = result?;

    // TODO: add in variables specified via args

    // `interactive_mode` will return None if they Ctrl-C out without selecting anything.
    if let Some((mut request_definition, vars)) = result {
        // Substitute the variables that we have at this point into all the places of the
        // RequestDefinitions that they can be used (URL, headers, body, query string)
        templating::substitute_all(&mut request_definition, &vars);

        // // If any unbound variables remain, prompt the user to enter them interactively
        let unbound_variables = templating::list_unbound_variables(&request_definition);

        let additional_vars: anyhow::Result<Option<Vec<KeyValue>>> = {
            if !unbound_variables.is_empty() {
                if is_tty {
                    // `terminal` and `keys` could have been initialized above, so only initialize them
                    // here if necessary.
                    if keys.is_none() {
                        terminal = Some(get_terminal()?);
                        keys = Some(termion::async_stdin().keys());
                    }
                    interactive::prompt_for_variables(
                        &config,
                        unbound_variables,
                        &mut keys.as_mut().unwrap(),
                        &mut terminal.as_mut().unwrap(),
                    )
                } else {
                    Err(anyhow!("Running in interactive mode requires a TTY"))
                }
            } else {
                Ok(Some(vec![]))
            }
        };

        let additional_vars = additional_vars?;

        // Switch back to the original screen
        drop(terminal);

        // Flush stdout so the interactive terminal screen is cleared immediately
        std::io::stdout().flush().ok();

        // `prompt_for_variables` returning None means the user aborted with Ctrl-C and we
        // should not send the request
        if let Some(additional_vars) = additional_vars {
            // Do the final substition with the user-provided variables
            templating::substitute_all(&mut request_definition, &additional_vars);

            let mut sp: Option<Spinner> = None;
            if is_tty {
                sp = Some(Spinner::new(Spinners::Dots, "Sending request...".into()));
            }

            let res = http::send_request(request_definition).context("Failed sending request")?;
            sp.map(|s| s.stop());
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

                    value.contains("application/json")
                        || value.contains("text/json")
                        || value.contains("application/javascript")
                })
                .unwrap_or(false);

            if is_json {
                // If the content-type header on the response suggests that the response is JSON,
                // try to parse it as a generic Value, then pretty-print it with highlighting via
                // syntect. If the parsing fails, give up on the pretty-printing and just print the
                // raw text response (still with JSON highlighting, if possible)
                let body: Value = res.json()?;
                let body = to_string_pretty(&body).unwrap_or_else(|_| body.to_string());

                let ps = SyntaxSet::load_defaults_newlines();
                let syntax = ps.find_syntax_by_extension("json").unwrap();
                let ts = ThemeSet::load_defaults();

                // If the user has specified no theme in their config file, fall back to a default
                // included in syntect. If they specify a name of a default syntect theme, use
                // that. Otherwise, treat their provided value as a file path and try to load a
                // theme.
                let theme: Result<Cow<Theme>, LoadingError> = match config.theme.as_ref() {
                    None => Ok(Cow::Borrowed(&ts.themes["base16-eighties.dark"])),
                    Some(theme_file) => ts
                        .themes
                        .get(theme_file)
                        .map(|t| Ok(Cow::Borrowed(t)))
                        .unwrap_or_else(|| {
                            let expanded: Cow<str> = shellexpand::tilde(theme_file);
                            let path: &Path = Path::new(expanded.as_ref());
                            ThemeSet::get_theme(path).map(Cow::Owned)
                        }),
                };

                match theme {
                    Ok(theme) => {
                        let mut h = HighlightLines::new(syntax, theme.as_ref());
                        for line in LinesWithEndings::from(&body) {
                            let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
                            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                            print!("{}", escaped);
                        }
                        println!("");
                    }
                    Err(e) => {
                        eprintln!(
                            "Could not load theme at {}, continuing with no theme",
                            &config.theme.unwrap()
                        );
                        eprintln!("{}", e);

                        println!("{}", body);
                    }
                }
            } else {
                let body = res.text()?;
                println!("{}", body);
            }
        }
    }
    Ok(())
}
