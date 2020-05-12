use crate::keyvalue::KeyValue;
use crate::request_definition::RequestDefinition;
use crate::templating::substitute;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::path::PathBuf;
use tui::widgets::Text;

/// Items that appear in the interactive list that the user can select.
pub struct Choice {
    pub path: PathBuf,

    // The length of the common directory prefix that should be trimmed from the beginning of each
    // path for display/search purposes
    pub prefix_length: usize,

    pub request_definition: Option<anyhow::Result<RequestDefinition>>,
}

// Ord, etc. needed for sorting
impl Ord for Choice {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl PartialOrd for Choice {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Choice {}

impl PartialEq for Choice {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Choice {
    pub fn new(path: PathBuf, prefix_length: usize) -> Choice {
        Choice {
            path,
            request_definition: None,
            prefix_length,
        }
    }

    // Used for displaying and as a target for searching
    pub fn url_or_blank<'a>(&'a self, variables: Option<&'a Vec<KeyValue>>) -> Cow<'a, str> {
        match &self.request_definition {
            Some(Ok(request_definition)) => {
                let initial_url = &request_definition.request.url;
                variables
                    .map(|vars| substitute(initial_url, vars).0)
                    .unwrap_or(Cow::Borrowed(initial_url))
            }
            _ => "".into(),
        }
    }

    pub fn description_or_blank(&self) -> &str {
        match &self.request_definition {
            Some(Ok(def)) => def.metadata.as_ref().map_or("", |m| &m.description),
            _ => "",
        }
    }

    // Also used for displaying/searching. The full path with the common prefix trimmed off the
    // beginning, and the ".toml" extension trimmed from the end
    pub fn trimmed_path(&self) -> String {
        let path_str = &self.path.to_string_lossy();
        path_str[(self.prefix_length + 1)..(path_str.len() - 5)].to_owned()
    }

    pub fn to_text_widget(&self, variables: Option<&Vec<KeyValue>>) -> Text {
        let path = self.trimmed_path();

        match &self.request_definition {
            None => Text::raw(path),
            Some(Ok(def)) => {
                let url = self.url_or_blank(variables);
                if let Some(metadata) = &def.metadata {
                    Text::raw(format!(
                        "{}  |  {}  |  {}",
                        path, url, &metadata.description
                    ))
                } else {
                    Text::raw(format!("{}  |  {}", path, url))
                }
            }
            Some(Err(_)) => {
                let right_part = "(Could not parse definition file)";
                Text::raw(format!("{}  |  {}", path, right_part))
            }
        }
    }
}
