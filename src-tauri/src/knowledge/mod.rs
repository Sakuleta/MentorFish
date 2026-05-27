// ─── Knowledge / RAG System ───
//
// Section 8 of the PRD.
// Handles ingestion, storage, and retrieval of chess knowledge.

pub mod ingestion;

use crate::agents::{ChunkType, KnowledgeChunk, OpeningMove, OpeningNode};
use crate::FEN;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

// ─── Knowledge Directory (resolved at startup) ───

/// Global knowledge directory path, initialized during app startup.
/// Set once by `initialize_knowledge_dir()` in `lib.rs`.
static KNOWLEDGE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Initialize the global knowledge directory path.
/// Called once during Tauri app setup. Must be called before
/// any retrieval or ingestion functions are used.
pub fn initialize_knowledge_dir(dir: PathBuf) {
    log::info!("Knowledge directory initialized: {:?}", dir);
    KNOWLEDGE_DIR.set(dir).ok();
}

/// Returns the resolved knowledge directory path.
/// Falls back to a relative `"knowledge"` path if
/// `initialize_knowledge_dir` was not called (e.g., in tests).
pub fn knowledge_dir() -> &'static Path {
    KNOWLEDGE_DIR
        .get()
        .map(|p| p.as_path())
        .unwrap_or_else(|| Path::new("knowledge"))
}

// ─── Knowledge Corpus (Section 8.1) ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeTier {
    /// Tier 1 — Required (RAG backbone)
    Tier1,
    /// Tier 2 — High Priority
    Tier2,
    /// Tier 3 — Supplementary
    Tier3,
    /// User's own games
    UserGames,
    /// Personal notes
    PersonalNotes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSource {
    pub title: String,
    pub author: String,
    pub tier: KnowledgeTier,
    pub file_path: String,
    pub chunk_count: u32,
    pub ingested_at: Option<String>,
}

// ─── Retrieval System (Section 8.5) ───

/// Result of a RAG retrieval query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub chunks: Vec<KnowledgeChunk>,
    pub query_time_ms: u64,
    pub total_chunks_searched: u64,
}

/// Classification of similarity confidence.
#[derive(Debug, Clone, PartialEq)]
pub enum SimilarityClass {
    /// Cosine similarity >= 0.72
    High,
    /// 0.60 <= similarity < 0.72
    Medium,
    /// Similarity < 0.60 — discarded
    Low,
}

impl SimilarityClass {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.72 {
            SimilarityClass::High
        } else if score >= 0.60 {
            SimilarityClass::Medium
        } else {
            SimilarityClass::Low
        }
    }
}

// ─── In-Memory Knowledge Store ───

/// Raw chunk from the JSON ingestion output, stored for retrieval.
#[derive(Debug, Clone, Deserialize)]
struct RawStoredChunk {
    content: String,
    source: String,
    chunk_type: String,
    #[serde(default)]
    position_fen: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    token_count: usize,
    #[serde(default)]
    embedding: Option<Vec<f32>>,
}

/// In-memory index: maps lowercase words (≥3 chars) to chunk indices.
type InvertedIndex = HashMap<String, Vec<usize>>;

/// Holds loaded knowledge data and indexes for fast retrieval.
struct KnowledgeStore {
    /// All chunks loaded from chunks_all.json (or chunks_indexed.json)
    chunks: Vec<RawStoredChunk>,
    /// Embeddings parallel to chunks (same index). None if chunk has no embedding.
    embeddings: Vec<Option<Vec<f32>>>,
    /// Inverted index: word → list of chunk indices containing that word
    index: InvertedIndex,
    /// Opening book: FEN → OpeningNode
    openings: HashMap<FEN, OpeningNode>,
    /// Whether the store has been successfully loaded
    loaded: bool,
}

impl KnowledgeStore {
    fn new() -> Self {
        KnowledgeStore {
            chunks: Vec::new(),
            embeddings: Vec::new(),
            index: InvertedIndex::new(),
            openings: HashMap::new(),
            loaded: false,
        }
    }

    /// Load chunks and openings from the knowledge directory.
    fn load(&mut self) {
        if self.loaded {
            return;
        }

        let knowledge_dir = knowledge_dir();

        // ── Load chunks ──
        // Prefer the indexed file (from ingestion), fall back to chunks_all.json
        let chunks_path = knowledge_dir.join("chunks_indexed.json");
        let fallback_path = knowledge_dir.join("chunks_all.json");

        let path = if chunks_path.exists() {
            &chunks_path
        } else if fallback_path.exists() {
            &fallback_path
        } else {
            log::warn!("No chunks file found. RAG retrieval will be empty.");
            self.loaded = true;
            return;
        };

        match std::fs::read_to_string(path) {
            Ok(json) => match serde_json::from_str::<serde_json::Value>(&json) {
                Ok(root) => {
                    if let Some(chunks_arr) = root.get("chunks").and_then(|c| c.as_array()) {
                        self.chunks = chunks_arr
                            .iter()
                            .filter_map(|c| {
                                serde_json::from_value::<RawStoredChunk>(c.clone()).ok()
                            })
                            .collect();

                        // Extract embeddings in parallel to chunks
                        self.embeddings = self
                            .chunks
                            .iter()
                            .map(|chunk| chunk.embedding.clone())
                            .collect();

                        let with_embeddings =
                            self.embeddings.iter().filter(|e| e.is_some()).count();
                        log::info!(
                            "Loaded {} chunks from {} ({} with embeddings)",
                            self.chunks.len(),
                            path.display(),
                            with_embeddings
                        );

                        // Build inverted index
                        self.build_index();
                    } else {
                        log::warn!("Chunks file has no 'chunks' array.");
                    }
                }
                Err(e) => {
                    log::warn!("Failed to parse chunks JSON: {}", e);
                }
            },
            Err(e) => {
                log::warn!("Failed to read chunks file {}: {}", path.display(), e);
            }
        }

        // ── Load openings ──
        let openings_path = knowledge_dir.join("openings_tree_merged.json");

        if openings_path.exists() {
            match std::fs::read_to_string(&openings_path) {
                Ok(json) => match serde_json::from_str::<serde_json::Value>(&json) {
                    Ok(root) => {
                        if let Some(positions) = root.get("positions").and_then(|p| p.as_array()) {
                            for pos in positions {
                                if let Some(node) = parse_opening_node(pos) {
                                    self.openings.insert(node.fen.clone(), node);
                                }
                            }

                            // Enrich openings with ECO/name detection for
                            // positions that lack metadata (e.g. from ABK files).
                            // Note: JSON fields may be empty strings "" (not null),
                            // so we check `is_empty()` rather than `is_none()`.
                            let mut enriched = 0usize;
                            for (_fen, node) in &mut self.openings {
                                let eco_missing = node.eco.as_ref().map_or(true, |s| s.is_empty());
                                let name_missing =
                                    node.opening_name.as_ref().map_or(true, |s| s.is_empty());
                                if eco_missing && name_missing {
                                    if let Some((eco, name)) = detect_opening(_fen) {
                                        node.eco = Some(eco);
                                        node.opening_name = Some(name);
                                        enriched += 1;
                                    }
                                }
                            }
                            if enriched > 0 {
                                log::info!("Enriched {} opening positions with ECO/name", enriched);
                            }

                            log::info!(
                                "Loaded {} opening positions from {}",
                                self.openings.len(),
                                openings_path.display()
                            );
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to parse openings JSON: {}", e);
                    }
                },
                Err(e) => {
                    log::warn!(
                        "Failed to read openings file {}: {}",
                        openings_path.display(),
                        e
                    );
                }
            }
        }

        self.loaded = true;
    }

    /// Build an inverted index from loaded chunks.
    fn build_index(&mut self) {
        self.index.clear();
        for (i, chunk) in self.chunks.iter().enumerate() {
            let words = tokenize(&chunk.content);
            for word in words {
                self.index.entry(word).or_default().push(i);
            }
        }
        log::info!(
            "Built inverted index with {} unique terms across {} chunks",
            self.index.len(),
            self.chunks.len()
        );
    }

    /// Hybrid retrieval: combines semantic (cosine) and keyword (inverted-index) scoring.
    /// When `embedding` is provided, uses 80% semantic / 20% keyword weighting.
    /// Falls back to keyword-only if no embedding is given.
    fn search(
        &self,
        position_fen: Option<&str>,
        query: &str,
        max_chunks: usize,
        embedding: Option<&[f32]>,
    ) -> RetrievalResult {
        let start = Instant::now();
        let total = self.chunks.len() as u64;

        if self.chunks.is_empty() || query.is_empty() {
            return RetrievalResult {
                chunks: Vec::new(),
                query_time_ms: start.elapsed().as_millis() as u64,
                total_chunks_searched: 0,
            };
        }

        // ── Semantic scoring (if embedding is available) ──
        let semantic_scores: HashMap<usize, f64> = if let Some(query_emb) = embedding {
            semantic_search(&self.embeddings, query_emb)
        } else {
            HashMap::new()
        };

        // ── Keyword scoring ──
        let query_words: Vec<String> = tokenize(query);
        let keyword_scores = if !query_words.is_empty() {
            keyword_search(&self.index, &query_words, max_chunks)
        } else {
            HashMap::new()
        };

        // ── Combine scores ──
        let use_hybrid = !semantic_scores.is_empty();

        // Collect all candidate indices
        let mut all_indices: Vec<usize> = semantic_scores.keys().copied().collect();
        for &idx in keyword_scores.keys() {
            if !all_indices.contains(&idx) {
                all_indices.push(idx);
            }
        }

        let max_kw = keyword_scores
            .values()
            .fold(0.0f64, |a, &b| a.max(b))
            .max(1.0);
        let max_sem = semantic_scores
            .values()
            .fold(0.0f64, |a, &b| a.max(b))
            .max(1.0);

        let mut combined: Vec<(usize, f64)> = all_indices
            .iter()
            .map(|&idx| {
                let sem = semantic_scores.get(&idx).copied().unwrap_or(0.0);
                let kw = keyword_scores.get(&idx).copied().unwrap_or(0.0);

                let sem_norm = if max_sem > 0.0 { sem / max_sem } else { 0.0 };
                let kw_norm = if max_kw > 0.0 { kw / max_kw } else { 0.0 };

                let score = if use_hybrid {
                    0.8 * sem_norm + 0.2 * kw_norm
                } else {
                    kw_norm
                };

                (idx, score)
            })
            .collect();

        // Sort by combined score descending
        combined.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        // Boost chunks with matching position_fen (adds a fixed bonus)
        let fen_boost = 0.15;
        if position_fen.is_some() {
            for (idx, score) in &mut combined {
                if let Some(ref chunk_fen) = self.chunks[*idx].position_fen {
                    if Some(chunk_fen.as_str()) == position_fen {
                        *score += fen_boost;
                    }
                }
            }
            // Re-sort after FEN boost
            combined
                .sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        }

        // Build final score lookup for the result
        let final_scores: HashMap<usize, f64> = combined.iter().map(|&(idx, s)| (idx, s)).collect();

        // Deduplicate by content similarity (simple prefix check)
        let mut seen_prefixes: Vec<&str> = Vec::new();
        let chunks: Vec<KnowledgeChunk> = combined
            .iter()
            .filter(|(_, score)| *score > 0.0)
            .filter(|&&(idx, _)| {
                let chunk = &self.chunks[idx];
                let prefix = &chunk.content[..chunk.content.len().min(80)];
                if seen_prefixes.iter().any(|s| {
                    let common = s
                        .chars()
                        .zip(prefix.chars())
                        .take_while(|(a, b)| a == b)
                        .count();
                    common >= 40
                }) {
                    false
                } else {
                    seen_prefixes.push(prefix);
                    true
                }
            })
            .take(max_chunks)
            .map(|&(idx, _)| {
                let chunk = &self.chunks[idx];
                KnowledgeChunk {
                    id: uuid::Uuid::new_v4(),
                    chunk_type: parse_chunk_type_str(&chunk.chunk_type),
                    content: chunk.content.clone(),
                    source: chunk.source.clone(),
                    position_fen: chunk.position_fen.clone(),
                    opening_eco: None,
                    similarity: final_scores.get(&idx).copied().unwrap_or(0.0),
                }
            })
            .collect();

        RetrievalResult {
            chunks,
            query_time_ms: start.elapsed().as_millis() as u64,
            total_chunks_searched: total,
        }
    }

    /// Retrieve chunks by their chunk_type label.
    fn search_by_type(&self, chunk_type: &str, max_chunks: usize) -> RetrievalResult {
        let start = Instant::now();
        let total = self.chunks.len() as u64;

        let matching: Vec<KnowledgeChunk> = self
            .chunks
            .iter()
            .filter(|c| c.chunk_type == chunk_type)
            .take(max_chunks)
            .map(|c| KnowledgeChunk {
                id: uuid::Uuid::new_v4(),
                chunk_type: parse_chunk_type_str(&c.chunk_type),
                content: c.content.clone(),
                source: c.source.clone(),
                position_fen: c.position_fen.clone(),
                opening_eco: None,
                similarity: 1.0,
            })
            .collect();

        RetrievalResult {
            chunks: matching,
            query_time_ms: start.elapsed().as_millis() as u64,
            total_chunks_searched: total,
        }
    }

    /// Retrieve chunks by their source (book title).
    /// Uses substring matching: returns chunks where `source` contains the given
    /// search string or vice versa.
    fn search_by_source(&self, source: &str, max_chunks: usize) -> RetrievalResult {
        let start = Instant::now();
        let total = self.chunks.len() as u64;

        if source.is_empty() {
            return RetrievalResult {
                chunks: Vec::new(),
                query_time_ms: start.elapsed().as_millis() as u64,
                total_chunks_searched: total,
            };
        }

        let matching: Vec<KnowledgeChunk> = self
            .chunks
            .iter()
            .filter(|c| c.source.contains(source) || source.contains(&c.source))
            .take(max_chunks)
            .map(|c| KnowledgeChunk {
                id: uuid::Uuid::new_v4(),
                chunk_type: parse_chunk_type_str(&c.chunk_type),
                content: c.content.clone(),
                source: c.source.clone(),
                position_fen: c.position_fen.clone(),
                opening_eco: None,
                similarity: 1.0,
            })
            .collect();

        RetrievalResult {
            chunks: matching,
            query_time_ms: start.elapsed().as_millis() as u64,
            total_chunks_searched: total,
        }
    }

    /// Look up an opening node by exact FEN match.
    fn search_opening(&self, fen: &str) -> Option<OpeningNode> {
        // Try exact match first
        if let Some(node) = self.openings.get(fen) {
            return Some(node.clone());
        }

        // Try normalized FEN: strip move number and half-move clock parts
        let fen_parts: Vec<&str> = fen.split_whitespace().collect();
        if fen_parts.len() >= 4 {
            // Try with just board + active color + castling + en passant (no halfmove/fullmove)
            let partial = format!(
                "{} {} {} {}",
                fen_parts[0], fen_parts[1], fen_parts[2], fen_parts[3]
            );
            for (full_fen, node) in &self.openings {
                let full_parts: Vec<&str> = full_fen.split_whitespace().collect();
                if full_parts.len() >= 4 {
                    let full_partial = format!(
                        "{} {} {} {}",
                        full_parts[0], full_parts[1], full_parts[2], full_parts[3]
                    );
                    if partial == full_partial {
                        return Some(node.clone());
                    }
                }
            }
        }

        None
    }
}

// ─── Global Store (lazy-loaded) ───

#[allow(clippy::incompatible_msrv)]
static STORE: std::sync::LazyLock<Mutex<KnowledgeStore>> =
    std::sync::LazyLock::new(|| Mutex::new(KnowledgeStore::new()));

fn ensure_loaded(store: &mut KnowledgeStore) {
    store.load();
}

// ─── Public Retrieval API ───

/// Retrieve relevant knowledge chunks for a position + query.
/// Performs hybrid retrieval: when an embedding is provided, uses 80% semantic
/// (cosine similarity) + 20% keyword weighting. Falls back to keyword-only otherwise.
pub async fn retrieve(
    position_fen: &FEN,
    query: &str,
    max_chunks: usize,
    embedding: Option<&[f32]>,
) -> anyhow::Result<RetrievalResult> {
    let mut store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded(&mut store);

    Ok(store.search(Some(position_fen), query, max_chunks, embedding))
}

/// Retrieve knowledge by chunk type (e.g., all endgame_technique chunks).
pub async fn retrieve_by_type(
    chunk_type: &str,
    max_chunks: usize,
) -> anyhow::Result<RetrievalResult> {
    let mut store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded(&mut store);

    Ok(store.search_by_type(chunk_type, max_chunks))
}

/// Retrieve knowledge by source (book title).
/// Filters chunks whose `source` field contains the given book title
/// (or whose book title contains the chunk source — substring match).
pub async fn retrieve_by_source(
    source: &str,
    max_chunks: usize,
) -> anyhow::Result<RetrievalResult> {
    let mut store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded(&mut store);

    Ok(store.search_by_source(source, max_chunks))
}

/// Retrieve opening-specific knowledge for a FEN position.
/// Looks up the openings_tree_merged.json data for the exact or
/// partial FEN match and returns the corresponding OpeningNode.
pub async fn retrieve_opening(fen: &FEN) -> anyhow::Result<Option<OpeningNode>> {
    let mut store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded(&mut store);

    Ok(store.search_opening(fen))
}

// ─── Helpers ───

/// Cosine similarity between two vectors.
/// Returns a value in [-1, 1] where 1 = identical direction.
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

/// Compute cosine similarity between a query embedding and all stored chunk embeddings.
/// Returns a map of chunk index → similarity score (clamped to [0, 1]).
fn semantic_search(
    embeddings: &[Option<Vec<f32>>],
    query_embedding: &[f32],
) -> HashMap<usize, f64> {
    let mut scores = HashMap::new();
    for (i, emb_opt) in embeddings.iter().enumerate() {
        if let Some(emb) = emb_opt {
            let sim = cosine_similarity(query_embedding, emb);
            // Clamp to [0, 1] — negative similarities are not useful for retrieval
            let clamped = sim.clamp(0.0, 1.0);
            if clamped > 0.0 {
                scores.insert(i, clamped);
            }
        }
    }
    scores
}

/// Keyword-only search using the inverted index.
/// Returns a map of chunk index → match count (before normalization).
fn keyword_search(
    index: &InvertedIndex,
    query_words: &[String],
    max_chunks: usize,
) -> HashMap<usize, f64> {
    let mut scores: HashMap<usize, f64> = HashMap::new();

    // Exact word matches
    for word in query_words {
        if let Some(hits) = index.get(word) {
            for &idx in hits {
                *scores.entry(idx).or_default() += 1.0;
            }
        }
    }

    // Also add partial matching: check if any query word is a substring of indexed words
    // and collect those hits too (for fuzzy matching)
    if scores.len() < max_chunks * 2 {
        for word in query_words {
            for (index_word, hits) in index.iter() {
                if index_word.contains(word) || word.contains(index_word) {
                    for &idx in hits {
                        *scores.entry(idx).or_default() += 0.5;
                    }
                }
            }
        }
    }

    scores
}

/// Tokenize a text into lowercase words (≥3 chars, alphanumeric).
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() >= 3 && !is_stop_word(w))
        .collect()
}

/// Common stop words to exclude from indexing.
fn is_stop_word(word: &str) -> bool {
    matches!(
        word,
        "the"
            | "and"
            | "for"
            | "that"
            | "this"
            | "with"
            | "from"
            | "have"
            | "are"
            | "was"
            | "not"
            | "but"
            | "all"
            | "has"
            | "had"
            | "been"
            | "were"
            | "its"
            | "his"
            | "her"
            | "also"
            | "can"
            | "may"
            | "each"
            | "out"
            | "then"
            | "them"
            | "these"
            | "some"
            | "what"
            | "when"
            | "will"
            | "more"
            | "does"
            | "there"
            | "which"
            | "their"
            | "about"
            | "would"
            | "could"
            | "other"
            | "into"
            | "than"
            | "just"
            | "over"
            | "such"
            | "only"
            | "very"
            | "your"
    )
}

/// Parse chunk_type string to the ChunkType enum.
fn parse_chunk_type_str(s: &str) -> ChunkType {
    match s {
        "concept" => ChunkType::Concept,
        "opening" => ChunkType::Opening,
        "motif" => ChunkType::Motif,
        "instructive_example" => ChunkType::InstructiveExample,
        "endgame_technique" => ChunkType::EndgameTechnique,
        _ => ChunkType::Concept,
    }
}

/// Get a static HashMap of known opening positions mapped to (ECO, name).
/// Uses `OnceLock` for lazy initialization.
fn get_opening_names() -> &'static HashMap<&'static str, (&'static str, &'static str)> {
    static NAMES: OnceLock<HashMap<&'static str, (&'static str, &'static str)>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR",
            ("", "Starting Position"),
        );
        m.insert(
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR",
            ("C00", "King's Pawn"),
        );
        m.insert(
            "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR",
            ("A40", "Queen's Pawn"),
        );
        m.insert(
            "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR",
            ("B20", "Sicilian Defense"),
        );
        m.insert(
            "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR",
            ("C40", "King's Pawn Game"),
        );
        m.insert(
            "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR",
            ("A10", "English Opening"),
        );
        m.insert(
            "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R",
            ("A04", "Réti Opening"),
        );
        m.insert(
            "rnbqkbnr/pp1ppppp/8/2p5/2P5/8/PP1PPPPP/RNBQKBNR",
            ("A10", "English, Sicilian"),
        );
        m.insert(
            "rnbqkbnr/pp1ppppp/8/2p5/3P4/8/PPP1PPPP/RNBQKBNR",
            ("A40", "Queen's Pawn, Sicilian"),
        );
        m.insert(
            "rnbqkbnr/pppp1ppp/8/4p3/2P5/8/PP1PPPPP/RNBQKBNR",
            ("A20", "English Opening"),
        );
        m.insert(
            "rnbqkbnr/pppp1ppp/8/4p3/3P4/8/PPP1PPPP/RNBQKBNR",
            ("C00", "French Defense"),
        );
        m.insert(
            "rnbqkbnr/ppp1pppp/8/3p4/3P4/8/PPP1PPPP/RNBQKBNR",
            ("D00", "Queen's Pawn, d5"),
        );
        m.insert(
            "rnbqkbnr/ppp1pppp/8/3p4/2PP4/8/PP2PPPP/RNBQKBNR",
            ("D06", "Queen's Gambit"),
        );
        m.insert(
            "rnbqkbnr/ppp2ppp/4p3/3p4/2PP4/8/PP2PPPP/RNBQKBNR",
            ("D30", "Queen's Gambit Declined"),
        );
        m.insert(
            "rnbqkbnr/ppp2ppp/4p3/3p4/3P4/8/PPP1PPPP/RNBQKBNR",
            ("D00", "Queen's Pawn"),
        );
        m.insert(
            "rnbqkbnr/pp2pppp/8/2pp4/3P4/8/PPP1PPPP/RNBQKBNR",
            ("D00", "Queen's Pawn"),
        );
        m.insert(
            "rnbqkb1r/pppppppp/5n2/8/3P4/8/PPP1PPPP/RNBQKBNR",
            ("A40", "Indian Defense"),
        );
        m.insert(
            "rnbqkb1r/pppppppp/5n2/8/2P5/8/PP1PPPPP/RNBQKBNR",
            ("A10", "English, Indian"),
        );
        m.insert(
            "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR",
            ("B00", "King's Pawn, Nf6"),
        );
        m.insert(
            "rnbqkb1r/pp1ppppp/5n2/2p5/4P3/8/PPPP1PPP/RNBQKBNR",
            ("B27", "Sicilian, Nf6"),
        );
        m.insert(
            "rnbqkb1r/pp1ppppp/3p4/2p5/2PP4/8/PP2PPPP/RNBQKBNR",
            ("D30", "QGD, Sicilian"),
        );
        m.insert(
            "rnbqkb1r/ppp1pppp/5n2/3p4/3P4/8/PPP1PPPP/RNBQKBNR",
            ("D00", "Queen's Pawn, Nf6"),
        );
        m.insert(
            "rnbqkb1r/ppp1pppp/3p4/8/3P4/8/PPP1PPPP/RNBQKBNR",
            ("D00", "Queen's Pawn, d6"),
        );
        m.insert(
            "rnbqkbnr/ppppp1pp/8/5p2/3P4/8/PPP1PPPP/RNBQKBNR",
            ("A40", "Queen's Pawn, f5"),
        );
        m.insert(
            "rnbqkbnr/ppppp1pp/8/5p2/4P3/8/PPPP1PPP/RNBQKBNR",
            ("C00", "King's Pawn, f5"),
        );
        m.insert(
            "rnbqkbnr/pp1ppppp/2p5/8/4P3/8/PPPP1PPP/RNBQKBNR",
            ("B10", "Caro-Kann Defense"),
        );
        m.insert(
            "rnbqkbnr/pp1ppppp/2p5/8/3P4/8/PPP1PPPP/RNBQKBNR",
            ("A40", "Queen's Pawn, c6"),
        );
        m.insert(
            "rnbqkbnr/ppp2ppp/4p3/3p4/4P3/8/PPPP1PPP/RNBQKBNR",
            ("C00", "French Defense"),
        );
        m.insert(
            "rnbqkbnr/pp2pppp/8/2pp4/4P3/8/PPPP1PPP/RNBQKBNR",
            ("B20", "Sicilian Defense"),
        );
        m.insert(
            "rnbqkb1r/pp1ppppp/3p4/2p5/4P3/3P4/PPP2PPP/RNBQKBNR",
            ("B50", "Sicilian, Modern"),
        );
        m
    })
}

/// Detect the opening name and ECO code for a given FEN.
/// Matches on the piece placement part (the first field before any space).
fn detect_opening(fen: &str) -> Option<(String, String)> {
    let board_part = fen.split_whitespace().next()?;
    get_opening_names()
        .get(board_part)
        .map(|&(eco, name)| (eco.to_string(), name.to_string()))
}

/// Parse a JSON value into an OpeningNode.
fn parse_opening_node(value: &serde_json::Value) -> Option<OpeningNode> {
    let fen = value.get("fen")?.as_str()?.to_string();
    let eco = value
        .get("eco")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let opening_name = value
        .get("opening_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let frequency = value
        .get("frequency")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);
    let white_score = value.get("white_score").and_then(|v| v.as_f64());

    let children = value
        .get("children")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    Some(OpeningMove {
                        uci: c.get("uci")?.as_str()?.to_string(),
                        san: c.get("san")?.as_str()?.to_string(),
                        frequency: c.get("frequency").and_then(|v| v.as_i64())? as i32,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(OpeningNode {
        fen,
        eco,
        opening_name,
        frequency,
        white_score,
        children,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_empty() {
        let result = tokenize("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_tokenize_stop_words() {
        // All stop words should be filtered
        let result = tokenize("the and for that this with from");
        assert!(result.is_empty(), "All stop words, got: {:?}", result);
    }

    #[test]
    fn test_similarity_class_high() {
        assert_eq!(SimilarityClass::from_score(0.72), SimilarityClass::High);
        assert_eq!(SimilarityClass::from_score(0.85), SimilarityClass::High);
        assert_eq!(SimilarityClass::from_score(1.0), SimilarityClass::High);
    }

    #[test]
    fn test_similarity_class_medium() {
        assert_eq!(SimilarityClass::from_score(0.60), SimilarityClass::Medium);
        assert_eq!(SimilarityClass::from_score(0.65), SimilarityClass::Medium);
        assert_eq!(SimilarityClass::from_score(0.719), SimilarityClass::Medium);
    }

    #[test]
    fn test_similarity_class_low() {
        assert_eq!(SimilarityClass::from_score(0.0), SimilarityClass::Low);
        assert_eq!(SimilarityClass::from_score(0.30), SimilarityClass::Low);
        assert_eq!(SimilarityClass::from_score(0.599), SimilarityClass::Low);
    }

    #[test]
    fn test_parse_chunk_type_valid() {
        assert_eq!(parse_chunk_type_str("concept"), ChunkType::Concept);
        assert_eq!(parse_chunk_type_str("opening"), ChunkType::Opening);
        assert_eq!(parse_chunk_type_str("motif"), ChunkType::Motif);
        assert_eq!(
            parse_chunk_type_str("instructive_example"),
            ChunkType::InstructiveExample
        );
        assert_eq!(
            parse_chunk_type_str("endgame_technique"),
            ChunkType::EndgameTechnique
        );
    }

    #[test]
    fn test_parse_chunk_type_invalid() {
        // Unknown types fall back to Concept
        assert_eq!(parse_chunk_type_str("unknown"), ChunkType::Concept);
        assert_eq!(parse_chunk_type_str(""), ChunkType::Concept);
        assert_eq!(parse_chunk_type_str("random_type"), ChunkType::Concept);
    }
}
