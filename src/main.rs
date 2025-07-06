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
        cli::Cli::Backup { game, desc } => todo!(),
        cli::Cli::Open { game } => open(game, games),
        cli::Cli::OpenSave { game } => open_save(game, games),
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
        std::os::unix::fs::symlink(&save_location, &save_symlink).with_context(
            || format!("Could not create symlink from {save_location:?} to {save_symlink:?}"),
        )?;
    }

    let game = Game::new(game, root, save_location);

    let backups_location = game.backups_location();
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
