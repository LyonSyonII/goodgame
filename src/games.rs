use crate::config::Config;
use anyhow::{Context, Result, anyhow, bail};
use std::{
    io::Seek,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Games {
    inner: Vec<Game>,
    data_dir: PathBuf,
    games_file: std::fs::File,
    config: Config,
}

impl Games {
    pub fn load() -> Result<Games> {
        let config = std::fs::File::open("/etc/goodgame/config.json")
            .with_context(|| "Could not open config file /etc/goodgame/config.json".to_string())
            .and_then(|config| {
                serde_json::from_reader::<_, Config>(config).with_context(|| {
                    "Could not parse config file /etc/goodgame/config.json".to_string()
                })
            })
            .unwrap_or_default();

        let data_dir = std::env::var("XDG_DATA_HOME")
            .or_else(|_| std::env::var("HOME").map(|h| h + "/.local/share"))
            .map(|s| PathBuf::from(s + "/goodgame"))
            .map_err(|_| anyhow!("Could not obtain data directory"))?;
        std::fs::create_dir_all(&data_dir)?;

        let games_path = data_dir.join(Self::games_file_name());
        let games_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .create(true)
            .open(&games_path)
            .with_context(|| format!("Could not read {}", games_path.display()))?;
        let games = if games_file.metadata()?.len() == 0 {
            Vec::new()
        } else {
            serde_json::from_reader::<_, Vec<Game>>(&games_file)
                .with_context(|| format!("Could not parse {}", games_path.display()))?
        };

        Ok(Games {
            inner: games,
            config,
            data_dir,
            games_file,
        })
    }

    pub fn store(&mut self) -> Result<()> {
        self.games_file.set_len(0)?;
        if self.inner.is_empty() {
            return Ok(());
        }
        self.games_file.rewind()?;
        serde_json::to_writer(&mut self.games_file, &self.inner)
            .with_context(|| format!("Could not save to {}", self.games_path().display()))
    }

    /// Pushes or updates the provided game.
    pub fn push(&mut self, game: Game) {
        match self.inner.binary_search(&game) {
            Ok(i) => self.inner[i].merge(game),
            Err(i) => self.inner.insert(i, game),
        }
    }

    pub fn delete(&mut self, name: impl AsRef<str>) -> Option<Game> {
        let name = name.as_ref();
        let i = self
            .inner
            .binary_search_by(|g| g.name.as_str().cmp(name))
            .ok()?;
        Some(self.inner.remove(i))
    }

    pub fn games(&self) -> &[Game] {
        &self.inner
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn names(&self) -> impl IntoIterator<Item = &str> {
        self.inner.iter().map(|g| g.name.as_str())
    }

    pub fn games_file_name() -> &'static str {
        "games.json"
    }

    pub fn games_path(&self) -> PathBuf {
        self.data_dir.join(Self::games_file_name())
    }

    pub fn get_by_name(&self, name: impl AsRef<str>) -> Result<&Game> {
        let name = name.as_ref();
        if let Ok(i) = self.inner.binary_search_by(|g| g.name.as_str().cmp(name)) {
            Ok(&self.inner[i])
        } else {
            bail!("The game {name:?} does not exist")
        }
    }

    pub fn get_by_root(&self, path: impl AsRef<Path>) -> Option<&Game> {
        let path = path.as_ref();
        self.inner.iter().find(|g| g.root == path)
    }

    pub fn get_by_save(&self, path: impl AsRef<Path>) -> Option<&Game> {
        let path = path.as_ref();
        self.inner.iter().find(|g| g.save_location == path)
    }

    pub fn get_by_current_dir(&self) -> Option<&Game> {
        let curr = std::env::current_dir().ok()?;
        self.inner
            .iter()
            .find(|g| g.root == curr || g.save_location == curr)
    }

    pub fn try_get(&self, game: Option<impl AsRef<str>>) -> Result<&Game> {
        if let Some(game) = game {
            self.get_by_name(game)
        } else if let Some(game) = self.get_by_current_dir() {
            Ok(game)
        } else {
            bail!(
                "Could not infer game by the current directory {}",
                std::env::current_dir()?.canonicalize()?.display()
            )
        }
    }

    pub fn commands_to_process(&self, cmds: &[String], game: Option<&Game>) -> Option<std::process::Command> {
        if cmds.is_empty() {
            return None;
        }
        let mut cmds = cmds.join("&&");
        let mut p = std::process::Command::new(&self.config.shell);
        if let Some(game) = game {
            cmds = game.replace_vars(cmds);
        }
        p.args([String::from("-c"), cmds]);
        Some(p)
    }
    pub fn cloud_init_command(&self, game: &Game) -> Option<std::process::Command> {
        self.commands_to_process(&self.config.backup.cloud_init_commands, Some(game))
    }
    pub fn cloud_commit_command(&self, game: &Game) -> Option<std::process::Command> {
        self.commands_to_process(&self.config.backup.cloud_commit_commands, Some(game))
    }
    pub fn cloud_push_command(&self, game: &Game) -> Option<std::process::Command> {
        self.commands_to_process(&self.config.backup.cloud_push_commands, Some(game))
    }
    pub fn run_command(&self, game: &Game) -> Option<std::process::Command> {
        let cmds: std::borrow::Cow<[String]> = game.run_commands.clone().map(|mut cmds| {
            let global_run = self.config.run.commands.join("&&");
            for cmd in cmds.iter_mut() {
                if let Some(i) = cmd.find("$RUN") {
                    cmd.replace_range(i..i+"$RUN".len(), &global_run);
                }
            }
            cmds.into()
        }).unwrap_or(self.config.run.commands.as_slice().into());
        self.commands_to_process(&cmds, Some(game))
    }
}

impl std::fmt::Display for Games {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Trick serde_json into writing to std::fmt::Formatter
        struct FormatterWriter<'a, 'b>(&'a mut std::fmt::Formatter<'b>);
        impl std::io::Write for FormatterWriter<'_, '_> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                // SAFETY: The original message is already utf8
                let FormatterWriter(fmt) = self;
                let _ = fmt.write_str(unsafe { std::str::from_utf8_unchecked(buf) });
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        serde_json::to_writer_pretty(FormatterWriter(f), &self.games()).map_err(|_| std::fmt::Error)
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Game {
    name: String,
    root: PathBuf,
    save_location: PathBuf,
    executable: Option<PathBuf>,
    run_commands: Option<Vec<String>>,
}

impl Game {
    pub fn new(
        name: String,
        root: PathBuf,
        save_location: PathBuf,
        executable: Option<PathBuf>,
        run_commands: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            root,
            save_location,
            executable,
            run_commands,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn save_location(&self) -> &Path {
        &self.save_location
    }

    pub fn backups_path(&self) -> PathBuf {
        self.root.join("gg-saves")
    }
    
    pub fn merge(&mut self, game: Game) {
        self.root = game.root;
        self.save_location = game.save_location;
        if game.executable.is_some() {
            self.executable = game.executable;
        }
        if game.run_commands.is_some() {
            self.run_commands = game.run_commands;
        }
    }
    
    pub fn merged_with(self, name: Option<String>, root: Option<PathBuf>, save_location: Option<PathBuf>, executable: Option<PathBuf>, run_commands: Option<Vec<String>>) -> Game {
        Game {
            name: name.unwrap_or(self.name),
            root: root.unwrap_or(self.root),
            save_location: save_location.unwrap_or(self.save_location),
            executable: executable.or(self.executable),
            run_commands: run_commands.or(self.run_commands),
        }
    }
    
    fn replace_vars(&self, mut template: String) -> String {
        if let Some(exe) = &self.executable {
            template = template.replace("$EXE", &format!("'{}'", exe.display()));
        }
        template
            .replace("$GAME", &self.name)
    }
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            || self.root == other.root
            || self.save_location == other.save_location
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

impl std::fmt::Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Trick serde_json into writing to std::fmt::Formatter
        struct FormatterWriter<'a, 'b>(&'a mut std::fmt::Formatter<'b>);
        impl std::io::Write for FormatterWriter<'_, '_> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                // SAFETY: The original message is already utf8
                let FormatterWriter(fmt) = self;
                let _ = fmt.write_str(unsafe { std::str::from_utf8_unchecked(buf) });
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        serde_json::to_writer_pretty(FormatterWriter(f), &self).map_err(|_| std::fmt::Error)
    }
}