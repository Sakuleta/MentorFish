// ─── PostgreSQL Store ───
//
// Implements GameStore and ProfileStore using sqlx + PostgreSQL.

use async_trait::async_trait;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

use crate::agents::UserProfile;
use crate::database::{DatabaseConfig, GameStore, ProfileStore};
use crate::memory::GameRecord;

pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    /// Create a new PostgresStore and run migrations.
    pub async fn connect(config: &DatabaseConfig) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.postgres_url)
            .await?;

        let store = Self { pool };
        store.run_migrations().await?;
        store.ensure_default_user().await?;

        Ok(store)
    }

    /// Run SQL migrations from the migrations directory.
    /// Splits the schema file into individual statements and executes each one.
    async fn run_migrations(&self) -> anyhow::Result<()> {
        let schema = include_str!("../../migrations/001_schema.sql");
        for statement in schema.split(';') {
            // Strip comment lines and whitespace; skip empty statements
            let stmt: String = statement
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed.is_empty() && !trimmed.starts_with("--")
                })
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();
            if !stmt.is_empty() {
                sqlx::query(&stmt).execute(&self.pool).await?;
            }
        }
        log::info!("Database migrations applied successfully");
        Ok(())
    }

    /// Ensure a default user exists (single-user local app).
    async fn ensure_default_user(&self) -> anyhow::Result<uuid::Uuid> {
        let row = sqlx::query("SELECT id FROM users LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(row.get("id"))
        } else {
            let row = sqlx::query("INSERT INTO users DEFAULT VALUES RETURNING id")
                .fetch_one(&self.pool)
                .await?;
            let user_id: uuid::Uuid = row.get("id");
            log::info!("Created default user: {}", user_id);
            Ok(user_id)
        }
    }

    /// Get the default user ID.
    pub async fn default_user_id(&self) -> anyhow::Result<uuid::Uuid> {
        let row = sqlx::query("SELECT id FROM users LIMIT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("id"))
    }
}

// ─── GameStore Implementation ───

#[async_trait]
impl GameStore for PostgresStore {
    async fn save_game(&self, record: GameRecord) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO games (id, user_id, pgn, result, played_at, source, opening_eco, time_control)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(record.game_id)
        .bind(record.user_id)
        .bind(&record.pgn)
        .bind(&record.result)
        .bind(&record.played_at)
        .bind(&record.source)
        .bind(&record.opening_eco)
        .bind(&record.time_control)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_game(&self, game_id: uuid::Uuid) -> anyhow::Result<Option<GameRecord>> {
        let row = sqlx::query(
            r#"SELECT id, user_id, pgn, result, played_at, source, opening_eco, time_control
               FROM games WHERE id = $1"#,
        )
        .bind(game_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| GameRecord {
            game_id: r.get("id"),
            user_id: r.get("user_id"),
            pgn: r.get("pgn"),
            result: r.get("result"),
            played_at: r.get("played_at"),
            source: r.get("source"),
            opening_eco: r.get("opening_eco"),
            time_control: r.get("time_control"),
        }))
    }

    async fn get_user_games(
        &self,
        user_id: uuid::Uuid,
        limit: u32,
    ) -> anyhow::Result<Vec<GameRecord>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, pgn, result, played_at, source, opening_eco, time_control
               FROM games WHERE user_id = $1 ORDER BY played_at DESC LIMIT $2"#,
        )
        .bind(user_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| GameRecord {
                game_id: r.get("id"),
                user_id: r.get("user_id"),
                pgn: r.get("pgn"),
                result: r.get("result"),
                played_at: r.get("played_at"),
                source: r.get("source"),
                opening_eco: r.get("opening_eco"),
                time_control: r.get("time_control"),
            })
            .collect())
    }

    async fn get_all_user_games(&self, user_id: uuid::Uuid) -> anyhow::Result<Vec<GameRecord>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, pgn, result, played_at, source, opening_eco, time_control
               FROM games WHERE user_id = $1 ORDER BY played_at DESC"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| GameRecord {
                game_id: r.get("id"),
                user_id: r.get("user_id"),
                pgn: r.get("pgn"),
                result: r.get("result"),
                played_at: r.get("played_at"),
                source: r.get("source"),
                opening_eco: r.get("opening_eco"),
                time_control: r.get("time_control"),
            })
            .collect())
    }

    async fn save_move(&self, game_id: uuid::Uuid, mv: &crate::Move) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO moves (game_id, move_number, color, uci_move, fen_before,
                   eval_cp, eval_cp_after, eval_swing, move_time_ms, classification)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
        )
        .bind(game_id)
        .bind(mv.move_number as i32)
        .bind(color_to_str(&mv.color))
        .bind(&mv.uci)
        .bind(&mv.fen_before)
        .bind(mv.eval_cp_before)
        .bind(mv.eval_cp_after)
        .bind(mv.eval_swing)
        .bind(mv.move_time_ms.map(|t| t as i32))
        .bind(mv.classification.as_ref().map(|c| classification_to_str(c)))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn save_moves(&self, game_id: uuid::Uuid, moves: &[crate::Move]) -> anyhow::Result<()> {
        for mv in moves {
            self.save_move(game_id, mv).await?;
        }
        Ok(())
    }

    async fn get_game_moves(&self, game_id: uuid::Uuid) -> anyhow::Result<Vec<crate::Move>> {
        let rows = sqlx::query(
            r#"SELECT move_number, color, uci_move, fen_before,
                      eval_cp, eval_cp_after, eval_swing, move_time_ms, classification
               FROM moves WHERE game_id = $1 ORDER BY move_number"#,
        )
        .bind(game_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let color_str: String = r.get("color");
                let class_str: Option<String> = r.get("classification");
                crate::Move {
                    uci: r.get("uci_move"),
                    san: None,
                    move_number: r.get::<i32, _>("move_number") as u32,
                    color: str_to_color(&color_str),
                    fen_before: r.get("fen_before"),
                    fen_after: String::new(),
                    eval_cp_before: r.get("eval_cp"),
                    eval_cp_after: r.get("eval_cp_after"),
                    eval_swing: r.get("eval_swing"),
                    move_time_ms: r.get::<Option<i32>, _>("move_time_ms").map(|t| t as u32),
                    classification: class_str.as_deref().map(str_to_classification),
                }
            })
            .collect())
    }
}

// ─── ProfileStore Implementation ───

#[async_trait]
impl ProfileStore for PostgresStore {
    async fn get_profile(&self, user_id: uuid::Uuid) -> anyhow::Result<Option<UserProfile>> {
        let row = sqlx::query(
            r#"SELECT user_id, tactical_accuracy, positional_accuracy, opening_knowledge,
                      endgame_technique, time_management, tilt_resistance,
                      style_profile, sample_counts, last_updated
               FROM user_profiles WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let style: serde_json::Value =
                serde_json::from_value(r.get("style_profile")).unwrap_or_default();
            let sample_counts: serde_json::Value =
                serde_json::from_value(r.get("sample_counts")).unwrap_or_default();

            // Calculate confidence from sample counts
            let min_samples = sample_counts
                .as_object()
                .map(|o| o.values().filter_map(|v| v.as_u64()).min().unwrap_or(0))
                .unwrap_or(0) as u32;

            let confidence = (min_samples as f64 / 20.0).min(1.0);

            UserProfile {
                user_id,
                tactical_accuracy: r.get("tactical_accuracy"),
                positional_accuracy: r.get("positional_accuracy"),
                opening_knowledge: r.get("opening_knowledge"),
                endgame_technique: r.get("endgame_technique"),
                time_management: r.get("time_management"),
                tilt_resistance: r.get("tilt_resistance"),
                style_profile: style,
                weakness_patterns: Vec::new(), // loaded separately
                confidence,
            }
        }))
    }

    async fn save_profile(&self, profile: &UserProfile) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO user_profiles (user_id, tactical_accuracy, positional_accuracy,
                      opening_knowledge, endgame_technique, time_management,
                      tilt_resistance, style_profile, last_updated)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
               ON CONFLICT (user_id) DO UPDATE SET
                   tactical_accuracy = EXCLUDED.tactical_accuracy,
                   positional_accuracy = EXCLUDED.positional_accuracy,
                   opening_knowledge = EXCLUDED.opening_knowledge,
                   endgame_technique = EXCLUDED.endgame_technique,
                   time_management = EXCLUDED.time_management,
                   tilt_resistance = EXCLUDED.tilt_resistance,
                   style_profile = EXCLUDED.style_profile,
                   last_updated = now()"#,
        )
        .bind(profile.user_id)
        .bind(profile.tactical_accuracy)
        .bind(profile.positional_accuracy)
        .bind(profile.opening_knowledge)
        .bind(profile.endgame_technique)
        .bind(profile.time_management)
        .bind(profile.tilt_resistance)
        .bind(&profile.style_profile)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

// ─── Move serialization helpers ───

fn color_to_str(color: &crate::Color) -> &'static str {
    match color {
        crate::Color::White => "White",
        crate::Color::Black => "Black",
    }
}

fn str_to_color(s: &str) -> crate::Color {
    match s.to_lowercase().as_str() {
        "white" => crate::Color::White,
        _ => crate::Color::Black,
    }
}

fn classification_to_str(c: &crate::MoveClassification) -> &'static str {
    match c {
        crate::MoveClassification::Best => "best",
        crate::MoveClassification::Good => "good",
        crate::MoveClassification::Inaccuracy => "inaccuracy",
        crate::MoveClassification::Mistake => "mistake",
        crate::MoveClassification::Blunder => "blunder",
    }
}

fn str_to_classification(s: &str) -> crate::MoveClassification {
    match s {
        "best" => crate::MoveClassification::Best,
        "good" => crate::MoveClassification::Good,
        "inaccuracy" => crate::MoveClassification::Inaccuracy,
        "mistake" => crate::MoveClassification::Mistake,
        _ => crate::MoveClassification::Blunder,
    }
}
