use std::path::PathBuf;

use clap::{
    ValueHint,
    builder::{Styles, styling::AnsiColor},
};
use clap_complete::{ArgValueCandidates, ArgValueCompleter, CompletionCandidate};
use goodgame::games::Games;

const CLAP_STYLE: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Green.on_default());

#[derive(clap::Parser)]
#[clap(styles = CLAP_STYLE, version)]
pub enum Cli {
    /// Starts to manage the provided game.
    ///
    /// If the game is already being managed, the provided details will override the current ones.
    #[clap(alias = "a", alias = "init")]
    Add {
        /// The path of the game executable.
        #[arg(short, long="executable", value_hint = ValueHint::FilePath)]
        executable: Option<PathBuf>,
        /// Comma separated list of the commands that will be used in 'gg run @GAME'.
        ///
        /// If not provided, the global one will be used, replacing @EXE with the above executable.
        ///
        /// Supported variables:
        /// - @RUN: Global run command, similar to Steam's %command%.
        #[arg(short, long = "run")]
        run_commands: Option<Vec<String>>,
        /// Skips cloud saving features completely.
        #[arg(short, long = "skip-cloud")]
        skip_cloud: bool,
        /// Skips cloud saving initialization.
        #[arg(long = "skip-init")]
        skip_cloud_init: bool,
        /// The name of the game to manage.
        #[arg(value_hint = ValueHint::AnyPath)]
        game: String,
        /// The root path of the game.
        #[arg(value_hint = ValueHint::DirPath)]
        root: PathBuf,
        /// The path where the game stores its save files.
        #[arg(value_hint = ValueHint::AnyPath)]
        save_location: PathBuf,
    },
    /// Edits the configuration of the specified game.
    ///
    /// If no extra argument is provided, an editable JSON file will be opened.
    #[clap(alias = "e")]
    Edit {
        /// New game name.
        #[arg(long, value_hint = ValueHint::AnyPath)]
        name: Option<String>,
        /// New root path.
        #[arg(long, value_hint = ValueHint::DirPath)]
        root: Option<PathBuf>,
        /// New save location path.
        #[arg(long, value_hint = ValueHint::AnyPath)]
        save_location: Option<PathBuf>,
        /// New executable path.
        #[arg(long, value_hint = ValueHint::FilePath)]
        executable: Option<PathBuf>,
        /// New run commands.
        #[arg(long = "run")]
        run_commands: Option<Vec<String>>,
        /// The name of the game to edit.
        #[arg(add = game_name_completer())]
        game: Option<String>,
    },
    /// Removes the game from the managed list.
    #[clap(alias = "rm", alias = "delete", alias = "del")]
    Remove {
        /// The name of the game to remove.
        #[arg(add = game_name_completer())]
        game: String,
    },
    /// Creates a backup of the current save.
    ///
    /// If no game name is provided, one will try to be selected based on the current directory.
    ///
    /// The backup is compressed and called "GAME-IDX" by default.
    /// If a backup description is provided, the backup will be called "GAME-IDX-DESCRIPTION"
    #[clap(alias = "b", alias = "bk")]
    Backup {
        /// The name of the game to make the backup.
        #[arg(add = game_name_completer())]
        game: Option<String>,
        /// Description that will be appended to the backup name.
        #[arg(long, short, value_hint = ValueHint::Other)]
        desc: Option<String>,
        #[arg(short, long = "skip-cloud")]
        skip_cloud: bool,
    },
    /// Restores the selected save backup.
    ///
    /// A backup of the current save will be created.
    #[clap()]
    Restore {
        #[arg(short, long = "skip-cloud")]
        skip_cloud: bool,
        /// Name of the game to restore the save backup.
        #[arg(add = game_name_completer())]
        game: String,
        /// Name of the backup to restore.
        #[arg(add = game_backup_candidates(), requires = "game")]
        backup: String,
    },
    /// Lists all managed games.
    #[clap(alias = "l", alias = "ls")]
    List,
    /// Opens the root directory of the game.
    #[clap(alias = "o")]
    Open {
        /// Open the save directory instead of the root.
        #[arg(long, short)]
        save: bool,
        /// Name of the game to open the directory.
        #[arg(add = game_name_completer())]
        game: String,
    },
    /// Runs the selected game.
    #[clap(alias = "r")]
    Run {
        /// Skip creating a backup of the saves when the game exits.
        #[clap(short, long = "skip-cloud")]
        skip_cloud: bool,
        /// Name of the game to run.
        #[arg(add = game_name_completer())]
        game: Option<String>,
    },
    /// Prints the current configuration.
    ///
    /// Located on /etc/goodgame/config.json
    Config,
}

static GAMES: std::sync::LazyLock<Games> = std::sync::LazyLock::new(|| Games::load().unwrap());

fn game_name_completer() -> ArgValueCompleter {
    fn inner(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
        // fixes game names showing up with commands
        if std::env::args().count() <= 2 {
            return Vec::new();
        }

        GAMES
            .names()
            .into_iter()
            .filter(|c| {
                current
                    .as_encoded_bytes()
                    .iter()
                    .copied()
                    .zip(c.bytes())
                    .all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
            })
            .map(CompletionCandidate::new)
            .collect()
    }
    ArgValueCompleter::new(inner)
}

fn game_backup_candidates() -> ArgValueCandidates {
    if std::env::args().count() <= 2 {
        return ArgValueCandidates::new(std::vec::Vec::new);
    }
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
