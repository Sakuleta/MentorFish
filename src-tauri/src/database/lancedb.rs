// ─── LanceDB Vector Store ───
//
// File-based vector store using JSON persistence.
// Stores knowledge chunks with embeddings and retrieves via cosine similarity.
// For production, this could be replaced with the actual LanceDB crate.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::agents::KnowledgeChunk;
use crate::database::VectorStore;

/// A stored chunk with its embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredChunk {
    chunk: KnowledgeChunk,
    embedding: Vec<f32>,
}

/// Persistent vector store backed by a JSON file.
pub struct LanceDbStore {
    path: PathBuf,
    data: Mutex<Vec<StoredChunk>>,
}

impl LanceDbStore {
    pub fn new(path: PathBuf) -> Self {
        let data = Self::load_from_file(&path);
        Self {
            path,
            data: Mutex::new(data),
        }
    }

    fn load_from_file(path: &PathBuf) -> Vec<StoredChunk> {
        if let Ok(content) = std::fs::read_to_string(path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    fn save_to_file(&self, data: &[StoredChunk]) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(data)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    /// Cosine similarity between two vectors.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() {
            return 0.0;
        }
        let dot: f64 = a
            .iter()
            .zip(b)
            .map(|(x, y)| (*x as f64) * (*y as f64))
            .sum();
        let norm_a: f64 = a
            .iter()
            .map(|x| (*x as f64) * (*x as f64))
            .sum::<f64>()
            .sqrt();
        let norm_b: f64 = b
            .iter()
            .map(|x| (*x as f64) * (*x as f64))
            .sum::<f64>()
            .sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }
}

#[async_trait]
impl VectorStore for LanceDbStore {
    async fn search(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> anyhow::Result<Vec<KnowledgeChunk>> {
        let data = self.data.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        let mut scored: Vec<(f64, &KnowledgeChunk)> = data
            .iter()
            .filter(|sc| !sc.embedding.is_empty())
            .map(|sc| {
                let sim = Self::cosine_similarity(embedding, &sc.embedding);
                (sim, &sc.chunk)
            })
            .collect();

        // Sort by similarity descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored.into_iter().map(|(_, chunk)| chunk.clone()).collect())
    }

    async fn insert(
        &self,
        chunks: &[KnowledgeChunk],
        embeddings: &[Vec<f32>],
    ) -> anyhow::Result<()> {
        let mut data = self.data.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
            data.push(StoredChunk {
                chunk: chunk.clone(),
                embedding: embedding.clone(),
            });
        }

        self.save_to_file(&data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let sim = LanceDbStore::cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = LanceDbStore::cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = LanceDbStore::cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = LanceDbStore::cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 0.0];
        let sim = LanceDbStore::cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }
}
