use log::error;
use ndarray::Array2;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    symbols::border,
    text::Span,
    widgets::{Block, BorderType, Paragraph, Widget},
};
use round_robin::tournament::{Match, TournamentBlock};

pub struct NightsGrid<'a> {
    pub block: &'a TournamentBlock,
    pub position: (usize, usize),
}

impl Widget for NightsGrid<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let n_competitors = self.block.competitors.len();
        let col_constraints = (0..n_competitors).map(|_| Constraint::Length(5));
        let row_constraints = (0..n_competitors).map(|_| Constraint::Length(3));
        let horizontal = Layout::horizontal(
            std::iter::once(Constraint::Min(20))
                .chain(col_constraints)
                .chain(std::iter::once(Constraint::Length(9))),
        )
        .spacing(0);
        let vertical =
            Layout::vertical(std::iter::once(Constraint::Length(1)).chain(row_constraints))
                .spacing(0);

        let rows = vertical.split(area);

        let mut table = Array2::from_elem((n_competitors, n_competitors), None);

        for (night_number, night) in self.block.nights.iter().enumerate() {
            for Match(a, b) in &night.matches {
                let Some(a) = self.block.competitors.get_index_of(a) else {
                    continue;
                };
                let Some(b) = self.block.competitors.get_index_of(b) else {
                    continue;
                };
                if let Some(other_night) = table[(a, b)] {
                    error!("Same match is in nights {other_night} and {night_number}");
                }
                if let Some(other_night) = table[(b, a)] {
                    error!("Same match is in nights {other_night} and {night_number}");
                }
                table[(a, b)] = Some(night_number);
                table[(b, a)] = Some(night_number);
            }
        }

        let short_names: Vec<_> = self.block.competitors.keys().cloned().collect();

        for (idx, &row) in rows.iter().enumerate() {
            let cells = horizontal.split(row);
            let mut it = cells.iter();
            let name_cell = it.next().unwrap();
            if idx == 0 {
                for (&cell, name) in it.zip(&short_names) {
                    Paragraph::new(Span::from(name).into_centered_line()).render(cell, buf);
                }
            } else {
                let long_name = self.block.competitors.get(&short_names[idx - 1]).unwrap();
                Paragraph::new(Span::from(long_name).into_right_aligned_line())
                    .block(Block::bordered().border_set(border::EMPTY))
                    .render(*name_cell, buf);

                for (x_idx, (&cell, result)) in it.zip(table.row(idx - 1)).enumerate() {
                    if x_idx == idx - 1 {
                        continue;
                    }
                    Paragraph::new(
                        Span::from(
                            result
                                .map(|n| format!("{}", n + 1))
                                .unwrap_or("?".to_string()),
                        )
                        .into_centered_line(),
                    )
                    // .bg(Color::Green)
                    .block(
                        Block::bordered().border_type(if (x_idx, idx) == self.position {
                            BorderType::Double
                        } else {
                            BorderType::Plain
                        }),
                    )
                    .render(cell, buf);
                }
            }
        }
    }
}
