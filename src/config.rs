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
    /// Missing or unreadable/invalid file yields defaults.
    pub fn load(paths: &Paths) -> Config {
        fs::read_to_string(&paths.config)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, paths: &Paths) -> io::Result<()> {
        let text = toml::to_string_pretty(self).map_err(io::Error::other)?;
        fs::create_dir_all(&paths.home)?;
        fs::write(&paths.config, text)
    }
}
