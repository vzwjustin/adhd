use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::util::errors::{AnchorError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub data_dir: PathBuf,
    pub theme: ThemeConfig,
    pub provider: ProviderConfig,
    pub repo: RepoConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub style: ThemeStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeStyle {
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub default_provider: String,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub ollama_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub max_scan_depth: usize,
    pub ignore_patterns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("anchor");

        Self {
            data_dir,
            theme: ThemeConfig {
                style: ThemeStyle::Dark,
            },
            provider: ProviderConfig {
                default_provider: "ollama".to_string(),
                openai_api_key: None,
                anthropic_api_key: None,
                openrouter_api_key: None,
                ollama_url: Some("http://localhost:11434".to_string()),
            },
            repo: RepoConfig {
                max_scan_depth: 8,
                ignore_patterns: vec![
                    "node_modules".to_string(),
                    "target".to_string(),
                    ".git".to_string(),
                    "dist".to_string(),
                    "__pycache__".to_string(),
                    ".venv".to_string(),
                ],
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            toml::from_str(&content)
                .map_err(|e| AnchorError::Config(format!("Failed to parse config: {e}")))
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| AnchorError::Config(format!("Failed to serialize config: {e}")))?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("anchor.db")
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("anchor")
            .join("config.toml")
    }

    pub fn ensure_data_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        Ok(())
    }

    /// Resolve repo path: use provided path or detect from CWD
    pub fn resolve_repo_path(path: Option<&Path>) -> Option<PathBuf> {
        if let Some(p) = path {
            return Some(p.to_path_buf());
        }
        // Try to detect git repo from CWD
        let cwd = std::env::current_dir().ok()?;
        Self::find_git_root(&cwd)
    }

    fn find_git_root(start: &Path) -> Option<PathBuf> {
        let mut current = start.to_path_buf();
        loop {
            if current.join(".git").exists() {
                return Some(current);
            }
            if !current.pop() {
                return None;
            }
        }
    }
}
