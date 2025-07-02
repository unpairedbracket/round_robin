use ndarray::{Array1, Zip};
use petgraph::{prelude::DiGraphMap, visit::EdgeIndexable};

use crate::results::ResultsTable;

pub struct FlowNetwork {
    graph: DiGraphMap<NodeType, u32>,

    target_flow: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum NodeType {
    Source,
    Match(usize, usize),
    Competitor(usize),
    PreSink,
    Sink,
}

#[derive(Debug)]
pub enum FlowNetworkConstructionError {
    FixedHasMatches {
        competitor: usize,
    },
    TooManyPoints,
    NoSlack,
    NegativePreSinkCapacity,
    NotEnoughPointsAvailable {
        competitor: usize,
        needed: u32,
        available: u32,
    },
}

impl FlowNetwork {
    pub fn empty() -> FlowNetwork {
        let mut graph = DiGraphMap::new();
        graph.add_node(NodeType::Source);
        graph.add_node(NodeType::Sink);

        FlowNetwork {
            graph,
            target_flow: 0,
        }
    }
    pub fn new(
        results: &ResultsTable,
        lower_bounds: &Array1<u32>,
        upper_bounds: &Array1<u32>,
    ) -> Result<FlowNetwork, FlowNetworkConstructionError> {
        let mut this = Self::empty();
        this.graph.add_node(NodeType::PreSink);

        let scores = results.scores();
        let lower_bounds = Zip::from(lower_bounds)
            .and(&scores)
            .map_collect(|lb, score| *lb.max(score));

        let min_needed = lower_bounds - &scores; // demand

        if Zip::from(upper_bounds)
            .and(&scores)
            .any(|ub, score| ub < score)
        {
            Err(FlowNetworkConstructionError::TooManyPoints)?;
        }

        let max_needed = upper_bounds - &scores; // capacity

        if Zip::from(&max_needed)
            .and(&min_needed)
            .any(|max, min| max < min)
        {
            Err(FlowNetworkConstructionError::NoSlack)?;
        }

        let slack = &max_needed - &min_needed;

        let matches_left = results.all_remaining_matches();

        let mut pre_t_capacity = 2 * matches_left.len() as u32;
        this.target_flow = pre_t_capacity;

        if min_needed.sum() > pre_t_capacity {
            Err(FlowNetworkConstructionError::NegativePreSinkCapacity)?;
        }

        let matches_left_per_competitor = results.number_remaining_per_competitor();

        for (competitor, &remaining) in matches_left_per_competitor.indexed_iter() {
            if min_needed[competitor] > 2 * remaining as u32 {
                Err(FlowNetworkConstructionError::NotEnoughPointsAvailable {
                    competitor,
                    needed: min_needed[competitor],
                    available: 2 * remaining as u32,
                })?;
            }

            this.graph.add_node(NodeType::Competitor(competitor));

            pre_t_capacity -= min_needed[competitor];
            if min_needed[competitor] > 0 {
                this.graph.add_edge(
                    NodeType::Competitor(competitor),
                    NodeType::Sink,
                    min_needed[competitor],
                );
            }
            if slack[competitor] > 0 {
                this.graph.add_edge(
                    NodeType::Competitor(competitor),
                    NodeType::PreSink,
                    slack[competitor],
                );
            }
        }
        for (i, j) in matches_left {
            let this_matchup = NodeType::Match(i, j);
            this.graph.add_node(this_matchup);
            this.graph
                .add_edge(this_matchup, NodeType::Competitor(i), 2);
            this.graph
                .add_edge(this_matchup, NodeType::Competitor(j), 2);
            this.graph.add_edge(NodeType::Source, this_matchup, 2);
        }

        if pre_t_capacity > 0 {
            this.graph
                .add_edge(NodeType::PreSink, NodeType::Sink, pre_t_capacity);
        }

        Ok(this)
    }

    pub fn solve(&self, results_table: &ResultsTable) -> Option<ResultsTable> {
        let (max_flow, edge_flows) =
            petgraph::algo::ford_fulkerson(&self.graph, NodeType::Source, NodeType::Sink);
        if max_flow < self.target_flow {
            None
        } else {
            let mut solved_table = results_table.clone();
            for (source, target, _) in self.graph.all_edges() {
                match (source, target) {
                    (NodeType::Match(i, j), NodeType::Competitor(k)) => {
                        let idx = self.graph.to_index((source, target));
                        let flow = edge_flows[idx];
                        if i == k {
                            if flow == 2 {
                                solved_table.win(i, j);
                            } else if flow == 1 {
                                solved_table.draw((i, j));
                            }
                        } else if j == k {
                            if flow == 2 {
                                solved_table.win(j, i);
                            } else if flow == 1 {
                                solved_table.draw((i, j))
                            }
                        } else {
                            panic!("rogue edge from {source:?} to {target:?}");
                        }
                    }
                    _ => {}
                }
            }
            assert!(
                solved_table.number_remaining_per_competitor().sum() == 0,
                "Some matches were undecided"
            );
            Some(solved_table)
        }
    }
}
