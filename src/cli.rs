use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use clap::{
    ValueHint,
    builder::{PossibleValue, Styles, ValueParser, styling::AnsiColor},
};
use clap_complete::{ArgValueCandidates, ArgValueCompleter, CompletionCandidate};
use goodgame::{Game, Games};

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
        #[arg(add = game_name_candidates())]
        game: String,
    },
    /// Creates a backup of the game.  
    ///
    /// If no game name is provided, one will try to be selected based on the current directory.  
    ///
    /// The backup is compressed and called "GAME-IDX" by default.  
    /// If a backup description is provided, the backup will be called "GAME-IDX-DESCRIPTION"
    #[clap(alias = "b", alias = "bk")]
    Backup {
        #[arg(add = game_name_candidates())]
        game: Option<String>,
        #[arg(long, short, value_hint = ValueHint::Other)]
        desc: Option<String>,
    },
    /// Restores the selected backup.
    ///
    /// A backup of the current save will be created.
    #[clap(alias = "r", alias = "rs")]
    Restore {
        #[arg(add = game_name_candidates())]
        game: String,
        #[arg(add = game_backup_candidates(), requires = "game")]
        backup: String,
    },
    /// Lists all managed games.
    #[clap(alias = "l", alias = "ls")]
    List,
    /// Opens the root directory of the game.
    #[clap(alias = "o")]
    Open {
        #[arg(add = game_name_candidates())]
        game: String,
    },
    /// Opens the save directory of the game.
    #[clap(alias = "os")]
    OpenSave {
        #[arg(add = game_name_candidates())]
        game: String,
    },
}

static GAMES: std::sync::LazyLock<Games> = std::sync::LazyLock::new(|| Games::load().unwrap());

fn possible_game_names() -> impl IntoIterator<Item = &'static str> {
    GAMES.names()
}
fn game_name_candidates() -> ArgValueCandidates {
    ArgValueCandidates::new(|| {
        possible_game_names()
            .into_iter()
            .map(CompletionCandidate::new)
            .collect::<Vec<_>>()
    })
}

fn game_backup_candidates() -> ArgValueCandidates {
    let Some(game) = std::env::args()
        .rfind(|a| !a.is_empty())
        .and_then(|chosen| GAMES.get_by_name(chosen).ok())
    else {
        return ArgValueCandidates::new(Vec::new);
    };

    ArgValueCandidates::new(|| {
        game.backups_path()
            .read_dir()
            .unwrap()
            .flatten()
            .map(|f| f.file_name().to_string_lossy().into_owned())
            .map(CompletionCandidate::new)
            .collect()
    })
}
