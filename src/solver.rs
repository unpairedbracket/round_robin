use std::sync::atomic::{AtomicBool, Ordering};

use indexmap::IndexMap;
use indicatif::{ParallelProgressIterator as _, ProgressBar, ProgressFinish, ProgressStyle};
use itertools::Itertools;
use ndarray::{Array1, Axis, Zip, concatenate};
use rayon::iter::{ParallelBridge, ParallelIterator as _};

use crate::{
    partitions::{PartitionClass, partitions},
    results::{MatchResult, ResultsTable},
    solver::flow_network::FlowNetwork,
};

mod flow_network;

pub fn analyse_state(
    current_results: &ResultsTable,
    n_winners: usize,
) -> IndexMap<Vec<usize>, (Vec<u32>, ResultsTable)> {
    let bar = ProgressBar::no_length()
        .with_style(
            ProgressStyle::with_template(
                "{msg}: {percent}%|{wide_bar}| {pos}/{len} [{elapsed:5}<{eta},{per_sec}]",
            )
            .unwrap(),
        )
        .with_finish(ProgressFinish::Abandon);
    let cancel = AtomicBool::new(false);
    analyse_state_progress(current_results, n_winners, bar, &cancel)
}

pub fn analyse_state_progress(
    current_results: &ResultsTable,
    n_winners: usize,
    bar: ProgressBar,
    done: &AtomicBool,
) -> IndexMap<Vec<usize>, (Vec<u32>, ResultsTable)> {
    let n_competitors = current_results.n_competitors();
    let competitor_ids = 0..n_competitors;
    let mut all_partitions = partitions(n_competitors, n_winners);
    all_partitions.sort_by_cached_key(|pc| {
        let mut unique_scores = pc.strict_winner_scores.clone();
        unique_scores.dedup();
        (
            *pc.augmentation_range.start(),
            pc.strict_winner_scores.len() - unique_scores.len(),
        )
    });
    let current_scores = current_results.scores();
    let max_scores = current_results.max_possible_scores();

    let n_perms: usize = (n_competitors - n_winners + 1..=n_competitors).product();
    bar.reset();
    bar.set_length(n_perms as u64);

    let possible_results = competitor_ids
        .permutations(n_winners)
        .par_bridge()
        .progress_with(bar)
        .filter_map(|strict_winner_set| {
            if done.load(Ordering::Relaxed) {
                return None;
            }

            let scores_table = check_top_permutation(
                current_results,
                &strict_winner_set,
                &all_partitions,
                &current_scores,
                &max_scores,
            )?;

            Some((strict_winner_set, scores_table))
        })
        .collect();
    possible_results
}

pub fn analyse_state_bfs(
    current_results: &ResultsTable,
    n_winners: usize,
) -> IndexMap<Vec<usize>, (Vec<u32>, ResultsTable)> {
    let bar = ProgressBar::no_length()
        .with_style(
            ProgressStyle::with_template(
                "{msg}: {percent}%|{wide_bar}| {pos}/{len} [{elapsed:5}<{eta},{per_sec}]",
            )
            .unwrap(),
        )
        .with_finish(ProgressFinish::Abandon);
    let cancel = AtomicBool::new(false);
    analyse_state_bfs_progress(current_results, n_winners, bar, &cancel)
}

pub fn analyse_state_bfs_progress(
    current_results: &ResultsTable,
    max_depth: usize,
    bar: ProgressBar,
    done: &AtomicBool,
) -> IndexMap<Vec<usize>, (Vec<u32>, ResultsTable)> {
    let n_competitors = current_results.n_competitors();
    let mut prev_depth_winner_sets =
        IndexMap::from_iter([(Vec::new(), (Vec::new(), current_results.clone()))]);
    let current_scores = current_results.scores();
    let max_scores = current_results.max_possible_scores();

    for current_depth in 1..=max_depth {
        let mut all_partitions = partitions(n_competitors, current_depth);
        all_partitions.sort_by_cached_key(|pc| {
            let mut unique_scores = pc.strict_winner_scores.clone();
            unique_scores.dedup();
            (
                *pc.augmentation_range.start(),
                pc.strict_winner_scores.len() - unique_scores.len(),
            )
        });

        let new_strict_winners_iter = prev_depth_winner_sets.keys().flat_map(|prev_set| {
            (0..n_competitors)
                .filter(|c| !prev_set.contains(c))
                .map(|c| {
                    let mut new_set = prev_set.clone();
                    new_set.push(c);
                    new_set
                })
        });

        bar.reset();
        bar.set_length((prev_depth_winner_sets.len() * (1 + n_competitors - current_depth)) as u64);

        prev_depth_winner_sets = new_strict_winners_iter
            .par_bridge()
            .progress_with(bar.clone())
            .flat_map(|strict_winners| {
                if done.load(Ordering::Relaxed) {
                    None?;
                }

                let scores_table = check_top_permutation(
                    current_results,
                    &strict_winners,
                    &all_partitions,
                    &current_scores,
                    &max_scores,
                )?;

                Some((strict_winners, scores_table))
            })
            .collect();
    }
    prev_depth_winner_sets
}

fn check_top_permutation(
    current_results: &ResultsTable,
    strict_winner_set: &[usize],
    all_partitions: &[PartitionClass],
    current_scores: &Array1<u32>,
    max_scores: &Array1<u32>,
) -> Option<(Vec<u32>, ResultsTable)> {
    let others = (0..current_results.n_competitors())
        .filter(|c| !strict_winner_set.contains(c))
        .collect::<Vec<_>>();
    for n_tied_losers in 0usize..=others.len() {
        for score_partition_class in all_partitions {
            if *score_partition_class.augmentation_range.start() > n_tied_losers {
                break;
            } else if *score_partition_class.augmentation_range.end() < n_tied_losers {
                continue;
            };
            let scores_table = check_score_partition(
                current_results,
                score_partition_class,
                strict_winner_set,
                &current_scores,
                &max_scores,
                &others,
                n_tied_losers,
            );
            if scores_table.is_some() {
                return scores_table;
            }
        }
    }
    None
}

fn check_score_partition(
    current_results: &ResultsTable,
    score_partition_class: &PartitionClass,
    strict_winner_set: &[usize],
    current_scores: &Array1<u32>,
    max_scores: &Array1<u32>,
    other_competitors: &[usize],
    n_tied_losers: usize,
) -> Option<(Vec<u32>, ResultsTable)> {
    if Zip::from(strict_winner_set)
        .and(&score_partition_class.strict_winner_scores)
        .any(|&player, &req| (current_scores[player] > req) | (max_scores[player] < req))
    {
        return None;
    }

    let repeatable_score = *score_partition_class.strict_winner_scores.last().unwrap();

    let required_scores = score_partition_class.required(n_tied_losers);

    for tied_losers in other_competitors
        .iter()
        .copied()
        .filter(|&competitor_idx| {
            current_scores[competitor_idx] <= repeatable_score
                && max_scores[competitor_idx] >= repeatable_score
        })
        .combinations(n_tied_losers)
    {
        if let Some(scores_table) = check_augmented_winner_set(
            current_results.clone(),
            &required_scores,
            &strict_winner_set,
            &tied_losers,
        ) {
            return Some(scores_table);
        }
    }

    None
}

fn check_augmented_winner_set(
    mut results: ResultsTable,
    required_scores: &[u32],
    strict_winner_set: &[usize],
    tied_losers: &[usize],
) -> Option<(Vec<u32>, ResultsTable)> {
    let n_competitors = results.n_competitors(); // N
    let n_strict_winners = strict_winner_set.len(); // M

    let augmented_winner_set = concatenate![Axis(0), strict_winner_set, tied_losers];

    let n_augmented_winners = augmented_winner_set.len();

    // Set up necessary wins to avoid condorcet cycles
    // within sets of people with the same number of points
    for j in 0..n_strict_winners {
        // print(f"j={j}")
        for k in (j + 1)..n_augmented_winners {
            // print(f'k={k}')
            if required_scores[k] == required_scores[j] {
                let cj = augmented_winner_set[j];
                let ck = augmented_winner_set[k];
                // j and k tie on final score so j must beat k
                match results.table()[(cj, ck)] {
                    MatchResult::Win => {
                        // print(f"{cj} already beats {ck}")
                    }
                    MatchResult::None => {
                        // print(f"{cj} hasn't fought {ck} yet, setting to win")
                        results.win(cj, ck)
                    }
                    MatchResult::Loss => {
                        // print(f"{cj} lost to {ck}, aborting")
                        return None;
                    }
                    MatchResult::Draw => {
                        // print(f"{cj} drew against {ck}, aborting")
                        return None;
                    }
                    MatchResult::SelfMatch => {
                        panic!("error: {cj} and {ck} seem to be the same person??")
                    }
                    MatchResult::NoContest => {
                        // print(f"{cj} had a no-contest with {ck}, aborting")
                        return None;
                    }
                }
            } else {
                // required_scores[k] < required_scores[j] (scores are monotonically non-increasing)
                // We're out of the drawn set for competitor j now
                break;
            }
        }
    }
    // println!("Augmented results for {required_scores:?}");
    // println!("{results}");
    let modified_scores = results.scores();
    let modified_max = results.max_possible_scores();
    let mut lower_bounds = modified_scores.clone();
    let mut upper_bounds = Array1::from_elem(n_competitors, required_scores.last().unwrap() - 1);

    // now the draws are settled, check we can satisfy the required scores
    Zip::from(&augmented_winner_set)
        .and(required_scores)
        .for_each(|&competitor, &req| {
            // no slack for these guys, you must have _exactly_ the required score
            lower_bounds[competitor] = req;
            upper_bounds[competitor] = req;
        });

    if Zip::from(&lower_bounds)
        .and(&upper_bounds)
        .any(|lb, ub| lb > ub)
    {
        return None;
    }
    if Zip::from(&modified_scores)
        .and(&upper_bounds)
        .any(|sc, ub| sc > ub)
    {
        return None;
    }
    if Zip::from(&modified_max)
        .and(&lower_bounds)
        .any(|max, lb| max < lb)
    {
        return None;
    }
    if Zip::from(&lower_bounds)
        .and(&modified_scores)
        .fold(0, |acc, &lb, &sc| acc + lb.saturating_sub(sc))
        > results.number_remaining_per_competitor().sum() as u32
    {
        return None;
    }

    // we can finally make the graph now
    // println!("making network for {required_scores:?}");
    let g = FlowNetwork::new(&results, &lower_bounds, &upper_bounds).unwrap();
    // println!("solving network");
    // let t0 = std::time::Instant::now();
    if let Some(solved_results) = g.solve(&results) {
        // println!("success in {:?}", t0.elapsed());
        // success!
        // println!("success!");
        return Some((Vec::from(required_scores), solved_results));
    }
    // println!("failed!");
    // println!("failed in {:?}", t0.elapsed());
    None
}
