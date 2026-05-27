// ─── Application Configuration ───
//
// Loads/saves a JSON config file from the OS-standard config directory.
// All tunable backend parameters live here.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Stockfish binary path
    pub stockfish_path: String,
    /// Stockfish threads for analysis
    pub stockfish_threads: u32,
    /// Stockfish hash size in MB
    pub stockfish_hash_mb: u32,
    /// Analysis depth
    pub analysis_depth: u32,
    /// MultiPV count
    pub multipv: u32,
    /// Ollama base URL
    pub ollama_url: String,
    /// Primary model tag
    pub primary_model: String,
    /// Fast model tag
    pub fast_model: String,
    /// Embedding model tag
    pub embedding_model: String,
    /// Syzygy tablebase path (optional)
    pub syzygy_path: Option<String>,
    /// Coaching persona
    pub persona: String,
    /// Knowledge directory path
    pub knowledge_dir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            stockfish_path: "bin/stockfish/stockfish-windows-x86-64-avx2.exe".into(),
            stockfish_threads: 14,
            stockfish_hash_mb: 2048,
            analysis_depth: 22,
            multipv: 5,
            ollama_url: "http://localhost:11434".into(),
            primary_model: "qwen3:14b".into(), // Q4_K_M (~9GB) — fits 16GB VRAM comfortably
            fast_model: "qwen3:8b".into(),     // Q4_K_M (~5GB)
            embedding_model: "nomic-embed-text".into(),
            syzygy_path: None,
            persona: "modernGM".into(),
            knowledge_dir: "knowledge".into(),
        }
    }
}

impl AppConfig {
    /// Load config from the app's data directory, creating default if missing.
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let json = std::fs::read_to_string(&path)?;
            let config: AppConfig = serde_json::from_str(&json)?;
            log::info!("Loaded config from {}", path.display());
            Ok(config)
        } else {
            let config = Self::default();
            if let Err(e) = config.save() {
                log::warn!("Could not save default config: {}", e);
            }
            Ok(config)
        }
    }

    /// Save config to the app's data directory.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        log::info!("Saved config to {}", path.display());
        Ok(())
    }

    fn config_path() -> anyhow::Result<PathBuf> {
        let dir = dirs_next::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine config directory"))?;
        Ok(dir.join("MentorFish").join("config.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.stockfish_threads, 14);
        assert_eq!(cfg.multipv, 5);
        assert_eq!(cfg.persona, "modernGM");
    }

    #[test]
    fn test_serialize_roundtrip() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.stockfish_threads, cfg.stockfish_threads);
        assert_eq!(parsed.persona, cfg.persona);
    }
}
