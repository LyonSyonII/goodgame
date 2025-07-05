use directories::ProjectDirs;
use std::{io::Seek, path::{Path, PathBuf}};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Game {
    name: String,
    root: PathBuf,
    save_location: PathBuf,
}

impl Game {
    pub fn new(name: String, root: PathBuf, save_location: PathBuf) -> Self {
        Self { name, root, save_location }
    }
}

#[derive(Debug)]
pub struct Games {
    inner: Vec<Game>,
    dirs: ProjectDirs,
    games_path: PathBuf,
    games_file: std::fs::File
}

impl Games {
    pub fn load() -> std::io::Result<Games> {
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

    pub fn store(&mut self) -> std::io::Result<()> {
        self.games_file.set_len(0)?;
        if self.inner.is_empty() {
            return Ok(());
        }
        self.games_file.rewind()?;
        serde_json::to_writer(&mut self.games_file, &self.inner).map_err(|e| {
            std::io::Error::other(format!("Could not save to {:#?}: {e}", self.games_path))
        })
    }

    pub fn push(&mut self, game: Game) {
        let Err(idx) = self.inner.binary_search(&game) else {
            return
        };
        self.inner.insert(idx, game);
    }

    pub fn delete(&mut self, name: impl AsRef<str>) -> Option<Game> {
        let name = name.as_ref();
        let i = self.inner.binary_search_by(|g| g.name.as_str().cmp(name)).ok()?;
        Some(self.inner.remove(i))
    }

    pub fn games(&self) -> &[Game] {
        &self.inner
    }

    pub fn names(&self) -> impl IntoIterator<Item = &str> {
        self.inner.iter().map(|g| g.name.as_str())
    }

    pub fn get_by_name(&mut self, name: impl AsRef<str>) -> Option<&mut Game> {
        let name = name.as_ref();
        let idx = self.inner.binary_search_by(|g| g.name.as_str().cmp(name)).ok()?;
        self.inner.get_mut(idx)
    }

    pub fn get_by_root(&mut self, path: impl AsRef<Path>) -> Option<&mut Game> {
        let path = path.as_ref();
        self.inner.iter_mut().find(|g| g.root == path)
    }

    pub fn get_by_save(&mut self, path: impl AsRef<Path>) -> Option<&mut Game> {
        let path = path.as_ref();
        self.inner.iter_mut().find(|g| g.save_location == path)
    }

    pub fn get_by_current_dir(&mut self) -> Option<&mut Game> {
        let curr = std::env::current_dir().ok()?;
        self.inner
            .iter_mut()
            .find(|g| g.root == curr || g.save_location == curr)
    }
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name || self.root == other.root || self.save_location == other.save_location
    }
}

impl Eq for Game {}

impl PartialOrd for Game {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Game {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl std::fmt::Display for Games {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct FormatterWriter<'a, 'b>(&'a mut std::fmt::Formatter<'b>);
        impl std::io::Write for FormatterWriter<'_, '_> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                let _ = self.0.write_str(unsafe { std::str::from_utf8_unchecked(buf) });
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let writer = FormatterWriter(f);
        serde_json::to_writer_pretty(writer, &self.games()).map_err(|_| std::fmt::Error)
    }
}