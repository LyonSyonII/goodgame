use std::{ffi::OsString, path::PathBuf};

use clap::{
    ValueHint,
    builder::{Styles, styling::AnsiColor},
};
use clap_complete::{ArgValueCandidates, ArgValueCompleter, CompletionCandidate};

const CLAP_STYLE: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Green.on_default());

#[derive(clap::Parser)]
#[clap(styles = CLAP_STYLE)]
pub enum Cli {
    /// Starts to manage the provided game.
    #[clap(alias = "a", alias = "init")]
    Add {
        #[arg(value_hint = ValueHint::AnyPath)]
        game: String,
        #[arg(value_hint = ValueHint::DirPath)]
        root: PathBuf,
        #[arg(value_hint = ValueHint::AnyPath)]
        save_location: PathBuf,
    },
    /// Deletes the game from the managed list.
    #[clap(alias = "del", alias = "remove", alias = "rm")]
    Delete {
        #[arg(value_parser = game_name_parser(), add = ArgValueCandidates::new(game_name_candidates))]
        game: String,
    },
    /// Creates a backup of the game.  
    ///
    /// If no game name is provided, one will try to be selected based on the current directory.  
    ///
    /// The backup is compressed and called "GAME_IDX" by default.  
    /// If a backup description is provided, the backup will be called "GAME_IDX_DESCRIPTION"
    #[clap(alias = "b")]
    Backup {
        #[arg(value_parser = game_name_parser(), add = ArgValueCandidates::new(game_name_candidates))]
        game: Option<String>,
        #[arg(long, short, value_hint = ValueHint::Other)]
        desc: Option<OsString>,
    },
    /// Lists all managed games.
    #[clap(alias = "l", alias = "ls")]
    List,
    /// Opens the root directory of the game.
    #[clap(alias = "o")]
    Open {
        #[arg(value_parser = game_name_parser(), add = ArgValueCandidates::new(game_name_candidates))]
        game: String,
    },
    /// Opens the save directory of the game.
    #[clap(alias = "os")]
    OpenSave {
        #[arg(value_parser = game_name_parser(), add = ArgValueCandidates::new(game_name_candidates))]
        game: String,
    },
}

fn game_name_parser() -> clap::builder::PossibleValuesParser {
    const N: usize = 5;
    let games = crate::Games::load().unwrap();
    let mut games = games
        .names()
        .into_iter()
        .map(str::to_owned)
        .take(N)
        .collect::<Vec<_>>();
    if games.len() == N {
        games.push(String::from("..."));
    }
    clap::builder::PossibleValuesParser::new(games)
}

fn game_name_candidates() -> Vec<clap_complete::CompletionCandidate> {
    crate::Games::load()
        .unwrap()
        .names()
        .into_iter()
        .map(CompletionCandidate::new)
        .collect()
}
