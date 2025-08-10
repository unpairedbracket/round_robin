use std::sync::atomic::AtomicBool;

use indexmap::IndexMap;
use indicatif::ProgressBar;
use itertools::Itertools;
use ratatui::{
    style::Color,
    text::{Line, Span, Text},
};
use round_robin::{
    results::ResultsTable, solver::analyse_state_progress, tournament::TournamentBlock,
};

#[derive(Debug)]
pub enum NightAnalysis {
    Skipped,
    Future,
    Analysis(IndexMap<Vec<usize>, (Vec<u32>, ResultsTable)>),
}

#[derive(Debug)]
pub struct BlockAnalysis(pub Vec<NightAnalysis>);

#[derive(Default)]
pub struct ThreadState {
    pub done: AtomicBool,
}

pub fn analyse_tournament(
    block: TournamentBlock,
    n_winners: usize,
    state: &ThreadState,
    progress_bar: ProgressBar,
) -> Option<(String, BlockAnalysis)> {
    let competitor_map = &block.competitors;
    let competitor_vec: Vec<_> = competitor_map.iter().collect();

    let comp_ids: Vec<_> = competitor_vec.iter().map(|(k, _)| k.to_string()).collect();

    let mut r = ResultsTable::new(&comp_ids);

    let mut all_results = Vec::new();

    for night in block.nights.iter() {
        r.apply_results(&night);

        let possible_results = if night.skip {
            NightAnalysis::Skipped
        } else if night.future() {
            NightAnalysis::Future
        } else {
            let mut night =
                analyse_state_progress(&r, n_winners, progress_bar.clone(), &state.done);
            night.sort_unstable_keys();
            NightAnalysis::Analysis(night)
        };
        all_results.push(possible_results);
    }
    Some((block.name, BlockAnalysis(all_results)))
}

impl NightAnalysis {
    pub fn summary(&self) -> String {
        match self {
            NightAnalysis::Skipped => "skipped".into(),
            NightAnalysis::Future => "future".into(),
            NightAnalysis::Analysis(results) => format!("{} permutations possible", results.len()),
        }
    }

    pub fn print_night(&self, lines: &mut Vec<Text>, names: &[String]) {
        match self {
            NightAnalysis::Skipped => {
                lines.push("Night marked as skipped".into());
            }
            NightAnalysis::Future => {
                lines.push("Night has no results listed".into());
            }
            NightAnalysis::Analysis(results) => {
                let medal_colours = [
                    Color::Rgb(175, 149, 0),
                    Color::Rgb(180, 180, 180),
                    Color::Rgb(173, 138, 86),
                ];
                let colour_iter = medal_colours.iter().chain(std::iter::repeat(&Color::White));
                let max_len = names.iter().map(|n| n.len()).max().unwrap_or(0) + 1;
                for (top_guys, (scores, _)) in results.iter() {
                    let result_line = Line::from_iter(
                        top_guys
                            .iter()
                            .zip(scores)
                            .zip(colour_iter.clone())
                            .map(|((&guy, &score), &color)| {
                                Span::styled(format!("{:>max_len$}: {score:<3}", names[guy]), color)
                            })
                            .intersperse(Span::from("|")),
                    );
                    lines.push(result_line.into());
                }
            }
        }
    }

    pub fn best_positions(&self, n: usize) -> Vec<usize> {
        let mut result = vec![usize::MAX; n];

        if let NightAnalysis::Analysis(results) = self {
            for (top_guys, _) in results {
                for (place, &guy) in top_guys.iter().enumerate() {
                    if result[guy] > place {
                        result[guy] = place
                    }
                }
            }
        }

        result
    }
}

pub fn colour_for_position(n: usize) -> Color {
    const MEDAL_COLOURS: [Color; 3] = [
        Color::Rgb(175, 149, 0),
        Color::Rgb(180, 180, 180),
        Color::Rgb(173, 138, 86),
    ];

    MEDAL_COLOURS.get(n).copied().unwrap_or(Color::LightRed)
}
