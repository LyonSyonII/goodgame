mod cli;

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser};
use goodgame::games::{Game, Games};
use std::{
    io::Seek,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
};

fn main() -> Result<()> {
    // echo "source (COMPLETE=fish your_program | psub)" >> ~/.config/fish/config.fish
    clap_complete::CompleteEnv::with_factory(cli::Cli::command)
        .bin("gg")
        .complete();

    let games = Games::load()?;
    let cli = cli::Cli::parse();

    match cli {
        cli::Cli::Add {
            game,
            root,
            save_location,
            skip_cloud,
            skip_cloud_init,
            executable,
            run_commands,
        } => add(
            game,
            root,
            save_location,
            skip_cloud,
            skip_cloud_init,
            executable,
            run_commands,
            games,
        ),
        cli::Cli::Edit {
            name,
            root,
            save_location,
            executable,
            run_commands,
            game,
        } => edit(
            name,
            root,
            save_location,
            executable,
            run_commands,
            game,
            games,
        ),
        cli::Cli::Remove { game } => remove(game, games),
        cli::Cli::List => list(games),
        cli::Cli::Backup {
            game,
            desc,
            skip_cloud,
        } => backup(game.as_deref(), desc.as_deref(), skip_cloud, &games),
        cli::Cli::Restore {
            game,
            backup,
            skip_cloud,
        } => restore(game, backup, skip_cloud, games),
        cli::Cli::Open { game, save } => open(game, save, games),
        cli::Cli::Run { game, skip_cloud } => run(game, skip_cloud, games),
        cli::Cli::Config => print_config(games),
    }
}

fn add(
    game: Option<String>,
    root: Option<PathBuf>,
    save_location: Option<PathBuf>,
    skip_cloud: bool,
    skip_cloud_init: bool,
    mut executable: Option<PathBuf>,
    run_commands: Option<Vec<String>>,
    mut games: Games,
) -> Result<()> {
    let (Some(game), Some(root), Some(save_location)) = (game, root, save_location) else {
        // TODO: Read current directory for a gg-config.json and take details from there
        // TODO: Make paths relative if able, if not, add custom relative paths ($STEAM, $WINEPREFIX, etc)
        todo!()
    };
    
    let root = root
        .canonicalize()
        .with_context(|| format!("Failed to get root {}", root.display()))?;
    let save_location = save_location
        .canonicalize()
        .with_context(|| format!("Failed to get save location {}", save_location.display()))?;
    if let Some(exe) = &mut executable {
        *exe = exe
            .canonicalize()
            .with_context(|| format!("Failed to get executable {}", exe.display()))?;
    }

    if !root.is_dir() {
        bail!("The root must be a directory");
    }

    if root == save_location {
        bail!("The root and save locations can't be the same");
    }

    let save_symlink = root.join("gg-save-loc");
    if !save_symlink.exists() {
        std::os::unix::fs::symlink(&save_location, &save_symlink).with_context(|| {
            format!(
                "Could not create symlink from {} to {}",
                save_location.display(),
                save_symlink.display()
            )
        })?;
    }

    let game = Game::new(game, root, save_location, executable, run_commands);

    let backups_location = game.backups_path();
    if !backups_location.exists() {
        std::fs::create_dir(&backups_location).with_context(|| {
            format!(
                "Could not create backups location {}",
                backups_location.display()
            )
        })?;
    }

    if !skip_cloud && !skip_cloud_init && games.get_by_name(game.name()).is_err() {
        run_command(games.cloud_init_command(&game), "cloud init", game.root())?;
    }

    let game_s = format!("{game:#?}");
    games.push(game);
    games.store()?;
    println!("Now managing {game_s}");

    Ok(())
}

fn edit(
    name: Option<String>,
    root: Option<PathBuf>,
    save_location: Option<PathBuf>,
    executable: Option<PathBuf>,
    run_commands: Option<Vec<String>>,
    game: Option<impl AsRef<str>>,
    mut games: Games,
) -> std::result::Result<(), anyhow::Error> {
    use std::io::Write;

    let original = games.try_get(game)?.clone();
    let merged = original
        .clone()
        .merged_with(name, root, save_location, executable, run_commands);
    if original != merged {
        games.push(merged);
        games.store()?;
        return Ok(());
    }

    let fname = format!(".gg-{}", original.name());
    let fpath = PathBuf::from("/tmp").join(fname);
    let mut tmp = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&fpath)
        .with_context(|| {
            format!(
                "Could not create temporary file for game config to {}",
                fpath.display()
            )
        })?;
    write!(tmp, "{original}")
        .with_context(|| format!("Could not write game config to {}", fpath.display()))?;

    let cmd = games
        .commands_to_process(&[format!("$EDITOR '{}'", fpath.display())], None)
        .unwrap();
    run_command(Some(cmd), "editing game", fpath.parent().unwrap())?;

    tmp.seek(std::io::SeekFrom::Start(0))?;
    let new_game = serde_json::from_reader::<_, Game>(tmp)
        .with_context(|| format!("Could not parse temporary file {}", fpath.display()))?;

    games.delete(original.name());
    games.push(new_game);
    games.store()?;

    Ok(())
}

fn remove(game: String, mut games: Games) -> Result<()> {
    match games.delete(&game) {
        Some(game) => println!("Deleted {game:#?} successfully"),
        None => bail!("The game {game:#?} is not being managed"),
    };
    games.store()
}

fn list(games: Games) -> Result<()> {
    println!("{games}");
    Ok(())
}

/// The backup is compressed and called "GAME-IDX" by default.
/// If a backup description is provided, the backup will be called "GAME-IDX-DESCRIPTION"
fn backup(game: Option<&str>, desc: Option<&str>, skip_cloud: bool, games: &Games) -> Result<()> {
    let game = games.try_get(game)?;
    let backups_path = game.backups_path();
    let name = game.name();
    let idx = backups_path.read_dir()?.count();
    let desc = if let Some(desc) = desc {
        format!("-{desc}")
    } else {
        String::new()
    };
    let backups_path = backups_path.join(format!("{name}-{idx:0>3}{desc}"));

    let zstd_path = backups_path.with_extension("tar.zst");
    let zstd = std::fs::File::create(&zstd_path)
        .with_context(|| format!("Could not create save backup {}", zstd_path.display()))?;
    let zstd = zstd::Encoder::new(zstd, 9)?;

    let mut tar_builder = tar::Builder::new(zstd);
    if game.save_location().is_dir() {
        tar_builder
            .append_dir_all("", game.save_location())
            .with_context(|| {
                format!(
                    "Could not archive directory {}",
                    game.save_location().display()
                )
            })?;
    } else {
        tar_builder
            .append_file(
                game.save_location().file_name().unwrap(),
                &mut std::fs::File::open(game.save_location())?,
            )
            .with_context(|| {
                format!("Could not archive file {}", game.save_location().display())
            })?;
    }
    tar_builder
        .into_inner()
        .and_then(|zstd| zstd.finish())
        .with_context(|| format!("Could not create backup {}", zstd_path.display()))?;

    println!("Created backup {}", zstd_path.display());

    if !skip_cloud {
        run_command(
            games.cloud_commit_command(game),
            "cloud commit",
            game.root(),
        )?;
        run_command(games.cloud_push_command(game), "cloud push", game.root())?;
    }

    Ok(())
}

fn restore(game: String, target: String, skip_cloud: bool, games: Games) -> Result<()> {
    let game = games.get_by_name(game)?;
    let backups_path = game.backups_path();
    let target_path = backups_path.join(&target);
    target_path
        .try_exists()
        .with_context(|| format!("The backup {} does not exist", target_path.display()))?;
    let target_idx = target
        .split("-")
        .nth(1)
        .unwrap()
        .trim_end_matches(|c: char| !c.is_ascii_digit());
    backup(
        Some(game.name()),
        Some(&format!("replaced-with-{target_idx}")),
        skip_cloud,
        &games,
    )?;

    let target = std::fs::File::open(&target_path)
        .with_context(|| format!("Could not open backup {}", target_path.display()))?;
    let zstd = zstd::Decoder::new(target)?;

    let save_location = game.save_location();
    tar::Archive::new(zstd)
        .unpack(save_location)
        .with_context(|| {
            format!(
                "Could not extract backup {} to {}",
                target_path.display(),
                save_location.display()
            )
        })?;

    if !skip_cloud {
        run_command(
            games.cloud_commit_command(game),
            "cloud commit",
            game.root(),
        )?;
        run_command(games.cloud_push_command(game), "cloud push", game.root())?;
    }

    println!(
        "Successfully restored backup {} to {}",
        target_path.display(),
        save_location.display()
    );

    Ok(())
}

fn open(game: String, save: bool, games: Games) -> Result<()> {
    let game = games.get_by_name(&game)?;
    let dir = if save {
        game.save_location()
    } else {
        game.root()
    };
    let _ = Command::new("xdg-open").arg(dir).spawn()?;
    Ok(())
}

fn run(
    game: Option<String>,
    skip_cloud: bool,
    games: Games,
) -> std::result::Result<(), anyhow::Error> {
    let game = games.try_get(game)?;
    run_command(games.run_command(game), "run game", game.root())?;

    backup(Some(game.name()), None, skip_cloud, &games)?;

    Ok(())
}

fn print_config(games: Games) -> std::result::Result<(), anyhow::Error> {
    println!("{:#?}", games.config());
    Ok(())
}

fn run_command(cmd: Option<Command>, desc: &str, cwd: &Path) -> Result<()> {
    let Some(mut cmd) = cmd else {
        println!("Command {desc} not configured, skipping...");
        return Ok(());
    };
    println!(
        "[gg] Running {desc}: {}",
        cmd.get_args()
            .skip(1)
            .next()
            .unwrap_or(std::ffi::OsStr::from_bytes(b"<EMPTY COMMAND>"))
            .display()
    );

    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(cwd)
        .with_context(|| format!("Could not access directory {}", cwd.display()))?;

    let out = cmd.status().with_context(|| {
        format!(
            "Failed to execute command '{desc}': {}",
            cmd.get_args().nth(1).unwrap().display()
        )
    })?;
    if !out.success() {
        bail!(
            "Command '{desc}' exited with code {}: {}",
            out.code().unwrap_or(0),
            cmd.get_args().nth(1).unwrap().display()
        )
    }

    std::env::set_current_dir(original_dir)?;

    Ok(())
}
