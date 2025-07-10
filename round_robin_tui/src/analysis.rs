use std::sync::atomic::AtomicBool;

use indexmap::IndexMap;
use indicatif::ProgressBar;
use ratatui::text::Text;
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
            NightAnalysis::Analysis(analyse_state_progress(
                &r,
                n_winners,
                progress_bar.clone(),
                &state.done,
            ))
        };
        all_results.push(possible_results);
    }
    Some((block.name, BlockAnalysis(all_results)))
}

impl BlockAnalysis {
    pub fn print_night(&self, lines: &mut Vec<Text>, night_idx: usize, names: &[String]) {
        let results = &self.0[night_idx];

        match results {
            NightAnalysis::Skipped => {
                lines.push("Night marked as skipped".into());
            }
            NightAnalysis::Future => {
                lines.push("Night has no results listed".into());
            }
            NightAnalysis::Analysis(results) => {
                for (top_guys, (scores, _)) in results.iter() {
                    let result_lines = Text::from_iter(
                        top_guys
                            .iter()
                            .zip(scores)
                            .map(|(&guy, &score)| format!("{}: {score}", names[guy])),
                    );
                    lines.push(result_lines);
                }
            }
        }
    }
}
