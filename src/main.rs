mod cli;

use anyhow::{Context, Result, anyhow, bail};
use clap::{CommandFactory, Parser};
use goodgame::{Game, Games};
use std::path::PathBuf;

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
        } => add(game, root, save_location, games),
        cli::Cli::Delete { game } => remove(game, games),
        cli::Cli::List => list(games),
        cli::Cli::Backup { game, desc } => backup(game, desc, games),
        cli::Cli::Open { game } => open(game, games),
        cli::Cli::OpenSave { game } => open_save(game, games),
        cli::Cli::Restore { game, backup } => todo!(),
    }
}

fn add(game: String, root: PathBuf, save_location: PathBuf, mut games: Games) -> Result<()> {
    let root = root
        .canonicalize()
        .with_context(|| format!("Failed to get root {root:?}"))?;
    let save_location = save_location
        .canonicalize()
        .with_context(|| format!("Failed to get save location {save_location:?}"))?;

    if !root.is_dir() {
        bail!("The root must be a directory");
    }

    if root == save_location {
        bail!("The root and save locations can't be the same");
    }

    if games.get_by_name(&game).is_some() {
        bail!("A game with the name {game:#?} already exists");
    }
    if games.get_by_root(&root).is_some() {
        bail!("A game with the root {root:#?} already exists");
    }
    if games.get_by_save(&save_location).is_some() {
        bail!("A game with the save location {save_location:#?} already exists");
    }

    let save_symlink = root.join("gg-save-loc");
    if !save_symlink.exists() {
        std::os::unix::fs::symlink(&save_location, &save_symlink).with_context(|| {
            format!("Could not create symlink from {save_location:?} to {save_symlink:?}")
        })?;
    }

    let game = Game::new(game, root, save_location);

    let backups_location = game.backups_path();
    if !backups_location.exists() {
        std::fs::create_dir(&backups_location)
            .with_context(|| format!("Could not create backups location {backups_location:?}"))?;
    }

    games.push(game.clone());

    games.store()?;

    println!("Now managing {game:#?}");

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
fn backup(game: Option<String>, desc: Option<String>, games: Games) -> Result<()> {
    let Some(game) = game
        .and_then(|n| games.get_by_name(n))
        .or_else(|| games.get_by_current_dir())
    else {
        bail!(
            "Could not infer game by the current directory {:?}",
            std::env::current_dir()?.canonicalize()
        )
    };
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
        .with_context(|| format!("Could not create save backup {zstd_path:?}"))?;
    let zstd = zstd::Encoder::new(zstd, 9)?;

    let mut tar_builder = tar::Builder::new(zstd);
    if game.save_location().is_dir() {
        tar_builder
            .append_dir_all("", game.save_location())
            .with_context(|| format!("Could not archive directory {:?}", game.save_location()))?;
    } else {
        tar_builder
            .append_file(
                game.save_location().file_name().unwrap(),
                &mut std::fs::File::open(game.save_location())?,
            )
            .with_context(|| format!("Could not archive file {:?}", game.save_location()))?;
    }
    tar_builder
        .into_inner()
        .and_then(|zstd| zstd.finish())
        .with_context(|| format!("Could not create backup {zstd_path:?}"))?;

    Ok(())
}

fn open(game: String, games: Games) -> Result<()> {
    let dir = games
        .get_by_name(&game)
        .ok_or_else(|| anyhow!("The Game '{game}' does not exist."))?
        .root();
    let _ = std::process::Command::new("xdg-open").arg(dir).spawn()?;
    Ok(())
}

fn open_save(game: String, games: Games) -> Result<()> {
    let dir = games
        .get_by_name(&game)
        .ok_or_else(|| anyhow!("The Game '{game}' does not exist."))?
        .save_location();
    let _ = std::process::Command::new("xdg-open").arg(dir).spawn()?;
    Ok(())
}
