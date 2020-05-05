use crate::request_definition::RequestDefinition;
use pad::{Alignment, PadStr};
use std::cmp::Ordering;
use std::path::PathBuf;
use tui::widgets::Text;
use unicode_width::UnicodeWidthStr;

/// Items that appear in the interactive list that the user can select.
pub struct Choice {
    pub path: PathBuf,
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
    pub fn new(path: PathBuf) -> Choice {
        Choice {
            path,
            request_definition: None,
        }
    }

    pub fn get_url_or_blank(&self) -> &str {
        match &self.request_definition {
            Some(Ok(request_definition)) => &request_definition.request.url,
            _ => "",
        }
    }

    pub fn to_text_widget(&self, width: usize) -> Text {
        let path = self.path.to_string_lossy();

        // Width of everything past the path, needs to be padded
        let right_part_width = width - path.width();

        match &self.request_definition {
            None => Text::raw(path),
            Some(Ok(_)) => {
                let url = self.get_url_or_blank();
                let right_part =
                    url.pad_to_width_with_alignment(right_part_width, Alignment::Right);
                Text::raw(format!("{}{}", path, right_part))
            }
            Some(Err(_)) => {
                let right_part = "(Could not parse definition file)"
                    .pad_to_width_with_alignment(right_part_width, Alignment::Right);
                Text::raw(format!("{}{}", path, right_part))
            }
        }
    }
}
