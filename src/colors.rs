use crate::config::CustomColors;
use lazy_static::lazy_static;
use regex::Regex;
use std::num::ParseIntError;
use tui::style::Color;

/// The final set of colors that will be used in interactive mode. Created by merging the user's
/// provided colors (if any) with the default colors.
pub struct Colors {
    /// Foreground color for interactive choices that aren't selected
    pub default_fg: Option<Color>,

    /// Background color for interactive choices that aren't selected
    pub default_bg: Option<Color>,

    /// Foreground color for interactive choices that are selected
    pub selected_fg: Color,

    /// Background color for interactive choices that are selected
    pub selected_bg: Option<Color>,

    /// Foreground color for the prompt
    pub prompt_fg: Color,

    /// Background color for the prompt
    pub prompt_bg: Option<Color>,

    /// Foreground color for unbound variables
    pub variable_fg: Color,

    /// Background color for unbound variables
    pub variable_bg: Option<Color>,
}

impl From<&Option<CustomColors>> for Colors {
    fn from(custom_colors: &Option<CustomColors>) -> Self {
        fn parse_or_default(s: Option<&str>, default: Color) -> Color {
            match s {
                Some(s) => parse_color(s).unwrap_or(default),
                None => default,
            }
        }

        fn parse_or_none(s: Option<&str>) -> Option<Color> {
            s.and_then(|s| parse_color(s))
        }

        let custom_colors = custom_colors.as_ref();

        Colors {
            default_fg: parse_or_none(
                custom_colors.and_then(|c| c.default_fg.as_deref()),
            ),
            default_bg: parse_or_none(
                custom_colors.and_then(|c| c.default_bg.as_deref()),
            ),
            selected_fg: parse_or_default(
                custom_colors.and_then(|c| c.selected_fg.as_deref()),
                Color::Green,
            ),
            selected_bg: parse_or_none(
                custom_colors.and_then(|c| c.selected_bg.as_deref()),
            ),
            prompt_fg: parse_or_default(
                custom_colors.and_then(|c| c.prompt_fg.as_deref()),
                Color::LightMagenta,
            ),
            prompt_bg: parse_or_none(
                custom_colors.and_then(|c| c.prompt_bg.as_deref()),
            ),
            variable_fg: parse_or_default(
                custom_colors.and_then(|c| c.variable_fg.as_deref()),
                Color::LightMagenta,
            ),
            variable_bg: parse_or_none(
                custom_colors.and_then(|c| c.variable_bg.as_deref())
            ),
        }
    }
}

lazy_static! {
    static ref RGB_RE: Regex = Regex::new(r"rgb\((\d+),\s?(\d+),\s?(\d+)\)").unwrap();
    static ref INDEXED_RE: Regex = Regex::new(r"indexed\((\d+)\)").unwrap();
}

fn parse_color(input: &str) -> Option<Color> {
    let lowered = input.to_lowercase();
    let lowered = lowered.as_str();
    match lowered {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" => Some(Color::Gray),
        "darkgray" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ => match RGB_RE.captures(lowered) {
            Some(cap) => match (as_u8(&cap[1]), as_u8(&cap[2]), as_u8(&cap[3])) {
                (Ok(r), Ok(g), Ok(b)) => Some(Color::Rgb(r, g, b)),
                _ => {
                    eprintln!("Could not parse `{}` as an RGB color", input);
                    None
                }
            },
            None => INDEXED_RE
                .captures(lowered)
                .and_then(|cap| match as_u8(&cap[1]) {
                    Ok(i) => Some(Color::Indexed(i)),
                    _ => {
                        eprintln!("Could not parse `{}` as an indexed color", input);
                        None
                    }
                }),
        },
    }
}

fn as_u8(s: &str) -> Result<u8, ParseIntError> {
    s.parse()
}
