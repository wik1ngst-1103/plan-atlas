use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Locations of everything Plan Atlas stores on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub home: PathBuf,
    pub plans: PathBuf,
    pub unsorted: PathBuf,
    pub db: PathBuf,
    pub config: PathBuf,
    pub taxonomy: PathBuf,
}

impl Paths {
    /// `PLAN_ATLAS_HOME` if set, otherwise `~/.plan-atlas`.
    pub fn resolve() -> Paths {
        let home = match std::env::var_os("PLAN_ATLAS_HOME") {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".plan-atlas"),
        };
        Paths::from_home(home)
    }

    pub fn from_home(home: PathBuf) -> Paths {
        Paths {
            plans: home.join("plans"),
            unsorted: home.join("plans").join("unsorted"),
            db: home.join("atlas.db"),
            config: home.join("config.toml"),
            taxonomy: home.join("taxonomy.toml"),
            home,
        }
    }

    /// Create the data directories if missing.
    pub fn ensure(&self) -> io::Result<()> {
        fs::create_dir_all(&self.unsorted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub port: u16,
    pub locale: String,
    pub setup_done: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 7420,
            locale: "en".to_string(),
            setup_done: false,
        }
    }
}

impl Config {
    /// Missing file yields defaults silently; an unreadable or invalid file
    /// yields defaults with a warning on stderr (the file is not modified).
    pub fn load(paths: &Paths) -> Config {
        match Config::load_strict(paths) {
            Ok(c) => c.unwrap_or_default(),
            Err(e) => {
                eprintln!("plan-atlas: {e}; using defaults");
                Config::default()
            }
        }
    }

    /// `Ok(None)` if the file is missing; an error if it cannot be read or
    /// parsed. Use before `save` so a broken file is never overwritten.
    pub fn load_strict(paths: &Paths) -> Result<Option<Config>, String> {
        let text = match fs::read_to_string(&paths.config) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("cannot read {}: {e}", paths.config.display())),
        };
        toml::from_str(&text)
            .map(Some)
            .map_err(|e| format!("invalid {}: {e}", paths.config.display()))
    }

    pub fn save(&self, paths: &Paths) -> io::Result<()> {
        let text = toml::to_string_pretty(self).map_err(io::Error::other)?;
        fs::create_dir_all(&paths.home)?;
        fs::write(&paths.config, text)
    }
}
