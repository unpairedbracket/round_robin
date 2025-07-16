use std::{
    collections::HashMap,
    fs::{read_to_string, write},
    path::{Path, PathBuf},
    sync::Arc,
    thread::JoinHandle,
};

use color_eyre::eyre;
use indicatif::ProgressBar;
use ratatui::widgets::ListState;
use round_robin::tournament::Tournament;

use crate::analysis::{BlockAnalysis, ThreadState};

#[derive(Default)]
pub struct App {
    pub path: PathBuf,
    pub data: Tournament,
    pub state: AppState,
    pub selected_block: usize,
    pub status_message: Option<String>,
    pub analysis_results: HashMap<String, BlockAnalysis>,
}

fn load_tournament(path: impl AsRef<Path>) -> Option<Tournament> {
    let contents = read_to_string(path).ok()?;
    toml::from_str(&contents).ok()?
}

fn save_tournament(path: impl AsRef<Path>, tournament: &Tournament) -> Result<(), eyre::Error> {
    let contents = toml::to_string(tournament)?;
    write(path, contents)?;
    Ok(())
}

impl App {
    pub fn initialise_from_file(path: PathBuf) -> Self {
        let tournament = match load_tournament(&path) {
            Some(t) => t,
            None => {
                let mut t = Tournament::default();
                t.number_advance = 1;
                t
            }
        };

        let mut app = App {
            path,
            data: tournament,
            ..Default::default()
        };
        if app.data.block.len() == 0 {
            app.state = AppState::BlocksEdit {
                editing: Some("New Block".to_string()),
                delete_warning: false,
            };
        }
        app
    }

    pub fn save(&self) -> Result<(), eyre::Error> {
        save_tournament(&self.path, &self.data)
    }
}

pub enum AppState {
    Quitting,
    BlocksEdit {
        editing: Option<String>,
        delete_warning: bool,
    },
    CompetitorsEdit {
        competitor_index: usize,
        editing: Option<(NameType, (String, String))>,
        delete_warning: bool,
    },
    NightsAssign {
        cursor_position: (usize, usize),
        list_state: ListState,
    },
    ResultsEdit {
        cursor_position: (usize, usize),
        list_state: ListState,
    },
    Analysis {
        results: AnalysisState,
    },
}

impl Default for AppState {
    fn default() -> Self {
        AppState::BlocksEdit {
            editing: None,
            delete_warning: false,
        }
    }
}

impl AppState {
    pub fn title(&self) -> &'static str {
        match self {
            AppState::Quitting { .. } => "Quitting",
            AppState::BlocksEdit { .. } => "Add or Edit Blocks",
            AppState::CompetitorsEdit { .. } => "Edit Competitors",
            AppState::NightsAssign { .. } => "Assign Nights To Matches",
            AppState::ResultsEdit { .. } => "Edit Match Results",
            AppState::Analysis { .. } => "Analyse Predictions",
        }
    }
    pub fn prev_title(&self) -> Option<&'static str> {
        Some(match self {
            AppState::CompetitorsEdit { .. } => " (Shift + Tab) Blocks",
            AppState::NightsAssign { .. } => " (Shift + Tab) Competitors",
            AppState::ResultsEdit { .. } => " (Shift + Tab) Schedule",
            AppState::Analysis { .. } => " (Shift + Tab) Results",
            _ => None?,
        })
    }
    pub fn next_title(&self) -> Option<&'static str> {
        Some(match self {
            AppState::BlocksEdit { .. } => "Competitors (Tab) ",
            AppState::CompetitorsEdit { .. } => "Schedule (Tab) ",
            AppState::NightsAssign { .. } => "Results (Tab) ",
            AppState::ResultsEdit { .. } => "Analysis (Tab) ",
            _ => None?,
        })
    }
    pub fn instructions(&self) -> &'static str {
        match self {
            AppState::Quitting => {
                "(Y) Save and quit | (N) Quit without saving | (Esc) Don't quit actually"
            }
            AppState::BlocksEdit {
                editing,
                delete_warning,
            } => {
                if *delete_warning {
                    "Delete block? This will result in loss of all the block's data (⌫  Delete)"
                } else if editing.is_some() {
                    "(⏎) Submit changes | (Esc) Reject changes"
                } else {
                    "(▲ ▼) Select block | (Shift + ▲ ▼) Move block | (⏎) Edit name | (Ctrl + n) New block | (⌫ ) Delete block"
                }
            }
            AppState::CompetitorsEdit {
                delete_warning,
                editing,
                ..
            } => {
                if *delete_warning {
                    "Delete competitor? This will remove all match listings and results involving them (⌫  Delete)"
                } else if editing.is_some() {
                    "(◄ ►) Switch Short/Long name | (⏎) Submit changes | (Esc) Reject changes"
                } else {
                    "(▲ ▼) Select competitor | (Shift + ▲ ▼) Move competitor | (⏎) Edit name | (Ctrl + n) New competitor | (⌫ ) Delete competitor"
                }
            }
            AppState::NightsAssign { .. } => {
                "(◄ ▲ ▼ ►) Select match | (-+) Move earlier/later | (1-9) Set night | (⌫ ) Unassign night | (Shift + ▲ ▼) Select nights"
            }
            AppState::ResultsEdit { .. } => {
                "(◄ ▲ ▼ ►) Select match | (w) Win | (l) Loss | (d) Draw | (z) No-contest | (o) Reset | (Shift + ▲ ▼) Select nights"
            }
            AppState::Analysis { .. } => {
                "(⏎) Run analysis | (◄ ►) Change night | (▲ ▼) Scroll results | (⌫ ) Return to results table so far | (+-) Change number of winners"
            }
        }
    }

    pub fn forwards(&mut self) {
        use AppState::*;
        *self = match self {
            Quitting => Quitting,
            BlocksEdit { .. } => CompetitorsEdit {
                competitor_index: 0,
                editing: None,
                delete_warning: false,
            },
            CompetitorsEdit { .. } => NightsAssign {
                cursor_position: (1, 0),
                list_state: ListState::default().with_selected(Some(0)),
            },
            NightsAssign { .. } => ResultsEdit {
                cursor_position: (1, 0),
                list_state: ListState::default().with_selected(Some(0)),
            },
            ResultsEdit { .. } => Analysis {
                results: AnalysisState::Display {
                    night_number: 0,
                    list_state: ListState::default(),
                },
            },
            Analysis { .. } => return,
        }
    }

    pub fn backwards(&mut self) {
        use AppState::*;
        *self = match self {
            Quitting => Quitting,
            BlocksEdit { .. } => return,
            CompetitorsEdit { .. } => BlocksEdit {
                editing: None,
                delete_warning: false,
            },
            NightsAssign { .. } => CompetitorsEdit {
                competitor_index: 0,
                editing: None,
                delete_warning: false,
            },
            ResultsEdit { .. } => NightsAssign {
                cursor_position: (1, 0),
                list_state: ListState::default().with_selected(Some(0)),
            },
            Analysis { .. } => ResultsEdit {
                cursor_position: (1, 0),
                list_state: ListState::default().with_selected(Some(0)),
            },
        }
    }
}

pub enum AnalysisState {
    Analysing {
        state: Arc<ThreadState>,
        handle: JoinHandle<Option<(String, BlockAnalysis)>>,
        progress_bar: ProgressBar,
    },
    Display {
        night_number: usize,
        list_state: ListState,
    },
}

pub enum NameType {
    LongName,
    ShortName,
}
