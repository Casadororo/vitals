//! What vitals remembers between runs: theme, graph style, which parts show
//! and how often it reads, in one JSON file saved on every change. Copies
//! running side by side share the file and pick up each other's changes.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::App;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub theme: String,
    /// "braille" or "blocks".
    pub graph: String,
    pub graphs: bool,
    pub cores: bool,
    /// Milliseconds between readings.
    pub interval: u64,
    /// Whether the AI tools' usage limits show, and are read.
    pub limits: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            theme: String::new(),
            graph: String::new(),
            graphs: true,
            cores: true,
            interval: 1000,
            limits: true,
        }
    }
}

/// `$XDG_CONFIG_HOME/vitals/state.json`, or `~/.config/...`, or `%APPDATA%\...`.
pub fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))?;
    Some(base.join("vitals").join("state.json"))
}

/// Keeps the state file and the app in step.
pub struct Store {
    path: PathBuf,
    /// The file's text when last read or written here.
    seen: Option<String>,
    /// What this copy last wrote, to tell its own writes from the others'.
    written: Option<String>,
    /// The app's state as last saved or loaded, to notice changes.
    synced: Option<State>,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            seen: None,
            written: None,
            synced: None,
        }
    }

    /// Restores the saved state. A file that cannot be read as a state is set
    /// aside as `*.broken`, not lost.
    pub fn load(&mut self, app: &mut App) {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return;
        };
        match serde_json::from_str::<State>(&text) {
            Ok(state) => {
                app.restore(&state);
                self.seen = Some(text);
                self.synced = Some(app.state());
            }
            Err(_) => {
                let _ = fs::rename(&self.path, self.path.with_extension("json.broken"));
            }
        }
    }

    /// Takes in what another copy saved, then saves what changed here.
    pub fn sync(&mut self, app: &mut App) -> io::Result<()> {
        if let Ok(text) = fs::read_to_string(&self.path)
            && self.seen.as_ref() != Some(&text)
        {
            if self.written.as_ref() != Some(&text)
                && let Ok(state) = serde_json::from_str::<State>(&text)
            {
                app.restore(&state);
                self.synced = Some(app.state());
            }
            self.seen = Some(text);
        }

        let state = app.state();
        if self.synced.as_ref() == Some(&state) {
            return Ok(());
        }
        let text = serde_json::to_string_pretty(&state).map_err(io::Error::other)? + "\n";
        write_atomically(&self.path, &text)?;
        self.seen = Some(text.clone());
        self.written = Some(text);
        self.synced = Some(state);
        Ok(())
    }
}

/// A crash or power cut leaves either the old file or the new one, whole.
fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let mut file = fs::File::create(&temporary)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::GraphStyle;
    use crate::i18n::Lang;
    use crate::theme;

    fn app() -> App {
        App::new(Lang::En)
    }

    /// A state file path in a fresh directory of its own.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vitals-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("state.json")
    }

    fn open(path: &Path) -> (App, Store) {
        let mut app = app();
        let mut store = Store::new(path.to_owned());
        store.load(&mut app);
        (app, store)
    }

    #[test]
    fn everything_comes_back_after_a_restart() {
        let path = scratch("restart");
        let (mut first, mut store) = open(&path);
        first.theme = theme::find("Tokyo Night").unwrap();
        first.graph_style = GraphStyle::Blocks;
        first.show_cores = false;
        first.interval = 2000;
        store.sync(&mut first).unwrap();

        let (second, _) = open(&path);
        assert_eq!(second.state(), first.state());
        assert_eq!(theme::THEMES[second.theme].name, "Tokyo Night");
        assert_eq!(second.graph_style, GraphStyle::Blocks);
        assert_eq!(second.interval, 2000);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn copies_running_side_by_side_share_their_changes() {
        let path = scratch("share");
        let (mut left, mut left_store) = open(&path);
        let (mut right, mut right_store) = open(&path);
        left_store.sync(&mut left).unwrap();
        right_store.sync(&mut right).unwrap();

        right.theme = theme::find("Dracula").unwrap();
        right_store.sync(&mut right).unwrap();
        left_store.sync(&mut left).unwrap();
        assert_eq!(theme::THEMES[left.theme].name, "Dracula");

        left.show_graphs = false;
        left_store.sync(&mut left).unwrap();
        right_store.sync(&mut right).unwrap();
        assert!(!right.show_graphs);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn writes_only_when_something_changed() {
        let path = scratch("quiet");
        let (mut app, mut store) = open(&path);
        store.sync(&mut app).unwrap();
        assert!(path.exists());
        fs::remove_file(&path).unwrap();
        store.sync(&mut app).unwrap();
        assert!(!path.exists(), "nothing changed, nothing written");
        app.show_graphs = false;
        store.sync(&mut app).unwrap();
        assert!(path.exists());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn sets_a_broken_file_aside_instead_of_losing_it() {
        let path = scratch("broken");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ not json").unwrap();
        let (mut app, mut store) = open(&path);
        let broken = path.with_extension("json.broken");
        assert_eq!(fs::read_to_string(&broken).unwrap(), "{ not json");
        store.sync(&mut app).unwrap();
        assert!(serde_json::from_str::<State>(&fs::read_to_string(&path).unwrap()).is_ok());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn reads_files_missing_newer_fields() {
        let state: State = serde_json::from_str(r#"{"theme": "Nord"}"#).unwrap();
        assert!(state.graphs && state.cores && state.limits);
        assert_eq!(state.interval, 1000);
        let mut app = app();
        app.restore(&state);
        assert_eq!(theme::THEMES[app.theme].name, "Nord");
    }
}
