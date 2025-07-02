use std::{
    fmt::{Debug, Display, Write},
    ops::{AddAssign, Not},
};

use itertools::Itertools;
use ndarray::{Array1, Array2, ArrayView2, Axis};

use crate::tournament::{Match, MatchSet, TournamentBlock};

#[derive(Debug, Clone, Copy)]
pub enum MatchResult {
    None,
    Win,
    Loss,
    Draw,
    NoContest,
    SelfMatch,
}

impl Not for MatchResult {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            MatchResult::Win => MatchResult::Loss,
            MatchResult::Loss => MatchResult::Win,
            _ => self,
        }
    }
}

impl Display for MatchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&char::from(self), f)
    }
}

impl From<&MatchResult> for u32 {
    fn from(value: &MatchResult) -> Self {
        match value {
            MatchResult::Win => 2,
            MatchResult::Draw => 1,
            _ => 0,
        }
    }
}

impl From<&MatchResult> for char {
    fn from(value: &MatchResult) -> Self {
        match value {
            MatchResult::None => 'o',
            MatchResult::Win => 'w',
            MatchResult::Loss => 'l',
            MatchResult::Draw => 'd',
            MatchResult::NoContest => 'z',
            MatchResult::SelfMatch => 'x',
        }
    }
}

impl TryFrom<char> for MatchResult {
    fn try_from(value: char) -> Result<Self, ()> {
        Ok(match value {
            'o' => MatchResult::None,
            'w' => MatchResult::Win,
            'l' => MatchResult::Loss,
            'd' => MatchResult::Draw,
            'z' => MatchResult::NoContest,
            'x' => MatchResult::SelfMatch,
            _ => Err(())?,
        })
    }

    type Error = ();
}

#[derive(Clone, Debug)]
pub struct ResultsTable {
    competitors: Vec<String>,
    table: Array2<MatchResult>,
}

impl Display for ResultsTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.table.map(char::from), f)
    }
}

impl ResultsTable {
    pub fn new<'a, C, S>(names: C) -> Self
    where
        S: ToString + 'a,
        C: IntoIterator<Item = &'a S>,
        C::IntoIter: ExactSizeIterator,
    {
        let names = names.into_iter();
        let n = names.len();
        let competitors = names.map(ToString::to_string).collect::<Vec<_>>();
        let table = Array2::from_shape_fn((n, n), |(i, j)| {
            if i == j {
                MatchResult::SelfMatch
            } else {
                MatchResult::None
            }
        });
        Self { competitors, table }
    }
}

impl ResultsTable {
    pub fn n_competitors(&self) -> usize {
        self.competitors.len()
    }
    pub fn scores(&self) -> Array1<u32> {
        let score_rows = self.table.map(u32::from);
        score_rows.sum_axis(Axis(1))
    }

    pub fn max_possible_scores(&self) -> Array1<u32> {
        self.scores()
            + self
                .number_remaining_per_competitor()
                .map(|&n| 2 * n as u32)
    }

    pub fn all_remaining_matches(&self) -> Array1<(usize, usize)> {
        self.table
            .indexed_iter()
            .filter_map(|((i, j), &res)| {
                if (i < j) && matches!(res, MatchResult::None) {
                    Some((i, j))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn number_remaining_per_competitor(&self) -> Array1<usize> {
        self.table
            .map(|r| if let MatchResult::None = r { 1 } else { 0 })
            .sum_axis(Axis(0))
    }

    pub fn table(&self) -> ArrayView2<MatchResult> {
        self.table.view()
    }

    pub fn names(&self) -> &[String] {
        &self.competitors[..]
    }
}

impl ResultsTable {
    pub fn win(&mut self, winner: usize, loser: usize) {
        self.table[(winner, loser)] = MatchResult::Win;
        self.table[(loser, winner)] = MatchResult::Loss;
    }

    pub fn draw(&mut self, draws: (usize, usize)) {
        self.table[draws] = MatchResult::Draw;
        self.table[(draws.1, draws.0)] = MatchResult::Draw;
    }

    pub fn apply_results(&mut self, night: &MatchSet) {
        for (Match(a, b), result) in night.matches.iter().zip(&night.results) {
            let a = self.names().iter().position(|n| n == a).unwrap();
            let b = self.names().iter().position(|n| n == b).unwrap();
            match result {
                MatchResult::Win => self.win(a, b),
                MatchResult::Loss => self.win(b, a),
                MatchResult::Draw => self.draw((a, b)),
                _ => {}
            }
        }
    }
    pub fn print_diff(&self, to: &mut impl Write, fixed: &Self, long_names: &[String]) {
        let maxlen = long_names.iter().map(String::len).max().unwrap_or(0) + 4;
        let scores = self.scores();
        for (((name, row), fixedrow), score) in long_names
            .iter()
            .zip(self.table.rows())
            .zip(fixed.table.rows())
            .zip(scores)
        {
            let scores_string = row
                .iter()
                .zip(fixedrow)
                .map(|(r, f)| {
                    if matches!(f, MatchResult::None) {
                        format!("\x1b[6;30;42m{}\x1b[0m", char::from(r))
                    } else {
                        format!("{}", char::from(r))
                    }
                })
                .join(" ");
            let _ = writeln!(to, "{:>maxlen$} {} {score:>6}", name, scores_string);
        }
    }
}

impl From<&TournamentBlock> for ResultsTable {
    fn from(block: &TournamentBlock) -> Self {
        let mut table = ResultsTable::new(block.competitors.keys());

        for night in &block.nights {
            table += night;
        }

        table
    }
}

impl AddAssign<&MatchSet> for ResultsTable {
    fn add_assign(&mut self, night: &MatchSet) {
        for (Match(a, b), &result) in night.matches.iter().zip(&night.results) {
            let a = self.names().iter().position(|n| n == a).unwrap();
            let b = self.names().iter().position(|n| n == b).unwrap();
            self.table[(a, b)] = result;
            self.table[(b, a)] = !result;
        }
    }
}
