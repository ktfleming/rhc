use crate::choice::Choice;
use anyhow::Context;
use std::path::Path;
use walkdir::WalkDir;

/// Try to load the appropriate struct from the provided file path,
/// if present, and fail with a helpful error message if necessary.
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

/// Try to load all request definitions from TOML files under the
/// base definitions directory. If any can't be parsed, return an
/// error to display to the user; this way any malformed TOML files
/// won't cause the whole program to be unusable.
pub fn list_all_choices() -> Vec<Choice> {
    // First, just load all the paths
    let mut choices: Vec<Choice> = WalkDir::new("test_definitions")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|path| path.to_string_lossy().ends_with(".toml"))
        .map(Choice::new)
        .collect();

    choices.sort();

    choices
}
