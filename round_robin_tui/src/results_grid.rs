use indexmap::IndexMap;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Stylize},
    symbols::border,
    text::Span,
    widgets::{Block, BorderType, Paragraph, Widget},
};
use round_robin::results::{MatchResult, ResultsTable};

pub struct ResultsGrid<'a> {
    pub results: &'a ResultsTable,
    pub competitors: &'a IndexMap<String, String>,
    pub positions: &'a [(usize, usize)],
    pub name_styles: Vec<Color>,
}

impl Widget for ResultsGrid<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let col_constraints = (0..self.results.n_competitors()).map(|_| Constraint::Length(5));
        let row_constraints = (0..self.results.n_competitors()).map(|_| Constraint::Length(3));
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

        let table = self.results.table();
        let short_names = self.results.names();
        let scores = self.results.scores();
        let max_scores = self.results.max_possible_scores();

        for (idx, &row) in rows.iter().enumerate() {
            let cells = horizontal.split(row);
            let mut it = cells.iter();
            let name_cell = it.next().unwrap();
            if idx == 0 {
                for (&cell, name) in it.zip(short_names) {
                    Paragraph::new(Span::from(name).into_centered_line()).render(cell, buf);
                }
            } else {
                let long_name = self.competitors.get(&short_names[idx - 1]).unwrap();
                let style = self
                    .name_styles
                    .get(idx - 1)
                    .copied()
                    .unwrap_or(Color::White);
                Paragraph::new(Span::styled(long_name, style).into_right_aligned_line())
                    .block(Block::bordered().border_set(border::EMPTY))
                    .render(*name_cell, buf);

                for (idx_x, (&cell, result)) in it.zip(table.row(idx - 1)).enumerate() {
                    result
                        .render(self.positions.contains(&(idx_x, idx - 1)))
                        .render(cell, buf);
                }
                let score_cell = cells.last().unwrap();
                Paragraph::new(Span::from(format!(
                    "{:>2} {:>4}",
                    scores[idx - 1],
                    format!("({})", max_scores[idx - 1])
                )))
                .block(Block::bordered().border_set(border::EMPTY))
                .render(*score_cell, buf);
            }
        }
    }
}

trait RenderResult {
    fn render(&self, highlighted: bool) -> Paragraph;
}

impl RenderResult for MatchResult {
    fn render(&self, highlighted: bool) -> Paragraph {
        let colour = match self {
            MatchResult::None => Color::White,
            MatchResult::Win => Color::Green,
            MatchResult::Loss => Color::Red,
            MatchResult::Draw => Color::Blue,
            MatchResult::NoContest => Color::Gray,
            MatchResult::SelfMatch => Color::Black,
        };

        Paragraph::new(
            Span::from(format!("{}", char::from(self).to_ascii_uppercase())).into_centered_line(),
        )
        // .bg(Color::Green)
        .block(Block::bordered().border_type(if highlighted {
            BorderType::Double
        } else {
            BorderType::Plain
        }))
        .fg(colour)
    }
}
