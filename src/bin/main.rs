mod args;

use std::fs::read_to_string;

use clap::Parser as _;
use indexmap::IndexMap;
use itertools::Itertools;
use round_robin::{results::ResultsTable, solver::analyse_state, tournament::Tournament};

use crate::args::TournamentArgs;

#[test]
fn test_partitions() {
    let mut total_cases = 0;
    let mut case_classes = 0;
    for class in round_robin::partitions::partitions(10, 3) {
        // println!("{p:?} + {m:?}");
        case_classes += 1;
        total_cases += class.n_cases();
    }
    println!("cases: {total_cases} in {case_classes} classes");
}

#[test]
fn test_empty() {
    let names = &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
    let r = ResultsTable::new(names);
    let results = analyse_state(&r, 3);
    println!("{results:?}")
}

fn main() {
    let args = TournamentArgs::parse();
    let contents = read_to_string(args.tournament_file).unwrap();
    let g1: Tournament = toml::from_str(&contents).unwrap();
    analyse_tournament(g1);
}

fn analyse_tournament(g1: Tournament) {
    let n_winners = g1.number_advance;

    for block in &g1.block {
        let competitor_map = &block.competitors;
        let competitor_vec: Vec<_> = competitor_map.iter().collect();

        let comp_ids: Vec<_> = competitor_vec.iter().map(|(k, _)| k.to_string()).collect();
        let competitors: Vec<_> = competitor_vec.iter().map(|(_, v)| v.to_string()).collect();

        let mut r = ResultsTable::new(&comp_ids);

        for (night_number, night) in block.nights.iter().enumerate() {
            let skip = night.skip;
            let do_print_detailed = night.print_detailed;
            let do_print_summary = night.print_summary.unwrap_or(!do_print_detailed);
            let _do_plot = night.plot;

            r.apply_results(&night);
            println!("{r}");

            if skip {
                continue;
            }
            println!("After night {}:", night_number + 1);

            let possible_results = analyse_state(&r, n_winners);

            if do_print_summary {
                print_summary(&possible_results, &competitors, n_winners)
            }
            if do_print_detailed {
                print_detailed(&possible_results, &competitors, &r)
            }

            // if do_plot:
            //     result_matrix = np.zeros(n_winners * (n_competitors,)) + 3.5
            //     for result, (scores, table) in possible_results.items():
            //         biggest_draw = max(Counter(scores).values())
            //         if biggest_draw == 1:
            //             result_matrix[result] = 2.5
            //         elif biggest_draw == 2:
            //             result_matrix[result] = 0.5
            //         elif biggest_draw > 2:
            //             result_matrix[result] = 1.5
        }
    }
}

fn print_summary(
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
        println!("{guy}:");
        if ranks.iter().sum::<usize>() > 0 {
            for (i, n) in ranks.iter().enumerate() {
                println!("Place {}: {n} options", i + 1);
            }
        } else {
            println!("  Completely eliminated");
        }
    }
}

fn print_detailed(
    results: &IndexMap<Vec<usize>, (Vec<u32>, ResultsTable)>,
    competitor_names: &[String],
    existing_results: &ResultsTable,
) {
    for (top_guys, (scores, scoreboard)) in results.iter() {
        let mut result_string = top_guys
            .iter()
            .zip(scores)
            .map(|(&guy, &score)| format!("{}: {score}", competitor_names[guy]))
            .join("\n");
        result_string.push('\n');
        scoreboard.print_diff(&mut result_string, existing_results, competitor_names);
        println!("{result_string}");
        println!("---------------------------------");
    }
}
