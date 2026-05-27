// ─── Database Layer ───
//
// Section 16 of the PRD.
// PostgreSQL 16 for structured data, LanceDB for vectors, Redis 7 for cache.

pub mod lancedb;
pub mod postgres;
pub mod redis;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::agents::{KnowledgeChunk, UserProfile};
use crate::memory::GameRecord;

// ─── Database Config ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub postgres_url: String,
    pub redis_url: String,
    pub lancedb_path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            postgres_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://mentorfish:mentorfish@localhost:5432/mentorfish".to_string()
            }),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            lancedb_path: std::env::var("LANCEDB_PATH")
                .unwrap_or_else(|_| "./data/lancedb".to_string()),
        }
    }
}

// ─── Database Traits ───

#[async_trait]
pub trait GameStore: Send + Sync {
    async fn save_game(&self, record: GameRecord) -> anyhow::Result<()>;
    async fn get_game(&self, game_id: uuid::Uuid) -> anyhow::Result<Option<GameRecord>>;
    async fn get_user_games(
        &self,
        user_id: uuid::Uuid,
        limit: u32,
    ) -> anyhow::Result<Vec<GameRecord>>;
    /// Fetch all games for a user without a limit (used for PGN export).
    async fn get_all_user_games(&self, user_id: uuid::Uuid) -> anyhow::Result<Vec<GameRecord>>;

    /// Persist a single analyzed move.
    async fn save_move(&self, game_id: uuid::Uuid, mv: &crate::Move) -> anyhow::Result<()>;
    /// Persist multiple analyzed moves in bulk.
    async fn save_moves(&self, game_id: uuid::Uuid, moves: &[crate::Move]) -> anyhow::Result<()>;
    /// Retrieve all persisted moves for a game.
    async fn get_game_moves(&self, game_id: uuid::Uuid) -> anyhow::Result<Vec<crate::Move>>;
}

#[async_trait]
pub trait ProfileStore: Send + Sync {
    async fn get_profile(&self, user_id: uuid::Uuid) -> anyhow::Result<Option<UserProfile>>;
    async fn save_profile(&self, profile: &UserProfile) -> anyhow::Result<()>;
}

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn search(&self, embedding: &[f32], limit: usize) -> anyhow::Result<Vec<KnowledgeChunk>>;
    async fn insert(
        &self,
        chunks: &[KnowledgeChunk],
        embeddings: &[Vec<f32>],
    ) -> anyhow::Result<()>;
}

// ─── Unified Database Manager ───

pub struct DatabaseManager {
    postgres: postgres::PostgresStore,
    vectors: lancedb::LanceDbStore,
    pub cache: redis::MemoryCache,
}

impl DatabaseManager {
    pub async fn init(config: &DatabaseConfig) -> anyhow::Result<Self> {
        let postgres = postgres::PostgresStore::connect(config).await?;
        let vectors = lancedb::LanceDbStore::new(config.lancedb_path.clone().into());

        Ok(Self {
            postgres,
            vectors,
            cache: redis::MemoryCache::new(),
        })
    }

    pub fn game_store(&self) -> &dyn GameStore {
        &self.postgres
    }

    pub fn profile_store(&self) -> &dyn ProfileStore {
        &self.postgres
    }

    pub fn vector_store(&self) -> &dyn VectorStore {
        &self.vectors
    }

    /// Get the default user ID (single-user local app).
    pub async fn default_user_id(&self) -> anyhow::Result<uuid::Uuid> {
        self.postgres.default_user_id().await
    }
}

// Test helpers
#[async_trait]
impl GameStore for DatabaseManager {
    async fn save_game(&self, record: GameRecord) -> anyhow::Result<()> {
        self.postgres.save_game(record).await
    }
    async fn get_game(&self, game_id: uuid::Uuid) -> anyhow::Result<Option<GameRecord>> {
        self.postgres.get_game(game_id).await
    }
    async fn get_user_games(
        &self,
        user_id: uuid::Uuid,
        limit: u32,
    ) -> anyhow::Result<Vec<GameRecord>> {
        self.postgres.get_user_games(user_id, limit).await
    }
    async fn get_all_user_games(&self, user_id: uuid::Uuid) -> anyhow::Result<Vec<GameRecord>> {
        self.postgres.get_all_user_games(user_id).await
    }
    async fn save_move(&self, game_id: uuid::Uuid, mv: &crate::Move) -> anyhow::Result<()> {
        self.postgres.save_move(game_id, mv).await
    }
    async fn save_moves(&self, game_id: uuid::Uuid, moves: &[crate::Move]) -> anyhow::Result<()> {
        self.postgres.save_moves(game_id, moves).await
    }
    async fn get_game_moves(&self, game_id: uuid::Uuid) -> anyhow::Result<Vec<crate::Move>> {
        self.postgres.get_game_moves(game_id).await
    }
}

#[async_trait]
impl ProfileStore for DatabaseManager {
    async fn get_profile(&self, user_id: uuid::Uuid) -> anyhow::Result<Option<UserProfile>> {
        self.postgres.get_profile(user_id).await
    }
    async fn save_profile(&self, profile: &UserProfile) -> anyhow::Result<()> {
        self.postgres.save_profile(profile).await
    }
}

#[async_trait]
impl VectorStore for DatabaseManager {
    async fn search(&self, embedding: &[f32], limit: usize) -> anyhow::Result<Vec<KnowledgeChunk>> {
        self.vectors.search(embedding, limit).await
    }
    async fn insert(
        &self,
        chunks: &[KnowledgeChunk],
        embeddings: &[Vec<f32>],
    ) -> anyhow::Result<()> {
        self.vectors.insert(chunks, embeddings).await
    }
}
