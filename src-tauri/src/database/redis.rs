// ─── Redis Cache ───
//
// Sub-millisecond cache for active session state and hot position cache.
// Stub implementation — real impl uses the redis crate.

use crate::engine::EngineOutput;

/// Cache key prefixes for namespacing.
pub mod keys {
    pub const ENGINE_OUTPUT: &str = "engine:";
    pub const FEATURE_BUNDLE: &str = "features:";
    pub const SESSION_STATE: &str = "session:";
}

/// In-memory cache stub for when Redis is not available.
pub struct MemoryCache {
    engine_outputs: std::collections::HashMap<String, EngineOutput>,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self {
            engine_outputs: std::collections::HashMap::new(),
        }
    }

    pub fn get_engine_output(&self, fen: &str) -> Option<&EngineOutput> {
        self.engine_outputs.get(fen)
    }

    pub fn set_engine_output(&mut self, fen: String, output: EngineOutput) {
        self.engine_outputs.insert(fen, output);
    }

    pub fn clear(&mut self) {
        self.engine_outputs.clear();
    }
}

impl Default for MemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineOutput;

    #[test]
    fn test_memory_cache() {
        let mut cache = MemoryCache::new();
        let fen = "startpos".to_string();
        let output = EngineOutput {
            fen: fen.clone(),
            eval_cp: 25,
            eval_mate: None,
            best_move: Some("e2e4".into()),
            best_move_san: None,
            ponder: None,
            depth: 10,
            multipv: vec![],
            nodes: Some(1000),
            nps: Some(50000),
            time_ms: Some(20),
        };

        cache.set_engine_output(fen.clone(), output);
        let cached = cache.get_engine_output(&fen);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().eval_cp, 25);
    }
}
