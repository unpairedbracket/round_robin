use ratatui::{
    style::Color,
    text::{Line, Span, Text},
};
use round_robin::{
    results::MatchResult,
    tournament::{Match, TournamentBlock},
};

pub trait TerminalRenderBlock {
    fn render_matches(&self) -> Vec<Text>;
    fn render_results(&self) -> Vec<Text>;
}

impl TerminalRenderBlock for TournamentBlock {
    fn render_matches(&self) -> Vec<Text> {
        let mut texts = Vec::new();
        let max_name_length = self
            .competitors
            .values()
            .map(|s| s.len())
            .max()
            .unwrap_or(0);
        for (night_index, night) in self.nights.iter().enumerate() {
            let mut lines = Vec::new();
            lines.push(Line::from(format!("Night {}:", night_index + 1)));
            for Match(a, b) in &night.matches {
                let name_a = &self.competitors[a];
                let name_b = &self.competitors[b];
                lines.push(Line::raw(format!(
                    "  {name_a:>max_name_length$} vs. {name_b}"
                )));
            }
            texts.push(lines.into());
        }
        texts
    }

    fn render_results(&self) -> Vec<Text> {
        let mut texts = Vec::new();

        let max_name_length = self
            .competitors
            .values()
            .map(|s| s.len())
            .max()
            .unwrap_or(0);
        for (night_index, night) in self.nights.iter().enumerate() {
            let mut lines = Vec::new();
            lines.push(Line::from(format!("Night {}:", night_index + 1)));
            for (Match(a, b), result) in night.matches.iter().zip(&night.results) {
                let name_a = &self.competitors[a];
                let name_b = &self.competitors[b];
                let (left_colr, result_char, right_colr) = match result {
                    MatchResult::None => (Color::White, 'v', Color::White),
                    MatchResult::Win => (Color::Green, '>', Color::Red),
                    MatchResult::Loss => (Color::Red, '<', Color::Green),
                    MatchResult::Draw => (Color::Blue, '-', Color::Blue),
                    MatchResult::NoContest | MatchResult::SelfMatch => {
                        (Color::Gray, 'x', Color::Gray)
                    }
                };

                lines.push(Line::from_iter([
                    Span::styled(format!("  {name_a:>max_name_length$}"), left_colr),
                    Span::raw(format!(" {result_char} ")),
                    Span::styled(name_b, right_colr),
                ]));
            }
            texts.push(lines.into());
        }
        texts
    }
}
