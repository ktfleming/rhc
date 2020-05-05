use crate::choice::Choice;
use crate::files;
use crate::request_definition::RequestDefinition;
use anyhow::Context;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use termion::cursor::Goto;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{List, ListState, Paragraph, Text};
use tui::Terminal;
use unicode_width::UnicodeWidthStr;

struct InteractiveState {
    /// What the user has entered into the input buffer
    query: String,

    /// Holds which item is selected
    list_state: ListState,

    // When exiting the UI loop, if this is Some, that Choice
    // will have its request sent.
    primed: Option<PathBuf>,
}

impl InteractiveState {
    fn new() -> InteractiveState {
        InteractiveState {
            query: String::new(),
            list_state: ListState::default(),
            primed: None,
        }
    }

    pub fn append_to_query(&mut self, to_append: char) {
        self.query.push(to_append);
    }

    pub fn backspace(&mut self) {
        self.query.pop();
    }

    /// Like readline Ctrl-W
    pub fn cut_to_current_word_start(&mut self) {
        let mut cut_a_letter = false;
        while !self.query.is_empty() {
            let popped = self.query.pop();
            if let Some(' ') = popped {
                if cut_a_letter {
                    self.query.push(' ');
                    break;
                }
            } else {
                cut_a_letter = true;
            }
        }
    }

    pub fn clear_query(&mut self) {
        self.query.clear();
    }
}

pub fn interactive_mode() -> anyhow::Result<()> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut stdin = termion::async_stdin().keys();

    // This Vec<Choice> serves as the source-of-truth that will be filtered on and eventually
    // selected from. Initially only Paths are populated in the Choice structs, and the associated
    // RequestDefinition is not present.  The main UI loop accesses it through the read mode of the
    // RwLock and only uses it to display the List widget. Another thread is spawned to parse each
    // path and update the Choice's request_definition field via the write mode of the RwLock.
    let all_choices = Arc::new(RwLock::new(files::list_all_choices()));

    // Using the fuzzy matcher from the `skim` app/crate
    let matcher = SkimMatcherV2::default();

    let mut app_state = InteractiveState::new();

    let num_choices = all_choices.read().unwrap().len();
    if num_choices > 0 {
        app_state.list_state.select(Some(0));
    }

    let highlight_symbol = ">> ";

    let write_access = Arc::clone(&all_choices);
    std::thread::spawn(move || {
        for i in 0..num_choices {
            let mut writer = write_access.write().unwrap();

            // Try to load the RequestDefinition, and put the Result, whether Ok or Err, into the
            // Choice
            let request_definition: anyhow::Result<RequestDefinition> = files::load_file(
                &writer[i].path,
                RequestDefinition::new,
                "request definition",
            );
            writer[i].request_definition = Some(request_definition);
        }
    });

    // Default styling for the list
    let style = Style::default().fg(Color::Black).bg(Color::White);
    let highlight_style = style
        .fg(Color::Black)
        .bg(Color::LightGreen)
        .modifier(Modifier::BOLD);

    loop {
        // Needed to prevent cursor flicker when navigating the list
        io::stdout().flush().ok();

        // Inside this loop we only need read access to the Vec<Choice>
        let inner_guard = Arc::clone(&all_choices);
        let inner_guard = inner_guard.read().unwrap();

        // Use fuzzy matching on the Choices' path, and name if present
        let mut matching_choices: Vec<(i64, &Choice)> = inner_guard
            .iter()
            .filter_map(|choice| {
                let target = format!(
                    "{}{}",
                    &choice.path.to_string_lossy(),
                    &choice.get_url_or_blank()
                );
                let score = matcher.fuzzy_match(&target, &app_state.query);
                score.map(|score| (score, choice))
            })
            .collect();

        // We want to sort descending so the Choice with the highest score is as position 0
        matching_choices.sort_unstable_by(|(score1, _), (score2, _)| score2.cmp(score1));
        let final_matching_choices: Vec<&Choice> =
            matching_choices.iter().map(|(_, choice)| *choice).collect();

        if final_matching_choices.is_empty() {
            // Nothing to select
            app_state.list_state.select(None);
        } else if app_state.list_state.selected().is_none() {
            // Went from nothing selected (everything filtered out) to having results, so select
            // the result with the best score.
            app_state.list_state.select(Some(0));
        } else if let Some(selected) = app_state.list_state.selected() {
            // Since the filtered list could have changed, prevent the selection from going past
            // the end of the list, which could happen if the user navigates up the list and then
            // changes the search query.
            if selected >= final_matching_choices.len() {
                app_state
                    .list_state
                    .select(Some(final_matching_choices.len() - 1));
            }
        }

        terminal.draw(|mut f| {
            let width = f.size().width;
            let height = f.size().height;

            let num_items = final_matching_choices.len() as u16;
            let items = final_matching_choices
                .iter()
                // Have to make room for the highlight symbol, and a 1-column margin on the right
                .map(|choice| choice.to_text_widget(width as usize - highlight_symbol.len() - 1));
            let list = List::new(items)
                .style(style)
                .start_corner(tui::layout::Corner::BottomLeft)
                .highlight_style(highlight_style)
                .highlight_symbol(highlight_symbol);

            // The list of choices takes up the whole terminal except for the very bottom row
            let list_rect = tui::layout::Rect::new(0, height - num_items - 1, width, num_items);

            f.render_stateful_widget(list, list_rect, &mut app_state.list_state);

            // The bottom row is used for the query input
            let query_rect =
                tui::layout::Rect::new(0, f.size().height - 1, f.size().width as u16, 1);
            let text = [Text::raw(&app_state.query)];
            let input = Paragraph::new(text.iter());

            f.render_widget(input, query_rect);
        })?;

        let height = terminal.size()?.height;

        // Place the cursor at the end of the query input
        write!(
            terminal.backend_mut(),
            "{}",
            Goto(app_state.query.width() as u16 + 1, height)
        )?;

        let input = stdin.next();

        if let Some(Ok(key)) = input {
            match key {
                Key::Ctrl('c') => break,
                Key::Ctrl('w') => app_state.cut_to_current_word_start(),
                Key::Ctrl('u') => app_state.clear_query(),
                Key::Ctrl('k') | Key::Up => {
                    // Navigate up (increase selection index)
                    if let Some(selected) = app_state.list_state.selected() {
                        if selected < final_matching_choices.len() - 1 {
                            app_state.list_state.select(Some(selected + 1));
                        }
                    }
                }
                Key::Ctrl('j') | Key::Down => {
                    // Navigate down (decrease selection index)
                    if let Some(selected) = app_state.list_state.selected() {
                        if selected > 0 {
                            app_state.list_state.select(Some(selected - 1));
                        }
                    }
                }
                Key::Char('\n') => {
                    // Only prime and break from the loop if something is actually selected
                    if let Some(i) = app_state.list_state.selected() {
                        app_state.primed = final_matching_choices.get(i).map(|c| c.path.clone());
                        break;
                    }
                }
                Key::Char(c) => app_state.append_to_query(c),
                Key::Backspace => app_state.backspace(),
                _ => {}
            }
        }
    }

    // Switch back to the original screen
    drop(terminal);

    // Flush stdout so the list screen is cleared immediately
    io::stdout().flush().ok();

    if let Some(path) = app_state.primed {
        let def = files::load_file(&path, RequestDefinition::new, "request definition")?;
        let res = crate::http::send_request(def, &[]).context("Failed sending request")?;
        println!("{}", res);
    }
    Ok(())
}

#[test]
fn test_cut_to_current_word_start() {
    let mut state = InteractiveState::new();

    let tests = vec![
        ("one two three four", "one two three "),
        ("one two three four ", "one two three "),
        ("one ", ""),
        ("one  ", ""),
        ("one   two   three", "one   two   "),
        ("a", ""),
    ];

    for (start, expected) in tests {
        state.query = start.to_owned();
        state.cut_to_current_word_start();
        assert_eq!(state.query, expected)
    }
}
