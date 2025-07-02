use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::results::MatchResult;

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    pub matches: Vec<Match>,
    #[serde(default)]
    pub results: Vec<MatchResult>,
}

impl TournamentBlock {
    pub fn relabel_competitor(&mut self, old_short_name: &str, new_short_name: &str) {
        for night in &mut self.nights {
            for Match(a, b) in &mut night.matches {
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

impl Serialize for MatchResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        char::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MatchResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        char::deserialize(deserializer).and_then(|ch| {
            ch.try_into().map_err(|_| {
                D::Error::unknown_variant(&ch.to_string(), &["o", "w", "l", "d", "z", "x"])
            })
        })
    }
}

impl TournamentBlock {
    pub fn find_result(&self, first: &str, second: &str) -> Option<(usize, usize, bool)> {
        for (night_id, night) in &mut self.nights.iter().enumerate() {
            for (match_id, Match(a, b)) in night.matches.iter().enumerate() {
                if (first, second) == (&a, &b) {
                    return Some((night_id, match_id, false));
                }
                if (first, second) == (&b, &a) {
                    return Some((night_id, match_id, true));
                }
            }
        }
        None
    }

    pub fn find_result_by_indices(
        &self,
        first: usize,
        second: usize,
    ) -> Option<(usize, usize, bool)> {
        let (first_name, _) = self.competitors.get_index(first)?;
        let (second_name, _) = self.competitors.get_index(second)?;
        self.find_result(first_name, second_name)
    }
}
