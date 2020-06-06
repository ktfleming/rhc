use crate::choice::Choice;
use crate::config::Config;
use crate::environment::Environment;
use crate::files;
use crate::keyvalue::KeyValue;
use crate::{colors::Colors, request_definition::RequestDefinition};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use sublime_fuzzy::best_match;
use termion::cursor::{Goto, Hide, Show};
use termion::event::Key;
use termion::input::Keys;
use tui::style::{Modifier, Style};
use tui::widgets::{List, ListState, Paragraph, Text};
use tui::Terminal;
use unicode_width::UnicodeWidthStr;

/// Like readline Ctrl-W
pub fn cut_to_current_word_start(s: &mut String) {
    let mut cut_a_letter = false;
    while !s.is_empty() {
        let popped = s.pop();
        if let Some(' ') = popped {
            if cut_a_letter {
                s.push(' ');
                break;
            }
        } else {
            cut_a_letter = true;
        }
    }
}

struct InteractiveState {
    /// What the user has entered into the input buffer
    query: String,

    /// Holds which item is selected
    list_state: ListState,

    // When exiting the UI loop, if this is Some, that Choice
    // will have its request sent.
    primed: Option<PathBuf>,

    active_env_index: Option<usize>,
}

impl InteractiveState {
    fn new() -> InteractiveState {
        InteractiveState {
            query: String::new(),
            list_state: ListState::default(),
            primed: None,
            active_env_index: None,
        }
    }
}

pub struct SelectedValues {
    pub def: RequestDefinition,
    pub env: Option<Environment>,
}

pub fn interactive_mode<R: std::io::Read, B: tui::backend::Backend + std::io::Write>(
    config: &Config,
    env_path: Option<&Path>,
    stdin: &mut Keys<R>,
    terminal: &mut Terminal<B>,
) -> anyhow::Result<Option<SelectedValues>> {
    // This Vec<Choice> serves as the source-of-truth that will be filtered on and eventually
    // selected from. Initially only Paths are populated in the Choice structs, and the associated
    // RequestDefinition is not present.  The main UI loop accesses it through the read mode of the
    // RwLock and only uses it to display the List widget. Another thread is spawned to parse each
    // path and update the Choice's request_definition field via the write mode of the RwLock.
    let all_choices = Arc::new(RwLock::new(files::list_all_choices(config)));

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

    let colors = Colors::from(&config.colors);
    let mut default_style = Style::default();
    if let Some(default_fg) = colors.default_fg {
        default_style = default_style.fg(default_fg);
    }
    if let Some(default_bg) = colors.default_bg {
        default_style = default_style.bg(default_bg);
    }

    let mut selected_style = Style::default()
        .fg(colors.selected_fg)
        .modifier(Modifier::BOLD);
    if let Some(selected_bg) = colors.selected_bg {
        selected_style = selected_style.bg(selected_bg);
    }

    let mut prompt_style = Style::default().fg(colors.prompt_fg);
    if let Some(prompt_bg) = colors.prompt_bg {
        prompt_style = prompt_style.bg(prompt_bg);
    }

    // Load all the environments available
    let mut environments: Vec<(Environment, PathBuf)> = files::list_all_environments(&config);

    // If the user started with the --environment flag, find the matching environment, if there is
    // one, and set that as the selected environment.
    if let Some(env_path) = env_path {
        for (i, (_, path)) in environments.iter().enumerate() {
            if path == env_path {
                app_state.active_env_index = Some(i);
            }
        }
    }

    loop {
        // Needed to prevent cursor flicker when navigating the list
        io::stdout().flush().ok();

        // Inside this loop we only need read access to the Vec<Choice>
        let inner_guard = Arc::clone(&all_choices);
        let inner_guard = inner_guard.read().unwrap();

        // Look up the active environment to use it in the prompt
        let active_env = app_state
            .active_env_index
            .map(|i| environments.get(i).unwrap());
        let active_vars = active_env.map(|(e, _)| &e.variables);
        let prompt = match active_env {
            Some((env, _)) => format!("{} > ", env.name),
            None => "> ".to_string(),
        };

        // Use fuzzy matching on the Choices' path, and URL/description if present
        let filtered_choices: Vec<&Choice> = if app_state.query.is_empty() {
            inner_guard.iter().collect()
        } else {
            let mut matching_choices: Vec<(isize, &Choice)> = inner_guard
                .iter()
                .filter_map(|choice| {
                    let target = format!(
                        "{}{}{}",
                        &choice.trimmed_path(),
                        choice.url_or_blank(active_vars),
                        choice.description_or_blank(),
                    );
                    best_match(&app_state.query, &target).map(|result| (result.score(), choice))
                })
                .collect();

            // We want to sort descending so the Choice with the highest score is as position 0
            matching_choices.sort_unstable_by(|(score1, _), (score2, _)| score2.cmp(score1));

            matching_choices.iter().map(|(_, choice)| *choice).collect()
        };

        if filtered_choices.is_empty() {
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
            if selected >= filtered_choices.len() {
                app_state
                    .list_state
                    .select(Some(filtered_choices.len() - 1));
            }
        }

        terminal.draw(|mut f| {
            let width = f.size().width;
            let height = f.size().height;

            // The maximum number of items we can display is limited by the height of the terminal
            let list_rows = std::cmp::min(filtered_choices.len() as u16, height.checked_sub(1).unwrap_or(0));
            let items = filtered_choices
                .iter()
                // Have to make room for the highlight symbol, and a 1-column margin on the right
                .map(|choice| choice.to_text_widget(active_vars));
            let list = List::new(items)
                .style(default_style)
                .start_corner(tui::layout::Corner::BottomLeft)
                .highlight_style(selected_style)
                .highlight_symbol(highlight_symbol);

            // The list of choices takes up the whole terminal except for the very bottom row
            let list_rect = tui::layout::Rect::new(0, height - list_rows - 1, width, list_rows);

            f.render_stateful_widget(list, list_rect, &mut app_state.list_state);

            // The bottom row is used for the query input
            let query_rect = tui::layout::Rect::new(0, height - 1, width, 1);
            let query_text = [
                Text::Styled((&prompt).into(), prompt_style),
                Text::raw(&app_state.query),
            ];
            let input = Paragraph::new(query_text.iter());

            f.render_widget(input, query_rect);
        })?;

        let height = terminal.size()?.height;

        // Place the cursor at the end of the query input
        write!(
            terminal.backend_mut(),
            "{}",
            Goto(
                prompt.width() as u16 + app_state.query.width() as u16 + 1,
                height
            )
        )?;

        let input = stdin.next();

        if let Some(Ok(key)) = input {
            match key {
                Key::Ctrl('c') => break,
                Key::Ctrl('w') => cut_to_current_word_start(&mut app_state.query),
                Key::Ctrl('u') => {
                    app_state.query.clear();
                }
                Key::Ctrl('p') | Key::Up => {
                    // Navigate up (increase selection index)
                    if let Some(selected) = app_state.list_state.selected() {
                        if selected < filtered_choices.len() - 1 {
                            app_state.list_state.select(Some(selected + 1));
                        }
                    }
                }
                Key::Ctrl('n') | Key::Down => {
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
                        app_state.primed = filtered_choices.get(i).map(|c| c.path.clone());
                        break;
                    }
                }
                Key::Backspace => {
                    app_state.query.pop();
                }
                Key::Char('\t') => {
                    // Select next environment
                    match app_state.active_env_index {
                        None => {
                            if !environments.is_empty() {
                                app_state.active_env_index = Some(0);
                            }
                        }
                        Some(i) => {
                            if i < environments.len() - 1 {
                                app_state.active_env_index = Some(i + 1);
                            } else {
                                app_state.active_env_index = None;
                            }
                        }
                    }
                }
                Key::BackTab => {
                    // Select previous environment
                    match app_state.active_env_index {
                        None => {
                            if !environments.is_empty() {
                                app_state.active_env_index = Some(environments.len() - 1);
                            }
                        }
                        Some(i) => {
                            if i > 0 {
                                app_state.active_env_index = Some(i - 1);
                            } else {
                                app_state.active_env_index = None;
                            }
                        }
                    }
                }
                Key::Char(c) => app_state.query.push(c),
                _ => {}
            }
        }
    }

    let result = match app_state.primed {
        None => None,
        Some(path) => {
            let def: RequestDefinition =
                files::load_file(&path, RequestDefinition::new, "request definition")?;
            let env: Option<Environment> = app_state
                .active_env_index
                .map(|i| environments.remove(i))
                .map(|(e, _)| e);
            Some(SelectedValues { def, env })
        }
    };

    Ok(result)
}

struct PromptState {
    query: String,
    list_state: ListState,

    // Which item in the history list is currently selected. If None, this means that either there
    // are no filtered options to be selected, or the history pane is not active, meaning the user
    // is in "query input" move.
    active_history_item_index: Option<usize>,
}

impl PromptState {
    fn new() -> PromptState {
        PromptState {
            query: String::new(),
            list_state: ListState::default(),
            active_history_item_index: None,
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
struct HistoryItem {
    name: String,
    value: String,
    env_name: String,
}

/// Given a list of unbound variable names, prompt the user to interactively enter values to bind
/// them to, and return those created KeyValues. Returning None means the user aborted with Ctrl-C
/// and we should not send the request.
pub fn prompt_for_variables<R: std::io::Read, B: tui::backend::Backend + std::io::Write>(
    config: &Config,
    names: Vec<&str>,
    env_name: &str,
    stdin: &mut Keys<R>,
    terminal: &mut Terminal<B>,
) -> anyhow::Result<Option<Vec<KeyValue>>> {
    // This will ensure that the cursor is restored even if this function panics, the user presses
    // Ctrl+C, etc
    let mut terminal = scopeguard::guard(terminal, |t| {
        write!(t.backend_mut(), "{}", Show).unwrap();
    });

    let mut state = PromptState::new();
    let mut result: Vec<KeyValue> = Vec::new();

    let colors = Colors::from(&config.colors);
    let mut default_style = Style::default();
    if let Some(default_fg) = colors.default_fg {
        default_style = default_style.fg(default_fg);
    }
    if let Some(default_bg) = colors.default_bg {
        default_style = default_style.bg(default_bg);
    }

    let mut selected_style = Style::default()
        .fg(colors.selected_fg)
        .modifier(Modifier::BOLD);
    if let Some(selected_bg) = colors.selected_bg {
        selected_style = selected_style.bg(selected_bg);
    }

    let mut prompt_style = Style::default().fg(colors.prompt_fg);
    if let Some(prompt_bg) = colors.prompt_bg {
        prompt_style = prompt_style.bg(prompt_bg);
    }

    let mut variable_style = Style::default().fg(colors.variable_fg);
    if let Some(variable_bg) = colors.variable_bg {
        variable_style = variable_style.bg(variable_bg);
    }

    // Which item in the `names` vector we are currently prompting for
    let mut current_name_index = 0;

    let prompt = "> ";

    let history_location = shellexpand::tilde(&config.history_file);
    let history_file = OpenOptions::new()
        .append(true)
        .read(true)
        .create(true)
        .open(history_location.as_ref())?;

    // Clone the file handle since we need to read from it here, and append to it in the loop
    let mut history_reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(history_file.try_clone()?);
    let mut history_writer = csv::Writer::from_writer(history_file);

    let full_history: Vec<HistoryItem> = history_reader
        .records()
        .filter_map(|record| {
            if let Ok(record) = record {
                // let split: Vec<&str> = l.split("|||").collect();
                let split: Vec<&str> = record.iter().collect();
                if let [name, value, env_name] = split.as_slice() {
                    Some(HistoryItem {
                        name: (*name).to_string(),
                        value: (*value).to_string(),
                        env_name: (*env_name).to_string(),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // The new HistoryItems, which don't already appear in full_history, that the user creates
    // interactively
    let mut created_items: Vec<HistoryItem> = vec![];

    let highlight_symbol = ">> ";

    loop {
        io::stdout().flush().ok();

        // First, filter to just the history items that were used for this variable name and
        // environment
        let mut filtered_history_items: Vec<&HistoryItem> = full_history
            .iter()
            .filter(|item| item.name == names[current_name_index] && item.env_name == env_name)
            .collect();

        // Fuzzy matching is basically the same as for choosing a request definition
        if !state.query.is_empty() {
            let mut matching_items: Vec<(isize, &HistoryItem)> = filtered_history_items
                .iter()
                .filter_map(|item| {
                    let result = best_match(&state.query, &item.value).map(|result| (result.score(), *item));
                    result
                })
                .collect();

            matching_items.sort_unstable_by(|(score1, _), (score2, _)| score2.cmp(score1));

            filtered_history_items = matching_items.iter().map(|(_, item)| *item).collect();
        };

        state.list_state.select(state.active_history_item_index);

        let in_history_mode = state.active_history_item_index.is_some();
        let matching_history_items = filtered_history_items.iter().map(|item| {
            if in_history_mode {
                Text::raw(item.value.to_string())
            } else {
                Text::raw(format!("   {}", item.value))
            }
        });

        let list = List::new(matching_history_items)
            .start_corner(tui::layout::Corner::BottomLeft)
            .style(default_style)
            .highlight_style(selected_style)
            .highlight_symbol(highlight_symbol);

        let explanation_text = [
            Text::raw("Enter a value for "),
            Text::styled(names[current_name_index], variable_style),
        ];
        let explanation_widget = Paragraph::new(explanation_text.iter());

        terminal.draw(|mut f| {
            let width = f.size().width;
            let height = f.size().height;

            // Similar to selecting a request definition, the number of items we can display in the
            // vertical list is limited by the terminal's height. We also need to reserve 2 rows
            // for the explanation and query rows. Be careful not to run into overflow, as these
            // are unsigned integers.
            let list_rows = std::cmp::min(filtered_history_items.len() as u16, height.checked_sub(2).unwrap_or(0));

            // History selection box is all of the screen except the bottom 2 rows
            let history_rect = tui::layout::Rect::new(0, height - list_rows - 2, width, list_rows);
            f.render_stateful_widget(list, history_rect, &mut state.list_state);

            // After that is the prompt/explanation row
            let explanation_rect = tui::layout::Rect::new(0, height - 2, width, 1);
            f.render_widget(explanation_widget, explanation_rect);

            // The bottom row is for input
            let query_rect = tui::layout::Rect::new(0, height - 1, width, 1);
            let query_text = [
                Text::Styled(prompt.into(), prompt_style),
                Text::raw(&state.query),
            ];

            let query_widget = Paragraph::new(query_text.iter());
            f.render_widget(query_widget, query_rect);
        })?;

        let height = terminal.size()?.height;

        if !in_history_mode {
            write!(terminal.backend_mut(), "{}", Show)?;
            write!(
                terminal.backend_mut(),
                "{}",
                Goto(
                    prompt.width() as u16 + state.query.width() as u16 + 1,
                    height
                )
            )?;
        }

        let input = stdin.next();
        if let Some(Ok(key)) = input {
            match key {
                Key::Ctrl('c') => break,
                Key::Ctrl('w') => cut_to_current_word_start(&mut state.query),
                Key::Ctrl('u') => {
                    state.query.clear();
                }
                Key::Char('\t') | Key::BackTab => {
                    if in_history_mode {
                        state.active_history_item_index = None;
                    } else {
                        // Can only move to "history selection" mode if there is actually something
                        // to select
                        if !filtered_history_items.is_empty() {
                            state.active_history_item_index = Some(0);
                            write!(terminal.backend_mut(), "{}", Hide)?;
                        }
                    }
                }
                Key::Ctrl('p') | Key::Up => {
                    if let Some(i) = state.active_history_item_index {
                        if i < filtered_history_items.len() - 1 {
                            state.active_history_item_index = Some(i + 1);
                        }
                    }
                }
                Key::Ctrl('n') | Key::Down => {
                    if let Some(i) = state.active_history_item_index {
                        if i > 0 {
                            state.active_history_item_index = Some(i - 1);
                        }
                    }
                }
                Key::Char('\n') => {
                    if let Some(index) = state.active_history_item_index {
                        let answer = KeyValue::new(
                            names[current_name_index],
                            &filtered_history_items[index].value,
                        );
                        result.push(answer);
                    } else if !&state.query.is_empty() {
                        // Assume that an empty string answer is never what they want
                        let answer = KeyValue::new(names[current_name_index], &state.query);

                        let new_item = HistoryItem {
                            name: answer.name.clone(),
                            value: answer.value.clone(),
                            env_name: env_name.to_string(),
                        };

                        if !full_history.contains(&new_item) {
                            history_writer.write_record(&[
                                answer.name.clone(),
                                answer.value.clone(),
                                env_name.to_string(),
                            ])?;

                            // Keep track of the new items so we can re-write the file at the end of
                            // this function, which is necessary if the number of history items exceeds
                            // the max_history_items setting in the user's Config
                            created_items.push(new_item);
                        }

                        result.push(answer);
                    }

                    // If an answer was pushed, the means the current variable is done and we can
                    // move on to the next one. We also want to start each variable in "query
                    // mode", so we reset the active_history_item_index field.
                    if result.len() == current_name_index + 1 {
                        current_name_index += 1;
                        state.active_history_item_index = None;
                        state.query.clear();
                        write!(terminal.backend_mut(), "{}", Show)?;
                        if current_name_index >= names.len() {
                            println!("Breaking...");
                            break;
                        }
                    }
                }
                Key::Backspace => {
                    state.query.pop();
                }
                Key::Char(c) => state.query.push(c),
                _ => {}
            }
        }
    }

    // If the total number of history items exceeds the max, rewrite the history file with just the
    // tail of appropriate size
    let mut all_history = full_history;
    all_history.append(&mut created_items);
    let max = config.max_history_items.unwrap_or(1000) as usize;

    if all_history.len() > max {
        drop(history_writer);

        let excess_items = all_history.len() - max;

        let rewrite_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(history_location.as_ref())?;
        let mut history_rewriter = csv::Writer::from_writer(rewrite_file);
        for item in all_history.iter().skip(excess_items) {
            history_rewriter.write_record(&[
                item.name.clone(),
                item.value.clone(),
                item.env_name.clone(),
            ])?;
        }
    }

    if result.len() == names.len() {
        // All variables set, go ahead with the request
        Ok(Some(result))
    } else {
        // The user aborted with Ctrl-C, don't send the request
        Ok(None)
    }
}

#[test]
fn test_cut_to_current_word_start() {
    let tests = vec![
        ("one two three four", "one two three "),
        ("one two three four ", "one two three "),
        ("one ", ""),
        ("one  ", ""),
        ("one   two   three", "one   two   "),
        ("a", ""),
    ];

    for (start, expected) in tests {
        let mut s = start.to_owned();
        cut_to_current_word_start(&mut s);
        assert_eq!(s, expected)
    }
}
