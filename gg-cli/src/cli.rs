use std::{
    ffi::OsString,
    path::PathBuf,
};

use clap::builder::{Styles, styling::AnsiColor};

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
        game: String,
        root: PathBuf,
        save_location: PathBuf,
    },
    /// Deletes the game from the managed list.
    #[clap(alias = "del", alias = "remove")]
    Delete { 
        #[arg(value_parser = game_name_parser())]
        game: String
    },
    /// Creates a backup of the game.  
    ///
    /// If no game name is provided, one will try to be selected based on the current directory.  
    ///
    /// The backup is compressed and called "GAME_IDX" by default.  
    /// If a backup description is provided, the backup will be called "GAME_IDX_DESCRIPTION"
    #[clap(alias = "b")]
    Backup {
        game: Option<String>,
        #[clap(long, short)]
        desc: Option<OsString>,
    },
    /// Lists all managed games.
    #[clap(alias = "l", alias = "ls")]
    List,
    /// Opens the root directory of the game.
    #[clap(alias = "o")]
    Open { game: String },
    /// Opens the save directory of the game.
    #[clap(alias = "os")]
    OpenSave { game: String },
}

fn game_name_parser() -> clap::builder::PossibleValuesParser {
    let games = goodgame_impl::Games::load().unwrap();
    clap::builder::PossibleValuesParser::new(games.names().into_iter().map(str::to_owned))
}
