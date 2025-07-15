mod analysis;
mod args;
mod nights_grid;
mod results_grid;
mod terminal_render;

use std::{
    error::Error,
    io, mem,
    sync::{Arc, atomic::Ordering},
    thread,
    time::Duration,
};

use clap::Parser as _;
use indexmap::map::Entry;
use indicatif::ProgressBar;
use ratatui::{
    Terminal,
    backend::Backend,
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    widgets::ListState,
};
use round_robin::{
    results::MatchResult,
    tournament::{Match, MatchSet, TournamentBlock},
};

mod app;
mod ui;
use crate::{
    analysis::ThreadState,
    app::{AnalysisState, App, AppState, NameType},
    args::TournamentArgs,
    ui::ui,
};

fn main() -> Result<(), Box<dyn Error>> {
    color_eyre::install()?;
    let terminal = ratatui::init();

    // create app and run it
    let args = TournamentArgs::parse();
    let mut app = App::initialise_from_file(args.tournament_file);

    let should_save = run_app(terminal, &mut app);

    ratatui::restore();
    if should_save? {
        app.save().unwrap();
    }

    Ok(())
}

fn run_app<B: Backend>(mut terminal: Terminal<B>, app: &mut App) -> io::Result<bool> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        let maybe_event = if event::poll(Duration::from_millis(100))? {
            Some(event::read()?)
        } else {
            None
        };
        if let Some(Event::Key(key)) = maybe_event {
            if key.kind == event::KeyEventKind::Release {
                // Skip events that are not KeyEventKind::Press
                continue;
            }
            app.status_message = None;
            if let KeyCode::Char('q') = key.code {
                app.state = AppState::Quitting;
                continue;
            }

            let shift = key.modifiers.contains(KeyModifiers::SHIFT);
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

            if !matches!(
                app.state,
                AppState::BlocksEdit {
                    editing: Some(_),
                    ..
                } | AppState::CompetitorsEdit {
                    editing: Some(_),
                    ..
                }
            ) {
                match key.code {
                    KeyCode::Tab => {
                        app.state.forwards();
                        continue;
                    }
                    KeyCode::BackTab => {
                        app.state.backwards();
                        continue;
                    }
                    KeyCode::Char('b') if ctrl => {
                        app.selected_block = (app.selected_block + 1) % app.data.block.len();
                        continue;
                    }
                    KeyCode::Char('s') if ctrl => {
                        app.status_message = Some(match app.save() {
                            Ok(()) => "Saved successfully".into(),
                            Err(e) => format!("Save failed: {e}"),
                        });
                    }
                    _ => {}
                }
            }

            let n_blocks = app.data.block.len();
            let Some(block) = app.data.block.get_mut(app.selected_block) else {
                if let AppState::BlocksEdit { editing, .. } = &mut app.state {
                    let mut edit_name = editing.take().unwrap_or("Block Name".into());
                    match key.code {
                        KeyCode::Char(c) => {
                            edit_name.push(c);
                        }
                        KeyCode::Backspace => {
                            edit_name.pop();
                        }

                        KeyCode::Enter => {
                            if let Some(existing_block) = app.data.block.get_mut(app.selected_block)
                            {
                                existing_block.name = edit_name;
                            } else {
                                app.data.block.push(TournamentBlock::empty(edit_name));
                            }
                            continue;
                        }
                        _ => {}
                    };
                    *editing = Some(edit_name);
                }
                continue;
            };
            let n_competitors = block.competitors.len();
            match &mut app.state {
                AppState::Quitting => match key.code {
                    KeyCode::Char('y') => return Ok(true),
                    KeyCode::Char('n') => return Ok(false),
                    KeyCode::Esc => {
                        app.state = AppState::BlocksEdit {
                            editing: None,
                            delete_warning: false,
                        }
                    }
                    _ => {}
                },
                AppState::BlocksEdit {
                    editing,
                    delete_warning,
                } => match editing.take() {
                    None => match key.code {
                        KeyCode::Down if !shift => {
                            *delete_warning = false;
                            app.selected_block = (app.selected_block + 1) % n_blocks;
                            continue;
                        }
                        KeyCode::Up if !shift => {
                            *delete_warning = false;
                            app.selected_block = (app.selected_block + n_blocks - 1) % n_blocks;
                            continue;
                        }
                        KeyCode::Up if shift => {
                            *delete_warning = false;
                            if app.selected_block > 0 {
                                app.data
                                    .block
                                    .swap(app.selected_block, app.selected_block - 1);
                                app.selected_block -= 1;
                            }
                        }
                        KeyCode::Down if shift => {
                            *delete_warning = false;
                            if app.selected_block < n_blocks - 1 {
                                app.data
                                    .block
                                    .swap(app.selected_block, app.selected_block + 1);
                                app.selected_block += 1;
                            }
                        }
                        KeyCode::Char('n') if ctrl => {
                            *delete_warning = false;
                            app.selected_block = app.data.block.len();
                            *editing = Some("New Block".to_owned());
                        }

                        KeyCode::Enter => {
                            *delete_warning = false;
                            *editing = Some(block.name.clone());
                        }

                        KeyCode::Delete | KeyCode::Backspace => {
                            if *delete_warning {
                                app.data.block.remove(app.selected_block);
                                if app.selected_block >= app.data.block.len() {
                                    app.selected_block = app.data.block.len() - 1;
                                }
                                *delete_warning = false;
                            } else {
                                *delete_warning = true;
                            }
                        }
                        _ => {}
                    },

                    Some(mut edit_name) => {
                        match key.code {
                            KeyCode::Char(c) => {
                                edit_name.push(c);
                            }
                            KeyCode::Backspace => {
                                edit_name.pop();
                            }

                            KeyCode::Enter => {
                                if let Some(existing_block) =
                                    app.data.block.get_mut(app.selected_block)
                                {
                                    existing_block.name = edit_name;
                                } else {
                                    app.data.block.push(TournamentBlock::empty(edit_name));
                                }
                                continue;
                            }
                            KeyCode::Esc => {
                                // Skip the editing = Some(..) below
                                continue;
                            }
                            _ => {}
                        };
                        *editing = Some(edit_name);
                    }
                },
                AppState::CompetitorsEdit {
                    competitor_index,
                    editing,
                    delete_warning,
                } => {
                    match editing.take() {
                        None => {
                            match key.code {
                                KeyCode::Down if !shift => {
                                    *delete_warning = false;
                                    *competitor_index = (*competitor_index + 1) % n_competitors;
                                    continue;
                                }
                                KeyCode::Up if !shift => {
                                    *delete_warning = false;
                                    *competitor_index =
                                        (*competitor_index + n_competitors - 1) % n_competitors;
                                    continue;
                                }
                                KeyCode::Up if shift => {
                                    *delete_warning = false;
                                    if *competitor_index > 0 {
                                        block
                                            .competitors
                                            .swap_indices(*competitor_index, *competitor_index - 1);
                                        *competitor_index -= 1;
                                    }
                                }
                                KeyCode::Down if shift => {
                                    *delete_warning = false;
                                    if *competitor_index < n_competitors - 1 {
                                        block
                                            .competitors
                                            .swap_indices(*competitor_index, *competitor_index + 1);
                                        *competitor_index += 1;
                                    }
                                }

                                KeyCode::Char('n') if ctrl => {
                                    *delete_warning = false;
                                    // let (new_idx, None) = block.competitors.insert_full(
                                    //     "..".to_owned(),
                                    //     "New Competitor".to_owned(),
                                    // ) else {
                                    //     app.status_message = Some("Error: Cannot create new competitor while a competitor has the short name '..'. This short name is reserved.".into());
                                    //     continue;
                                    // };
                                    *competitor_index = block.competitors.len();
                                    *editing = Some((
                                        NameType::LongName,
                                        ("..".to_owned(), "New Competitor".to_owned()),
                                    ));
                                }

                                KeyCode::Enter => {
                                    *delete_warning = false;
                                    if let Some((short, long)) =
                                        block.competitors.get_index(*competitor_index)
                                    {
                                        *editing = Some((
                                            NameType::LongName,
                                            (short.clone(), long.clone()),
                                        ));
                                    }
                                }

                                KeyCode::Delete | KeyCode::Backspace => {
                                    if *delete_warning {
                                        if let Some((short, _long)) =
                                            block.competitors.shift_remove_index(*competitor_index)
                                        {
                                            block.remove_competitor(&short);
                                        }
                                        if *competitor_index >= block.competitors.len() {
                                            *competitor_index = block.competitors.len() - 1;
                                        }
                                        *delete_warning = false;
                                    } else {
                                        *delete_warning = true;
                                    }
                                }
                                _ => {}
                            }
                        }
                        Some((mut name_type, (mut short, mut long))) => {
                            let edit_name = match name_type {
                                NameType::ShortName => &mut short,
                                NameType::LongName => &mut long,
                            };
                            match key.code {
                                KeyCode::Char(c) => {
                                    edit_name.push(c);
                                }
                                KeyCode::Backspace => {
                                    edit_name.pop();
                                }
                                KeyCode::Left
                                | KeyCode::Right
                                | KeyCode::Tab
                                | KeyCode::BackTab => {
                                    name_type = match name_type {
                                        NameType::LongName => NameType::ShortName,
                                        NameType::ShortName => NameType::LongName,
                                    };
                                }
                                KeyCode::Esc => {
                                    // Skip the editing = Some(..) below
                                    continue;
                                }
                                KeyCode::Enter => {
                                    match block.competitors.entry(short.clone()) {
                                        Entry::Occupied(mut entry) => {
                                            let existing_index = entry.index();
                                            let old_long = entry.get_mut();
                                            if existing_index == *competitor_index {
                                                *old_long = long;
                                                *editing = None;
                                                continue;
                                            } else {
                                                app.status_message = Some(format!(
                                                    "Entered short name clashes with {old_long}. Please select a different short name for {long}"
                                                ))
                                            }
                                        }
                                        Entry::Vacant(entry) => {
                                            let new_entry = entry.insert_entry(long);
                                            // If the new index is exactly the competitor index,
                                            // we're adding a new entry not editing one
                                            if new_entry.index() > *competitor_index {
                                                if let Some((old_short, _old_long)) = block
                                                    .competitors
                                                    .swap_remove_index(*competitor_index)
                                                {
                                                    block.relabel_competitor(&old_short, &short);
                                                }
                                            }
                                            *editing = None;
                                            continue;
                                        }
                                    }

                                    // if let Some((existing_index, _, old_long)) =
                                    //     block.competitors.get_full_mut(&short)
                                    // {
                                    //     if existing_index == *competitor_index {
                                    //         *old_long = long;
                                    //         *editing = None;
                                    //         app.status_message = None;
                                    //         continue;
                                    //     } else {
                                    //         app.status_message = Some(format!(
                                    //             "Entered short name clashes with {old_long}. Please select a different short name for {long}"
                                    //         ))
                                    //     }
                                    // } else {
                                    //     let None = block.competitors.insert(short.clone(), long)
                                    //     else {
                                    //         panic!("nooo")
                                    //     };
                                    //     let (old_short, _old_long) = block
                                    //         .competitors
                                    //         .swap_remove_index(*competitor_index)
                                    //         .expect("competitors contains item");
                                    //     block.relabel_competitor(&old_short, &short);
                                    //     *editing = None;
                                    //     continue;
                                    // }
                                }
                                _ => {}
                            }
                            *editing = Some((name_type, (short, long)));
                        }
                    }
                }
                AppState::NightsAssign {
                    cursor_position: (x_pos, y_pos),
                    list_state,
                } => match key.code {
                    KeyCode::Up | KeyCode::Down if shift => {
                        if let KeyCode::Up = key.code {
                            list_state.select_previous();
                        } else {
                            list_state.select_next();
                        }
                    }

                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                        update_cursor_no_diagonal(x_pos, y_pos, key.code, n_competitors);
                    }

                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        if let Some((current_night, match_index, _)) =
                            block.find_result_by_indices(*x_pos, *y_pos)
                        {
                            let night = &mut block.nights[current_night];
                            let moved_match = night.matches.remove(match_index);
                            let moved_result = night.results.remove(match_index);
                            let should_remove = night.matches.is_empty();
                            let new_night =
                                if let Some(night) = block.nights.get_mut(current_night + 1) {
                                    night
                                } else {
                                    block.nights.push(MatchSet::default());
                                    &mut block.nights[current_night + 1]
                                };
                            new_night.matches.push(moved_match);
                            new_night.results.push(moved_result);
                            if should_remove {
                                block.nights.remove(current_night);
                            }
                        } else {
                            let Some((a, _)) = block.competitors.get_index(*y_pos) else {
                                continue;
                            };
                            let Some((b, _)) = block.competitors.get_index(*x_pos) else {
                                continue;
                            };
                            let moved_match = Match(a.clone(), b.clone());
                            let moved_result = MatchResult::None;
                            let new_night = if let Some(night) = block.nights.get_mut(0) {
                                night
                            } else {
                                block.nights.push(MatchSet::default());
                                &mut block.nights[0]
                            };
                            new_night.matches.push(moved_match);
                            new_night.results.push(moved_result);
                        }
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') => {
                        if let Some((current_night, match_index, _)) =
                            block.find_result_by_indices(*x_pos, *y_pos)
                        {
                            let night = &mut block.nights[current_night];
                            let moved_match = night.matches.remove(match_index);
                            let moved_result = night.results.remove(match_index);
                            let should_remove = night.matches.is_empty();
                            if current_night > 0 {
                                let new_night = &mut block.nights[current_night - 1];
                                new_night.matches.push(moved_match);
                                new_night.results.push(moved_result);
                            }
                            if should_remove {
                                block.nights.remove(current_night);
                            }
                        }
                    }
                    KeyCode::Backspace | KeyCode::Delete => {
                        if let Some((current_night, match_index, _)) =
                            block.find_result_by_indices(*x_pos, *y_pos)
                        {
                            let night = &mut block.nights[current_night];
                            let _moved_match = night.matches.remove(match_index);
                            let _moved_result = night.results.remove(match_index);
                            let should_remove = night.matches.is_empty();

                            if should_remove {
                                block.nights.remove(current_night);
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        if let Some(digit) = c.to_digit(10) {
                            let target_night = if digit == 0 { 9 } else { digit as usize - 1 };
                            if let Some((current_night, match_index, _)) =
                                block.find_result_by_indices(*x_pos, *y_pos)
                            {
                                let night = &mut block.nights[current_night];
                                let moved_match = night.matches.remove(match_index);
                                let moved_result = night.results.remove(match_index);
                                let should_remove = night.matches.is_empty();
                                let new_night =
                                    if let Some(night) = block.nights.get_mut(target_night) {
                                        night
                                    } else {
                                        let old_len = block.nights.len();
                                        block.nights.push(MatchSet::default());
                                        &mut block.nights[old_len]
                                    };
                                new_night.matches.push(moved_match);
                                new_night.results.push(moved_result);
                                if should_remove {
                                    block.nights.remove(current_night);
                                }
                            } else {
                                let Some((a, _)) = block.competitors.get_index(*y_pos) else {
                                    continue;
                                };
                                let Some((b, _)) = block.competitors.get_index(*x_pos) else {
                                    continue;
                                };
                                let moved_match = Match(a.clone(), b.clone());
                                let moved_result = MatchResult::None;
                                let new_night =
                                    if let Some(night) = block.nights.get_mut(target_night) {
                                        night
                                    } else {
                                        let old_len = block.nights.len();
                                        block.nights.push(MatchSet::default());
                                        &mut block.nights[old_len]
                                    };
                                new_night.matches.push(moved_match);
                                new_night.results.push(moved_result);
                            }
                        }
                    }
                    _ => {}
                },
                AppState::ResultsEdit {
                    cursor_position: (x_pos, y_pos),
                    list_state,
                } => match key.code {
                    KeyCode::Up | KeyCode::Down if shift => {
                        if let KeyCode::Up = key.code {
                            list_state.select_previous();
                        } else {
                            list_state.select_next();
                        }
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                        update_cursor_no_diagonal(x_pos, y_pos, key.code, n_competitors);
                    }
                    KeyCode::Char(c) => {
                        if let Ok(result) = MatchResult::try_from(c) {
                            if matches!(result, MatchResult::SelfMatch) {
                                continue;
                            }
                            let Some((night, match_number, swapped)) =
                                block.find_result_by_indices(*y_pos, *x_pos)
                            else {
                                let Some((_, col_person)) = block.competitors.get_index(*x_pos)
                                else {
                                    continue;
                                };
                                let Some((_, row_person)) = block.competitors.get_index(*y_pos)
                                else {
                                    continue;
                                };
                                app.status_message = Some(format!(
                                    "Match {} vs {} is not scheduled. Please return to previous screen to assign a night",
                                    row_person, col_person
                                ));
                                continue;
                            };

                            block.nights[night].results[match_number] =
                                if swapped { !result } else { result };
                        }
                    }
                    _ => {}
                },
                AppState::Analysis { results } => match key.code {
                    KeyCode::Enter => match results {
                        AnalysisState::Display { .. } => {
                            let state = Arc::new(ThreadState::default());
                            let n_winners = app.data.number_advance;
                            let block = block.clone();
                            let progress_bar = ProgressBar::hidden();
                            let handle = {
                                let state = state.clone();
                                let bar = progress_bar.clone();
                                thread::spawn(move || {
                                    let answer =
                                        analysis::analyse_tournament(block, n_winners, &state, bar);
                                    state.done.store(true, Ordering::Relaxed);
                                    answer
                                })
                            };
                            *results = AnalysisState::Analysing {
                                state,
                                handle,
                                progress_bar,
                            };
                        }
                        AnalysisState::Analysing { state, .. } => {
                            state.done.store(true, Ordering::Relaxed);
                        }
                    },
                    KeyCode::Right => {
                        if let AnalysisState::Display {
                            night_number,
                            list_state,
                        } = results
                        {
                            let n_nights = block.nights.len();
                            *night_number = night_number.saturating_add(1).clamp(0, n_nights - 1);
                            list_state.select(None);
                        }
                    }
                    KeyCode::Left => {
                        if let AnalysisState::Display {
                            night_number,
                            list_state,
                        } = results
                        {
                            *night_number = night_number.saturating_sub(1);
                            list_state.select(None);
                        }
                    }
                    KeyCode::Down => {
                        if let AnalysisState::Display { list_state, .. } = results {
                            list_state.select_next();
                        }
                    }
                    KeyCode::Up => {
                        if let AnalysisState::Display { list_state, .. } = results {
                            list_state.select_previous();
                        }
                    }
                    KeyCode::Delete | KeyCode::Backspace => {
                        if let AnalysisState::Display { list_state, .. } = results {
                            list_state.select(None);
                        }
                    }
                    _ => {}
                },
            };
        }

        if let AppState::Analysis { results } = &mut app.state {
            if let AnalysisState::Analysing { state, .. } = results
                && state.done.load(Ordering::Relaxed)
            {
                let mut results_state = AnalysisState::Display {
                    night_number: 0,
                    list_state: ListState::default(),
                };
                mem::swap(results, &mut results_state);
                let AnalysisState::Analysing { handle, .. } = results_state else {
                    panic!()
                };
                let result = handle.join().unwrap();

                if let Some((block_name, analysis)) = result {
                    app.analysis_results.insert(block_name, analysis);
                };
            }
        }
    }
}

fn update_cursor_no_diagonal(
    x_pos: &mut usize,
    y_pos: &mut usize,
    code: KeyCode,
    total_size: usize,
) {
    match code {
        KeyCode::Left => {
            if *x_pos > 0 {
                *x_pos -= 1;
            }
            if *x_pos == *y_pos {
                if *x_pos == 0 {
                    *x_pos += 1;
                } else {
                    *x_pos -= 1;
                }
            }
        }
        KeyCode::Right => {
            if *x_pos < total_size - 1 {
                *x_pos += 1;
            }
            if *x_pos == *y_pos {
                if *x_pos < total_size - 1 {
                    *x_pos += 1;
                } else {
                    *x_pos -= 1;
                }
            }
        }
        KeyCode::Up => {
            if *y_pos > 0 {
                *y_pos -= 1;
            }
            if *x_pos == *y_pos {
                if *y_pos > 0 {
                    *y_pos -= 1;
                } else {
                    *y_pos += 1;
                }
            }
        }
        KeyCode::Down => {
            if *y_pos < total_size - 1 {
                *y_pos += 1;
            }
            if *x_pos == *y_pos {
                if *y_pos < total_size - 1 {
                    *y_pos += 1;
                } else {
                    *y_pos -= 1;
                }
            }
        }
        _ => {}
    }
}
