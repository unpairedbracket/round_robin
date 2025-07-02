use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize},
    },
    thread::JoinHandle,
};

use indicatif::ProgressBar;
use round_robin::tournament::Tournament;

use crate::analysis::{Analysis, ThreadState};

#[derive(Default)]
pub struct App {
    pub data: Tournament,
    pub state: AppState,
    pub selected_block: usize,
    pub cached_analysis: Option<Analysis>,
    pub status_message: Option<String>,
    pub analysis_results: Vec<Analysis>,
}

impl From<Tournament> for App {
    fn from(data: Tournament) -> Self {
        App {
            data,
            ..Default::default()
        }
    }
}
#[derive(Default)]
pub enum AppState {
    #[default]
    BlocksEdit,
    CompetitorsEdit {
        competitor_index: usize,
        editing: Option<(NameType, (String, String))>,
    },
    NightsAssign {
        cursor_position: (usize, usize),
    },
    ResultsEdit {
        cursor_position: (usize, usize),
    },
    Analysis {
        results: AnalysisState,
    },
}

impl AppState {
    pub fn title(&self) -> &'static str {
        match self {
            AppState::BlocksEdit { .. } => "Add or Edit Blocks",
            AppState::CompetitorsEdit { .. } => "Edit Competitors",
            AppState::NightsAssign { .. } => "Assign Nights To Matches",
            AppState::ResultsEdit { .. } => "Edit Match Results",
            AppState::Analysis { .. } => "Analyse Predictions",
        }
    }

    pub fn forwards(&mut self) {
        use AppState::*;
        *self = match self {
            BlocksEdit => CompetitorsEdit {
                competitor_index: 0,
                editing: None,
            },
            CompetitorsEdit { .. } => NightsAssign {
                cursor_position: (1, 0),
            },
            NightsAssign { .. } => ResultsEdit {
                cursor_position: (1, 0),
            },
            ResultsEdit { .. } => Analysis {
                results: AnalysisState::NoAnalysis,
            },
            Analysis { .. } => return,
        }
    }

    pub fn backwards(&mut self) {
        use AppState::*;
        *self = match self {
            BlocksEdit => return,
            CompetitorsEdit { .. } => BlocksEdit,
            NightsAssign { .. } => CompetitorsEdit {
                competitor_index: 0,
                editing: None,
            },
            ResultsEdit { .. } => NightsAssign {
                cursor_position: (1, 0),
            },
            Analysis { .. } => ResultsEdit {
                cursor_position: (1, 0),
            },
        }
    }
}

pub enum AnalysisState {
    NoAnalysis,
    Analysing {
        state: Arc<ThreadState>,
        handle: JoinHandle<Option<Analysis>>,
        progress_bar: ProgressBar,
    },
    Complete {
        analysis: Analysis,
        scroll_position: usize,
        night_number: usize,
    },
}

pub enum NameType {
    LongName,
    ShortName,
}
