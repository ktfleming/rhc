use anyhow::{anyhow, Context};
use atty::Stream;
use rhc::args::Args;
use rhc::config::Config;
use rhc::environment::Environment;
use rhc::files::{get_all_toml_files, load_file};
use rhc::http;
use rhc::interactive;
use rhc::interactive::SelectedValues;
use rhc::keyvalue::KeyValue;
use rhc::request_definition::RequestDefinition;
use rhc::templating;
use serde_json::{to_string_pretty, Value};
use spinners::{Spinner, Spinners};
use std::borrow::Cow;
use std::env;
use std::io::{Stdout, Write};
use std::path::Path;
use std::path::PathBuf;
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
// use simplelog::{CombinedLogger, WriteLogger, LevelFilter, Config as LogConfig};
// use std::fs::File;

fn main() {
    if let Err(e) = run() {
        // If an error was raised during an interactive mode call while the alternate screen is in
        // use, we have to flush stdout here or the user will not see the error message.
        std::io::stdout().flush().unwrap();

        // Seems like this initial newline is necessary or the error will be printed with an offset
        eprintln!("\nError: {:#}", e);
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
    // CombinedLogger::init(
    //     vec![
    //         WriteLogger::new(LevelFilter::Debug, LogConfig::default(), File::create("rhc.log").unwrap()),
    //     ]
    // ).unwrap();
    let args: Args = Args::from_args();

    // If the user specifies a config location, make sure there's actually a file there
    args.config.as_ref().map_or(Ok(()), |c| {
        if c.is_file() {
            Ok(())
        } else {
            Err(anyhow!("No config file found at `{}`", c.to_string_lossy()))
        }
    })?;

    // Load the config file using this priority:
    // 1. The file specified with the --config arg, if present
    // 2. $XDG_CONFIG_HOME/rhc/config.toml, if XDG_CONFIG_HOME is defined
    // 3. ~/.config/rhc/config.toml, if present
    // If none of the above exist, use the default Config.
    let raw_config_location: PathBuf = args.config.unwrap_or_else(|| {
        match env::var_os("XDG_CONFIG_HOME") {
            Some(xdg_config_home) => PathBuf::from(xdg_config_home),
            None => PathBuf::from("~/.config"),
        }
        .join("rhc")
        .join("config.toml")
    });

    let raw_config_location = raw_config_location.to_string_lossy();
    let config_location: Cow<str> = shellexpand::tilde(raw_config_location.as_ref());
    let config_path = Path::new(config_location.as_ref());

    if args.verbose {
        println!("Looking for config file at {}", config_path.display());
    }

    let config = {
        if config_path.is_file() {
            Config::new(config_path).context(format!(
                "Could not load config file at {}",
                config_path.to_string_lossy()
            ))?
        } else {
            println!(
                "No config file found at {}, falling back to default config",
                config_path.display()
            );
            Config::default()
        }
    };

    let is_tty = atty::is(Stream::Stdout);

    // These two are necessary for use in interactive mode; but conversely, when not at an
    // interactive shell, trying to create this `Terminal` will cause an error. So they start as
    // None, and will be created on-demand if necessary (no request definition file provided, or
    // unbound variables exist).
    let mut keys: Option<Keys<AsyncReader>> = None;
    let mut terminal: Option<OurTerminal> = None;

    // If the user specified a request definition file, just use that; otherwise, enter interactive
    // mode to allow them to choose a request definition. In either case, we need to keep track of
    // the file names for the request definition that's either provided or selected, as well as the
    // environment being used (if any), as these are required for the prompt_for_variables
    // function.

    let result: anyhow::Result<Option<SelectedValues>> = {
        match &args.file {
            Some(path) => {
                let def: RequestDefinition =
                    load_file(&path, RequestDefinition::new, "request definition")?;
                let env_path: Option<PathBuf> = args.environment;
                let env: Option<Environment> = env_path
                    .as_deref()
                    .map(|path| load_file(&path, Environment::new, "environment"))
                    .transpose()?;

                Ok(Some(SelectedValues { def, env }))
            }
            None => {
                if is_tty {
                    // If we have to enter interactive mode, check if there is at least one request
                    // definition file available. If not, there's nothing that can be done, so
                    // print a warning and exit.
                    if get_all_toml_files(&config.request_definition_directory).is_empty() {
                        Err(anyhow!("No TOML files found under {}. Running rhc in interactive mode requres at least one request definition file.", &config.request_definition_directory))
                    } else {
                        // `terminal` and `keys` must be None at this point, so just create them
                        terminal = Some(get_terminal()?);
                        keys = Some(termion::async_stdin().keys());
                        let interactive_result = interactive::interactive_mode(
                            &config,
                            args.environment.as_deref(),
                            &mut keys.as_mut().unwrap(),
                            &mut terminal.as_mut().unwrap(),
                        )?;

                        Ok(interactive_result)
                    }
                } else {
                    Err(anyhow!("Running in interactive mode requires a TTY"))
                }
            }
        }
    };

    let result = result?;

    // `interactive_mode` will return None if they Ctrl-C out without selecting anything.
    // if let Some((mut request_definition, mut vars)) = result {
    if let Some(SelectedValues { mut def, env }) = result {
        // Split up the variables and environment name immediately to avoid difficulties with borrowing
        // `env` later on
        let (mut vars, env_name): (Vec<KeyValue>, String) =
            env.map_or((vec![], "<none>".to_string()), |e| (e.variables, e.name));

        vars.sort();
        if let Some(bindings) = args.binding {
            for binding in bindings {
                match vars.binary_search_by(|item| item.name.cmp(&binding.name)) {
                    Ok(index) => {
                        // If variable is already present, overwrite it with the one passed on the
                        // command line (these have the highest priority)
                        vars.remove(index);
                        vars.insert(index, binding);
                    }
                    Err(index) => vars.insert(index, binding),
                };
            }
        }

        // Substitute the variables that we have at this point into all the places of the
        // RequestDefinitions that they can be used (URL, headers, body, query string)
        templating::substitute_all(&mut def, &vars);

        // // If any unbound variables remain, prompt the user to enter them interactively
        let unbound_variables = templating::list_unbound_variables(&def);

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
                        &env_name,
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

        // Switch back to the original screen
        drop(terminal);

        // Flush stdout so the interactive terminal screen is cleared immediately
        std::io::stdout().flush().ok();

        let additional_vars = additional_vars?;

        // `prompt_for_variables` returning None means the user aborted with Ctrl-C and we
        // should not send the request
        if let Some(additional_vars) = additional_vars {
            // Do the final substition with the user-provided variables
            templating::substitute_all(&mut def, &additional_vars);

            let mut sp: Option<Spinner> = None;
            if is_tty {
                sp = Some(Spinner::new(Spinners::Dots, "Sending request...".into()));
            }

            let res = http::send_request(def, &config).context("Failed sending request")?;
            if let Some(s) = sp {
                s.stop();
                println!("\n");
            }

            let headers = res.headers();

            if !(&args.only_body) {
                println!("{}\n", res.status());
                for (name, value) in headers {
                    let value = value.to_str()?;
                    println!("{}: {}", name.as_str(), value);
                }

                println!();
            }

            let is_json = headers
                .get("content-type")
                .map(|h| {
                    let value = h.to_str().unwrap_or("");

                    value.contains("application/json")
                        || value.contains("text/json")
                        || value.contains("application/javascript")
                })
                .unwrap_or(false);

            if is_json && is_tty {
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
                        println!();
                    }
                    Err(e) => {
                        eprintln!(
                            "Error: Could not load theme at {}: {}, continuing with no theme",
                            &config.theme.unwrap(),
                            e
                        );

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
