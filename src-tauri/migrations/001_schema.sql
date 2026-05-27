-- MentorFish Database Schema — Section 16.2 of the PRD. PostgreSQL 16.

CREATE TABLE IF NOT EXISTS users (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username   TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS games (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id),
    pgn          TEXT NOT NULL,
    result       TEXT,
    played_at    TIMESTAMP NOT NULL,
    source       TEXT,
    opening_eco  TEXT,
    time_control TEXT,
    created_at   TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS moves (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    game_id        UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    move_number    INTEGER NOT NULL,
    color          TEXT NOT NULL,
    uci_move       TEXT NOT NULL,
    fen_before     TEXT NOT NULL,
    eval_cp        INTEGER,
    eval_cp_after  INTEGER,
    eval_swing     INTEGER,
    move_time_ms   INTEGER,
    classification TEXT,
    created_at     TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_moves_game_id ON moves(game_id);

CREATE TABLE IF NOT EXISTS user_profiles (
    user_id              UUID PRIMARY KEY REFERENCES users(id),
    tactical_accuracy    FLOAT NOT NULL DEFAULT 0.5,
    positional_accuracy  FLOAT NOT NULL DEFAULT 0.5,
    opening_knowledge    FLOAT NOT NULL DEFAULT 0.5,
    endgame_technique    FLOAT NOT NULL DEFAULT 0.5,
    time_management      FLOAT NOT NULL DEFAULT 0.5,
    tilt_resistance      FLOAT NOT NULL DEFAULT 0.5,
    style_profile        JSONB NOT NULL DEFAULT '{}',
    sample_counts        JSONB NOT NULL DEFAULT '{}',
    last_updated         TIMESTAMP
);

CREATE TABLE IF NOT EXISTS weakness_patterns (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id),
    pattern_name     TEXT NOT NULL,
    description      TEXT,
    example_fens     TEXT[],
    occurrence_count INTEGER NOT NULL DEFAULT 0,
    last_seen        TIMESTAMP,
    created_at       TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS study_sessions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id),
    session_type TEXT,
    started_at   TIMESTAMP NOT NULL,
    ended_at     TIMESTAMP,
    notes        TEXT
);

CREATE TABLE IF NOT EXISTS user_repertoire (
    user_id      UUID NOT NULL REFERENCES users(id),
    fen          TEXT NOT NULL,
    color        TEXT NOT NULL,
    familiarity  FLOAT NOT NULL DEFAULT 0.0,
    last_played  TIMESTAMP,
    PRIMARY KEY (user_id, fen, color)
);

CREATE TABLE IF NOT EXISTS opening_positions (
    fen              TEXT PRIMARY KEY,
    eco              TEXT,
    opening_name     TEXT,
    parent_fen       TEXT,
    move_from_parent TEXT,
    frequency        INTEGER,
    white_score      FLOAT,
    theory_chunk_ids UUID[],
    created_at       TIMESTAMP NOT NULL DEFAULT now()
);

-- Book study progress (Section 3.7)
CREATE TABLE IF NOT EXISTS book_progress (
    user_id       UUID NOT NULL REFERENCES users(id),
    book_source   TEXT NOT NULL,
    current_page  INTEGER NOT NULL DEFAULT 0,
    total_pages   INTEGER NOT NULL DEFAULT 0,
    progress_pct  FLOAT NOT NULL DEFAULT 0.0,
    last_read_at  TIMESTAMP,
    notes         TEXT,
    bookmarks     JSONB DEFAULT '[]',
    PRIMARY KEY (user_id, book_source)
);
