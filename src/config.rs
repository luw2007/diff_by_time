use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub storage: StorageConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StorageConfig {
    pub max_retention_days: u32,
    pub auto_archive: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct DisplayConfig {
    pub max_history_shown: usize,
    pub language: String,
    // TUI mode: "interactive" or "simple"
    pub tui_mode: String,
    // Whether to use terminal alternate screen in interactive mode
    pub alt_screen: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            max_history_shown: 10,
            language: "auto".to_string(),
            tui_mode: "interactive".to_string(),
            alt_screen: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            storage: StorageConfig {
                max_retention_days: 365, // Default 1 year
                auto_archive: true,
            },
            display: DisplayConfig::default(),
        }
    }
}

impl Config {
    pub fn new() -> Result<Self> {
        let config_path = Self::get_config_path();

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path();
        let config_dir = config_path.parent().unwrap();

        fs::create_dir_all(config_dir)?;

        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    fn get_config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".dt")
            .join("config.toml")
    }

    pub fn get_effective_language(&self) -> String {
        if self.display.language == "auto" {
            // Try to get system language
            std::env::var("LANG")
                .unwrap_or_else(|_| "en_US".to_string())
                .split('.')
                .next()
                .unwrap_or("en")
                .to_string()
        } else {
            self.display.language.clone()
        }
    }
}
