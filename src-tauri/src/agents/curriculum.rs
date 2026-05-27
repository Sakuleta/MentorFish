// ─── Curriculum Agent ───
//
// Section 7.4, Agent E.
// LLM call: Qwen3-14B.
// Generates personalized study plans.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudyPlan {
    pub weekly_sessions: Vec<StudySession>,
    pub opening_drills: Vec<OpeningDrill>,
    pub endgame_exercises: Vec<EndgameExercise>,
    pub tactical_puzzle_theme: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudySession {
    pub day: String,
    pub focus: String,
    pub duration_minutes: u32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningDrill {
    pub opening_name: String,
    pub color: String,
    pub focus: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndgameExercise {
    pub exercise_type: String,
    pub description: String,
    pub position_fen: Option<String>,
}

pub struct CurriculumInput<'a> {
    pub user_profile: &'a super::UserProfile,
    pub recent_games: Vec<crate::Move>, // simplified; real impl uses GameRecord
    pub requested_focus: Option<String>,
}

/// Placeholder stub.
pub fn generate_stub(_input: CurriculumInput) -> StudyPlan {
    StudyPlan {
        weekly_sessions: vec![
            StudySession {
                day: "Monday".to_string(),
                focus: "Tactics".to_string(),
                duration_minutes: 45,
                description: "Solve 10 tactical puzzles at your rating level.".to_string(),
            },
            StudySession {
                day: "Wednesday".to_string(),
                focus: "Endgame Technique".to_string(),
                duration_minutes: 30,
                description: "Practice Lucena and Philidor positions.".to_string(),
            },
            StudySession {
                day: "Friday".to_string(),
                focus: "Opening Repertoire".to_string(),
                duration_minutes: 40,
                description: "Review main lines in your repertoire.".to_string(),
            },
        ],
        opening_drills: Vec::new(),
        endgame_exercises: Vec::new(),
        tactical_puzzle_theme: "forks_and_pins".to_string(),
        rationale: "Based on your profile, focusing on tactical accuracy and endgame conversion will yield the most improvement. (stub)".to_string(),
    }
}
