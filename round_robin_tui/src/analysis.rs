use std::{collections::HashMap, fmt::Write, sync::atomic::AtomicBool};

use indexmap::IndexMap;
use indicatif::ProgressBar;
use round_robin::{
    results::ResultsTable,
    solver::{analyse_state_bfs_progress, analyse_state_progress},
    tournament::{MatchSet, TournamentBlock},
};

#[derive(Debug)]
pub struct Analysis(pub Vec<IndexMap<Vec<usize>, (Vec<u32>, ResultsTable)>>);

#[derive(Default)]
pub struct ThreadState {
    pub done: AtomicBool,
}

pub fn analyse_tournament(
    block: TournamentBlock,
    n_winners: usize,
    state: &ThreadState,
    progress_bar: ProgressBar,
) -> Option<Analysis> {
    let competitor_map = &block.competitors;
    let competitor_vec: Vec<_> = competitor_map.iter().collect();

    let comp_ids: Vec<_> = competitor_vec.iter().map(|(k, _)| k.to_string()).collect();

    let mut r = ResultsTable::new(&comp_ids);

    let mut all_results = Vec::new();

    for night in block.nights.iter() {
        let skip = night.skip;

        r.apply_results(&night);

        if skip {
            continue;
        }

        let possible_results =
            analyse_state_progress(&r, n_winners, progress_bar.clone(), &state.done);
        all_results.push(possible_results);
    }
    Some(Analysis(all_results))
}

pub fn analyse_tournament_bfs(
    block: TournamentBlock,
    n_winners: usize,
    state: &ThreadState,
    progress_bar: ProgressBar,
) -> Option<Analysis> {
    let competitor_map = &block.competitors;
    let competitor_vec: Vec<_> = competitor_map.iter().collect();

    let comp_ids: Vec<_> = competitor_vec.iter().map(|(k, _)| k.to_string()).collect();

    let mut r = ResultsTable::new(&comp_ids);

    let mut all_results = Vec::new();

    for night in block.nights.iter() {
        let skip = night.skip;

        r.apply_results(&night);

        if skip {
            continue;
        }

        let possible_results =
            analyse_state_bfs_progress(&r, n_winners, progress_bar.clone(), &state.done);
        all_results.push(possible_results);
    }
    Some(Analysis(all_results))
}

impl Analysis {
    pub fn print_night(
        &self,
        to: &mut impl Write,
        night_idx: usize,
        night: &MatchSet,
        names: &[String],
        previous_results: ResultsTable,
        n_winners: usize,
    ) {
        let result = &self.0[night_idx];
        if matches!(night.print_summary, Some(false)) && !night.print_detailed {
            return;
        }
        if night.print_summary.unwrap_or(!night.print_detailed) {
            print_summary(to, result, names, n_winners);
        }
        if night.print_detailed {
            print_detailed(to, result, names, &previous_results);
        }
    }
}
fn print_summary(
    to: &mut impl Write,
    results: &IndexMap<Vec<usize>, (Vec<u32>, ResultsTable)>,
    long_names: &[String],
    n_winners: usize,
) {
    let mut placings: IndexMap<_, _> = long_names
        .iter()
        .map(|guy| (guy, vec![0usize; n_winners]))
        .collect();
    for result in results.keys() {
        for (i, &guy_idx) in result.iter().enumerate() {
            placings
                .entry(&long_names[guy_idx])
                .and_modify(|counts| counts[i] += 1);
        }
    }
    for (guy, ranks) in placings.iter() {
        writeln!(to, "{guy}:");
        if ranks.iter().sum::<usize>() > 0 {
            for (i, n) in ranks.iter().enumerate() {
                writeln!(to, "Place {}: {n} options", i + 1);
            }
        } else {
            writeln!(to, "  Completely eliminated");
        }
    }
}

fn print_detailed(
    to: &mut impl Write,
    results: &IndexMap<Vec<usize>, (Vec<u32>, ResultsTable)>,
    competitor_names: &[String],
    existing_results: &ResultsTable,
) {
    for (top_guys, (scores, scoreboard)) in results.iter() {
        for (&guy, &score) in top_guys.iter().zip(scores) {
            writeln!(to, "{}: {score}", competitor_names[guy]);
        }

        scoreboard.print_diff(to, existing_results, competitor_names);
        writeln!(to, "---------------------------------");
    }
}
