use std::{fs::File, path::{Path, PathBuf}};

use clap::Parser;
use directories::ProjectDirs;

mod cli;

fn main() -> std::io::Result<()> {
    let cli = cli::Cli::parse();
    let games = Games::load()?;

    let res = match cli {
        cli::Cli::Add {
            game,
            root,
            save_location,
        } => add(game, root, save_location, games),
        cli::Cli::Delete { game } => remove(game, games),
        cli::Cli::Backup { game, desc } => todo!(),
        cli::Cli::List => list(games),
        cli::Cli::Open { game } => todo!(),
        cli::Cli::OpenSave { game } => todo!(),
    };
    if let Err(e) = res {
        eprintln!("{e}");
    }

    Ok(())
}

fn add(game: String, root: PathBuf, save_location: PathBuf, mut games: Games) -> std::io::Result<()> {
    root.try_exists()?;
    save_location.try_exists()?;

    if games.get_by_name(&game).is_some() {
        return Err(std::io::Error::other(format!("A game with the name {game:#?} already exists")));
    }
    if games.get_by_root(&root).is_some() {
        return Err(std::io::Error::other(format!("A game with the root {root:#?} already exists")));
    }
    if games.get_by_save(&save_location).is_some() {
        return Err(std::io::Error::other(format!("A game with the save location {save_location:#?} already exists")));
    }

    games.push(Game {
        name: game.clone(),
        root: root.canonicalize()?,
        save_location: save_location.canonicalize()?
    });

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
    serde_json::to_writer_pretty(std::io::stdout().lock(), &games.inner)
        .map_err(std::io::Error::other)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Game {
    name: String,
    root: PathBuf,
    save_location: PathBuf,
}

#[derive(Debug)]
struct Games {
    inner: Vec<Game>,
    dirs: ProjectDirs,
    games_path: PathBuf,
    games_file: File
}

impl Games {
    fn load() -> std::io::Result<Games> {
        let dirs = directories::ProjectDirs::from("com", "lyonsyonii", "gg")
            .expect("Could not get project directories");
        std::fs::create_dir_all(dirs.config_dir())?;

        let games_path = dirs.config_dir().join("games.json");
        let games_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .create(true)
            .open(&games_path)
            .map_err(|e| std::io::Error::other(format!("Could not read {}: {e}", games_path.display())))?;
        let games = if games_file.metadata()?.len() == 0 {
            Vec::new()
        } else {
            serde_json::from_reader::<_, Vec<Game>>(&games_file).map_err(|e| {
                std::io::Error::other(format!("Could not parse {}: {e}", games_path.display()))
            })?
        };

        Ok(Games {
            inner: games,
            dirs,
            games_path,
            games_file
        })
    }

    fn store(&self) -> std::io::Result<()> {
        self.games_file.set_len(0)?;
        if self.inner.is_empty() {
            return Ok(());
        }
        serde_json::to_writer(&self.games_file, &self.inner).map_err(|e| {
            std::io::Error::other(format!("Could not save to {:#?}: {e}", self.games_path))
        })
    }

    fn push(&mut self, game: Game) {
        self.inner.push(game)
    }

    fn delete(&mut self, name: impl AsRef<str>) -> Option<Game> {
        let name = name.as_ref();
        let i = self.inner.iter().position(|g| g.name == name)?;
        Some(self.inner.swap_remove(i))
    }

    fn get_by_name(&mut self, name: impl AsRef<str>) -> Option<&mut Game> {
        let name = name.as_ref();
        self.inner.iter_mut().find(|g| g.name == name)
    }

    fn get_by_root(&mut self, path: impl AsRef<Path>) -> Option<&mut Game> {
        let path = path.as_ref();
        self.inner.iter_mut().find(|g| g.root == path)
    }

    fn get_by_save(&mut self, path: impl AsRef<Path>) -> Option<&mut Game> {
        let path = path.as_ref();
        self.inner.iter_mut().find(|g| g.save_location == path)
    }

    fn get_by_current_dir(&mut self) -> Option<&mut Game> {
        let curr = std::env::current_dir().ok()?;
        self.inner
            .iter_mut()
            .find(|g| g.root == curr || g.save_location == curr)
    }
}
