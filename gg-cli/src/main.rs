use std::{
    fs::File,
    path::{Path, PathBuf},
};

use clap::{CommandFactory, Parser};
use goodgame_impl::{Game, Games};

mod cli;

fn main() -> std::io::Result<()> {
    // echo "source (COMPLETE=fish your_program | psub)" >> ~/.config/fish/config.fish
    clap_complete::CompleteEnv::with_factory(cli::Cli::command)
        .bin("gg")
        .complete();

    let games = Games::load()?;
    let cli = cli::Cli::parse();

    let res = match cli {
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
    };
    if let Err(e) = res {
        eprintln!("{e}");
    }

    Ok(())
}

fn add(
    game: String,
    root: PathBuf,
    save_location: PathBuf,
    mut games: Games,
) -> std::io::Result<()> {
    root.try_exists()?;
    save_location.try_exists()?;

    if games.get_by_name(&game).is_some() {
        return Err(std::io::Error::other(format!(
            "A game with the name {game:#?} already exists"
        )));
    }
    if games.get_by_root(&root).is_some() {
        return Err(std::io::Error::other(format!(
            "A game with the root {root:#?} already exists"
        )));
    }
    if games.get_by_save(&save_location).is_some() {
        return Err(std::io::Error::other(format!(
            "A game with the save location {save_location:#?} already exists"
        )));
    }

    games.push(Game::new(
        game.clone(),
        root.canonicalize()?,
        save_location.canonicalize()?,
    ));

    games.store()?;

    println!("Now managing {game:#?}");

    Ok(())
}

fn remove(game: String, mut games: Games) -> std::io::Result<()> {
    match games.delete(&game) {
        Some(game) => println!("Deleted {game:#?} successfully"),
        None => {
            return Err(std::io::Error::other(format!(
                "The game {game:#?} is not being managed"
            )));
        }
    };
    games.store()
}

fn list(games: Games) -> std::io::Result<()> {
    println!("{games}");
    Ok(())
}

fn open(game: String, games: Games) -> Result<(), std::io::Error> {
    let dir = games
        .get_by_name(&game)
        .ok_or_else(|| std::io::Error::other(format!("The Game '{game}' does not exist.")))?
        .root();
    let _ = std::process::Command::new("xdg-open").arg(dir).spawn()?;
    Ok(())
}

fn open_save(game: String, games: Games) -> Result<(), std::io::Error> {
    let dir = games
        .get_by_name(&game)
        .ok_or_else(|| std::io::Error::other(format!("The Game '{game}' does not exist.")))?
        .save_location();
    let _ = std::process::Command::new("xdg-open").arg(dir).spawn()?;
    Ok(())
}
