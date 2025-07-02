use clap::Parser;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
pub struct TournamentArgs {
    /// The path to the file to read
    pub tournament_file: std::path::PathBuf,
}
