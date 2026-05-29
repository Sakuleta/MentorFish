// ─── Agent Executor ───
//
// Wires each agent to the InferenceClient with structured prompts.
// Stateless functions: receive typed input, return typed output.

use crate::agents::curriculum::StudyPlan;
use crate::agents::memory::ProfileDelta;
use crate::agents::pedagogical::{ExplanationDepth, FinalExplanation, LayerContent, Persona};
use crate::agents::strategic::StrategicSummary;
use crate::agents::tactical::TacticalSummary;
use crate::agents::theory::TheoryOutput;
use crate::agents::{ChatHistoryEntry, RetrievalBundle, UserProfile};
use crate::engine::EngineOutput;
use crate::features::FeatureBundle;
use crate::inference::{
    InferenceClient, InferenceOptions, Message, MessageRole, ModelId, StreamingTokenCallback,
};
use futures::StreamExt;

/// Strip optional markdown code fences (```json ... ```) around a JSON string.
fn strip_json_fences(s: &str) -> &str {
    let s = s.trim();
    let s = s
        .strip_prefix("```json")
        .or_else(|| s.strip_prefix("```"))
        .unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    s.trim()
}

/// Execute the Tactical Agent (no LLM — pure computation).
pub fn run_tactical(engine_output: &EngineOutput, features: &FeatureBundle) -> TacticalSummary {
    crate::agents::tactical::compute(crate::agents::tactical::TacticalInput {
        engine_output,
        features,
    })
}

/// Execute the Strategic Agent via Qwen3-14B with thinking enabled.
pub async fn run_strategic(
    client: &dyn InferenceClient,
    features: &FeatureBundle,
    tactical: &TacticalSummary,
    rag: &RetrievalBundle,
) -> anyhow::Result<StrategicSummary> {
    let prompt = build_strategic_prompt(features, tactical, rag);
    let response = complete_text(client, ModelId::Primary, true, &prompt, None).await?;

    let json_str = strip_json_fences(&response);

    match serde_json::from_str::<StrategicSummary>(json_str) {
        Ok(summary) => Ok(summary),
        Err(e) => {
            log::warn!("Strategic JSON parse failed: {}. Using fallback.", e);
            Ok(StrategicSummary {
                imbalances: vec![],
                plans: vec![crate::agents::strategic::Plan {
                    description: response.clone(),
                    candidate_moves: vec![],
                    rationale: String::new(),
                }],
                pawn_structure: crate::agents::strategic::PawnStructureClassification {
                    structure_type: "unclassified".into(),
                    plans: vec![response],
                },
                key_weaknesses: vec![],
                positional_themes: vec![],
                confidence: 0.5,
            })
        }
    }
}

/// Execute the Theory Agent via Qwen3-14B with thinking enabled.
pub async fn run_theory(
    client: &dyn InferenceClient,
    fen: &str,
    rag: &RetrievalBundle,
) -> anyhow::Result<TheoryOutput> {
    let prompt = build_theory_prompt(fen, rag);
    let response = complete_text(client, ModelId::Primary, true, &prompt, None).await?;

    let json_str = strip_json_fences(&response);

    match serde_json::from_str::<TheoryOutput>(json_str) {
        Ok(output) => Ok(output),
        Err(e) => {
            log::warn!("Theory JSON parse failed: {}. Using fallback.", e);
            Ok(TheoryOutput {
                theoretical_lines: vec![],
                model_games: vec![],
                transpositions: vec![],
                novelty_flag: false,
                historical_context: Some(response),
                confidence: 0.5,
            })
        }
    }
}

/// Execute the Curriculum Agent via Qwen3-14B.
pub async fn run_curriculum(
    client: &dyn InferenceClient,
    profile: &UserProfile,
) -> anyhow::Result<StudyPlan> {
    let prompt = build_curriculum_prompt(profile);
    let response = complete_text(client, ModelId::Primary, false, &prompt, None).await?;

    let json_str = strip_json_fences(&response);

    match serde_json::from_str::<StudyPlan>(json_str) {
        Ok(plan) => Ok(plan),
        Err(e) => {
            log::warn!("Curriculum JSON parse failed: {}. Using fallback.", e);
            Ok(crate::agents::curriculum::generate_stub(
                crate::agents::curriculum::CurriculumInput {
                    user_profile: profile,
                    recent_games: vec![],
                    requested_focus: None,
                },
            ))
        }
    }
}

/// Execute the Memory Agent (no LLM — pure computation over game data).
///
/// Computes EMA-based dimension updates from tactical and strategic analysis.
/// Generates weakness flags for recurring tactical patterns.
pub fn run_memory(tactical: &TacticalSummary, strategic: &StrategicSummary) -> ProfileDelta {
    use crate::agents::memory::{ProfileDimension, DimensionUpdate, WeaknessFlag};

    let mut dimension_updates = Vec::new();
    let mut new_weakness_flags = Vec::new();

    // ── Tactical Accuracy ──
    // Score: 1.0 - (blunders * 0.2), clamped to [0.1, 1.0]
    let blunder_count = tactical.blunders.len() as f64;
    let tactical_game_value = (1.0 - blunder_count * 0.2).clamp(0.1, 1.0);
    dimension_updates.push(DimensionUpdate {
        dimension: ProfileDimension::TacticalAccuracy,
        game_value: tactical_game_value,
        new_value: 0.5, // Will be computed by EMA in caller
        sample_count: 1,
        confidence: if tactical.blunders.is_empty() { 0.1 } else { 0.05 },
    });

    // ── Positional Accuracy ──
    // Score based on strategic confidence and imbalance count
    let strategic_game_value = if strategic.imbalances.is_empty() {
        0.6 // No imbalances found — neutral
    } else {
        // More imbalances = more complex position = lower positional score
        (0.8 - strategic.imbalances.len() as f64 * 0.05).clamp(0.3, 1.0)
    };
    dimension_updates.push(DimensionUpdate {
        dimension: ProfileDimension::PositionalAccuracy,
        game_value: strategic_game_value,
        new_value: 0.5,
        sample_count: 1,
        confidence: strategic.confidence,
    });

    // ── Opening Knowledge ──
    // Score: 1.0 if theory agent found lines, 0.5 if not evaluated
    let opening_game_value = if strategic.confidence > 0.5 { 0.7 } else { 0.5 };
    dimension_updates.push(DimensionUpdate {
        dimension: ProfileDimension::OpeningKnowledge,
        game_value: opening_game_value,
        new_value: 0.5,
        sample_count: 1,
        confidence: strategic.confidence,
    });

    // ── Generate Weakness Flags ──
    // Create flags for recurring tactical patterns (blunders with specific themes)
    for blunder in &tactical.blunders {
        let pattern_name = if blunder.eval_swing_cp.abs() >= 300 {
            "severe_blunder".to_string()
        } else if blunder.eval_swing_cp.abs() >= 150 {
            "moderate_blunder".to_string()
        } else {
            continue; // Skip minor inaccuracies
        };

        new_weakness_flags.push(WeaknessFlag {
            pattern_name,
            description: format!(
                "Eval swing of {} cp: {} ({})",
                blunder.eval_swing_cp, blunder.uci_move, blunder.description
            ),
            example_fen: Some(blunder.position_fen.clone()),
        });
    }

    // ── Eval Swing Analysis ──
    // Large eval swings indicate time management or calculation issues
    let max_swing = tactical.eval_swings.iter().map(|s| s.swing_cp.abs()).max().unwrap_or(0);
    if max_swing >= 200 {
        dimension_updates.push(DimensionUpdate {
            dimension: ProfileDimension::TimeManagement,
            game_value: 0.4, // Poor time management indicator
            new_value: 0.5,
            sample_count: 1,
            confidence: 0.05,
        });
    }

    ProfileDelta {
        dimension_updates,
        new_weakness_flags,
        opening_repertoire_events: vec![],
    }
}

/// Execute the Conversational Chat agent — multi-turn dialogue with history.
///
/// Produces plain streaming text (not JSON). Uses the Fast model (Qwen3-8B)
/// without thinking enabled for low-latency responses.
///
/// The `history` parameter contains all messages in this conversation turn,
/// including the current user message as the last entry.
pub async fn run_conversational_chat(
    client: &dyn InferenceClient,
    tactical: Option<&TacticalSummary>,
    strategic: Option<&StrategicSummary>,
    persona: &Persona,
    history: &[ChatHistoryEntry],
    on_token: Option<StreamingTokenCallback>,
) -> anyhow::Result<String> {
    let mut messages: Vec<Message> = Vec::new();

    // 1. System prompt from persona (with optional position analysis context)
    messages.push(Message {
        role: MessageRole::System,
        content: build_conversational_system_prompt(persona, tactical, strategic),
    });

    // 2. Conversation history (limit to last 20 entries to stay within context window)
    let history_entries: Vec<&ChatHistoryEntry> = if history.len() > 20 {
        history.iter().rev().take(20).rev().collect()
    } else {
        history.iter().collect()
    };

    for entry in history_entries {
        let role = match entry.role.as_str() {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            _ => MessageRole::System,
        };
        messages.push(Message {
            role,
            content: entry.content.clone(),
        });
    }

    // 3. Use Fast model with no thinking for low-latency conversational responses
    let options = InferenceOptions {
        temperature: 0.7,
        max_tokens: 2048,
        enable_thinking: false,
        system_prompt: None, // already included as first message
    };

    let mut stream = client.complete(ModelId::Fast, messages, options).await?;
    let mut result = String::new();

    while let Some(token) = stream.next().await {
        if !token.content.is_empty() {
            result.push_str(&token.content);
            if let Some(ref cb) = on_token {
                cb(&token.content, token.is_final);
            }
        }
        if token.is_final {
            break;
        }
    }

    Ok(result)
}

/// Build a system prompt for conversational chat that includes
/// optional position-analysis context when the user provided a FEN.
fn build_conversational_system_prompt(
    persona: &Persona,
    tactical: Option<&TacticalSummary>,
    strategic: Option<&StrategicSummary>,
) -> String {
    let mut prompt = persona.system_prompt().to_string();

    prompt.push_str("\n\nYou are a chess coach engaged in a conversation with a student. ");
    prompt.push_str("Answer questions clearly and helpfully. ");
    prompt.push_str(
        "If the student asks about a specific position, refer to the analysis context provided. ",
    );
    prompt.push_str("If no position is provided, answer general chess questions knowledgeably. ");
    prompt.push_str("Keep responses concise but thorough. Use chess terminology appropriately.");

    // If engine analysis was performed (FEN was provided), include it as context
    if let Some(t) = tactical {
        let has_data =
            !t.blunders.is_empty() || !t.missed_tactics.is_empty() || !t.eval_swings.is_empty();

        if has_data {
            prompt.push_str("\n\n--- POSITION ANALYSIS CONTEXT ---");
            prompt.push_str(&format!("\n- Blunders detected: {}", t.blunders.len()));
            prompt.push_str(&format!("\n- Missed tactics: {}", t.missed_tactics.len()));
            if !t.eval_swings.is_empty() {
                let swings: Vec<String> = t
                    .eval_swings
                    .iter()
                    .take(3)
                    .map(|s| format!("move {}: {}cp", s.move_number, s.swing_cp))
                    .collect();
                prompt.push_str(&format!("\n- Key eval swings: {}", swings.join(", ")));
            }
            if !t.forcing_sequences.is_empty() {
                prompt.push_str(&format!(
                    "\n- Forcing sequences found: {}",
                    t.forcing_sequences.len()
                ));
            }
        }

        if let Some(s) = strategic {
            if !s.plans.is_empty() || !s.positional_themes.is_empty() {
                prompt.push_str(&format!(
                    "\n- Strategic plans: {}",
                    s.plans
                        .iter()
                        .map(|p| p.description.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
                let themes: Vec<&str> = s
                    .positional_themes
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect();
                if !themes.is_empty() {
                    prompt.push_str(&format!("\n- Positional themes: {}", themes.join(", ")));
                }
            }
        }

        prompt.push_str(
            "\n\nUse this analysis to ground your responses when the student asks about the position.",
        );
    }

    prompt
}

/// Execute the Pedagogical Agent — assembles the final explanation.
#[allow(clippy::too_many_arguments)]
pub async fn run_pedagogical(
    client: &dyn InferenceClient,
    tactical: Option<&TacticalSummary>,
    strategic: Option<&StrategicSummary>,
    theory: Option<&TheoryOutput>,
    profile: &UserProfile,
    persona: &Persona,
    depth: ExplanationDepth,
    confidence_flags: &[String],
    on_token: Option<StreamingTokenCallback>,
) -> anyhow::Result<FinalExplanation> {
    let prompt = build_pedagogical_prompt(
        tactical,
        strategic,
        theory,
        profile,
        persona,
        depth,
        confidence_flags,
    );

    let use_thinking = matches!(depth, ExplanationDepth::Standard | ExplanationDepth::Full);
    let model = if use_thinking {
        ModelId::Primary
    } else {
        ModelId::Fast
    };

    let response = complete_text(client, model, use_thinking, &prompt, on_token).await?;

    let json_str = strip_json_fences(&response);

    match serde_json::from_str::<FinalExplanation>(json_str) {
        Ok(explanation) => Ok(explanation),
        Err(e) => {
            log::warn!("Pedagogical JSON parse failed: {}. Using fallback.", e);
            Ok(FinalExplanation {
                text: response.clone(),
                layer_breakdown: vec![
                    LayerContent {
                        layer: 1,
                        layer_name: "Move Truth".into(),
                        content: tactical
                            .map(|t| {
                                format!(
                                    "{} blunders, {} missed tactics",
                                    t.blunders.len(),
                                    t.missed_tactics.len()
                                )
                            })
                            .unwrap_or_else(|| "N/A".into()),
                        confidence: 1.0,
                    },
                    LayerContent {
                        layer: 2,
                        layer_name: "Tactical Logic".into(),
                        content: response,
                        confidence: 0.8,
                    },
                ],
                confidence: 0.7,
                low_confidence_note: if confidence_flags.contains(&"low_confidence".into()) {
                    Some(
                        "Note: Limited reference material was found for this position type.".into(),
                    )
                } else {
                    None
                },
            })
        }
    }
}

// ─── Prompt Builders ───

fn build_strategic_prompt(
    features: &FeatureBundle,
    tactical: &TacticalSummary,
    rag: &RetrievalBundle,
) -> String {
    let rag_text: String = rag
        .chunks
        .iter()
        .take(3)
        .map(|c| format!("- {}: {}", c.source, c.content))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are a chess strategic analyst. Analyze the position using ONLY the facts below.

POSITION FACTS:
- Evaluation: {} centipawns
- Tactical features: {} blunders, {} missed tactics
- Positional features: {:?}

RELEVANT KNOWLEDGE:
{}

TASK: Identify strategic themes, imbalances, pawn structure classification, key weaknesses, and plans.
Respond ONLY with a JSON object matching this exact schema (no markdown, no extra text):
{{
  "imbalances": [{{"category": "...", "advantage_color": "...", "description": "..."}}],
  "plans": [{{"description": "...", "candidate_moves": ["e2e4"], "rationale": "..."}}],
  "pawn_structure": {{"structure_type": "...", "plans": ["..."]}},
  "key_weaknesses": ["e5"],
  "positional_themes": [{{"name": "...", "description": "..."}}],
  "confidence": 0.7
}}
"#,
        features.eval_cp,
        tactical.blunders.len(),
        tactical.missed_tactics.len(),
        features.positional.iter().take(5).collect::<Vec<_>>(),
        rag_text,
    )
}

fn build_theory_prompt(fen: &str, rag: &RetrievalBundle) -> String {
    let rag_text: String = rag
        .chunks
        .iter()
        .take(3)
        .map(|c| format!("- {}", c.content))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are a chess opening theorist. Analyze this position.

FEN: {}

RELEVANT KNOWLEDGE:
{}

TASK: Identify opening lines, model games, transpositions, and whether this is a theoretical novelty.
Respond ONLY with a JSON object matching this exact schema (no markdown, no extra text):
{{
  "theoretical_lines": [{{"uci_moves": ["e2e4", "e7e5"], "san_moves": ["e4", "e5"], "evaluation": "+0.3", "popularity": 0.5}}],
  "model_games": [{{"white": "Carlsen", "black": "Nakamura", "year": 2020, "event": "Tata Steel", "result": "1-0", "relevance": "Same pawn structure"}}],
  "transpositions": [{{"from_line": "Sicilian Najdorf", "to_line": "English Attack", "description": "Can transpose via 6.Be3"}}],
  "novelty_flag": false,
  "historical_context": "This position arises from the Ruy Lopez...",
  "confidence": 0.6
}}
"#,
        fen, rag_text,
    )
}

fn build_curriculum_prompt(profile: &UserProfile) -> String {
    format!(
        r#"You are a chess curriculum designer. Create a study plan for a player with:
- Tactical accuracy: {:.1}%
- Positional accuracy: {:.1}%
- Opening knowledge: {:.1}%
- Endgame technique: {:.1}%

TASK: Suggest 3 focused study sessions for the week, targeting the weakest areas.
Respond ONLY with a JSON object matching this exact schema (no markdown, no extra text):
{{
  "weekly_sessions": [{{"day": "Monday", "focus": "Tactics", "duration_minutes": 45, "description": "Solve 10 puzzles"}}],
  "opening_drills": [{{"opening_name": "Sicilian Defense", "color": "white", "focus": "Open Sicilian main lines"}}],
  "endgame_exercises": [{{"exercise_type": "rook_endgame", "description": "Practice Lucena position", "position_fen": null}}],
  "tactical_puzzle_theme": "forks_and_pins",
  "rationale": "Focus on tactical accuracy because..."
}}
"#,
        profile.tactical_accuracy * 100.0,
        profile.positional_accuracy * 100.0,
        profile.opening_knowledge * 100.0,
        profile.endgame_technique * 100.0,
    )
}

fn build_pedagogical_prompt(
    tactical: Option<&TacticalSummary>,
    strategic: Option<&StrategicSummary>,
    theory: Option<&TheoryOutput>,
    profile: &UserProfile,
    persona: &Persona,
    depth: ExplanationDepth,
    confidence_flags: &[String],
) -> String {
    let has_low_confidence = confidence_flags.contains(&"low_confidence".into());
    let confidence_note = if has_low_confidence {
        "\nNOTE: Some inputs have low confidence. Mention this in your response."
    } else {
        ""
    };

    let depth_instruction = match depth {
        ExplanationDepth::Brief => "Be very concise — 2-3 sentences max.",
        ExplanationDepth::Standard => {
            "Provide a balanced analysis with key tactical and strategic points."
        }
        ExplanationDepth::Full => "Give a thorough, layered analysis covering all aspects.",
    };

    let tactical_text = tactical
        .map(|t| {
            format!(
                "Blunders: {}, Missed tactics: {}",
                t.blunders.len(),
                t.missed_tactics.len()
            )
        })
        .unwrap_or_else(|| "No tactical data".into());

    let strategic_text = strategic
        .map(|s| {
            format!(
                "Plans: {:?}, Themes: {:?}",
                s.plans.len(),
                s.positional_themes.len()
            )
        })
        .unwrap_or_else(|| "No strategic data".into());

    let theory_text = theory
        .and_then(|t| t.historical_context.as_ref())
        .map(|c| c.as_str())
        .unwrap_or("No theory data");

    format!(
        r#"{persona_prompt}

INPUT FACTS:
- Tactical: {tactical}
- Strategic: {strategic}
- Theory context: {theory}
- User profile: tactical={t_acc:.1}%, positional={p_acc:.1}%
{confidence}

TASK: Explain this position. {depth_instruction}
Structure: Hook -> What happened -> Why it matters -> Principle.

Respond ONLY with a JSON object matching this exact schema (no markdown, no extra text):
{{
  "text": "Your full explanation text here...",
  "layer_breakdown": [
    {{"layer": 1, "layer_name": "Move Truth", "content": "Engine evaluation summary", "confidence": 1.0}},
    {{"layer": 2, "layer_name": "Tactical Logic", "content": "Concrete tactics", "confidence": 0.8}},
    {{"layer": 3, "layer_name": "Strategic Meaning", "content": "Positional principles", "confidence": 0.6}}
  ],
  "confidence": 0.7,
  "low_confidence_note": null
}}
"#,
        persona_prompt = persona.system_prompt(),
        tactical = tactical_text,
        strategic = strategic_text,
        theory = theory_text,
        t_acc = profile.tactical_accuracy * 100.0,
        p_acc = profile.positional_accuracy * 100.0,
        confidence = confidence_note,
        depth_instruction = depth_instruction,
    )
}

// ─── Inference Helpers ───

/// Send a completion request and collect the full response text.
///
/// If `on_token` is provided, each streaming token is forwarded before
/// being accumulated, enabling real-time UI updates.
async fn complete_text(
    client: &dyn InferenceClient,
    model: ModelId,
    enable_thinking: bool,
    prompt: &str,
    on_token: Option<StreamingTokenCallback>,
) -> anyhow::Result<String> {
    let messages = vec![Message {
        role: MessageRole::User,
        content: prompt.to_string(),
    }];

    let options = InferenceOptions {
        temperature: if enable_thinking { 0.3 } else { 0.7 },
        max_tokens: 4096,
        enable_thinking,
        system_prompt: Some(
            "You are a chess expert. Be accurate, concise, and pedagogical.".into(),
        ),
    };

    let mut stream = client.complete(model, messages, options).await?;
    let mut result = String::new();

    while let Some(token) = stream.next().await {
        if !token.content.is_empty() {
            result.push_str(&token.content);
            if let Some(ref cb) = on_token {
                cb(&token.content, token.is_final);
            }
        }
        if token.is_final {
            break;
        }
    }

    Ok(result)
}
