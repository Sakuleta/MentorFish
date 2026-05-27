// ─── Knowledge Ingestion (Rust side) ───
//
// Reads chunk JSON from the Python ingestion script,
// generates embeddings via Ollama, and stores in LanceDB.

use serde::{Deserialize, Serialize};

use crate::agents::{ChunkType, KnowledgeChunk};
use crate::inference::InferenceClient;

/// JSON output format from scripts/ingest.py
#[derive(Debug, Deserialize)]
struct IngestionOutput {
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    generated_at: String,
    #[allow(dead_code)]
    total_chunks: usize,
    chunks: Vec<RawChunk>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RawChunk {
    content: String,
    source: String,
    chunk_type: String,
    #[serde(default)]
    token_count: usize,
    #[serde(default)]
    position_fen: Option<String>,
    #[serde(default)]
    embedding: Option<Vec<f32>>,
}

/// Run the full ingestion pipeline: Python → embed → store.
pub async fn run_ingestion(
    client: &dyn InferenceClient,
    knowledge_dir: &std::path::Path,
) -> anyhow::Result<IngestionReport> {
    let chunks_path = knowledge_dir.join("chunks.json");

    if !chunks_path.exists() {
        // Also try chunks_all.json
        let fallback = knowledge_dir.join("chunks_all.json");
        if !fallback.exists() {
            return Ok(IngestionReport {
                books_processed: 0,
                chunks_created: 0,
                chunks_embedded: 0,
                message: "No chunks file found. Run: python scripts/ingest.py --all".into(),
            });
        }
        return run_ingestion_with_file(client, &fallback, knowledge_dir).await;
    }

    run_ingestion_with_file(client, &chunks_path, knowledge_dir).await
}

async fn run_ingestion_with_file(
    client: &dyn InferenceClient,
    chunks_path: &std::path::Path,
    knowledge_dir: &std::path::Path,
) -> anyhow::Result<IngestionReport> {
    let json = std::fs::read_to_string(chunks_path)?;
    let output: IngestionOutput = serde_json::from_str(&json)?;

    let mut chunks_with_embeddings: Vec<RawChunk> = Vec::new();
    let mut embedded = 0u64;
    let total = output.chunks.len();

    // Process chunks in batches for embedding
    for chunk in &output.chunks {
        let mut enriched = chunk.clone();
        match client.embed(&chunk.content).await {
            Ok(embedding) => {
                enriched.embedding = Some(embedding);
                embedded += 1;
            }
            Err(e) => {
                log::warn!("Failed to embed chunk from {}: {}", chunk.source, e);
            }
        }
        chunks_with_embeddings.push(enriched);
    }

    // ── Persist indexed chunks to JSON ──
    let output_path = knowledge_dir.join("chunks_indexed.json");
    let indexed_output = serde_json::json!({
        "version": "1.0",
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "total_chunks": chunks_with_embeddings.len(),
        "chunks": chunks_with_embeddings,
    });

    let json_out = serde_json::to_string_pretty(&indexed_output)?;
    std::fs::write(&output_path, json_out)?;
    log::info!(
        "Saved {} indexed chunks to {}",
        chunks_with_embeddings.len(),
        output_path.display()
    );

    Ok(IngestionReport {
        books_processed: chunks_with_embeddings
            .iter()
            .map(|c| c.source.clone())
            .collect::<std::collections::HashSet<_>>()
            .len() as u64,
        chunks_created: total as u64,
        chunks_embedded: embedded,
        message: format!(
            "Ingestion complete. {}/{} chunks embedded. Saved to chunks_indexed.json",
            embedded, total
        ),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionReport {
    pub books_processed: u64,
    pub chunks_created: u64,
    pub chunks_embedded: u64,
    pub message: String,
}

/// Convert a raw chunk to the internal KnowledgeChunk type.
#[allow(dead_code)]
fn to_knowledge_chunk(raw: &RawChunk) -> KnowledgeChunk {
    KnowledgeChunk {
        id: uuid::Uuid::new_v4(),
        chunk_type: parse_chunk_type(&raw.chunk_type),
        content: raw.content.clone(),
        source: raw.source.clone(),
        position_fen: raw.position_fen.clone(),
        opening_eco: None,
        similarity: 0.0,
    }
}

#[allow(dead_code)]
fn parse_chunk_type(s: &str) -> ChunkType {
    match s {
        "concept" => ChunkType::Concept,
        "opening" => ChunkType::Opening,
        "motif" => ChunkType::Motif,
        "instructive_example" => ChunkType::InstructiveExample,
        "endgame_technique" => ChunkType::EndgameTechnique,
        _ => ChunkType::Concept,
    }
}
