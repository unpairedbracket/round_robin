use itertools::Itertools;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph},
};
use round_robin::results::ResultsTable;

use crate::{
    app::{AnalysisState, App, AppState, NameType},
    nights_grid::NightsGrid,
    results_grid::ResultsGrid,
};

pub fn ui(frame: &mut Frame, app: &App) {
    // Create the layout sections.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default());

    let title_text = app.state.title();

    let title = Paragraph::new(Text::styled(title_text, Style::default().fg(Color::Green)))
        .block(title_block);

    frame.render_widget(title, chunks[0]);

    let active_block = &app.data.block[app.selected_block];
    match &app.state {
        AppState::BlocksEdit => {
            let mut list_items = Vec::<ListItem>::new();

            for (idx, block) in app.data.block.iter().enumerate() {
                let style = if idx == app.selected_block {
                    Style::default().bg(Color::LightGreen)
                } else {
                    Style::default()
                };
                list_items.push(ListItem::new(Line::styled(&block.name, style)));
            }
            let list = List::new(list_items);

            frame.render_widget(list, chunks[1]);
        }
        AppState::CompetitorsEdit {
            competitor_index,
            editing,
        } => match editing {
            None => {
                let mut list_items = Vec::<ListItem>::new();

                let longest_short = active_block
                    .competitors
                    .iter()
                    .map(|(short, _)| short.len())
                    .max()
                    .unwrap()
                    + 4;

                for (idx, (short_name, long_name)) in active_block.competitors.iter().enumerate() {
                    let style = if idx == *competitor_index {
                        Style::default().bg(Color::LightGreen)
                    } else {
                        Style::default()
                    };
                    list_items.push(ListItem::new(Line::styled(
                        format!("{short_name:>longest_short$}: {long_name}"),
                        style,
                    )));
                }
                let list = List::new(list_items);

                frame.render_widget(list, chunks[1]);
            }
            Some((name, (short_name, long_name))) => {
                let popup_block = Block::default()
                    .title("Edit Name")
                    .borders(Borders::ALL)
                    .style(Style::default().bg(Color::DarkGray));

                let area = centered_rect(60, 25, frame.area());
                frame.render_widget(popup_block, area);

                let popup_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area);

                let mut short_block = Block::default().title("Short Name").borders(Borders::NONE);
                let mut long_block = Block::default().title("Long Name").borders(Borders::NONE);

                let active_style = Style::default().bg(Color::LightYellow).fg(Color::Black);

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
        } => {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);
            let grid = NightsGrid {
                block: &active_block,
                position: (*x, *y + 1),
            };
            frame.render_widget(
                Paragraph::new(format!("{:#?}", active_block.nights)).block(Block::bordered()),
                main_chunks[0],
            );
            frame.render_widget(grid, main_chunks[1]);
        }
        AppState::ResultsEdit {
            cursor_position: (x, y),
        } => {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);
            let results_table = ResultsTable::from(active_block);
            let grid = ResultsGrid {
                results: results_table,
                competitors: &active_block.competitors,
                position: (*x, *y + 1),
            };
            frame.render_widget(
                Paragraph::new(format!("{:#?}", active_block.nights)).block(Block::bordered()),
                main_chunks[0],
            );
            frame.render_widget(grid, main_chunks[1]);
        }
        AppState::Analysis { results } => match results {
            AnalysisState::NoAnalysis => {
                let instructions = Paragraph::new("Press Enter to run Analysis")
                    .centered()
                    .block(Block::bordered().border_type(BorderType::Rounded));
                frame.render_widget(instructions, chunks[1]);
            }
            AnalysisState::Analysing { progress_bar, .. } => {
                if let Some(total) = progress_bar.length()
                    && total > 0
                {
                    let progress = progress_bar.position();
                    let g = Gauge::default().ratio((progress as f64 / total as f64).clamp(0., 1.));
                    // .block(Block::bordered().border_type(BorderType::Rounded));
                    let progress_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(3), Constraint::Fill(1)])
                        .split(chunks[1]);
                    frame.render_widget(g, progress_chunks[0]);
                }
            }
            AnalysisState::Complete {
                analysis,
                scroll_position,
                night_number,
            } => {
                let long_names: Vec<_> = active_block.competitors.values().cloned().collect();
                let results_table = ResultsTable::from(active_block);
                if let Some((night_idx, night)) = active_block
                    .nights
                    .iter()
                    .enumerate()
                    .filter(|(_, night)| !night.skip)
                    .nth(*night_number)
                {
                    let mut night_text = format!("--- After night {} ---\n", night_idx + 1);
                    analysis.print_night(
                        &mut night_text,
                        *night_number,
                        night,
                        &long_names,
                        results_table,
                        app.data.number_advance,
                    );
                    let output = Paragraph::new(night_text)
                        .block(Block::bordered().border_type(BorderType::Rounded))
                        .scroll((*scroll_position as u16, 0));
                    frame.render_widget(output, chunks[1]);
                }
            }
        },
    }

    // -------------------------------------------------------------------------------------

    let current_keys_hint = Span::styled("(q) to quit", Style::default().fg(Color::Red));

    let key_notes_footer =
        Paragraph::new(Line::from(current_keys_hint)).block(Block::default().borders(Borders::ALL));

    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(chunks[2]);

    let statusline_footer = Paragraph::new(Line::from(Span::styled(
        app.status_message.as_deref().unwrap_or(""),
        Style::default().fg(Color::Yellow),
    )))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(statusline_footer, footer_chunks[0]);

    frame.render_widget(key_notes_footer, footer_chunks[1]);

    // if let Some(editing) = &app.currently_editing {
    //     let popup_block = Block::default()
    //         .title("Enter a new key-value pair")
    //         .borders(Borders::NONE)
    //         .style(Style::default().bg(Color::DarkGray));

    //     let area = centered_rect(60, 25, frame.area());
    //     frame.render_widget(popup_block, area);

    //     let popup_chunks = Layout::default()
    //         .direction(Direction::Horizontal)
    //         .margin(1)
    //         .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
    //         .split(area);

    //     let mut key_block = Block::default().title("Key").borders(Borders::ALL);
    //     let mut value_block = Block::default().title("Value").borders(Borders::ALL);

    //     let active_style = Style::default().bg(Color::LightYellow).fg(Color::Black);

    //     match editing {
    //         CurrentlyEditing::Key => key_block = key_block.style(active_style),
    //         CurrentlyEditing::Value => value_block = value_block.style(active_style),
    //     };

    //     let key_text = Paragraph::new(app.key_input.clone()).block(key_block);
    //     frame.render_widget(key_text, popup_chunks[0]);

    //     let value_text = Paragraph::new(app.value_input.clone()).block(value_block);
    //     frame.render_widget(value_text, popup_chunks[1]);
    // }

    // if let CurrentScreen::Exiting = app.current_screen {
    //     frame.render_widget(Clear, frame.area()); //this clears the entire screen and anything already drawn
    //     let popup_block = Block::default()
    //         .title("Y/N")
    //         .borders(Borders::NONE)
    //         .style(Style::default().bg(Color::DarkGray));

    //     let exit_text = Text::styled(
    //         "Would you like to output the buffer as json? (y/n)",
    //         Style::default().fg(Color::Red),
    //     );
    //     // the `trim: false` will stop the text from being cut off when over the edge of the block
    //     let exit_paragraph = Paragraph::new(exit_text)
    //         .block(popup_block)
    //         .wrap(Wrap { trim: false });

    //     let area = centered_rect(60, 25, frame.area());
    //     frame.render_widget(exit_paragraph, area);
    // }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
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
