use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Tournament {
    pub number_advance: usize,
    pub block: Vec<TournamentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentBlock {
    pub name: String,
    #[serde(default)]
    pub competitors: IndexMap<String, String>,
    #[serde(default)]
    pub nights: Vec<MatchSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchSet {
    #[serde(default)]
    pub skip: bool,
    #[serde(default)]
    pub plot: bool,
    #[serde(default)]
    pub print_detailed: bool,
    #[serde(default)]
    pub print_summary: Option<bool>,
    #[serde(default)]
    pub wins: Vec<Match>,
    #[serde(default)]
    pub draws: Vec<Match>,
}

impl TournamentBlock {
    pub fn relabel_competitor(&mut self, old_short_name: &str, new_short_name: &str) {
        for night in &mut self.nights {
            for Match(winner, loser) in &mut night.wins {
                if winner == old_short_name {
                    *winner = new_short_name.to_string();
                }
                if loser == old_short_name {
                    *loser = new_short_name.to_string();
                }
            }
            for Match(a, b) in &mut night.draws {
                if a == old_short_name {
                    *a = new_short_name.to_string();
                }
                if b == old_short_name {
                    *b = new_short_name.to_string();
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match(pub String, pub String);

#[cfg(test)]
mod test {
    use super::Tournament;
    use std::fs::read_to_string;

    #[test]
    fn load_tournament() {
        let contents = read_to_string("G1_2024.toml").unwrap();
        let tourney: Tournament = toml::from_str(&contents).unwrap();
        println!("{tourney:#?}")
    }
}
