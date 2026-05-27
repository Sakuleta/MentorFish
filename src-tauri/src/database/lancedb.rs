// ─── LanceDB Vector Store ───
//
// Stores and retrieves knowledge chunks using cosine similarity.
// In production, uses the lancedb Rust crate. For now, uses file-based JSON.

use async_trait::async_trait;
use std::path::PathBuf;

use crate::agents::KnowledgeChunk;
use crate::database::VectorStore;

pub struct LanceDbStore {
    _path: PathBuf,
}

impl LanceDbStore {
    pub fn new(path: PathBuf) -> Self {
        Self { _path: path }
    }

    /// Cosine similarity between two vectors.
    #[allow(dead_code)]
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
        _embedding: &[f32],
        _limit: usize,
    ) -> anyhow::Result<Vec<KnowledgeChunk>> {
        Ok(Vec::new()) // stub — real impl queries LanceDB
    }

    async fn insert(
        &self,
        _chunks: &[KnowledgeChunk],
        _embeddings: &[Vec<f32>],
    ) -> anyhow::Result<()> {
        Ok(()) // stub — real impl writes to LanceDB
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
}
