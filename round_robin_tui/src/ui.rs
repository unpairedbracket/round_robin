use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Gauge, List, ListState, Paragraph},
};
use round_robin::results::ResultsTable;

use crate::{
    analysis::NightAnalysis,
    app::{AnalysisState, App, AppState, NameType},
    nights_grid::NightsGrid,
    results_grid::ResultsGrid,
    terminal_render::TerminalRenderBlock as _,
};

pub fn ui(frame: &mut Frame, app: &mut App) {
    // Create the layout sections.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let title_chunks = Layout::horizontal([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(chunks[0]);

    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default());
    frame.render_widget(title_block, title_chunks[0]);

    let title_left_chunks = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .split(title_chunks[0]);

    let title_text = app.state.title();

    let title = Paragraph::new(
        Span::styled(title_text, Style::default().fg(Color::Green)).into_centered_line(),
    )
    .block(Block::default().borders(Borders::TOP | Borders::BOTTOM));
    frame.render_widget(title, title_left_chunks[1]);

    if let Some(prev_title_text) = app.state.prev_title() {
        let prev_title = Paragraph::new(
            Span::styled(prev_title_text, Style::default().fg(Color::Green))
                .into_left_aligned_line(),
        )
        .block(Block::default().borders(Borders::TOP | Borders::BOTTOM | Borders::LEFT));
        frame.render_widget(prev_title, title_left_chunks[0]);
    }

    if let Some(next_title_text) = app.state.next_title() {
        let next_title = Paragraph::new(
            Span::styled(next_title_text, Style::default().fg(Color::Green))
                .into_right_aligned_line(),
        )
        .block(Block::default().borders(Borders::TOP | Borders::BOTTOM | Borders::RIGHT));
        frame.render_widget(next_title, title_left_chunks[2]);
    }

    let active_block_name = if let AppState::BlocksEdit {
        editing: Some(current_name),
        ..
    } = &app.state
    {
        current_name
    } else {
        let active_block = &app.data.block[app.selected_block];
        &active_block.name
    };

    let block_widget = Paragraph::new(
        Span::from(format!("Current Block: {}", active_block_name)).into_centered_line(),
    )
    .block(Block::bordered());

    frame.render_widget(block_widget, title_chunks[1]);
    match &mut app.state {
        AppState::Quitting => {
            let popup_block = Block::bordered()
                .title("Quitting")
                .style(Style::default().bg(Color::DarkGray));

            let area = centered_rect(60, 5, frame.area());
            frame.render_widget(Paragraph::new("Save file? (Y/N)").block(popup_block), area);
        }
        AppState::BlocksEdit { editing, .. } => match editing {
            None => {
                let mut list_items = Vec::new();

                for block in &app.data.block {
                    list_items.push(Span::from(&block.name));
                }
                let list = List::new(list_items)
                    .block(Block::bordered())
                    .highlight_style(Style::new().bg(Color::DarkGray));
                let mut state = ListState::default().with_selected(Some(app.selected_block));

                frame.render_stateful_widget(list, chunks[1], &mut state);
            }
            Some(block_name) => {
                let popup_block = Block::bordered()
                    .title("Edit Block Name")
                    .style(Style::default().bg(Color::DarkGray));
                let short_name_text = Paragraph::new(block_name.clone()).block(popup_block);

                let area = centered_rect(60, 3, frame.area());
                frame.render_widget(short_name_text, area);
            }
        },
        AppState::CompetitorsEdit {
            competitor_index,
            editing,
            ..
        } => match editing {
            None => {
                let active_block = &app.data.block[app.selected_block];
                let mut list_items = Vec::new();

                let longest_short = active_block
                    .competitors
                    .iter()
                    .map(|(short, _)| short.len())
                    .max()
                    .unwrap_or(0)
                    + 4;

                for (short_name, long_name) in &active_block.competitors {
                    list_items.push(Span::from(format!(
                        "{short_name:>longest_short$}: {long_name}"
                    )));
                }
                let list = List::new(list_items)
                    .block(Block::bordered())
                    .highlight_style(Style::new().bg(Color::DarkGray));
                let mut state = ListState::default().with_selected(Some(*competitor_index));

                frame.render_stateful_widget(list, chunks[1], &mut state);
            }
            Some((name, (short_name, long_name))) => {
                let popup_block = Block::bordered()
                    .title("Edit Name:")
                    .style(Style::new().bg(Color::DarkGray));

                let area = centered_rect(60, 5, frame.area());
                frame.render_widget(popup_block, area);

                let popup_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
                    .split(area);

                let mut short_block = Block::bordered().title("Short Name");
                let mut long_block = Block::bordered().title("Long Name");

                let active_style = Style::default().bg(Color::Reset);

                match name {
                    NameType::ShortName => short_block = short_block.style(active_style),
                    NameType::LongName => long_block = long_block.style(active_style),
                };

                let short_name_text = Paragraph::new(short_name.clone()).block(short_block);
                frame.render_widget(short_name_text, popup_chunks[0]);

                let long_name_text = Paragraph::new(long_name.clone()).block(long_block);
                frame.render_widget(long_name_text, popup_chunks[1]);
            }
        },
        AppState::NightsAssign {
            cursor_position: (x, y),
            list_state,
        } => {
            let active_block = &app.data.block[app.selected_block];
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);
            let grid = NightsGrid {
                block: &active_block,
                position: (*x, *y + 1),
            };
            frame.render_stateful_widget(
                List::new(active_block.render_matches())
                    .block(Block::bordered())
                    .highlight_symbol("> "),
                main_chunks[0],
                list_state,
            );
            frame.render_widget(grid, main_chunks[1]);
        }
        AppState::ResultsEdit {
            cursor_position: (x, y),
            list_state,
        } => {
            let active_block = &app.data.block[app.selected_block];
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);
            let results_table = ResultsTable::from(active_block);
            let grid = ResultsGrid {
                results: &results_table,
                competitors: &active_block.competitors,
                positions: &[(*x, *y)],
            };
            frame.render_stateful_widget(
                List::new(active_block.render_results())
                    .block(Block::bordered())
                    .highlight_symbol("> "),
                main_chunks[0],
                list_state,
            );
            frame.render_widget(grid, main_chunks[1]);
        }
        AppState::Analysis { results } => match results {
            AnalysisState::Display {
                night_number,
                list_state,
            } => {
                let active_block = &app.data.block[app.selected_block];
                if let Some(analysis) = app.analysis_results.get(&active_block.name) {
                    let long_names: Vec<_> = active_block.competitors.values().cloned().collect();
                    let mut possible_results: Vec<Text> = Vec::new();
                    let night_analysis = &analysis.0[*night_number];
                    night_analysis.print_night(&mut possible_results, &long_names);

                    let main_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(chunks[1]);
                    frame.render_stateful_widget(
                        List::new(possible_results)
                            .block(
                                Block::bordered()
                                    .border_type(BorderType::Rounded)
                                    .title(format!(
                                        "After night {}: {}",
                                        1 + *night_number,
                                        night_analysis.summary()
                                    ))
                                    .title(
                                        Line::from(format!(
                                            "(For {} winners)",
                                            app.data.number_advance
                                        ))
                                        .right_aligned(),
                                    ),
                            )
                            .highlight_style(Style::default().bg(Color::DarkGray)),
                        main_chunks[0],
                        list_state,
                    );

                    let results_table =
                        ResultsTable::from(&active_block.up_to_night(*night_number));
                    let new_results_table = if let Some(selected) = list_state.selected() {
                        if let NightAnalysis::Analysis(night_analysis) = &night_analysis {
                            let (_, results_table) = &night_analysis[selected];
                            results_table
                        } else {
                            &results_table
                        }
                    } else {
                        &results_table
                    };
                    let different_positions = results_table.diff_indices(new_results_table);
                    let grid = ResultsGrid {
                        results: new_results_table,
                        competitors: &active_block.competitors,
                        positions: &different_positions,
                    };

                    frame.render_widget(grid, main_chunks[1]);
                } else {
                    let instructions = Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title("Press Enter to run Analysis")
                        .title(
                            Line::from(format!("(For {} winners)", app.data.number_advance))
                                .right_aligned(),
                        );
                    frame.render_widget(instructions, chunks[1]);
                }
            }
            AnalysisState::Analysing { progress_bar, .. } => {
                if let Some(total) = progress_bar.length()
                    && total > 0
                {
                    let progress = progress_bar.position();
                    let g = Gauge::default().ratio((progress as f64 / total as f64).clamp(0., 1.));
                    let progress_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(3), Constraint::Fill(1)])
                        .split(chunks[1]);
                    frame.render_widget(g, progress_chunks[0]);
                }
            }
        },
    }

    // -------------------------------------------------------------------------------------

    let current_keys_hint = Span::styled(
        "(q) Quit | (ctrl+s) Save file | (ctrl+b) Cycle block",
        Style::default().fg(Color::Red),
    );

    let key_notes_footer = Paragraph::new(Line::from(current_keys_hint).centered())
        .block(Block::default().borders(Borders::ALL));

    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Length(56)])
        .split(chunks[2]);

    let statusline_footer = Paragraph::new(Line::from(Span::styled(
        app.status_message
            .as_deref()
            .unwrap_or(app.state.instructions()),
        if app.status_message.is_some() {
            Color::Red
        } else {
            Color::Yellow
        },
    )))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(statusline_footer, footer_chunks[0]);

    frame.render_widget(key_notes_footer, footer_chunks[1]);
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}
