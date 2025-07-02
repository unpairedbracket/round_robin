mod analysis;
mod args;
mod nights_grid;
mod results_grid;

use std::{
    error::Error,
    fs::read_to_string,
    io, mem,
    path::Path,
    sync::{Arc, atomic::Ordering},
    thread,
    time::Duration,
};

use clap::Parser as _;
use indexmap::map::Entry;
use indicatif::ProgressBar;
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};
use round_robin::{
    results::MatchResult,
    tournament::{Match, MatchSet, Tournament},
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
    // setup terminal
    enable_raw_mode()?;
    let mut stderr = io::stderr(); // This is a special case. Normally using stdout is fine
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let args = TournamentArgs::parse();
    let mut app = load_tournament(args.tournament_file)
        .unwrap_or_default()
        .into();

    let res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn load_tournament(path: impl AsRef<Path>) -> Option<Tournament> {
    let contents = read_to_string(path).ok()?;
    toml::from_str(&contents).ok()?
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<bool> {
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
            if let KeyCode::Char('q') = key.code {
                return Ok(true);
            }

            match key.code {
                KeyCode::Tab => {
                    app.state.forwards();
                    continue;
                }
                KeyCode::BackTab => {
                    app.state.backwards();
                    continue;
                }
                _ => {}
            }
            let Some(block) = app.data.block.get_mut(app.selected_block) else {
                continue;
            };
            let n_competitors = block.competitors.len();
            match &mut app.state {
                AppState::BlocksEdit => match key.code {
                    KeyCode::Down => {
                        app.selected_block = (app.selected_block + 1) % app.data.block.len()
                    }
                    KeyCode::Up => {
                        app.selected_block =
                            (app.selected_block + app.data.block.len() - 1) % app.data.block.len()
                    }
                    _ => {}
                },
                AppState::CompetitorsEdit {
                    competitor_index,
                    editing,
                } => {
                    match editing.take() {
                        None => {
                            let n_competitors = block.competitors.len();
                            match key.code {
                                KeyCode::Down => {
                                    *competitor_index = (*competitor_index + 1) % n_competitors;
                                    continue;
                                }
                                KeyCode::Up => {
                                    *competitor_index =
                                        (*competitor_index + n_competitors - 1) % n_competitors;
                                    continue;
                                }
                                KeyCode::Enter => {
                                    if let Some((short, long)) =
                                        block.competitors.get_index(*competitor_index)
                                    {
                                        *editing = Some((
                                            NameType::LongName,
                                            (short.clone(), long.clone()),
                                        ));
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
                                KeyCode::Left | KeyCode::Right => {
                                    name_type = match name_type {
                                        NameType::LongName => NameType::ShortName,
                                        NameType::ShortName => NameType::LongName,
                                    };
                                }
                                KeyCode::Enter => {
                                    match block.competitors.entry(short.clone()) {
                                        Entry::Occupied(mut entry) => {
                                            let existing_index = entry.index();
                                            let old_long = entry.get_mut();
                                            if existing_index == *competitor_index {
                                                *old_long = long;
                                                *editing = None;
                                                app.status_message = None;
                                                continue;
                                            } else {
                                                app.status_message = Some(format!(
                                                    "Entered short name clashes with {old_long}. Please select a different short name for {long}"
                                                ))
                                            }
                                        }
                                        Entry::Vacant(entry) => {
                                            entry.insert(long);
                                            let (old_short, _old_long) = block
                                                .competitors
                                                .swap_remove_index(*competitor_index)
                                                .expect("competitors contains item");
                                            block.relabel_competitor(&old_short, &short);
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
                } => match key.code {
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
                    _ => {}
                },
                AppState::ResultsEdit {
                    cursor_position: (x_pos, y_pos),
                } => match key.code {
                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                        update_cursor_no_diagonal(x_pos, y_pos, key.code, n_competitors);
                    }
                    KeyCode::Char(c) => {
                        if let Ok(result) = MatchResult::try_from(c) {
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
                        AnalysisState::NoAnalysis | AnalysisState::Complete { .. } => {
                            let state = Arc::new(ThreadState::default());
                            let n_winners = app.data.number_advance;
                            let block = block.clone();
                            let progress_bar = ProgressBar::hidden();
                            let handle = {
                                let state = state.clone();
                                let bar = progress_bar.clone();
                                thread::spawn(move || {
                                    let answer = analysis::analyse_tournament_bfs(
                                        block, n_winners, &state, bar,
                                    );
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
                        if let AnalysisState::Complete { night_number, .. } = results {
                            let n_nights = block
                                .nights
                                .iter()
                                .enumerate()
                                .filter(|(_, night)| !night.skip)
                                .count();
                            *night_number = night_number.saturating_add(1).clamp(0, n_nights - 1);
                        }
                    }
                    KeyCode::Left => {
                        if let AnalysisState::Complete { night_number, .. } = results {
                            *night_number = night_number.saturating_sub(1);
                        }
                    }
                    KeyCode::Down => {
                        if let AnalysisState::Complete {
                            scroll_position, ..
                        } = results
                        {
                            *scroll_position = scroll_position.saturating_add(1);
                        }
                    }
                    KeyCode::Up => {
                        if let AnalysisState::Complete {
                            scroll_position, ..
                        } = results
                        {
                            *scroll_position = scroll_position.saturating_sub(1);
                        }
                    }
                    _ => {}
                },
            };
            // match app.current_screen {
            //     CurrentScreen::Main => match key.code {
            //         KeyCode::Char('e') => {
            //             app.current_screen = CurrentScreen::Editing;
            //             app.currently_editing = Some(CurrentlyEditing::Key);
            //         }
            //         KeyCode::Char('q') => {
            //             app.current_screen = CurrentScreen::Exiting;
            //         }
            //         _ => {}
            //     },
            //     CurrentScreen::Exiting => match key.code {
            //         KeyCode::Char('y') => {
            //             return Ok(true);
            //         }
            //         KeyCode::Char('n') | KeyCode::Char('q') => {
            //             return Ok(false);
            //         }
            //         _ => {}
            //     },
            //     CurrentScreen::Editing if key.kind == KeyEventKind::Press => match key.code {
            //         KeyCode::Enter => {
            //             if let Some(editing) = &app.currently_editing {
            //                 match editing {
            //                     CurrentlyEditing::Key => {
            //                         app.currently_editing = Some(CurrentlyEditing::Value);
            //                     }
            //                     CurrentlyEditing::Value => {
            //                         app.save_key_value();
            //                         app.current_screen = CurrentScreen::Main;
            //                     }
            //                 }
            //             }
            //         }
            //         KeyCode::Backspace => {
            //             if let Some(editing) = &app.currently_editing {
            //                 match editing {
            //                     CurrentlyEditing::Key => {
            //                         app.key_input.pop();
            //                     }
            //                     CurrentlyEditing::Value => {
            //                         app.value_input.pop();
            //                     }
            //                 }
            //             }
            //         }
            //         KeyCode::Esc => {
            //             app.current_screen = CurrentScreen::Main;
            //             app.currently_editing = None;
            //         }
            //         KeyCode::Tab => {
            //             app.toggle_editing();
            //         }
            //         KeyCode::Char(value) => {
            //             if let Some(editing) = &app.currently_editing {
            //                 match editing {
            //                     CurrentlyEditing::Key => {
            //                         app.key_input.push(value);
            //                     }
            //                     CurrentlyEditing::Value => {
            //                         app.value_input.push(value);
            //                     }
            //                 }
            //             }
            //         }
            //         _ => {}
            //     },
            //     _ => {}
            // }
        }
        if let AppState::Analysis { results } = &mut app.state {
            if let AnalysisState::Analysing { state, .. } = results
                && state.done.load(Ordering::Relaxed)
            {
                let mut results_state = AnalysisState::NoAnalysis;
                mem::swap(results, &mut results_state);
                let AnalysisState::Analysing { handle, .. } = results_state else {
                    panic!()
                };
                let result = handle.join().unwrap();

                *results = if let Some(analysis) = result {
                    AnalysisState::Complete {
                        analysis,
                        scroll_position: 0,
                        night_number: 0,
                    }
                } else {
                    AnalysisState::NoAnalysis
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
