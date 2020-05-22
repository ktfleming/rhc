use crate::choice::Choice;
use crate::config::Config;
use crate::environment::Environment;
use anyhow::Context;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Try to load the appropriate struct from the provided file path, if present, and fail with a
/// helpful error message if necessary.
pub fn load_file<T, F>(path: &Path, loader: F, file_desc: &str) -> anyhow::Result<T>
where
    F: Fn(&Path) -> anyhow::Result<T>,
{
    loader(path).with_context(|| {
        format!(
            "Failed to parse {} file at {}",
            file_desc,
            path.to_string_lossy()
        )
    })
}

fn get_all_toml_files(dir: &str) -> Vec<PathBuf> {
    let def_directory = shellexpand::tilde(dir);
    WalkDir::new(Path::new(def_directory.as_ref()))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|path| path.to_string_lossy().ends_with(".toml"))
        .collect()
}

/// Try to load all request definitions from TOML files under the base definitions directory. If
/// any can't be parsed, return an error to display to the user; this way any malformed TOML files
/// won't cause the whole program to be unusable.
pub fn list_all_choices(config: &Config) -> Vec<Choice> {
    let prefix_length = shellexpand::tilde(&config.request_definition_directory).len();
    let mut choices: Vec<Choice> = get_all_toml_files(&config.request_definition_directory)
        .into_iter()
        .map(|path| Choice::new(path, prefix_length))
        .collect();

    choices.sort();

    choices
}

/// Try to load all environments from TOML files under the base environments directory. If any are
/// invalid, display a message on stderr and don't allow them to be used.
pub fn list_all_environments(config: &Config) -> Vec<(Environment, PathBuf)> {
    get_all_toml_files(&config.environment_directory)
        .into_iter()
        .filter_map(|path| match Environment::new(&path) {
            Ok(env) => Some((env, path)),
            Err(err) => {
                eprintln!("{}", err);
                None
            }
        })
        .collect()
}
